//! AST → IR lowering（`middle/lower/`）
//!
//! WBS 7.0.1 的第一批迁入点：`AstToIrGenerator` 的结构定义、构造族与模块级入口
//! （`generate_ir` / `generate_ir_with_context`），以及生成器状态所需的四个结构体。
//!
//! 子模块（WBS 7.0.1–7.0.3 全部迁入；原 `middle/ir/ir_gen.rs` 已删除）：
//! - `scope.rs`：作用域栈、寄存器帧、槽位/符号注册与 `note_*` written-type 族（7.0.1）；
//! - `query.rs`：纯查询族（只读状态/AST/类型的判定与提取，不发 IR、不写槽位）（7.0.1）；
//! - `stmt.rs` / `loops.rs`：语句与循环族（7.0.2）；
//! - `globals.rs` / `module_ir.rs` / `const_eval.rs`：全局定义、模块装配与常量求值族（7.0.2）；
//! - `curry.rs`：柯里化族（7.0.3）；
//! - `expr/`：表达式 lowering（入口/总派发 + 按方法簇分的子模块，7.0.3）。
//!
//! 结构体字段与方法的可见性（`pub(super)` / `pub(in crate::lower)` /
//! `pub(in crate::middle)`）是跨文件拆分的机械需要：这些字段与方法原先私有于单个
//! impl 文件，拆成 lower 子模块后仍要跨文件读写同一份生成器状态。

mod const_eval;
mod curry;
mod expr;
mod globals;
mod loops;
mod module_ir;
mod query;
mod scope;
mod stmt;

#[cfg(test)]
mod tests;

use crate::frontend::core::sema::layers::ownership::ReleasePlan;
use crate::frontend::core::sema::types::OverloadResolutions;
use crate::frontend::core::sema::{MonoType, TypeCheckResult};
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{ConstValue, FrameId, FunctionIR, Instruction, LocalSlot, Operand};
use crate::util::diagnostic::Diagnostic;
use crate::util::span::Span;
use std::collections::HashMap;

/// 符号表条目
#[derive(Debug, Clone)]
pub(super) struct SymbolEntry {
    local_idx: usize,
    /// 该变量绑定时**写出**的类型名（`x: TempB = ..` 的注解原文，或构造器调用
    /// `TempB(..)` / `ns.Temp(..)` 写出的名字）。WBS 4.9.8 / issue #465：
    /// `MonoType` 对导入类型只留**定义侧裸名**，两单元的同名类型分不出来；
    /// 「这个名字是从哪个单元带进来的」在本单元唯一有据的是 `use` 面
    /// （[`AstToIrGenerator::imported_type_owner`]）——故把写出名留在符号表里，
    /// 方法派发时据此归因到定义单元。
    written_type: Option<String>,
}

/// AST 到 IR 的生成器
///
/// 将 AST 节点转换为 IR 指令序列。
#[derive(Debug)]
pub struct AstToIrGenerator {
    /// 符号表（用于变量解析）
    pub(super) symbols: Vec<HashMap<String, SymbolEntry>>,
    /// 类型检查结果（包含变量绑定信息）
    pub(super) type_result: Option<Box<TypeCheckResult>>,
    /// 当前正在生成的函数绑定，其声明**返回位**若是 Fn（括号返回 / 闭包包装形态），
    /// 这里放该内层 Fn 的形参（名字, 类型）。供闭包 IR 定型用：parser 已不再往
    /// 实现头填声明类型，闭包自己拿不到外层声明（D54 修复轮 F1）。
    pub(super) closure_decl_params: Vec<(String, MonoType)>,
    /// 下一个临时寄存器编号
    pub(super) next_temp: usize,
    /// 本函数临时寄存器高水位（#393：语句级回收会回滚 next_temp，
    /// total_locals 必须取真高水位，否则最后一条语句的临时槽位被裁掉）
    pub(super) temp_high_water: usize,
    /// 循环态临时地板（#393：for 的 iterator/iterable 等临时跨语句存活，
    /// 语句级回收回滚不得低于此值；enter/exit_loop_targets 维护）
    pub(super) temp_floor: usize,
    /// 局部变量类型追踪（用于错误消息中显示实际类型）
    pub(super) local_var_types: HashMap<String, String>,
    /// 顶层绑定**写出**的类型名（Script 形态的 `t = Temp(1.0)` 是全局槽位，
    /// 与块内局部变量走同一条派发归因链——WBS 4.9.8 / issue #465）。
    /// 键 = 本文件的绑定名（不含模块限定）。
    pub(super) global_written_types: HashMap<String, String>,
    /// FFI 库绑定
    pub(super) ffi_libs: Vec<crate::middle::ir::ir::FfiLibBinding>,
    /// FFI 绑定 — 不透明类型或外部函数
    pub(super) ffi_bindings: Vec<crate::middle::ir::ir::FfiBinding>,
    /// 下一个库 ID
    pub(super) next_lib_id: usize,
    /// 结构体定义映射（类型名 -> 字段列表）
    /// 用于构造器调用时填充默认值
    pub(super) struct_definitions: HashMap<String, Vec<crate::ast::StructField>>,
    /// RFC-010 和类型的类型参数名（载荷类型替换用，见 sum_type_variants）
    pub(super) sum_type_param_names: HashMap<String, Vec<String>>,
    /// 本模块声明的**全部**类型名（含泛型类型定义）。
    ///
    /// 类型声明（含泛型，`E: (T: Type) -> Type = ..`）一律走 TypeDecl 函数路径；
    /// 名字解析需要知道「这是用户自己的类型」，
    /// 否则 `use std.math` 后用户写 `PI: Type = ..; PI(7)` 会被解析成
    /// `std.math.PI`（常量）——调用常量得到 void，静默错值（D10）。
    pub(super) declared_type_names: std::collections::HashSet<String>,
    /// 本单元声明的**顶层函数绑定名**（issue #477 / WBS 3.8.1，用户裁决乙案）。
    ///
    /// 裸名调用解析的用户定义优先：用户在本单元定义了与 std native 短名同名
    /// 的函数（如 `read`），check 面（sema）按作用域查到用户函数、检查全绿；
    /// lower 面若只认「类型名优先」（`declared_type_names`），函数名落到
    /// std 短名映射臂被改写成 native 限定名——check 与 run 对同一调用给出
    /// 不同派发目标（修前实测：native 静默执行给错误值 exit 0，或运行期
    /// E6007）。本集合与 `declared_type_names` 同点扫描填充，让裸名解析在
    /// 类型臂之后先查本单元函数，再落 use_aliases / native / std 短名映射。
    ///
    /// std 嵌入单元（std.test / std.json 等）单独做 IR 生成、各用各的生成器
    /// 实例——集合天然按单元作用域工作，std 模块内部互调命中的仍是 std 自己
    /// 的函数名（与原兜底臂 + `qualify_names` 改写产物同名，零漂移）。
    pub(super) declared_fn_names: std::collections::HashSet<String>,
    /// 类型级命名空间（谓词名可见面）：SPEC §7.1 的「谓词应用的实参位是值不是
    /// 类型」判据要用它——保守假值会让同形不遮蔽失效。
    ///
    /// **不得再用静态 `SemanticProbe` 独立判谓词**（6.9.1 观察项，D62 复核）：
    /// 用户声明的谓词（`Eq` / `SumUpTo` / `IsPositive`）不在内置名表里，静态探针
    /// 答「不是谓词」⇒ 谓词应用的实参位被当成类型引用 ⇒ 真值参数被误登进
    /// `generic_params`（`refined_relation_arg` 的 `f`/`g`、`refined_param_cross_arg`
    /// 的 `h`）。判据源只有这一份声明面扫描。
    ///
    /// **范围由调用方定，生成器不自扫**（F14-A）：多文件入口传 driver 算出的
    /// `scan(全部单元)` 并集，单文件入口传 `scan_one(module)`；构造参数是必需的，
    /// 故「空集 / 窄范围」只能是调用方的显式选择，不会因漏接而静默。
    pub(super) type_level: crate::frontend::core::sema::probe::TypeLevelNamespace,
    /// 类型绑定映射（类型名 -> (方法名 -> BindingInfo)）
    /// 用于方法调用时的参数重排和函数转发（RFC-004）
    pub(super) type_bindings: HashMap<String, HashMap<String, BindingInfo>>,
    /// 嵌套函数列表（在函数体内定义的函数）
    pub(super) nested_functions: Vec<FunctionIR>,
    /// 闭包计数器（用于生成唯一的闭包名称）
    pub(super) closure_counter: usize,
    /// 全局变量表 (name, type, initial_value)。
    /// 索引即全局槽位号（`Operand::Global(idx)`）。`initial_value` 恒为 None——
    /// 初始化改由运行时 `module_init` 序列承载（T2）；字段保留供跨文件登记占位。
    pub(super) global_vars: Vec<(String, MonoType, Option<ConstValue>)>,
    /// 跨文件全局布局（T5）：限定名 → 绝对槽位号。
    /// 多文件模式下由编排器预先分配（各文件共享），使引用方与定义方
    /// 用同一索引；单文件模式下为空，索引即 `global_vars` 下标。
    pub(super) global_slot_layout: HashMap<String, usize>,
    /// 本文件顶层绑定的槽位起始号（多文件模式由编排器分配；单文件恒为 0）。
    pub(super) global_slot_base: usize,
    /// 模块初始化序列（T2）：顶层绑定的求值 + 写入全局槽位的指令。
    /// 按源码顺序生成（T3 改为依赖拓扑序），由运行时入口先执行。
    pub(super) module_init: Vec<Instruction>,
    /// T3：顶层值绑定的待定初始化（拓扑排序前）。
    /// 按源码顺序收集，在 `generate_module_ir` 尾部按依赖关系重排后
    /// 写入 `module_init`——前向引用（`b = a + 1` 且 `a` 在后）需要 `a` 先初始化。
    pub(super) pending_inits: Vec<PendingInit>,
    /// T4：顶层可执行语句（Script 模式），按书写顺序。
    ///
    /// 与绑定初始化分两段：绑定是**声明**（先全部就绪，拓扑序），
    /// 语句是**程序主体**（后执行，源码序）。若不分开，语句会立即入队
    /// 而绑定被推迟到末尾排序，导致 `io.println(v)` 先于 `v` 的初始化执行。
    pub(super) module_stmts: Vec<Instruction>,
    /// 顶层已声明的绑定名（首次出现在 `name = ...` 的）。
    ///
    /// 用于区分**声明**与**重赋值**：Script 模式下顶层语句是程序主体，
    /// `i = i + 1` 是赋值而非重声明。旧实现把两者混为一谈，
    /// 导致重赋值被当成第二份初始化 → 自引用 → 误报 E3019。
    pub(super) declared_globals: std::collections::HashSet<String>,
    /// 本模块**函数**的声明期形参名（函数名 → 按声明序的参数名）。
    ///
    /// `FunctionIR.params` 只有类型没有名字，而命名参数调用（`add(b = 5, a = 3)`）
    /// 必须按**名字**把实参重排到声明序。struct 构造器有 `struct_definitions`
    /// 提供字段名，普通函数原先没有对应表——于是 `named_args` 被静默丢弃，
    /// 实参一个不进，运行期报 e6007 类型不符。此表补上那个缺口。
    ///
    /// curry 形态（`f: (A: Type) -> (x: A) -> R`）登记的是**最后一层**的形参名
    /// ——调用点传的就是最后那层。
    pub(super) fn_param_names: HashMap<String, Vec<String>>,
    /// 约束变量的具体类型映射（接口直接赋值优化）
    /// 当 `d: Drawable = Circle(1)` 时，记录 d -> "Circle"（具体类型名）
    /// 用于方法调用时选择直接调用而非 vtable 查找
    pub(super) constraint_var_concrete_types: HashMap<String, String>,
    /// RFC-004: 匿名函数绑定生成的独立 FunctionIR 列表
    pub(super) anon_function_irs: Vec<FunctionIR>,
    /// NLL 精确释放计划（所有权检查器产出；RFC-039 D20 / WBS 4.5.1）。
    ///
    /// 键 = 释放边界所在语句的 PlanId——本生成器在语句边界经
    /// [`ReleasePlan::consume`] 取条目（取即记账），故计划本体同时携带
    /// 「产出」与「已消费」两侧事实：结算（义务账本 W1090）读的是同一份账。
    pub(super) release_plan: ReleasePlan,
    /// 待捕获的环境变量（由 spawn for 等设置，供下一个 Expr::Lambda 使用）
    /// 在生成闭包函数体时，这些变量的当前寄存器值会被捕获到闭包环境中。
    pub(super) pending_env_vars: Vec<Operand>,
    /// #254：待捕获变量名（与 pending_env_vars 一一对应，供闭包体 Var 解析）
    pub(super) pending_env_names: Vec<String>,
    /// #254：当前闭包体的捕获表（变量名 → env 槽位索引）。
    /// 生成闭包体期间设置，闭包体内 Var 命中则生成 LoadUpvalue。
    pub(super) closure_captures: HashMap<String, usize>,
    /// RFC-029 #243：用户模块命名空间变量（别名 → 模块限定键）。
    /// 由 `use lib` / `use lib as l` 等整体导入填充，使 `lib.helper()` 在 IR 生成
    /// 阶段被识别为命名空间调用并解析为限定名 `lib.helper`（与 `qualify_module_ir`
    /// 产出的函数定义名天然契合）。std 命名空间走原有 `is_std_submodule` 分支，不在此表。
    pub(super) user_namespaces: HashMap<String, String>,
    /// 本文件 use 导入的本地名 → 限定名（含 #245 条目别名）。
    /// 生成期解析调用名时优先于 registry 的全局短名表（文件级精确）。
    pub(super) use_aliases: HashMap<String, String>,
    /// 本文件持有的模块注册表（std + 用户模块），取代生成期多处 `with_std()` 重建。
    pub(super) registry: ModuleRegistry,
    /// 本文件的模块限定键（多文件模式 Some → 启用限定；单文件 None → 不限定）。
    pub(super) module_key: Option<String>,
    /// Script/Bin 语义标志（E3023 的开关；3.4.8 ③ 回归修复与 `module_key` 解耦）：
    /// `true` = Script（无 manifest：顶层语句即程序主体、顶层同名重绑定后者生效）；
    /// `false` = Bin（顶层只允许定义，可执行语句报 E3023）。默认跟随 `module_key`
    ///（None = Script）；调用方用 [`Self::with_script_mode`] 覆盖——rootless check
    /// 需要「按单元键限定名 + Script 语义」这一组合：限定名是 Linking 合并无冲突的
    /// 前提，Script 语义是 standalone 的对外契约。
    pub(super) script_mode: bool,
    /// RFC-011a §6: 存在类型包装点（span → 接口名），来自 typecheck 强制点收集。
    /// generate_expr_ir 入口查表命中 → 值生成后包 CreateVariant。
    pub(super) existential_wraps: HashMap<Span, String>,

    /// RFC-011b: 运算符派发点（span → 接收者类型名, 方法名, 取反）。
    /// `==`/`!=` 命中显式 Equal 实例化时，原生比较指令替换为方法调用。
    pub(super) operator_dispatches: HashMap<Span, (String, String, bool)>,
    /// RFC-011b：`?` 表达式的 Try 实现类型名（span 键控）
    pub(super) try_expr_impls: HashMap<Span, String>,
    /// RFC-011a §3：方法级重载决议（**调用点 PlanId** 键控 → 混编 IR 名；
    /// RFC-039 D20 / WBS 4.5.2）。消费经 `OverloadResolutions::get`（命中即
    /// 记账），故本字段携带决议表本体（含消费台账），结算读的是同一份账。
    pub(super) overload_resolutions: OverloadResolutions,
    /// 方法定义序（key → 已生成定义数）：混编名的定义侧计数
    pub(super) method_def_ordinals: HashMap<String, usize>,

    /// RFC-010: 和类型变体名表（类型名 → 变体名列表，声明序）。
    /// 变体构造调用（形态检测命中）据此取变体序号。
    pub(super) sum_type_variants: HashMap<String, Vec<(String, Vec<MonoType>)>>,
    /// RFC-011a §6.2: 编译期类型收集——接口 → 实现类型列表（ImplementationProof
    /// 按类型名定序），变体分发与包装定序共用。
    pub(super) interface_variants: HashMap<String, Vec<String>>,
    /// #311: 循环上下文栈——while/for 生成时进出栈。
    /// break 跳到栈顶循环出口（占位 Jmp，出口确定后回填），continue 跳到栈顶条件重判点。
    /// 嵌套函数体生成前保存并清空：闭包体不继承外层循环上下文（typecheck E1102 已拦截，
    /// 此处为层间失联防御）。
    pub(super) loop_stack: Vec<LoopTargets>,
    /// 当前语句的源码 span：语句生成循环在语句边界设置，指令构造处直接读取。
    ///
    /// 这是指令级 span 的唯一来源——每个 `Instruction::X { .., span: self.cur_span }`
    /// 就地捕获当时的语句位置，不存在"先记录后配对"的时序契约。
    /// 嵌套函数体生成前保存、之后恢复（与 next_temp 同款处理）。
    pub(super) cur_span: Span,
    /// 当前正在生成的函数的局部槽位（含源码变量名）。
    ///
    /// 槽位下标即寄存器号；`name` 为 None 表示编译器临时寄存器。
    /// `register_local` 就地把名字写进来，函数体生成结束后交给
    /// `FunctionBody::Code::locals`。
    /// 嵌套函数体生成前保存、之后恢复——**必须与 next_temp 的
    /// 保存/恢复点物理相邻**，否则内层函数的名字会串到外层。
    pub(super) cur_locals: Vec<LocalSlot>,
    /// 当前生成帧号（`LocalSlot::origin` 的**唯一来源**，WBS 2.1.4）。
    ///
    /// 由 `NestedBodyGuard` 的两个构造函数切换、`Drop` 还原；`register_local`
    /// 与槽位补空处把它写进槽位。模块级初始化序列也占一个帧（构造时的初始帧）。
    pub(super) cur_frame: FrameId,
    /// 已发放的帧数（帧号唯一性来源，只增不减）。
    pub(super) next_frame: u32,
}

/// 绑定信息（用于 IR 生成阶段的方法调用转发）
///
/// 按 RFC-004 设计：记录方法绑定到哪个原始函数的哪些参数位置
#[derive(Debug, Clone)]
pub(super) struct BindingInfo {
    /// 原始函数名
    pub(super) function: String,
    /// 绑定位置列表（调用者 obj 填充到这些位置）
    pub(super) positions: Vec<i64>,
}

/// T3：一条待排序的顶层绑定初始化。
///
/// 收集后按依赖关系重排：`init_instructions` 里对其它绑定的全局写入
/// 决定依赖边（“我依赖谁”）。
#[derive(Debug)]
pub(super) struct PendingInit {
    /// 本绑定名（与槽位名同形，多文件下为限定名）
    pub(super) name: String,
    /// 初始化指令（已是“求值 + StoreGlobal”）
    pub(super) instructions: Vec<Instruction>,
    /// 本初始化引用的其它顶层绑定名
    pub(super) deps: Vec<String>,
    /// 声明位置（循环依赖诊断用）
    pub(super) span: Span,
}

/// 循环上下文（#311）：一次 while/for 生成期内的跳转目标记录
#[derive(Debug)]
pub(super) struct LoopTargets {
    /// 条件重判点（continue 的跳转目标），生成时即知
    pub(super) continue_target: usize,
    /// break 占位 Jmp 的指令下标，循环出口确定后回填
    pub(super) break_fixups: Vec<usize>,
    /// 进入循环时的 temp_floor（#393：出口恢复外层地板）
    pub(super) saved_temp_floor: usize,
}

impl AstToIrGenerator {
    /// 创建新的 IR 生成器（带类型信息）。
    ///
    /// `type_level` 是**必需输入**（F14-B）：SPEC §7.1 分类器与泛型名扫描都要问
    /// 「这个名字是不是谓词」，而答案只能在调用方那侧算对——单文件入口是
    /// `TypeLevelNamespace::scan_one(module)`，多文件入口是 driver 的
    /// `scan(全部单元)` 并集。此前构造器把它默认为空集，于是不经
    /// `generate_module_ir` 的入口（`frontend/proof_execution.rs` 直调
    /// `generate_function_ir`）静默拿到旧宽口径——空集不是「足够保守」，它会让
    /// 用户声明的谓词被当普通类型应用（谓词实参位因此被误计为类型引用）。
    pub fn new_with_type_result(
        type_result: &TypeCheckResult,
        registry: ModuleRegistry,
        module_key: Option<String>,
        type_level: crate::frontend::core::sema::probe::TypeLevelNamespace,
    ) -> Self {
        // Script/Bin 语义默认跟随 module_key（None = Script）；调用方可用
        // `with_script_mode` 覆盖（3.4.8 ③：与限定名键解耦）。
        let script_mode = module_key.is_none();
        Self {
            symbols: vec![HashMap::new()],
            type_result: Some(Box::new(type_result.clone())),
            closure_decl_params: Vec::new(),
            next_temp: 0,
            temp_high_water: 0,
            temp_floor: 0,
            local_var_types: HashMap::new(),
            global_written_types: HashMap::new(),
            ffi_libs: Vec::new(),
            ffi_bindings: Vec::new(),
            next_lib_id: 0,
            struct_definitions: HashMap::new(),

            declared_type_names: std::collections::HashSet::new(),
            declared_fn_names: std::collections::HashSet::new(),
            type_level,
            type_bindings: HashMap::new(),
            nested_functions: Vec::new(),
            closure_counter: 0,
            global_vars: Vec::new(),
            global_slot_layout: HashMap::new(),
            global_slot_base: 0,
            module_init: Vec::new(),
            pending_inits: Vec::new(),
            module_stmts: Vec::new(),
            declared_globals: std::collections::HashSet::new(),
            fn_param_names: HashMap::new(),
            constraint_var_concrete_types: HashMap::new(),
            anon_function_irs: Vec::new(),
            release_plan: type_result.release_plan.clone(),
            pending_env_vars: Vec::new(),
            pending_env_names: Vec::new(),
            closure_captures: HashMap::new(),
            user_namespaces: HashMap::new(),
            use_aliases: HashMap::new(),
            registry,
            module_key,
            script_mode,
            existential_wraps: type_result
                .existential_coercions
                .iter()
                .map(|c| (c.span, c.interface.clone()))
                .collect(),
            operator_dispatches: type_result
                .operator_dispatches
                .iter()
                .map(|d| (d.span, (d.type_name.clone(), d.method.clone(), d.negate)))
                .collect(),
            try_expr_impls: type_result
                .try_expr_impls
                .iter()
                .map(|(span, name)| (*span, name.clone()))
                .collect(),
            overload_resolutions: type_result.overload_resolutions.clone(),
            method_def_ordinals: HashMap::new(),
            sum_type_variants: type_result
                .sum_types
                .iter()
                .map(|(n, vs)| {
                    (
                        n.clone(),
                        vs.iter()
                            .map(|v| (v.name.clone(), v.params.clone()))
                            .collect::<Vec<(String, Vec<MonoType>)>>(),
                    )
                })
                .collect(),
            sum_type_param_names: type_result
                .sum_type_param_names
                .iter()
                .map(|(n, ps)| (n.clone(), ps.clone()))
                .collect(),
            interface_variants: {
                let mut map: HashMap<String, Vec<String>> = HashMap::new();
                for proof in &type_result.implementation_proofs {
                    let entry = map.entry(proof.interface_name.clone()).or_default();
                    if !entry.contains(&proof.type_name) {
                        entry.push(proof.type_name.clone());
                    }
                }
                for list in map.values_mut() {
                    list.sort();
                }
                map
            },
            loop_stack: Vec::new(),
            cur_span: Span::dummy(),
            cur_locals: Vec::new(),
            // 帧 0 = 模块级初始化序列；嵌套函数体在 NestedBodyGuard 里领新帧号。
            cur_frame: FrameId(0),
            next_frame: 0,
        }
    }

    /// 覆盖 Script/Bin 语义标志（默认跟随构造时的 `module_key`：None = Script）。
    ///
    /// 3.4.8 ③ 回归修复：`module_key` 此前**一职两用**——既决定是否限定名
    /// （`qualify_names`），又派生 Script/Bin 语义（E3023 开关）。解耦后，无项目根
    /// 的 standalone check 可以既按单元键限定名字（否则 Linking 合并把跨单元同名
    /// 函数判成「模块限定名冲突」），又保持 Script 语义（顶层语句即程序主体）。
    pub fn with_script_mode(
        mut self,
        script_mode: bool,
    ) -> Self {
        self.script_mode = script_mode;
        self
    }
}

/// 这是编译器流程中的关键入口点：
/// 类型检查 → IR 生成 → 代码生成
pub fn generate_ir(
    ast: &crate::ast::Module,
    result: &crate::frontend::core::sema::TypeCheckResult,
) -> Result<crate::middle::ModuleIR, Vec<Diagnostic>> {
    // 单文件模式：仅 std 注册表，module_key 为 None → 不启用模块限定。
    // 谓词名可见面 = 本单元（单文件编译的走查面同样只有这一个单元）。
    let mut generator = AstToIrGenerator::new_with_type_result(
        result,
        ModuleRegistry::with_std(),
        None,
        crate::frontend::core::sema::probe::TypeLevelNamespace::scan_one(ast),
    );
    generator.generate_module_ir(ast)
}

/// RFC-029 多文件编排：在跨文件上下文下为单个文件生成 IR。
///
/// 预注册其他文件的类型布局（`cross_file_types`）与全局变量名（`cross_file_globals`），
/// 使本文件的跨文件字段访问、方法调用与全局引用能正确解析；随后仅对 `ast`
/// 这一个文件生成 IR。各文件产出的 `ModuleIR` 由编排器在 IR 层链接。
#[allow(clippy::too_many_arguments)]
pub fn generate_ir_with_context(
    ast: &crate::ast::Module,
    result: &crate::frontend::core::sema::TypeCheckResult,
    cross_file_types: &[&crate::ast::Module],
    global_layout: &[(String, usize)],
    // 本文件顶层绑定的槽位起始号（编排器分配，保证各文件区间不相交）
    global_slot_base: usize,
    registry: &ModuleRegistry,
    module_key: &str,
    // Script/Bin 语义开关（顶层可执行语句是否放行，E3023）——与限定名键
    // 解耦（3.4.8 ③ 同源判据）：rootless（standalone）多单元形态传 true，
    // 项目（Bin）与嵌入模块传 false
    script_mode: bool,
    // 谓词名可见面（F14-A）：**调用方算出的那一份**（多文件形态 = 全部单元
    // 的 scan 并集，与走查期同一份）。生成器不自扫——按本单元自扫会让
    // 生成期与走查期的范围分叉。
    type_level: &crate::frontend::core::sema::probe::TypeLevelNamespace,
) -> Result<crate::middle::ModuleIR, Vec<Diagnostic>> {
    let mut generator = AstToIrGenerator::new_with_type_result(
        result,
        registry.clone(),
        Some(module_key.to_string()),
        type_level.clone(),
    )
    .with_script_mode(script_mode);
    generator.seed_cross_file_types(cross_file_types);
    generator.seed_cross_file_globals(global_layout);
    generator.set_global_slot_base(global_slot_base);
    generator.generate_module_ir(ast)
}
