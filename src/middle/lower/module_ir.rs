//! 模块装配族（`AstToIrGenerator` 的 ModuleIR 装配）
//!
//! `generate_module_ir` 入口、DefId 分配、限定名/导入别名/调用名改写、跨文件
//! 类型与全局布局登记、跳转目标平移。方法体逐字搬自原 `middle/ir/ir_gen.rs`
//! （WBS 7.0.2），语义未改。

use super::AstToIrGenerator;
use super::BindingInfo;
use crate::ast;
use crate::frontend::core::types::identity::{DefKind, DefPath, Nominal};
use crate::frontend::module::ExportKind;
use crate::middle::ir::ir::{ConstValue, Instruction, ModuleIR, Operand};
use crate::util::diagnostic::Diagnostic;
use std::collections::HashMap;

impl AstToIrGenerator {
    /// 从 AST 模块生成 IR 模块
    pub fn generate_module_ir(
        &mut self,
        module: &ast::Module,
    ) -> Result<ModuleIR, Vec<Diagnostic>> {
        // RFC-029：用户模块命名空间别名（`use lib` / `use lib as l`）由 typecheck 登记
        // （模块解析归 typecheck 所有），IR 生成直接消费，不再自行从 AST 重新推导。
        if let Some(ref tr) = self.type_result {
            self.user_namespaces = tr.module_namespaces.clone();
        }
        // use 导入别名表（含 #245 条目别名）：生成期解析调用名用。
        self.use_aliases = self.build_import_aliases(module);
        // 谓词名可见面**不在生成期扫描**（F14-A）：生成的可见范围必须与走查期
        // （typecheck / registry）**同一份**，因此由调用方经 `new_with_type_result`
        // 注入并在这里原样使用——`self.type_level` 已在构造期就位。
        //
        // 仍在入口扫的是**本文件声明的类型名**：那是逐单元的解析事实（`use std.math`
        // 之后 `PI(7)` 该不该解析成 `std.math.PI`），范围天然是本单元，与谓词的
        // 跨单元可见面不是同一个问题。
        for stmt in &module.items {
            if let ast::StmtKind::TypeDefinition { name, .. } = &stmt.kind {
                self.declared_type_names.insert(name.clone());
            }
        }

        // 同点扫**本文件声明的顶层函数绑定名**（issue #477 / WBS 3.8.1，乙案）：
        // 裸名调用解析要按单元区分「用户定义优先」。判定与
        // `register_global_slots` / Assign 臂（stmt.rs）同源
        // （`block_binding_is_function`——函数绑定进函数表、不占全局槽）。
        // 嵌套函数不在此列：嵌套体调用解析走局部符号表（`lookup_local`），
        // 裸名解析面只服务顶层名。
        for stmt in &module.items {
            if let ast::StmtKind::Assign {
                target,
                type_annotation,
                value,
                ..
            } = &stmt.kind
            {
                if let ast::Expr::Var(name, _) = target.as_ref() {
                    if ast::Expr::block_binding_is_function(
                        type_annotation.as_ref(),
                        value.as_deref(),
                    ) {
                        self.declared_fn_names.insert(name.clone());
                    }
                }
            }
        }

        // T3：先分配所有顶层值绑定的槽位，使前向引用在生成期可解析。
        self.register_global_slots(module);

        let mut functions = Vec::new();
        let mut errors = Vec::new();
        let mut constants = Vec::new();

        for stmt in &module.items {
            match self.generate_stmt_ir(stmt, &mut constants) {
                Ok(Some(func_ir)) => functions.push(func_ir),
                Ok(None) => {}
                Err(e) => errors.push(e),
            }
        }

        // Script 模式：顶层同名重绑定 = **后者生效**——被遮蔽的前一个定义从函数表
        // 里去掉。语义依据是单文件扁平函数表的按序插入（后者覆盖前者）：
        // `f: () -> Int = () => 1 / f = () => 2 / print(f())` 实测输出 2。
        // 留着两份同名定义会让 Linking 的合并校验把「同一模块内的重绑定」误报成
        // 「模块限定名冲突（同一路径 ×2）」并打红整个 check（3.4.8 ③ 回归，文档
        // 示例门禁 10 项）。只在 `script_mode` 下做：Bin 形态的顶层重声明不在本
        // 修复面（其合并校验现状保留），项目形态的 IR 不受影响。
        if self.script_mode && functions.len() > 1 {
            let mut last_index: HashMap<String, usize> = HashMap::new();
            for (idx, func) in functions.iter().enumerate() {
                last_index.insert(func.name.clone(), idx);
            }
            let mut idx = 0usize;
            functions.retain(|func| {
                let keep = last_index.get(&func.name) == Some(&idx);
                idx += 1;
                keep
            });
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // T3：顶层绑定的初始化按**依赖拓扑序**拼接（被依赖者先初始化）。
        // 前向引用（`b = a + 1` 且 `a` 在后）靠它把 `a` 的初始化提到前面。
        // 循环依赖在此报错（带环上的名字），而非运行时取到未初始化值。
        let ordered = match self.topo_sort_pending_inits() {
            Ok(v) => v,
            Err(e) => return Err(vec![e]),
        };
        let ordered_inits: Vec<Vec<Instruction>> = ordered
            .into_iter()
            .map(|i| i.instructions.clone())
            .collect();
        for instrs in ordered_inits {
            let base = self.module_init.len();
            let mut instrs = instrs;
            Self::rebase_jump_targets(&mut instrs, base);
            self.module_init.extend(instrs);
        }
        // T4/T3：绑定初始化（声明，已拓扑排序）之后才跑顶层语句（程序主体，源码序）。
        // 两段不可交换：语句可能读绑定（如 `io.println(v)`），必须先初始化。
        //
        // 跳转重定位：module_stmts 里的 target 是**段内**下标，
        // 前面接上绑定初始化后必须整体平移——否则顶层 while 的回跳会指向函数头。
        let base = self.module_init.len();
        let mut stmts = std::mem::take(&mut self.module_stmts);
        Self::rebase_jump_targets(&mut stmts, base);
        self.module_init.extend(stmts);

        // 添加嵌套函数到模块函数列表
        functions.extend(std::mem::take(&mut self.nested_functions));

        // RFC-004: 添加匿名函数绑定生成的 IR 到模块函数列表
        functions.extend(std::mem::take(&mut self.anon_function_irs));

        let mut ir = ModuleIR {
            globals: self
                .global_vars
                .iter()
                .enumerate()
                .map(|(i, (name, ty, _))| crate::middle::ir::ir::GlobalSlot {
                    name: name.clone(),
                    ty: ty.clone(),
                    index: self.global_slot_base + i,
                })
                .collect(),
            functions,
            init: std::mem::take(&mut self.module_init),
            // 单文件：恒 file_id 0，空表（translator 回退 0）
            init_file_ids: Vec::new(),
            // #368：模块级槽位表随 init 一起交给 codegen。
            // 顶层语句里的具名结构（如 `for i in ..` 的循环变量）在此登记，
            // 取值时已覆盖到最大已注册槽位（register_local 会补空格）。
            init_locals: std::mem::take(&mut self.cur_locals),
            ffi_libs: std::mem::take(&mut self.ffi_libs),
            ffi_bindings: std::mem::take(&mut self.ffi_bindings),
            entry_function: None,
            source_files: Vec::new(),
            function_files: HashMap::new(),
        };
        // RFC-029: 有模块键时限定本文件的顶层函数名与调用引用（定义键、调用名
        // 与 typecheck 的实例化请求名三者同源为 `{module_key}.{name}`）；
        // `module_key` 为 None（单文件管线/无项目根 standalone）时不限定——
        // 但 3.4.8 ③ 起「不限定」只发生在**单单元**程序上：多单元形态由调用方
        // 传单元键（见 `with_script_mode` 的注释），否则合并时跨单元同名函数会
        // 被判成「模块限定名冲突」。Script/Bin 语义与限定名开关已解耦，本闸门
        // 只看 `module_key`。
        if let Some(key) = &self.module_key {
            let aliases = self.build_import_aliases(module);
            // 改名与身份登记同源：`qualify_names` 给出「最终名 → 路径」，
            // `assign_defs` 只按该表登记——投影是路径的像，两处不各算一次。
            let def_paths = Self::qualify_names(&mut ir, key, &aliases);
            self.assign_defs(&mut ir, Some(&def_paths));
        } else {
            self.assign_defs(&mut ir, None);
        }
        // RFC-039 D20 / WBS 4.5.1：消费侧完整跑完一轮（生成成功返回）——
        // 结算据此区分「消费者跑完但漏了条目」（响亮）与「消费者没跑完 / 失败」
        //（不判：失败已按首因报出）。失败路径提前 `return Err`，不会标到这里。
        self.release_plan.consumption().mark_completed();
        // 同点标记重载决议表的消费侧（4.5.2 复用同一台账机制）
        self.overload_resolutions.consumption().mark_completed();
        Ok(ir)
    }

    /// Stage 3 尾部 pass：为所有函数分配 DefId 并解析静态引用。
    ///
    /// 名字此时已是最终形态（多文件已限定、单文件保持裸名）：
    /// - 登记规则：多文件按 `qualify_names` 给出的「最终名 → 路径」表登记——路径与
    ///   投影同源；单文件没有该表（`module_key` 关闭），名字即本单元扁平名，按
    ///   `main` 模块兜底（与 `module_key_or_main` 一致），投影与既有口径逐字相同。
    /// - 表里没有的函数 = `qualify_names` **有意不改名**的名字：先按投影索引复用既有
    ///   DefId（std 等别处已登记的名字，与旧 `intern` 的幂等语义同值）；投影从未登记
    ///   过则说明它是本单元生成期合成的扁平名（curry 内层 `__T.m_l0`），按**无模块
    ///   路径**登记——投影即名字本身，与历史整串登记逐字相同。
    /// - 方法在单文件首次登记时类别退化为 `Function`（多文件已在 register 期以
    ///   `Method` 登记，幂等保留）；下游只查 def 不查 kind，无行为差异。
    /// - 泛型函数经 mono 特化后重命名，def 随之失效——mono 路径保持按名分发（现状），
    ///   codegen 对 def 未命中回退按名，两条路径都正确。
    fn assign_defs(
        &self,
        ir: &mut ModuleIR,
        def_paths: Option<&HashMap<String, DefPath>>,
    ) {
        // intern 规则跟随**限定名开关**（不是 Script/Bin 语义）：未限定名的
        // 程序用 `main.` 前缀兜底，与 `module_key_or_main` 一致。
        let unqualified_names = self.module_key.is_none();
        let mut symbols = self.registry.symbols_mut();
        for func in &mut ir.functions {
            let kind = if func.is_type_decl() {
                DefKind::Type
            } else {
                DefKind::Function
            };
            if let Some(path) = def_paths.and_then(|paths| paths.get(&func.name)) {
                func.def = Some(symbols.intern(path.clone(), kind));
                continue;
            }
            if unqualified_names {
                // 单文件：名字即本单元扁平名，归属 `main`（见函数文档）
                func.def = Some(symbols.intern(DefPath::item("main", &func.name), kind));
                continue;
            }
            // 未改名的名字（见函数文档）：已登记 → 复用；否则按无模块扁平名登记
            func.def = Some(match symbols.def_by_projection(&func.name) {
                Some(id) => id,
                None => symbols.intern(DefPath::item("", &func.name), kind),
            });
        }
        // 解析静态引用：先查投影索引（限定名 / std 原生 / 扁平名），未限定名的程序
        // 裸名回退 `main.{name}`。所有函数已登记完毕，前向引用在此必然可解。
        let resolve = |name: &str| {
            symbols.def_by_projection(name).or_else(|| {
                if unqualified_names {
                    symbols.def_path(&DefPath::item("main", name))
                } else {
                    None
                }
            })
        };
        // 函数体：逐指令解析静态调用目标。
        for func in &mut ir.functions {
            if func.is_type_decl() {
                continue; // TypeDecl 无指令体
            }
            for block in func.blocks_mut() {
                for instr in &mut block.instructions {
                    Self::resolve_static_targets(instr, &resolve);
                }
            }
        }
        // 模块顶层初始化序列：与函数体**同一套解析规则**（2026-10-11 裁甲）。
        //
        // 此前 init 段被有意跳过，于是同一张扁平函数表出现两套身份口径：函数体
        // 走 DefId，init 段恒 None 只能按名回退。名字与 def 在 codegen 里指向同一
        // 个函数表下标（名字由 `qualify_names` 与函数定义同源改写），故本改动
        // 不改变翻译结果，只消除「def 是否为空」的口径分叉——将来任何假设
        // 「def 恒有值」的 IR pass 不会在 init 段踩空。
        for instr in &mut ir.init {
            Self::resolve_static_targets(instr, &resolve);
        }
    }

    /// 解析单条指令里的静态调用目标 DefId：`Call`/`TailCall` 的字符串操作数、
    /// `MakeClosure` 的目标名，其余指令不动。
    ///
    /// 函数体与模块初始化序列**共用本实现**（禁令三：一处生产、一处消费）——
    /// 两处若各写一套映射，正是 t12 要消除的口径分叉。
    fn resolve_static_targets(
        instr: &mut Instruction,
        resolve: &impl Fn(&str) -> Option<crate::frontend::core::types::identity::DefId>,
    ) {
        match instr {
            Instruction::Call { func, def, .. } | Instruction::TailCall { func, def, .. } => {
                if let Operand::Const(ConstValue::String(name)) = func {
                    *def = resolve(name);
                }
            }
            Instruction::MakeClosure { func, def, .. } => {
                *def = resolve(func);
            }
            _ => {}
        }
    }

    /// RFC-029 限定名：从一个文件的 `use` 语句构建导入别名表（本地短名 → 源模块限定名）。
    ///
    /// 供限定名重写解析跨文件调用目标。std 调用已由 IR 生成限定为 `std.*`，不在此表。
    fn build_import_aliases(
        &self,
        module: &ast::Module,
    ) -> HashMap<String, String> {
        Self::import_aliases_from(module, &self.registry)
    }

    /// `use` 导入别名表（本地调用名 → 限定名）的**唯一生产实现**（t10）。
    ///
    /// 为什么抽成 pub 关联函数：驱动侧聚合实例化请求时要把 typecheck 的裸名请求
    /// 对齐成 IR 调用指令里的名字，而那个名字正是由本表在 `qualify_names` 里
    /// 改写出来的——两边必须**同源**。抽出来后驱动直接调它（传单元 AST 与同一
    /// registry），不再另建平行表、也不复刻解析规则（禁令三：一处生产、一处消费）。
    ///
    /// 规则（原实现逐字迁移）：只登记 `use path.{item}` 的**项**形态（含 #245 内联
    /// 别名），且仅对 Function/Constant/Type 类导出——#244：类型也限定，构造调用
    /// `Point(...)` 与方法调用 `Point.get_x` 都要把本地短名别名到源模块限定名才能解析。
    pub fn import_aliases_from(
        module: &ast::Module,
        registry: &crate::frontend::module::registry::ModuleRegistry,
    ) -> HashMap<String, String> {
        let mut aliases = HashMap::new();
        for stmt in &module.items {
            if let ast::StmtKind::Use {
                path,
                items,
                item_aliases,
                ..
            } = &stmt.kind
            {
                let Some(exports) = registry.get_exports(path) else {
                    continue;
                };
                if let Some(names) = items {
                    // 本地名：内联别名（#245）> 原名
                    for (i, item) in names.iter().enumerate() {
                        if let Some(export) = exports.get(&item.name) {
                            if matches!(
                                export.kind,
                                ExportKind::Function | ExportKind::Constant | ExportKind::Type
                            ) {
                                let local_name = item_aliases
                                    .as_ref()
                                    .and_then(|v| v.get(i))
                                    .and_then(|a| a.as_ref())
                                    .map(|a| &a.name)
                                    .unwrap_or(&item.name);
                                aliases.insert(local_name.clone(), export.full_path.clone());
                            }
                        }
                    }
                }
            }
        }
        aliases
    }

    /// RFC-029 限定名：把本文件 IR 里的顶层函数名投影为 `{module_key}.{name}`，
    /// 并把**函数体与模块顶层初始化序列**里对这些函数与导入函数的调用/闭包
    /// 引用一并重写（init 段见 t3 F1——漏掉它就出现「定义已限定、调用仍裸名」
    /// 的不自洽 IR，运行期 E6006）。
    ///
    /// module=record 语义：`a.helper` 与 `b.helper` 是两个 record 的不同字段，本就不冲突；
    /// 解释器函数表是扁平 name→func，故用限定名作键使其共存。限定名一律是
    /// [`DefPath::project`] 的产物——全仓库限定名拼接的唯一实现。
    ///
    /// 返回「**最终名 → 路径**」：`assign_defs` 据此登记身份，投影与登记同源。
    /// 未改名的函数（带点且限定点不是本文件类型）不进表——它们要么是别处已投影
    /// 的名字（std 等），要么是本单元生成的扁平名（curry 内层），判据见 `assign_defs`。
    ///
    /// **方法派生函数不在改写面内**（WBS 4.9.10 D-8）：它们经 `method_def_ir_names`
    /// 由 typecheck 的方法身份直出，进本函数时名字**已是** `{模块键}.{Type}.{method}`
    /// ——首段是模块键而不是本文件类型名，故上面这条判据让它原样通过，`assign_defs`
    /// 按投影索引复用已登记的身份（Registry 期 `intern_method` 已登记同一条路径）。
    /// 调用名同理已是最终名：`rewrite_call_names` 以**本单元函数表的最终名集合**
    /// 短路，避免前缀规则把 `main.Temp.tag` 的 `main` 当成同名函数的裸名再限定一次。
    /// 声明域方法派生名 `{Type}.{method}` 的归属段：按**已知类型名集合**做前缀
    /// 匹配，不把名字按点切开（D64 §2.5：首段不是归属段——类型名/身份串都可能含点）。
    fn declared_method_owner<'a>(
        name: &'a str,
        type_names: &'a std::collections::HashSet<String>,
    ) -> Option<(&'a str, &'a str)> {
        type_names.iter().find_map(|owner| {
            let rest = name.strip_prefix(owner.as_str())?;
            let method = rest.strip_prefix('.')?;
            (!method.is_empty()).then_some((owner.as_str(), method))
        })
    }

    fn qualify_names(
        ir: &mut ModuleIR,
        module_key: &str,
        aliases: &HashMap<String, String>,
    ) -> HashMap<String, DefPath> {
        // 先收集本文件的类型名（TypeDecl），供方法派生函数前缀匹配。
        let mut type_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for func in &ir.functions {
            if func.is_type_decl() {
                type_names.insert(func.name.clone());
            }
        }

        // 本文件要限定的函数：TypeDecl、方法派生（前缀是本地类型）、普通函数。
        // `rename` 是改写用的「原名 → 限定名」，`paths` 是登记用的「最终名 → 路径」——
        // 两者同出一个 `DefPath`，故定义名与调用名不可能各算一次。
        let mut rename: HashMap<String, String> = HashMap::new();
        let mut paths: HashMap<String, DefPath> = HashMap::new();
        for func in &ir.functions {
            let bare = &func.name;
            let path = if func.is_type_decl() {
                Some(DefPath::item(module_key, bare))
            } else if bare.contains('.') {
                // 带点名只可能是「声明域两段名」或别处已投影的限定名；仅当前缀是
                // 本文件类型时才限定。归属段按已知类型名集合取（不切点）。
                Self::declared_method_owner(bare, &type_names)
                    .map(|(owner, method)| DefPath::method(module_key, owner, method))
            } else {
                Some(DefPath::item(module_key, bare))
            };
            let Some(path) = path else { continue };
            let qualified = path.project();
            rename.insert(bare.clone(), qualified.clone());
            paths.insert(qualified, path);
        }

        // 重写函数定义名。
        for func in &mut ir.functions {
            if let Some(q) = rename.get(&func.name) {
                func.name = q.clone();
            }
        }

        // 本单元函数表的**最终名**集合（定义改名之后取）：调用名命中它即已是
        // 最终形态，`rewrite_call_names` 据此短路（见那里的注释）。
        let final_names: std::collections::HashSet<String> =
            ir.functions.iter().map(|func| func.name.clone()).collect();

        // 重写全局槽位名（T5）：与 `allocate_global_slots` 的限定规则同源，
        // 使槽位名与跨文件布局键（`lib.value`）一致，同名全局得以共存。
        for slot in &mut ir.globals {
            slot.name = DefPath::item(module_key, &slot.name).project();
        }

        // 重写调用/闭包引用：本文件函数（rename）+ 导入函数（aliases）。
        for func in &mut ir.functions {
            if func.is_type_decl() {
                continue;
            }
            for block in func.blocks_mut() {
                for instr in &mut block.instructions {
                    Self::rewrite_call_names(instr, &rename, aliases, &final_names);
                }
            }
        }

        // 模块顶层初始化序列（`ir.init`）走**同一张 rename/aliases 表与同一
        // `rewrite_call_names`**（t3 F1）：此前只重写函数体，init 里对本文件
        // 顶层函数的调用名保持裸名，而函数定义已被限定——运行期按裸名查扁平
        // 函数表落空（E6006）。这与泛型无关，是既有的多文件正确性漏洞。
        for instr in &mut ir.init {
            Self::rewrite_call_names(instr, &rename, aliases, &final_names);
        }

        paths
    }

    /// **调用名解析的单点实现**：把一个调用名解析成 IR 函数表里的最终名字。
    ///
    /// 解析顺序：完全匹配 rename（本文件函数）→ 完全匹配 aliases（导入函数）→
    /// 前缀匹配（方法调用 `Point.get_x` 经 `Point` 的 rename/alias 限定为
    /// `lib.Point.get_x`）。
    ///
    /// 为什么单独成一个 pub 函数：驱动侧要把 typecheck 的实例化请求名对齐到
    /// 「IR 调用指令里的名字」，而这正是本函数给出的结果。请求名与指令名只要
    /// 有一处用了别的规则，就会一边限定、一边裸名——单态化改不到调用点
    /// （E6006）或误报未限定化（E3018）。故本函数是**唯一**的解析规则所有者：
    /// `rewrite_call_names` 在 IR 生成期用它改写指令，驱动在聚合期用它对齐请求名。
    ///
    /// 驱动侧传入的两张表与生成期同源：`rename` 是**已限定**名字的恒等映射
    /// （`qualify_names` 之后的函数表），`aliases` 由驱动侧**直调**
    /// [`Self::import_aliases_from`]`(单元 AST, 同一 registry)` 取得——即
    /// `qualify_names` 改写调用名所用的同一个实现，不是驱动侧另建的表。
    /// 细节见 `driver::helpers::aggregate_instantiation_requests` 的注释。
    pub fn resolve_call_target(
        name: &str,
        rename: &HashMap<String, String>,
        aliases: &HashMap<String, String>,
    ) -> Option<String> {
        rename
            .get(name)
            .cloned()
            .or_else(|| aliases.get(name).cloned())
            .or_else(|| {
                // 已知键的**最长前缀匹配**：不按首点切名字（D64 §2.5——身份串/限定名
                // 的首段不是类型名；3 段名与带点特化名在这里同样按整键匹配）。
                let mut best: Option<(&str, &str)> = None;
                for (key, qualified) in rename.iter().chain(aliases.iter()) {
                    if key.is_empty() {
                        continue;
                    }
                    let Some(rest) = name.strip_prefix(key.as_str()) else {
                        continue;
                    };
                    let Some(rest) = rest.strip_prefix('.') else {
                        continue;
                    };
                    if rest.is_empty() {
                        continue;
                    }
                    if best.is_none_or(|(best_key, _)| key.len() > best_key.len()) {
                        best = Some((key.as_str(), qualified.as_str()));
                    }
                }
                let (key, qualified) = best?;
                Some(format!("{}{}", qualified, &name[key.len()..]))
            })
            .or_else(|| {
                // 显式类型实参的预特化名（`mk(int64)`）：两层调用 `mk(Int)(5)`
                // 由 ir_gen 按 D6.3 直接发特化名（mono 会剥掉未特化原件），其
                // 基名同样是本单元函数——按同一套规则限定基名、类型参数串原样
                // 保留，否则限定后查表落空（E6006；4.10.3 语料实证）。
                let open = name.find('(')?;
                let base = &name[..open];
                let suffix = &name[open..]; // 含左括号
                rename
                    .get(base)
                    .or_else(|| aliases.get(base))
                    .map(|q| format!("{}{}", q, suffix))
            })
    }

    /// 重写单条指令里的函数名引用（Call/TailCall 的字符串 func、MakeClosure 的 func）。
    ///
    /// 解析规则见 [`Self::resolve_call_target`]（单点实现）。
    fn rewrite_call_names(
        instr: &mut Instruction,
        rename: &HashMap<String, String>,
        aliases: &HashMap<String, String>,
        final_names: &std::collections::HashSet<String>,
    ) {
        // 调用名已是本单元函数表里的**最终名** ⇒ 不需要任何改写。
        //
        // 这一档不是优化而是正确性：方法函数名由 typecheck 的权威映射直出、
        // 已带模块键（WBS 4.9.10 D-8：`main.Temp.tag`），而前缀规则会把首段
        // `main` 当成本单元同名函数的裸名再限定一次（`main.main.Temp.tag`，
        // 运行期 E6006）。定义名与调用名逐字相同 ⇒ 改写必是恒等。
        let resolve = |name: &str| -> Option<String> {
            if final_names.contains(name) {
                return None;
            }
            Self::resolve_call_target(name, rename, aliases)
        };
        match instr {
            Instruction::Call { func, .. } | Instruction::TailCall { func, .. } => {
                if let Operand::Const(ConstValue::String(name)) = func {
                    if let Some(q) = resolve(name) {
                        *name = q;
                    }
                }
            }
            Instruction::MakeClosure { func, .. } => {
                if let Some(q) = resolve(func) {
                    *func = q;
                }
            }
            _ => {}
        }
    }

    /// RFC-029 多文件编排：预注册其他文件的类型上下文（结构体字段布局 + 方法绑定），
    /// 使本文件 IR 生成能解析跨文件的字段索引与方法调用重排。
    ///
    /// 仅注册类型上下文，**不生成函数 IR**——各文件的函数与匿名绑定 IR 由
    /// 各自的 `generate_module_ir` 生成，链接后按名字解析。镜像 `generate_stmt_ir`
    /// 中 `TypeDefinition` 的结构体上下文注册逻辑。
    pub fn seed_cross_file_types(
        &mut self,
        // (单元键, AST)：键与 typecheck 侧的模块键同一份（driver 的
        // program.units[i].key），故这里登记的身份与 typecheck 写的身份逐字相同；
        // 空串 = 单文件/无模块键形态（平路径）。
        modules: &[(&str, &ast::Module)],
    ) {
        for (module_key, module) in modules {
            for stmt in &module.items {
                if let ast::StmtKind::TypeDefinition {
                    name, definition, ..
                } = &stmt.kind
                {
                    match definition {
                        ast::Type::NamedStruct {
                            name: struct_name,
                            fields,
                            ..
                        } => {
                            let identity = Nominal::item(module_key, struct_name)
                                .expect("类型定义名不含点且非空");
                            self.register_struct_definition(identity, fields.clone());
                        }
                        ast::Type::Struct { body } => {
                            let fields: Vec<ast::StructField> = body
                                .iter()
                                .filter_map(|it| {
                                    if let ast::TypeBodyItem::Field(f) = it {
                                        Some(f.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            let bindings: Vec<ast::TypeBodyBinding> = body
                                .iter()
                                .filter_map(|it| {
                                    if let ast::TypeBodyItem::Binding(b) = it {
                                        Some(b.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            let fields = self.classified_fields(name, &fields);
                            let identity =
                                Nominal::item(module_key, name).expect("类型定义名不含点且非空");
                            self.register_struct_definition(identity, fields);
                            self.register_type_bindings(name, &bindings);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// RFC-029 多文件编排：登记跨文件全局布局（限定名 → 绝对槽位号）。
    ///
    /// 使本文件对跨文件全局（`use lib.{value}` → `lib.value`）的引用
    /// 能用定义方的槽位号生成 `Load(Global(idx))`，而非误判为未定义。
    pub fn seed_cross_file_globals(
        &mut self,
        layout: &[(String, usize)],
    ) {
        for (name, idx) in layout {
            self.global_slot_layout.insert(name.clone(), *idx);
        }
    }

    /// 本文件顶层绑定的槽位分配起点。
    ///
    /// 多文件模式下各文件必须用不相交的槽位区间（否则跨文件索引冲突），
    /// 由编排器按发现顺序预先分配；单文件模式恒为 0（索引 = 声明序）。
    pub fn set_global_slot_base(
        &mut self,
        base: usize,
    ) {
        self.global_slot_base = base;
    }

    /// 把一段指令序列里的跳转目标整体平移 `base`。
    ///
    /// `Instruction::Jmp` / `JmpIf` / `JmpIfNot` 的 target 是**函数内展平后的绝对下标**
    /// （translator 按 `global_ir_index` 建表）。当一段指令在**空 Vec** 上生成、
    /// 再拼进更大的序列时，原下标基准失效，必须平移。
    ///
    /// 顶层语句与顶层重赋值都在空 Vec 上生成（循环体的回跳会指向 0），
    /// 拼进 `module_stmts` / `module_init` 前必须重定位。
    pub(super) fn rebase_jump_targets(
        instrs: &mut [Instruction],
        base: usize,
    ) {
        if base == 0 {
            return;
        }
        for instr in instrs.iter_mut() {
            match instr {
                Instruction::Jmp { target, .. }
                | Instruction::JmpIf { target, .. }
                | Instruction::JmpIfNot { target, .. } => *target += base,
                _ => {}
            }
        }
    }

    /// 注册类型绑定信息（RFC-004）
    ///
    /// 将类型定义体内的绑定（外部函数绑定和匿名函数绑定）记录到 type_bindings 映射中，
    /// 用于后续方法调用 IR 生成时的参数重排。
    pub(super) fn register_type_bindings(
        &mut self,
        type_name: &str,
        bindings: &[ast::TypeBodyBinding],
    ) {
        use ast::BindingKind;

        // 合并到已有的绑定映射中，而非覆盖
        let mut binding_map = self.type_bindings.remove(type_name).unwrap_or_default();

        for binding in bindings {
            match &binding.kind {
                BindingKind::External {
                    function,
                    positions,
                } => {
                    binding_map.insert(
                        binding.name.clone(),
                        BindingInfo {
                            function: function.clone(),
                            positions: positions.clone(),
                        },
                    );
                }
                BindingKind::Anonymous {
                    params: _,
                    return_type: _,
                    positions,
                    body: _,
                } => {
                    // 匿名函数绑定：函数名使用 "类型名.__anon_方法名" 格式
                    // 后续生成匿名函数的 IR 时使用此名称
                    let anon_func_name = format!("{}.__anon_{}", type_name, binding.name);
                    binding_map.insert(
                        binding.name.clone(),
                        BindingInfo {
                            function: anon_func_name,
                            positions: positions.clone(),
                        },
                    );
                }
                BindingKind::DefaultExternal { function } => {
                    // RFC-004: 默认绑定，位置由类型检查器自动推导，此处使用位置 0
                    binding_map.insert(
                        binding.name.clone(),
                        BindingInfo {
                            function: function.clone(),
                            positions: vec![0],
                        },
                    );
                }
            }
        }

        if !binding_map.is_empty() {
            self.type_bindings
                .insert(type_name.to_string(), binding_map);
        }
    }
}
