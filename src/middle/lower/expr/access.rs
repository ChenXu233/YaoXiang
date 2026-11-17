//! 变量与字段访问表达式（Var / FieldAccess / 模块导出作值）
//!
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast::Expr;
use crate::frontend::module::symbol::DefKind;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

impl AstToIrGenerator {
    /// **模块导出作值**的判据与发射点——值位置只有这一份实现（#479 / #493 / #492）。
    ///
    /// 三条别名/成员路径（`use lib.{f}` 的别名、`lib.f` 的命名空间成员、
    /// `io.println` 的模块变量成员）都经此处，按导出**实现来源**分流：
    ///
    /// - 函数代码（用户/vendor 导出 + 纯 yx 实现的 std 导出）→ `MakeClosure`：
    ///   函数值是一等值。零参 `Call` 会把被调体的返回值装进变量（静默错值、
    ///   exit 0）；泛型纯 yx 导出更会直接运行期 E6007。
    /// - 原生函数导出 → **原生函数值**（`ConstValue::NativeFn`）：原生函数没有
    ///   IR 函数体，`MakeClosure` 按名查函数表必然落空；经 `CallDyn` 间接调用时
    ///   按名分发到 FFI 注册表（#492）。
    /// - 其余 native 名（native 常量，如 `std.math.PI`）→ 零参调用：既有成功
    ///   形态，行为逐字节不变。
    ///
    /// 返回 `false` 表示该限定名不属上述三类，调用方按自己的既有分支继续。
    fn emit_module_export_value(
        &mut self,
        qualified: &str,
        result_reg: usize,
        span: Span,
        instructions: &mut Vec<Instruction>,
    ) -> bool {
        if self.qualified_export_is_code(qualified) {
            instructions.push(Instruction::MakeClosure {
                dst: Operand::Local(result_reg),
                func: qualified.to_string(),
                env: vec![],
                def: None,
                span,
            });
            return true;
        }
        if self.registry.is_native_fn_export(qualified) {
            instructions.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Const(ConstValue::NativeFn(qualified.to_string())),
                span,
            });
            return true;
        }
        if self.registry.is_native_name(qualified) {
            instructions.push(Instruction::Call {
                dst: Some(Operand::Local(result_reg)),
                func: Operand::Const(ConstValue::String(qualified.to_string())),
                args: vec![],
                span,
                def: None,
            });
            return true;
        }
        false
    }

    pub(super) fn generate_var_expr_ir(
        &mut self,
        var_name: &str,
        var_span: &Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), Diagnostic> {
        // #254：闭包体内——先查捕获表（外层变量经 env 捕获，LoadUpvalue 读）
        if let Some(&env_idx) = self.closure_captures.get(var_name) {
            instructions.push(Instruction::LoadUpvalue {
                dst: Operand::Local(result_reg),
                upvalue_idx: env_idx,
                span: self.cur_span,
            });
        } else if let Some(local_idx) = self.lookup_local(var_name) {
            // 局部变量：直接加载
            instructions.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Local(local_idx),
                span: self.cur_span,
            });
        } else if let Some(global_idx) = self.lookup_global(var_name) {
            // 全局变量（T2）：直接从全局槽位加载，不再调用访问器函数
            instructions.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Global(global_idx),
                span: *var_span,
            });
        } else if let Some(slot) = self.use_alias_value_slot(var_name) {
            // T5：`use lib.{value}`（含 `as` 别名）引入的跨文件顶层值 → 读全局槽位。
            // 别名 → 限定名 → 槽位这两表联查归 `use_alias_value_slot`——调用形态
            // 判据（`is_static_fn_name`）读同一实现，两条路径不会各猜一个答案。
            instructions.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Global(slot),
                span: *var_span,
            });
        } else if let Some(alias) = self.use_aliases.get(var_name).cloned() {
            // 三条实现来源的判据与发射点只有一份（emit_module_export_value）：
            // 代码 → MakeClosure、原生函数 → 原生函数值、native 常量 → 零参调用。
            // 别名未命中上述三类时按既有零参调用兜底（与修复前同形态）。
            if !self.emit_module_export_value(&alias, result_reg, *var_span, instructions) {
                instructions.push(Instruction::Call {
                    dst: Some(Operand::Local(result_reg)),
                    func: Operand::Const(ConstValue::String(alias)),
                    args: vec![],
                    span: *var_span,
                    def: None,
                });
            }
        } else if matches!(
            var_name,
            "Int" | "Float" | "Bool" | "String" | "Char" | "Bytes" | "Type" | "Void" | "Never"
        ) {
            // 内置类型名作为类型实参（如 SafeArray(Int, 3)）：类型宇宙的值，
            // 运行时无表示，加载 Void 占位（类型参数在编译期已被消费）
            instructions.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Const(ConstValue::Void),
                span: self.cur_span,
            });
        } else if self.binding_is_function(var_name) {
            // #348：名字的类型是 Fn ⇒ 它就是函数值。运行时函数值的唯一表示是
            // 闭包，故物化为 MakeClosure（env 为空：本处只处理顶层/自由函数名，
            // 捕获变量已由上方 closure_captures 处理）。
            //
            // 此前落到下方兜底 → E3006（"变量无法解析"），使函数名不能
            // 作一等值使用：`io.println(f)` / `g(f)` / `x = f` 全部失败。
            instructions.push(Instruction::MakeClosure {
                dst: Operand::Local(result_reg),
                func: var_name.to_string(),
                env: vec![],
                def: None,
                span: self.cur_span,
            });
        } else {
            // #271 #3：未解析变量 → 硬错误（#254 spawn 捕获已落地，不再需要静默 Load 0 兜底）。
            // 走到这里说明 typecheck 漏网，属编译器内部一致性问题。
            return Err(ErrorCodeDefinition::unresolved_variable(var_name)
                .at(*var_span)
                .build());
        }
        Ok(())
    }

    pub(super) fn generate_field_access_expr_ir(
        &mut self,
        expr: &Expr,
        field: &str,
        span: &Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // #396：命名空间数据访问（`use lib;` 后 `lib.v` / `lib.sub.v`，以及
        // `io.println` 这类 std 子模块成员作值）。此前只有**调用位置**有
        // namespace 降级，表达式位置落到「普通字段访问」→ 生成对 `lib` 的
        // 变量读取 → E3006 内部错误。
        // 降级与 generate_var_expr_ir 的 T5 分支（use lib.{v} 别名）同一数据面：
        // ① 跨文件全局槽位（顶层值绑定，键为限定名）→ Load Global；
        // ② native 命名空间常量（std.math.PI）→ 零参调用；
        // ③ 函数值（`lib.greet` 作一等值）→ MakeClosure（#348 的跨模块镜像）。
        // 头被本文件局部绑定/闭包捕获遮蔽时不进此分支（与变量解析的局部优先序
        // 一致）；①②③全未命中则落回下方既有逻辑——成员存在性已由 typecheck
        // 保证（E1043），落到这里属 typecheck 漏网，按原逻辑响亮失败。
        if let Some(qualified) = self.resolve_namespace_data_access(expr, field) {
            if let Some(&slot) = self.global_slot_layout.get(&qualified) {
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Global(slot),
                    span: *span,
                });
                return Ok(());
            }
            // ②③ 同一判据（emit_module_export_value）：native 常量 → 零参调用；
            // 纯 yx 实现的 std 导出 → MakeClosure；原生函数 → 原生函数值（#492）。
            if self.emit_module_export_value(&qualified, result_reg, *span, instructions) {
                return Ok(());
            }
            if let Some(def) = self.registry.symbols().def(&qualified) {
                if matches!(
                    self.registry.symbols().kind(def),
                    DefKind::Function | DefKind::Method
                ) {
                    instructions.push(Instruction::MakeClosure {
                        dst: Operand::Local(result_reg),
                        func: qualified,
                        env: vec![],
                        def: None,
                        span: *span,
                    });
                    return Ok(());
                }
            }
        }

        // 首先检查是否是模块变量的字段访问（如 io.println）
        // io 是通过 use std.{io} 导入的模块变量
        if let Expr::Var(module_name, _) = expr {
            if let Some(full_path) = {
                let reg = &self.registry;
                if reg.is_std_submodule(module_name) {
                    let path = format!("std.{}", field);
                    if reg.is_native_name(&path) {
                        Some(path)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } {
                // 模块变量成员作值（`io.println` / `json.parse`）：与 `use lib.{f}`
                // 别名同一判据（emit_module_export_value）——代码 → MakeClosure、
                // 原生函数 → 原生函数值（#492）、native 常量 → 零参调用。此前一律
                // 生成零参调用：原生函数作值时 stdout 空、exit 0、零诊断。
                if !self.emit_module_export_value(&full_path, result_reg, *span, instructions) {
                    instructions.push(Instruction::Call {
                        dst: Some(Operand::Local(result_reg)),
                        func: Operand::Const(ConstValue::String(full_path)),
                        args: vec![],
                        span: *span,
                        def: None,
                    });
                }
            } else if field == "length"
                && matches!(
                    self.get_expr_mono_type(expr),
                    // 剥掉 Ref 层：`&Vec(A)` 的 `.length` 也是长度读取
                    // （RFC-009 §2.8 读透明）。此前未剥离，泛型函数的
                    // `&Vec(A)` 形参上读长度会落到「普通字段访问」分支，
                    // 生成 `GetField(0)` 去写元素槽位 0，静默读出错误值。
                    Some(ref t) if {
                        let mut r = t.clone();
                        while let crate::frontend::core::types::mono::MonoType::Ref {
                            inner, ..
                        } = r
                        {
                            r = *inner;
                        }
                        r.is_vec() || r.is_array()
                    }
                )
            {
                // RFC-011 容器命名分层：`Vec(T)` / `Array(T, N)` 的 `.length`。
                // 复用 StringLength 指令（已泛化为通用长度读取），
                // 与 Range 具名字字段同模式：类型层认字段、IR 层脱糖成原语。
                let obj_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                instructions.push(Instruction::StringLength {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(obj_reg),
                    span: *span,
                });
            } else {
                // 普通字段访问
                let obj_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                let field_index = self.resolve_field_index(expr, field).ok_or_else(|| {
                    ErrorCodeDefinition::ir_internal_error(&format!(
                        "无法解析字段索引: '{}'",
                        field
                    ))
                    .at(Self::get_expr_span(expr))
                    .build()
                })?;
                instructions.push(Instruction::LoadField {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(obj_reg),
                    field: field_index,
                    span: *span,
                });
            }
        } else {
            // 提取完整的命名空间路径（如 std.math.PI）
            let full_path = self.resolve_field_path(expr, field);

            // 容器的 `.length` 读取（接收者非简单变量时）。
            //
            // 上面的 `.length` 分支嵌在 `if let Expr::Var(module_name, _) = expr`
            // 内部，只对变量接收者生效；`Vec(Int)().length`（接收者是两层构造调用）
            // 落到下面的普通字段分支，生成 `LoadField(0)`——那是**取元素 0**，
            // 空容器上直接报「field index 0 out of range」。
            if self.is_container_length_field(expr, field) {
                let obj_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                instructions.push(Instruction::StringLength {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(obj_reg),
                    span: *span,
                });
                return Ok(());
            }

            // 检查是否是命名空间常量访问
            if self.registry.is_native_name(&full_path) {
                // 命名空间常量访问：生成零参数函数调用
                instructions.push(Instruction::Call {
                    dst: Some(Operand::Local(result_reg)),
                    func: Operand::Const(ConstValue::String(full_path)),
                    args: vec![],
                    span: *span,
                    def: None,
                });
            } else {
                // 普通字段访问
                let obj_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                let field_index = self.resolve_field_index(expr, field).ok_or_else(|| {
                    ErrorCodeDefinition::ir_internal_error(&format!(
                        "无法解析字段索引: '{}'",
                        field
                    ))
                    .at(Self::get_expr_span(expr))
                    .build()
                })?;
                instructions.push(Instruction::LoadField {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(obj_reg),
                    field: field_index,
                    span: *span,
                });
            }
        }
        Ok(())
    }
}
