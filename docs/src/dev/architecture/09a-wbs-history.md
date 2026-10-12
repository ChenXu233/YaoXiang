# WBS 历史注记（落地实录 · 审计 · 裁决留痕）

> 本文是 [09-execution-wbs.md](./09-execution-wbs.md)（施工任务表）的**历史注记**：各阶段的落地实录、审计发现、裁决留痕与勘误。
> **分工**：09 只描述**计划**（看板、三级任务表、依赖、风险、验收）；本文只记录**历史**（做过什么、发现了什么、如何裁决），按时间追加，不改写历史。

## P2 建立等价性判据基线

> **P2 落地（2026-10-06，2.1）**：verify.rs 双模式 + 7 不变量 + 全语料跑绿。**D38 预言兑现**——跑绿过程实测并修复 ir_gen 三处真实缺陷（详见 06 §G6）：方法 locals 表截断至参数数（local_count 低估，运行时靠 Frame::set_slot 容错）、无注解参数被 filter_map 丢弃（三处，params 元数据与帧布局错位）、match 结果槽依赖帧隐式 Void 初始化（已显式预置化）。跳转 target 的现实语义是展平指令下标（非 07 文本的 label 键控），CFG 按指令级构建——verify.rs 文件头登记。**提交对照**：ir_gen 修复 = `a9ea199b`；verify 主体 = `d5b18390`（⚠ 并行流 COMMIT_EDITMSG 碰撞致其消息误标为「docs(rfc): RFC-027a…」，内容实为 verify.rs + 测试六文件，已埋于历史中间不可 amend，特此登记）。
>
> **P2 收窄部分落地（2026-10-06）**：2.3.2 + 2.4.1 + 2.4.3 全部就位，**P3 止血通道前置已解锁**。
>
> - 2.3.2：`tests/yaoxiang-multifile/` 5 夹具（行为 ×4 + compile-error ×1）+ 运行器 `tests/yx_multifile_runner.rs`；判定契约与单文件语料同一套 `TestFileSpec`（库侧单点），二进制定位/子进程拉起抽至 `tests/common/` 且 yx_runner 同步迁移（禁令三）。
> - 2.4.1/2.4.3：`tests/integration/proof_obligations.rs`。触发源采用 rfc027 测试钉死的 `SumUpTo(3, r)` 形态（返回位/调用点/绑定位）——02 的 `Sorted(3)` 注解示例在当前代码会被代入求值直接 Disproved，不呈现 Unproven 静默，07 伪码的 Sorted(3) 仅作示意。红态实测数据见测试文件头。
> - 两个红判据按 2.4.5 红骨架先例挂 `#[ignore]` 常驻：默认套件全绿，`-- --ignored` 复现红态；**P3 修复落地时移除属性即转绿**。
> - 比对规范化对 07 (code, file, line) 表的有意偏离（剔除 file/line、只比 error 码集）的理由写在 2.4.1 测试注释。
>
> 原 2.4.2（`test_program_stage_coverage`）与 2.4.5（`test_obligations_drained` 骨架）**移出 P2**：两者引用的 `Program` / `Obligations` 类型在 P4 才存在，P2 无法编译。前者即 P4 的 4.1.4（勿双重登记），后者的 `#[ignore]` 红骨架并入 4.3.1。
>
> **2.3.3 落地（2026-10-07）**：C4 必覆盖清单逐项核对（07 §143），10 类语义语料层全覆盖且均在 corpus-parity.jsonl 差分基线内（26 个代表文件 26/26 命中，grep 实证）——ref/borrow/move 05-ownership 全目录（ref_shared、borrow_immutable/mutable/return、move_basic、ownership_deep 等）、闭包捕获 closures + closure_arg_inference + spawn_capture、柯里化 curry_value_fix + curry_tail_expr、spawn 04-concurrency ×13、迭代器 for spawn_for 等 16 文件、和类型 sum_type_*+ pattern_or_guard + match_call_scrutinee、existential 强制点 interface_dynamic_dispatch（Vec(Animal) 存在类型异构包装触发 existential_coercions）+ interface_rebind_dispatch、`?`/Try 传播 question_propagation + option_try + try_methods + user_try_type、方法重载 method_overload（+ method_overload_ambiguous_err 在诊断差分面）、精化约束 Drop 序列 refined_* ×5 + 05-ownership 驱动 ReleasePlan（span 键契约另有 2.4.4 红判据常驻）。性能基线 `benches/pipeline.rs` 入库（07 §145 三冒烟之二）：`corpus_compile_frontend` 全语料编译首测 17.6s、`cli_cold_start_hello` 进程冷启动 169.7ms；解释吞吐由 benches/lib.rs hotpath 组（interp_fib_recursive_27 / interp_loop_steady_10m）已有覆盖，不重复建设。回归门禁 check-perf-regression.sh 归 P9 统一清单（2.5）。

## P3 修正确性漏洞（最小方案）

> **已落地行的错误类型标注**（追溯标注，不改动历史）：3.1 = B ｜ 3.2 = B（防御性空臂，判据转绿由 3.1 承担）｜ 3.3 = A ｜ 3.4.1 / 3.4.2 = A（新增 W 码）｜ 3.4.3 = 等价重构（人工 review）｜ 3.4.6 / 3.4.7 = D（用户可见面，已经用户裁决）｜ 3.4.8 ①② = C（主）＋ A（附带三处）｜ 3.5 / 3.6 / 3.7 / 3.8 = D（均已用户裁决）。

> **P3 诊断（2026-10-12，用户要求）**：针对「P3 条目持续增长」的观感做一次账目分诊。
>
> **一、先厘清规模（观感被两处账目缺陷放大）**
>
> 1. **编号重号**：`3.5.x` 被两个任务族共用——(a) 2026-10-08 的 **029g `pub` 可见性收敛**（7 条三级：词法与 parser 分支 / AST `is_pub` / checker 侧 / 死码豁免收敛 / 外围回显 / std 与诊断文案 / 语言参考收敛，均已勾选）；(b) 2026-10-10 登记的 **3.5 高阶类型参数调用路径补全**（3 条三级）。二者共用 `3.5.x` ⇒ 交叉引用歧义（**2026-10-12 已按用户裁决改号：029g 族改为 `029g.x`**），并让 029g 的行被误算进 P3。**待裁决**：把 029g 那族改号为 `029g.x`（或移入其所属阶段），文档内其余引用同步。
> 2. **漏勾**：`3.7` 的 4 个三级（3.7.1–3.7.4）均已完成（d922a551 / 38bd4b9e）却未勾选 ⇒ 本条本次修正。
>
> **二、P3 实际账目（12 个二级行）**
>
> | 行 | 状态 | 处置建议 |
> | --- | --- | --- |
> | 3.1 / 3.2 / 3.3 | 已完成 | — |
> | 3.4 静默丢弃点补遗（8 三级） | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 完成；3.4.4、3.4.5 前置 4.3.1；3.4.6 前置 4.2.1 | **阻塞主线**（等 P4 前置），随 P4 推进，不单独排期 |
> | 3.5 高阶类型参数调用路径补全（3 三级） | 未开始（声明侧活、调用侧断裂） | 保留 P3；**不阻塞重构主线**，但属「已接受特性未实现」，优先级高于纯清理项 |
> | 3.6 函数类型值调用路径（1 三级） | 未开始（语义已裁 (i)） | **保留**：正确性缺陷、判据明确 |
> | 3.7 init 段调用名限定（4 三级） | **全部完成** | 本次勾选并关闭 |
> | 3.8 native 遮蔽（1 三级） | 未开始 | **卡在决策**（先定名解析优先级），非排期问题 |
> | 3.9 / 3.10 / 3.11 | 已完成（2026-10-12） | — |
> | 3.12 运行时模式间的执行器配置传播（1 三级） | 未开始（D 类，用户裁决乙：另起队伍） | **建议移出 P3 → P8（执行层）**，待裁决 |
>
> **三、机制建议（防复发）**：新登记的二级/三级行**必须带两字段**——「归属阶段」（它真正属于哪个 P）与「是否阻塞主线」；`登记待排期` 不再作为默认状态，未标注即视为不可排期。这样膨胀是可见、可排序的，而不是无差别堆积。**待裁决**：是否把该条写进本文档的登记约定与 `src/dev/coding-rules.md`。

> **P3 落地（2026-10-06）**：3.1.1 `proof_calls` 收窄可见性 + 唯一 getter（当时记 `pub(crate)`；**2026-10-07 核实实际落地为 `pub(super)`**——代码注释引用的 `#434` 裁决 docs 侧无登记，见 3.4.3；全私有需改写 11 处构造字面量，止血阶段不取；P4 Obligations 迁移时收口）；执行机制抽至 `src/frontend/proof_execution.rs`（禁令三单点——不能放 typecheck/，那会造 L3→L4 反向依赖）；3.1.2 按 02 证据链第 5 步补齐**四入口**（WBS 行号漏列 compile_project `:99`，按 3.1.1 验收「判据转绿」所必需补齐并登记）；3.2.1 空臂改记账（proof_calls 上抛）+ 诊断（`into_result` 既定转换路径）；3.3.1 SOLVER 槽 Option 化，Z3 缺失按 `SMTResult::Unknown` 保守降级（E2031 族诊断取代进程崩溃）。两个漏洞判据移除 `#[ignore]` 转正常绿，补语料夹具 `proof-obligation-honored`；check-fanout 库存随之迁移（接线点 19=库存）。实测：判据 2.4.1/2.4.3 转绿，全套件 lib 2494 / integration 272 / 语料双层全绿。

> **3.4 审计（2026-10-07，P3 落地后复核）**：针对「失败不产生任何信号」这一类缺陷（定义见 02 §41-52），在未改动代码上重新盘点，又定位 8 处，全部登记为上表 3.4.x。方法与证据：
>
> - **消费面盘点**（16 个义务字段逐一 grep 全仓读取点）：`variant_ctor_calls` **零消费者**——唯一读取 `checker.rs:1468` 是「自己灌进 `TypeCheckResult`」，IR 侧走形态检测 `ir_gen.rs:7374`；`rfc011b.rs:659/688` 只断言生产端，正是 02 §186 批过的「只测生产端、对消费端缺陷天然免疫」形态 → 3.4.4。`method_overload_ir_names` 结果级字段同样零外部消费者，IR 用平行的 `overload_resolutions` → 3.4.5。
> - **阶段覆盖复核**：02 §91 表第 1 行（四入口「无 proof_execution」）已由 3.1.2 修复，现状为 5 处共用一个实现（`pipeline.rs:189`、`orchestrator.rs:141/340/507/1448`）；但第 2/4/5 行现状不变（多文件 `run` 仍不输出 W1003、不跑死代码族）→ 3.4.6；LSP 非项目内路径（02 §77「手工 lex→parse」）同样不跑 proof → 3.4.7。
> - **同形空臂扫描**（`Unproven` 全仓消费者仅四处）：checker 三处精化分支（3.1 已覆盖）、3.2.1 修的 ownership 臂（**源码不可达**）、以及 `environment.rs:422`（**可达、零诊断、零记账，且全仓 docs 零登记**）→ 3.4.1，是本次审计唯一的「全员未知」项。
> - **求解器获取面**：`default_solver()` 三处消费形态各异（`predicate.rs:32` 进程单例 / `checker.rs:1266` 每次 `check_module` 注入 / `ownership.rs:621` 每次回边判定新建）——3.3.1 只消掉了 panic 一条；另两条的**静默面**归 3.4.2，**单例化与缓存命中**归 P4 的 4.4.2（勿双重登记）。
> - **未计入 3.4 的既有归属**：2.4.4（`ReleasePlan` span 键失配，168 文件/373 键未被消费、Drop 静默丢失）仍是 P2 红态常驻，归属 4.5.1/D20 不变；4.4.1（层序声明与实际相反、`equivalence` 层不在管线）/ 4.4.3（`unwrap_or_default()` 空账本降级）已在 P4 在册。
>
> **执行约定**：3.4.1 / 3.4.2 / 3.4.3 无 P4 前置，可随时就地落地（建议先做 3.4.1——唯一「全仓零登记」的活体漏洞，改动面最小）；3.4.4–3.4.8 随 P4 执行，其判据依赖义务账本（4.3.1）或统一 Driver（4.2.x）。**3.4 不新立判据体系**，一律复用既有 C2/C3 与 P9 的 9.2 门禁。

> **3.4.8 ③④ 落地（2026-10-11，8ecf56f93）**：③ `CHECK_STAGES` 尾部加 `Linking`（**纯合并**，整块跳过 `is_bin_role` 入口校验——E3020 判据仍归 `RoleClassification` 防双报；另有硬性理由：run 路径按 merged IR 判「有无 main」，收集语义下「入口自身出错→无 IR」会被误判缺 main 而硬中止）→ `Monomorphization`（检查形态）；`ir_generation_check` 留存单元 IR（**仅内存**，供纯合并与 mono 消费，不外发/不写盘/不产快照——不留存则聚合请求为空、mono 以 `NoObligations` 假绿）；mono 失败在 Check 走逐文件收集通道（诊断归入口文件 + `failed_stages`，不置 failure），MultiFile 保持 `fail_with`；`into_outcome` 改为「先取收集再判故障」。④ 经实证**零生产改动**：全程序 BFS 已覆盖跨单元互递归（病态 → E3005、良构三层链执行出正确值），交付四组 e2e + 一条 mono 单测作为活体判据。
>
> **验证链**（全部独立于实现）：t3（③，9 组自建 fixture + 与 HEAD 二进制逐字节交叉）、t4（④，10 组 fixture + `max_depth` 参数化边界）、t10（F1 修复，11 例 + 双二进制对比）全 pass；t5 跨形态矩阵（single/multi/check 三形态 + 语料 340/340 IDENTICAL + IR 快照零漂移）；t6 复审 PASS；测试数 2643 → 2656（只增不减）。
>
> **F1（验证期发现，D59 A/B 类，同轮修）**：无项目根的 standalone 多文件 check 下，聚合请求名（`lib.f`）与单元 IR 定义键（`f`）不同源 → mono 落「native 无 IR 定义」的静默 `continue`，check 报 0 error/exit 0。修法：聚合侧新增按各单元**真实定义键**的反查跳（精确枚举 + 逐字相等 + 歧义放弃，非后缀启发式；项目形态不登记、既有路径不变）。**勘误（同日）**：该反查跳已在随后的回归修复中**删除**；根因改由 `module_key` 一职两用解耦（新增 `script_mode`）从命名口径上解决——IR 定义键 = IR 调用名 = 聚合请求名 = `{单元键}.{名字}`。**回归实录**：③ 落地后 CI 的 `Check tutorial/guide code examples compile` 红（258 块 / 10 失败：6 项同文件同名重绑定 + 4 项与 `<std/list>` 撞名，rootless check 不做限定触发 `link_module_irs` 的冲突校验）；修复 = ir_gen 拆 `script_mode`（限定名 ⊥ Script/Bin，E3023 闸门改看它）+ `ir_generation_check` 逐单元恒传单元键 + `.with_script_mode(rootless)` + Script 顶层同名重绑定「后者生效」（与 run 路径既有行为一致）；验证 = 文档门禁 258/0、三二进制对照（HEAD/③树/修复树）三类碰撞夹具、check 形态 E3005 不回归、E3023 未打开。
>
> **F2/F3（接受）**：mono 诊断归入口文件条目、span 可跨单元（与 run 路径 `path=入口` 同口径）；Check 形态为每单元持一份 ModuleIR（仅内存，check 结束即释放）。
>
> **留给后续的既有问题（非本轮回归）**：① E3005 的 help 文案仍写「internal error; please report this issue」，与资源保护语义不符（先例：`e3xxx.rs:47` 曾把同类「用户写法错误」移出 E3005）——登记待排；② multi-file E3005 的 span 渲染标签错位（已用 HEAD 二进制复现，预存在）；③ 请求收集面存在**类型参数位置依赖**（类型参数不在值参数位的泛型调用不发实例化请求、mono 记 `NoObligations`，收敛互递归即此形态）——与 3.5（HKT 调用路径）同面，登记至该行。

## 外部轨道 RFC-029g：删除 `pub` 与自动绑定（#399

> **P3.5 落地（2026-10-08）**：七个二级任务全部完成，整批一个 commit（可独立 revert）。
>
> - **实测与计划不符（如实登记）**：029g.1.2/029g.1.3 的验收原写「`pub f = ...` 报**解析错误**」——实际删除 `KwPub` 后 `pub` 退回普通标识符，旧写法落到语义层报 **E1001 Unknown variable: 'pub'**（span 指向 `pub`）。按 RFC-039 **D27**「专门探测已移除语法是反模式」，**不新增**迁移专用诊断；语料夹具 `pub_declaration_err.yx` 钉住 E1001，文档（`syntax.md`）如实写明该后果。
> - **设计收敛（超出 RFC 字面，须 review）**：`project_refs` 与 `cross_file_refs` 两个字段在 pub 删除后语义完全重合（同一份「包内引用并集」，禁令一禁止平行表示）→ 合并为单一 `project_refs`，`set_exempt_pub`/`set_cross_file_refs` 两个开关删除。行为上 Internal 角色的非 pub 项从「文件内可达性」改为「引用池判定」（放宽，宁漏报方向）；Lib 由「pub 绝对豁免」收紧为引用池判定；Script 未引用顶层绑定开始报 W1001/W1002/W1004/W1005。
> - **死表清理**：`env.exports` + `add_export`/`is_exported`/`is_visible` + `auto_bind_to_type` + `typecheck/mod.rs` 的 exports 回写全部删除（生产端灌表、全仓零消费）。
> - **测试迁移**（测试数不降）：新增 `lexer/tests/rfc029g_lexer.rs`（1）；`parser/statements/tests/bindings.rs` 的「pub 被识别」断言反转为「pub 不再是修饰符」；`passes/tests/dead_code.rs` 7 个角色语义用例重写（断言反转 2 个）；`tests/integration/cli_e2e.rs` 8 个角色用例改写（`lib`/`internal` 的豁免断言反转为报告断言）。
> - **基线**：IR 快照 **zero-diff**（`pub` 不参与 IR）；语料差分基线仅新增 1 条（`pub_declaration_err.yx` → E1001），命令 `cargo run --quiet --example corpus_probe` 再生后人工 review。
> - **实测**：lib 2556 / integration 272 / 语料 322 条基线全绿；`cargo fmt` / `clippy -D warnings` / `check-boundary` / `check-concepts` / `check-fanout` / `check_tracking.py` 全绿。
