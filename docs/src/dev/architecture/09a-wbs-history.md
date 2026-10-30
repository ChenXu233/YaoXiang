# WBS 历史注记（落地实录 · 审计 · 裁决留痕）

> 本文是 [09-execution-wbs.md](./09-execution-wbs.md)（施工任务表）的**历史注记**：各阶段的落地实录、审计发现、裁决留痕与勘误。
> **分工**：09 只描述**计划**（看板、三级任务表、依赖、风险、验收）；本文只记录**历史**（做过什么、发现了什么、如何裁决），按时间追加，不改写历史。**09 行的收口登记只写「已完成 + 本文指针」**：实测数字、探针结果、判据细节一律落在这里，不回填 09（AGENTS.md「团队编排约定」第 8 条）。

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
> | 3.4 静默丢弃点补遗（8 三级） | 3.4.1/3.4.2/3.4.3/3.4.7/3.4.8 完成；**3.4.6 已解锁**（前置 4.2.1 已于 2026-10-09 完成 be60e01f）；3.4.4 与 3.4.5 **仍阻塞**（前置 4.3.1 未开始） | 3.4.6 **现在可做**；3.4.4/3.4.5 随 4.3.1 解锁 |
> | 3.5 高阶类型参数调用路径补全（3 三级） | 未开始（声明侧活、调用侧断裂） | 保留 P3；**不阻塞重构主线**，但属「已接受特性未实现」，优先级高于纯清理项 |
> | 3.6 函数类型值调用路径（1 三级） | 未开始（语义已裁 (i)） | **保留**：正确性缺陷、判据明确 |
> | 3.7 init 段调用名限定（4 三级） | **全部完成** | 本次勾选并关闭 |
> | 3.8 native 遮蔽（1 三级） | 未开始 | **卡在决策**（先定名解析优先级），非排期问题 |
> | 3.9 / 3.10 / 3.11 | 已完成（2026-10-12） | — |
> | 3.12 运行时模式间的执行器配置传播（1 三级） | 未开始（D 类，用户裁决乙：另起队伍） | **建议移出 P3 → P8（执行层）**，待裁决 |
>
> **四、4.3.1 的杠杆（主线路标）**：`4.3.1 obligations.rs + assert_drained()` 同时是 **3.4.4**、**3.4.5** 与 **4.5.1**（跨层契约 PlanId 化）的前置 ⇒ 取它一次同时推进主线并清掉两个 P3 滞留项。3.5（HKT）、3.6（函数类型值调用）与 4.6.1/4.9.1 均**无未满足前置**，属随时可做；3.8 卡在语义决策（用户定义优先 vs 显式诊断）。
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

> **批次 3.13–3.16 实录（2026-10-13 登记 → 2026-10-15 收口）**：用户 2026-10-14 裁决把四个相关性小项（#451 方法绑定 panic / #452 命名实参入决议 / #453 死推断入口 / #454 原生导出命名实参——后一项是验证 3.14 时的边界发现，同日经归因实验证**非 #452 引入**、用户裁决①并入本批）合并为一支团队处理。收口状态：四项全部完成，裁决登记为 RFC-039 **D60**（六条绑定规则 + 共享判据单点化）。
>
> **一、两次契约修正（教训：契约本身也会错）**
>
> 1. **验收夹具自相矛盾**：#452 任务契约同时要求「混合实参输出 20」与「位置与命名争同一位报错」，但钉板夹具 `Temp.pick(self, a)` 只有一个值形参——同一实参组合既是「混合」又是「冲突」。首任实现者如实报 blocker 并置任务 failed、整体 revert（不拿静默换绿灯）。处置：任务重派 + 契约修订——给出**六条绑定规则**与 **8 条值级矩阵**（夹具改两值形参 `pick(self, n, a)`）后 attempt 2 由第二任实现者完成。六条规则全文已逐字收入 D60。
> 2. **inScope 漏机械必需面**：#452 契约 inScope 列了 `docs/src/rfc/accepted/013-error-code-specification.md`，**漏了生成器再生的 `docs/src/reference/error-code/`**。E1110 登记后只跑了 `code-tables --fix`、**没重跑** `scripts/docs/gen-error-code-docs.py` ⇒ CI Docs Truth 红（「源码已注册/发射但文档缺失 1 个：E1110」，源码 154 / 文档 153，run **38518624964**——本批唯一一次 CI 红线）。修复 = 机械再生（8 文件 +83/−70，只动 `docs/src/reference/`），对账 154/154，CI 38520209026 绿。
>
> **二、复审 → 修复 → 再复审两条闭环**
>
> 1. **#452 线**：round-1 复审 needs_revision（F1–F5；F1 = 元数守卫 `named_args.is_empty()` 未随决议收窄 ⇒ 含命名实参的调用在多候选零命中/单候选命中两条路径整链静默）→ 修复轮新增对外码 **E1111（argument_order_invalid）**（码表 + locales + RFC-013 + 生成器四件套同提交；码数 154→155）→ round-2 复审 pass。
> 2. **#454 线**：实现（共享解析入口 + 原生臂按名结算）→ 独立验证 PASS（值级矩阵 12 格、语料 342/342、5 处变异经 `scratch.py trial` 机械可逆重放全部抓获、还原逐字节一致）→ round-1 复审 needs_revision（F1 = typecheck 原生臂**内联复制**了一份绑定判据、缺规则④ ⇒ 方法与原生同形调用**裁决分歧**——方法报 E1111、原生静默 exit=0；ir_gen 另有第三份存量副本报旧码 E1015）→ 收敛轮把判据收为单一实现 `bind_args_to_params`（`arg_binding.rs:35`，四调用点 `expressions.rs:2727` / `:5416`、`ir_gen.rs:7776` / `:8279`）→ round-2 复审抓到收敛轮引入的**回归**（IR 收敛块丢失原生可省略位 ⇒ 省略可选位的原先成功程序被误报 E1010）→ 修复轮（可选位判据与形参名同出 `signature.rs:236` 唯一出口）→ round-3 复审 PASS（独立复现红→绿；变异下仓内钉板 FAILED 实证判据可失败）。**教训**：复审必须覆盖「同类路径的未修分支」（本批两次 needs_revision 都是探针全落在命中面所致）；修复轮本身也能引入回归（可省略位），靠下一轮换人复审抓住。
>
> **三、过程事件（如实登记，不指向个人）**
>
> 1. **外部整树 discard（2026-10-14 约 17:14）**：工作树在无声中被外部还原——实现者的修复在制品（typecheck/ir_gen/码表/locales/RFC-013 等）全部消失，**同一时刻用户自己编辑的两篇 review 文档也从 git status 消失** ⇒ 判定为仓库级 discard（最可能用户端手工清理/IDE「丢弃全部更改」）。三证排除组内成员：stash 空、无新提交、reflog 无 reset。实现者按上下文重放在制品并推送；纪律随之改为「**先存 patch 到 scratch，再采红态**」。
> 2. **验证成员上下文耗尽换人**：验证 3.16 的独立验证者中途 ran out of room（无收尾消息），按负载规则换人——接任者续用其已就绪产物（绿态结果、语料等价脚本、变异计划），验证最终 PASS。partial 产物交接的正面先例。
> 3. **复审第 5 轮轮换与历史轮次退回**：按负载上限第 ⑤ 条轮换复审时，`remove_member` 把已判决的终态复审/验证轮**一并退回共享池**，调度器随即重跑（t3/t8 被新成员重新领走，重复工作并堵住下游）。处置：队长接管并取消退回的历史轮次（t3/t8/t13），据此新增 AGENTS.md 第 12 条（换人后立刻处置退回轮次）。本批收口记录即按该条登记此事件。
> 4. **scratch 治理（约 6.3 GB）**：批次前期每任务一棵外部树、仓库外散落 6 个目录（约 780 MB，已清理）+ 两个成员的外部验证树（约 2.2 GB / 3.3 GB，用后自删）。产出治理工具 `scripts/dev/scratch.py`（init/use/build/release/**trial**/status/rm/gc）与 AGENTS.md 第 9 条（批次树唯一入口）、第 10 条（验证环境六则：机械可逆变异 / 行号取被引用 revision / CI 独有门禁必跑等）、第 11 条（写入方负载上限）、第 12 条（换人后处置退回轮次）与顶部元规则两条（无私有内容 / 先查证再断言）。
> 5. **契约缺陷（不指向成员）**：#454 修复轮的契约 inScope 只圈了 2 个文件，实际机械必需面 8 个（第 6 条要防的正是这个——契约模板应预置机械必需面清单）；另该任务终态字段曾被探针载荷占用、正式验收/命令证据由队长按规约补录。
>
> **四、遗留项（本批不修，登记备查）**：round-3 复审三条低severity finding——① 方法路径的 `build_arg_slots`（`ir_gen.rs:521`）仍是判据的**第二份**实现（存量，按上轮口径随收口登记、不收敛）——**2026-10-16 发版前审计补记三点**：(a) 它有**两个**调用点：`ir_gen.rs:7888`（方法绑定路径，用被绑函数 `binding.function` 的形参名）与 `:7975`（重载决议路径，用决议出的混编名叶子段），原登记只写「`ir_gen.rs:521` 一带」；(b) 它**已相对共享实现 `bind_args_to_params` 分叉**：缺规则⑤（重复命名 → E1015）、规则⑥（缺必需形参 → E1010）与「可选位」概念，并把接收者位硬编成 `1 + i`；两个调用点对空槽一律 `continue`（注释写「诊断归 typecheck（它保证决议通过即无空槽）」），是一处**写在注释里、无断言兜底的悬空不变量**；(c) **RFC-039 D60 的「全部调用点收敛于它」枚举漏了这两处**，属声明与事实不符，且被引行号未在被引用 revision 上重取，已随本条一并更正 D60 文本；② 批次验证树（worktree）内 `cargo fmt --all` 报错属 worktree workspace 伪影，门禁清单应注明 fmt 走主仓；③ repair 类契约模板应预置机械必需面进 inScope（见三.5）。
>
> **五、最终门禁（收口 revision，本地两笔实现提交 + 一笔文档提交统一推送——本批唯一一次推送）**：fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` **2730 passed / 0 failed / 2 ignored** / `--test integration` **284 passed** / `--test yx_runner` 2 passed / check-obligations **PASS（13·13·0·8）** / check-docs-truth exit=0 / corpus_probe **342 行与基线逐行一致**（唯一再生行：`named_arg_duplicate_err.yx` E1015@14 → E1111@16，期望码与行号同步，已人工 review）/ IR 快照零漂移。
>
> **六、run 链接（推送侧证据）**：3.16 实现推送（cb45de1fc）→ CI **38550504517**（8/8 job success，含 Docs Truth + Obligation Gate）/ Deploy Docs **38550504587**；#452 修复轮（9ec7d89a8）→ Docs Truth job 38545762349（同 run 的 MSRV/Test 被后继推送按 concurrency 取消——取消≠失败，以后代 run 覆盖）；治理工具推送 → CI 38546373435 / 38561405117；WBS 登记（3.16 行 + 表头 56→57 / 147→148）→ CI 38528560364。收口推送自身的 CI / Deploy Docs run 在推送后补登任务记录。

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
>
## P4 4.5 跨层契约 PlanId 化（D20）

> **4.5 落地实录（2026-10-13）**：4.5.1（`ReleasePlan` 键 `Span` → `PlanId`）与 4.5.2（`overload_resolutions` 键 `Span` → `PlanId`）
> 分两笔功能提交落地，收口（登记 / WBS 勾选 / 门禁 / 推送）由 t7 完成。前置 = 4.3.1 义务账本（红态留档 173 文件 / 533 键）。
>
> **一、载体裁决（甲：`PlanId` 随 AST 节点走）**：新增 `PlanId`（`src/util/plan_id.rs`，`u32` 单调、编译单元内唯一、
> `UNPLANNED = u32::MAX` 表解析器之外的合成节点），由**唯一分配器** `ParserState::alloc_plan_id` 发放；载体两处——
> `Stmt.plan_id`（语句身份）与 `Expr::Call.plan_id`（调用点身份），`Expr::call` 为全仓唯一调用构造器。
> 否决的是「两侧各自按 span 推导同一身份」的位置巧合形态（D20 的原始缺陷）。
>
> **二、两级验证与复审（成员独立）**：每笔功能提交各配一名独立验证者与一名复审者，**验证/复审不复用实现者证据**：
> 4.5.1 = t1 实现 / t2 独立验证（外部树 + 5 处变异）/ t3 复审；4.5.2 = t4 实现 / t5 独立验证（外部树 + 5 处变异）/ t6 复审。
> 复核时点与被复核 revision 均落在不可变历史（`git show` 可核）。
> **provider 更名导致 t5 / t6 换成员续派**：原验证/复审成员因 provider 更名不可续用，按编排约定换人续派并**在记录里写明原因**
> （跨轮次上下文保留优先，压缩不可用时才换人）；续派后结论与原轮一致（PASS），未复用实现者证据。
>
> **三、实现期自查发现并修掉的两个缺陷（t1，均在 inScope 内）**：① 消费豁免判据曾挂在 `UnitState.ir.is_some()`——
> `check` 形态的 Linking/Mono 会把 `unit.ir` 取走，真实未消费在 check 形态**静默通过**（变异实测：D41 红而 CLI check exit 0）；
> 改为读**台账自身的完成标记**，并补回归用例。② Drop 序列随进程哈希种子漂移（偏序 `sort_by` + `HashSet` 迭代）：
> 快照判据连续三次跑出 106/120/114 的行数漂移；改为**全序排序**（同名取最大深度 + 名字升序）后连跑两次零漂移。
>
> **四、判据与证据数字**：4.5.1 后 `test_release_plan_spans_consumed` 去 `#[ignore]` 常驻（`examined ≥ 173` 地板 + 无白名单），
> 172 个 IR 快照同提交再生（程序化 review：新增 1083 行 Drop + 86 行跳转索引平移，零删除、无重排），语料 342 条差分为空；
> 4.5.2 新增 `test_overload_resolutions_consumed`（差集空 + 证据面地板 `examined ≥ 1` / `consumed ≥ 2`；实测 examined=1、
> 消费条目 2、无决议跳过 210），决议形态 9 项钉板，`cargo test --lib` 2712 passed / 0 failed，IR 快照**零漂移**。
> 变异驱动：消费键偏移一位 ⇒ 判据红 + 真实程序 `error [W1090] ... 产出键未被消费` + 语料门禁差异；还原后复绿。
>
> **五、语义修正的分类依据（D59）**：`Type.method(位置实参, 名字 = 值)` 在改动前**静默给错答案**（实测编译干净、
> 运行输出 `1` 而非 `20`）⇒ **C 类**（旧行为非成功），修法取「响亮」：改动后同一程序编译期 `error [W1090]`；
> 「命名实参参与决议」的终态语义属 **D 类**，已登记 §3.14 并留 issue #452 待专门队伍承接。
> 另：`Drop` 在解释器是 no-op（`backends/interpreter/executor/ops/control.rs` 通配臂仅 `advance()`）——键迁移不改运行行为。
>
> **六、F-1 / F-2 处置（t5 独立验证发现）**：**F-1（medium）** 无绑定消费点（`ir_gen.rs:7766-7790`）在当前语料与测试面
> 不可达、键控正确性无红态可证伪 ⇒ 按「已知边界」登记进 [02](02-stage-contract.md) §2 修订注记第五段（保留理由 + 不新开任务）；
> **F-2（low）** 指针漂移 ⇒ `types.rs` 的 `OverloadResolutions` 文档注释改为精确指针（产出 `inference/expressions.rs:4550`、
> 消费 `ir_gen.rs:7697` / `:7772`），单独一笔小提交，未混入文档提交。
>
> **七、收口（t7）**：02 义务字段表两行改为 `PlanId` 键 + 当前消费点行号（`:360` 读 / `:3103` 用 / `:1095` 与 `:1097` 结算标记）；
> 07 判据表与 04-ssa 缺陷类 3 改过去式；09 勾选 4.5 / 4.5.1 / 4.5.2；CHANGELOG 记一行行为变化；`check-obligations.py` 与
> `check_tracking.py` 绿、CI 与 Deploy Docs 绿。**收口发现的记账偏差**：t3 / t6 报告里的指针 `ir_gen.rs:357` / `:3102` / `:1096`
> 与代码事实差一至三行（实为 `:360` / `:3103` / `:1095`+`:1097`，4.5.2 的插入使然）——以代码事实为准登记，本注记同时留痕。

## P4 4.10.3 standalone run 与 MultiFile 编排统一（4.2.5 裁决 A 的 run 半边）

> **丙批实录（2026-10-15 登记 → 2026-10-15 收口）**：用户 2026-10-15 裁决丙——本轮只做 4.10.3
>（同批候选 3.4.6 按 B1（[02](02-stage-contract.md) §运行时行为表「死代码族警告不进 compile 路径」）
> 缓议至 4.9 后，前置已改挂并注记）；实施形态 = **完全体单队**（用户指示不开 DAG、不建团队，
> 单会话实现 + 自验）。批前欠账先行落地：#455 依赖公告修复（rustls 0.23.46 + serde_yaml_ng，
> `cargo audit` 由 exit 1 恢复 exit 0）、WBS 登记债（#456 登记为 3.17 + 未完成行补
> 「归属阶段 / 是否阻塞主线」字段）。
>
> **一、先红后绿**：driver 级红判据 3 条（`src/driver/tests/run_rootless.rs`——rootless
> merged IR 不得带 entry_function（红态 `Some("main.main")`）、rootless 顶层可执行语句不得报
> E3023（红态不产 module）、Bin 对照 entry_function 保持（绿态常驻））+ CLI e2e 3 钉
>（standalone 相对导入 run（红态 E5001）、Script 顶层语句执行与 #413 提示两枚绿态钉板）。
>
> **二、实现面**：CLI `run` 统一走 `compile_project`（`run_file_with_diagnostics` 的项目/
> standalone 二分岔删除）；Linking 的 `entry_function` 按 `is_bin_role` 门控（#413 T4——
> Script 不自动调 main）；#413 提示键控由 `module.source_files.is_empty()` 改为调用点项目
> 探针（统一后 ModuleIR 恒带 source_files，原标记失效）；`script_main_never_called` 适配
> 限定名形态（函数 `{key}.main`，全局槽位实测同为限定名——以 IR 取证为准，非凭注释推断）。
>
> **三、统一暴露的既有缺口（同批顺路修，A/B/C 类——仅把失败程序修成成功，原先成功程序零影响）**：
>
> 1. **显式类型实参预特化名**（`mk(Int)(5)`）：ir_gen 按 D6.3 直发特化名（mono 会剥未特化
>    原件），`resolve_call_target` 三臂（rename / aliases / 点前缀）都不覆盖 `mk(int64)`
>    形态 ⇒ 调用点悬空 E6006。修法 = 该单点解析器加第四臂（基名限定、类型参数串原样保留）。
>    **Bin 项目同形复现（预存在缺陷，非本批引入）**。
> 2. **Script 顶层 `mut` 赋值语句重复配槽**：`extract_global_defs` 与
>    `register_global_slots` 把 `counter = counter + 1`（与声明同形的 Assign）逐语句
>    计数 ⇒ 同一变量配 3 槽、初始化写后配的槽而读取用先配的槽（运行期 Add 类型错配）。
>    修法 = 两处按名合并（同一判据同源）。
> 3. **同模块约束泛型运算符派发**（RFC-011 §5.2）：mono 派发改写产裸名 `Vec3.add`，
>    merged 表实为 `{key}.Vec3.add`。修法 = `qualify_deferred_method` 确定性候选 +
>    查表验证（精确命中 → 泛型所属单元键限定命中 → 回退裸名现状；非后缀启发式，与
>    `resolve_generic_name` 同形态）。
>
> **四、登记缺口（本批不修）**：约束派发在**类型与泛型定义不同模块**时仍 E6006（rootless
> 双文件探针实证；Bin 同形）——根因 = MonoType 用户类型名不携带定义模块，正确修法属 P6
> 类型身份收敛；登记 WBS **3.18**（归属 P3、前置待 P6、不阻塞主线），
> `qualify_deferred_method` 注记同面。
>
> **五、测试适配（诚实登记）**：`tests/integration/multifile.rs` 夹具补 manifest——其测的
> 是 RFC-029 项目编排（Bin 自动入口）而非 Script 语义；`cli_e2e` 一枚钉板（顶层诊断变量名
> 可见性）与 `script_main_hint` 夹具（7 例）按限定名形态同步——生产消费端只吃 merged IR，
> 夹具直驱同形态，不用管线产物替身。
>
> **六、最终门禁（收口 revision 084eaf87b）**：fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2733 passed**（+3 新钉）/ `--test integration` **287 passed** /
> check-obligations PASS（13·13·0·8）/ check-docs-truth exit=0 / check_tracking exit=0 /
> 边界三门禁 report-only / markdownlint 0 / pnpm docs:build 0 / IR 快照零漂移 /
> 语料差分基线再生 **12 行**——逐行结构化核对：exit / stdout / 诊断码全部逐条相同，仅
> 栈帧函数名与变量名按单元键限定（人工 review 通过）；再生后 `check-corpus-parity.py` exit=0。
>
> **七、过程事件（如实登记）**：本批一次在 pre-commit 钩子运行期间并发编辑工作区文档，
> 被钩子的 stash 回滚吃掉且该次提交失败（配置注释里预告过的并发形态）——纪律 = 钩子
> 运行期间不动工作区；#455 的提交因此拆成两次尝试（第二次全绿）。

## P4 4.6.1 LSP 语义数据管线统一（D55 收口）

> **4.6 批实录（2026-10-16 登记 → 2026-10-16 收口）**：用户 2026-10-16 指示按 4.6 → 4.9 →
> 4.4 → P5 顺序推进、先做 4.6；实施形态 = **完全体单队**（沿上批用户指示：不开 DAG、不建
> 团队，单会话实现 + 自验）。任务定义（09 三级表）：项目内文件的语义数据（SemanticDB）走
> orchestrator 同管线、与诊断同源；跨文件引用带 `resolves_to`。验收：项目内跨文件跳转命中
> 定义；语义数据与诊断出自同一次编译。
>
> **一、现状实证（先查证再断言）**：didOpen/didChange 触发**两趟**编译——诊断走统一 Driver
> 管线（4.2.3/4.2.6 已统一），语义数据走 `src/lsp/server.rs` 的手工 lex→parse→
> `TypeChecker::new(uri).check_module_collect_all`（无模块注册表 ⇒ 项目内 `use` 解析失败、
> 跨文件引用事件不产生，跳转/重命名/引用查找对导入符号静默落空——D55 登记的静默失败）。
> 编排臂的 checker 统一命名 `"<module>"`（SemanticDB 文件身份丢失），`type_result`（含
> semantic_db）在诊断非空时被丢弃且从不外发；`checker.rs` 排空把 `resolves_to` 的 DefId
> 文件身份恒盖成当前文件；`Export`/`ModuleInfo` 均无定义位置面。
>
> **二、先红后绿**：LSP e2e 钉（`src/lsp/tests/cross_file.rs`）——temp 双文件项目 didOpen
> main.yx 后对 `add_one` 调用点发 GotoDefinition，**修复前返回 None（红，实证上节静默失败）**，
> 修复后命中 lib.yx 第 0 行定义（绿）。过程中一次探针定位：编译器侧链路（driver 探针 dump）
> 已全程贯通而 e2e 仍空——根因是 `resolve_reference` 按引用**起点**精确匹配（D55
> precise-only 现状口径），测试光标落在标识符中部；该容差属既有行为面，本批不动（见七）。
>
> **三、实现面**：(1) `Export.definition_span`（extract_module_info 自 AST 标识符节点收割，
> 与 typecheck 定义事件的 `def_span` 同源同值；native std 恒 None）+ `ModuleInfo.
> definition_file`（Registry 臂按 DiscoveredFile 填，含 vendor；std 恒 None）；(2) checker
> 引用排空经 `import_def_locations`（use 行绑定 span → 定义文件+定义 span）把跨文件导入项的
> `resolves_to` 重定向出本文件；std 导入无登记，维持落 use 行；(3) checker 模块身份由统一
> `"<module>"` 改为单元文件路径（实证 `env.module_name` 全部 4 个消费点零诊断消费）；
> (4) `DriverOutcome.semantic_db` 通道——CollectAll 形态（Lsp 目标文件**无论诊断成败** +
> SingleFile 兜底）汇入，Lsp 臂尾从 registry 注入非目标文件顶层导出定义面
> （`external_definitions`，未打开的文件也能跳转；类型/子模块导出不注入——typecheck 侧
> 类型注册不产生定义事件，两侧保持一致的空缺）；(5) `orchestrator::analyze_source_in_
> project`（`check_source_in_project` 转调取诊断半边）；(6) LSP `analysis.rs` 单编译入口
> 取代手工管线（`update_semantic_db` 删除），World 键统一规范化 file URI（`uri.rs`：
> `path_to_uri(uri_to_path(u))` 幂等归一，盘符大小写与 %XX 编码差异归一到同键，跨文件
> DefId 因此稳定命中）；5 个 handler + semanticTokens 双请求接 `db_key`。
>
> **四、顺路修（D59 分流 B 类——旧行为静默不动作，原先成功程序对外表现零影响）**：
> ① 项目内文件跨文件跳转/重命名/引用查找自静默落空转为命中真实定义；② 词法错误时旧手工
> 管线删除语义信息（染色消失）→ 残缺 AST 染色保留（4.2.6 编辑器哲学延伸）；③ 目标文件带
> 类型错误时语义数据不再随 `type_result` 一并丢弃（打字中间态染色/跳转不消失）。
>
> **五、钉板**：driver 级 6 枚（`src/driver/tests/lsp_semantic_db.rs`——目标文件路径键+同源
> 诊断通道、跨文件 resolves_to+外部定义面往返、std 导入回退 use 行、错误文件保留、单文件
> 兜底路径键、单文件错误保留）+ LSP e2e 1 枚（先红后绿）+ uri 规范化 4 枚
> （`src/lsp/tests/uri.rs`——盘符换算、空格编码、幂等归一、非 file scheme 透传）。
>
> **六、变异验证（scratch.py trial，机械可逆）**：批次树 wbs-46 @ 收口 revision 39546020，
> 两枚变异均使命中钉板转红、还原后 blob 与 HEAD 一致 + git status CLEAN——
> m1 摘 `import_def_locations` 登记（statements.rs）⇒ 跨文件 driver 钉 + LSP e2e 钉红、
> 其余 7 枚绿；m2 摘 `external_definitions` 注入（arms.rs）⇒ 同两钉红（往返解析失锚）、
> 其余 7 枚绿。批次树已 gc（释放 4.8 GB，`status` 无批次树）。
>
> **七、最终门禁（收口 revision 39546020）**：fmt --check 0 / clippy -D warnings 0 /
> `cargo test --lib` **2744 passed**（+11 新钉）/ `--test integration` **287 passed** /
> 语料差分**逐字节零漂移** / IR 快照零漂移 / check-obligations PASS（13·13·0·8，
> `semantic_db` 非义务理由的消费者锚点同步 server.rs:482 与 02 §义务字段表）/ check-docs-
> truth 0 / docs-examples 258 条 0 失败 / docs-orphan 0 / markdownlint 全仓仅剩用户未提交
> 的 032 两处 MD056（用户自有编辑，未动）/ pnpm docs:build 0。**过程事件（如实登记）**：
> en/09a-wbs-history.md:54 一处 MD037 为 auto-translate bot 重译上一批 09a 时引入（zh 原文
> 无空格），本批顺手修回——markdownlint 红非本批改动面但与 09a 同文件，一并登记。
> **遗留观察（登记待议，未动）**：`resolve_reference` 只按引用起点精确匹配——光标落在
> 标识符中部时跳转/悬停/重命名静默不动作（D55 precise-only 现状，全文件形态同构），
> 如需容差匹配属独立行为变更，报用户裁决。

> **同日续批（4.6.2，用户裁决后落地）**：收尾报告交付后用户就「遗留观察①」裁决
> 「可以，那就改」——查库键容差按方案甲实施：definition/hover/references/rename
> 四 handler 的查库键由光标位置（经 UTF-16 换算）改用 `find_identifier_at_position`
> 已定位出的**标识符自身起点**（与语义事件同坐标系，免换算）；`resolve_reference`
> 的起点精确匹配语义不动（D55 precise-only 不变）。references/rename 的定义点回退
> 判定（start <= col < end）对标识符起点同构成立。先红后绿：definition 中部光标钉
> 先红（None）后绿；四 handler 中部光标各 1 钉 + 跨文件 e2e 起点/中部双位置断言，
> `--lib` 2748（+4）全绿、clippy -D warnings 0。登记 WBS 4.6.2（三级计数 150→151）。

## P4 4.9 引用池正确实现（RFC-029f 修正）— 轮 1 收口（4.9.1 / 4.9.2 / 4.9.3）

> **4.9 批实录（2026-10-16 登记 → 2026-10-16 收口）**：用户 2026-10-16 裁决将 4.9.1 / 4.9.2 / 4.9.3
> 三条相关三级任务**并为一支团队一轮**（理由：它们同属「池的来源与身份」一面，合并落地可避免
> `collect_project_refs` 删后的中间态反复重写；4.9.4 / 4.9.5 分属消费细化与测试池分离，
> 依赖 4.6.1 的 `use` 路径解析，强行同批会扩大闸口面）。#450 归属**待裁决**——证据
> 表明属「过载解析后名字未传回引用计数」类，仍在轮 1 已落地形态下复现，按独立 WBS 条目登记处置（队长推荐 A：并入轮 2 时登记为独立条目；**待用户裁决**）。冻结 rev = `3863a9ff0`（父提交 `62f6e1a59` 为修复前基线）；验证 PASS attempt
> `aab92fe0`（即 t2）；复审 pass attempt `96452696`（即 t3）。**收口登记提交 = `38d30b846`（docs(docs): 4.9 引用池轮 1 收口登记）**——该笔实际由收口成员 integration-refpool 编写并推送；其收尾探针曾把收口任务误标为 terminal failed 并覆盖 output，由队长接管新 attempt 按已核实交付事实改回 completed，特此更正归属与标签。证据留档：
> 证据摘要（8 份变异补丁清单、`warn_compare_v8.json` 的口径与结论）已内联于本节「三、+8 与 +16
> 两套计数的对应关系」与「四、变异试验」；原始文件随工作区回收，**不入库、不得作为文档引用出处**
> （AGENTS 元规则 ①）。
>
> **一、合并裁决**
>
> 1. **裁决本身**：三条相关三级任务合为轮 1。**4.9.4（消费细化）/ 4.9.5（测试池分离）下轮**——
>    前者依赖 4.6.1 交付的 `use` 路径解析（已于 `39546020` 落地，可启动）；后者与 RFC-029f 表格
>    收编同批，登记 RFC-029f 末节待改。
> 2. **理由**：(a) 4.9.1 / 4.9.2 / 4.9.3 互为前置（4.9.2 改池类型 → 4.9.1 装配面要重写 → 4.9.3 才
>    有分母判定的角色表），分轮做必然要写一次中间态后改两次；(b) 三条都属于「池的来源与身份」
>    一面，4.9.4 / 4.9.5 属另一面，分批是按「内聚」而非「可独立 revert」做的切分。
> 3. **合并结果**：单提交 `3863a9ff0`（一行 `:recycle: refactor(driver)`），单次推送，单批门禁。
>    **哈希说明**：推送前 `git pull --rebase` 并入自动翻译 bot 提交（`72f364169`）；rebase 落点在对方提交之后，
>    本批两笔（实现 `3863a9ff0`、其修复前基线兼对方文档提交 `62f6e1a59`）**未被重写、保持原哈希**——初版把这里
>    误写成「`3863a9ff0`→`3863a9ff0`」式恒等映射（形似「重写后恰同值」，实为「未被重写」），2026-10-17 收口时更正；
>    对方代码提交 `33f14e8ab`→`b122b7ad1`。**对外引用一律以 push 后的哈希为准**（被重写的旧哈希在新克隆中不可达，
>    同一性以补丁 blob 比对判定）。
>
> **二、五条判据的红/绿实测口径**
>
> | # | 判据 | 红态（修复前 / P1 目录扫描回潮补丁） | 绿态（轮 1 落地） |
> |---|------|------------------------------------|-----------------|
> | ① | 池的根 = 包目标（#334） | 目录递归扫描 ⇒ tests/ 下的文件标识符也入池，豁免生产代码未使用定义 | `collect_package_refs`（`helpers.rs:481`）按 `ExplicitSurfaces` 根调 `discover_with_used`；全根退化时退路 `collect_compilation_refs`（`helpers.rs:454`） |
> | ② | 池粒度 = `(module, name)` | 同名局部变量（`let foo = …`）意外豁免同名顶级定义（`fn foo()`）；a.yx 的 `foo` 和 b.yx 的 `foo` 串味 | `HashMap<String, HashSet<String>>`（键 = 模块键）；`collect_ref_pool`（`helpers.rs:429-449`）装配唯一入口；`arms.rs:603-609` 的 `other_module_refs` 把「除本单元外」的全部模块条目并集后送 `DeadCodeAnalyzer`；模块自有名（顶层定义与局部绑定，`use` 导入名例外）不再豁免别模块同名定义 |
> | ③ | `FileRole::Script` 不用池 | Script 受项目内其它文件的引用影响 | `arms.rs:611-623` 的 `match us.role` 只对 `Bin`/`Internal`（`set_exempt_pub`）与 `Lib`（`set_cross_file_refs`）装配，Script/Test 走 `_ => {}` 默认臂（按文件内定义-使用图判定） |
> | ④ | 夹具 A：lib.yx 第 4 行定义须报 W1001 | 修复前：报 `W1003@1`（实际「未使用导入」），不是 W1001 | 红态实测（红 1 夹具 red1）报 `W1001@3`；绿态所有 6 夹具 PASS |
> | ⑤ | 夹具 B：main.yx 第 4 行定义须报 W1001 | 修复前：报 `W1003@1` | 同上 |
>
> **判据②的「推演红」**（实现者自陈 + 验证者闭合）：red2 夹具的 `let foo = …` 在 helper.yx；
> 旧池在裸名并集下把 `foo` 整体当成「被 helper 用了」，lib.yx 的 `foo` 豁免。**实测**：red2
> 夹具 vs 修复前二进制 → 无 W1001（旧口径豁免）；red2 vs 修复后二进制 → W1001@1。
>
> **判据③**（Script 不用池）的红态夹具**首版失败**：首版夹具在同文件里放同名局部变量
> 触发 E2010（该语言不许同模块遮蔽顶层定义），改到另一模块（helper.yx）后为推演红态。
> 这一点 t3 复审与 t2 验证均独立复现，**非契约缺陷**——同模块遮蔽是该语言的硬约束，夹具
> 必须跨模块。
>
> **三、+8 与 +16 两套计数的对应关系**（**以 t3 复审证死**为准）
>
> 4.9 轮 1 证据摘要的 C 节写 `+16/−0`，但漏了「**新夹具记录**」这一
> 成分——v8 仪器（`warn_compare_v8.json`，331 文件 symmetric diff，`only_before=0`）的口径
> **含**新夹具；t1 实现者自建的「+8」是 **warn-before / warn-after（各 325 条）按夹具
> 聚合**（每个成员文件 check 一次）。复审逐条映射：
>
> - **+8**（`8` 条新警告，全部是 `check <非入口文件>`，`check <入口文件>` 零变化）
>   - `cross-file-method/lib.yx`：`W1002@2(Point)`、`W1005@4(get_x)`、`W1005@8(norm_sq)`
>   - `subdirectory-module/util/math.yx`：`W1001@2(double)`
>   - `type-error-in-lib/lib.yx`：`W1004@2(value)`
>   - `use-constant/lib.yx`：`W1004@2(value)`
>   - `use-type-and-function/lib.yx`：`W1001@4(distance)`、`W1002@2(Point)`
>   - **成因（统一归因）**：这 8 条夹具均**未声明包目标** ⇒ 4.9.1 根退化为被检文件自身编译集，
>     消费者 main.yx 不在其中；本就是设计内变化（before 时这些文件的 check 面本就零警告，
>     非豁免路径）。仓库根 `yaoxiang.toml` 为空文件 ⇒ 同一退化路径实测成立。
> - **+16 残差 8**（检查面 = `ref-pool-compile-set/{lib.yx,main.yx,stray.yx}` + `ref-pool-module-identity/{helper.yx,lib.yx,main.yx}`）
>   - 这些检查面文件在 t1 的 `warn-before.jsonl` 中 `exit=None`（拍摄时点**不存在**——是本批新增的
>     夹具）。故 +8 的「+ 0 − 0」差分里没有它们，+16 的 v8 对称差分里有。
>   - `v8` 的 `stray.yx` 只剩 `W1001@2(probe)` 一条：原因是修复前二进制同样报 `probe@stray.yx:4`
>     （该夹具对每个文件跑一次 `probe`），**相消**后只剩 lib 的 1 条——三份数据（+8 / +16 / 双二进制
>     ABBA）互洽。
> - **新夹具 8 条** vs **+16 残差 8 条**：同一批文件（4.9.1 / 4.9.2 新加的 `ref-pool-compile-set/`
>   与 `ref-pool-module-identity/`），仪器把它们的每次 check 当成独立记录——同一文件可能对多个
>   `角色（role）/ context` 触发不同诊断条目。
>
> **结论**：**+16 = +8（按夹具聚合）∪ 新夹具 8 条**——两种计数都正确，**口径不同**：t1 报
> 「新增 8 条」是夹具层口径，t2 报「+16 条」是文件层口径（symmetric diff 含新夹具）。
> t3 复审 ① +8⊆+16 已证死；**复审 ② 写「不要照抄 EVIDENCE.md C 节」**——C 节漏了
> 「新夹具记录」成分，本条据 t3 复审 `pass` 结论改写。
>
> **四、变异试验（机械可逆，全部 `scratch.py trial`）**
>
> 8 份变异补丁（p1 目录扫描、p2b use 导入名剔除、p3b script
> 接池、p3 flat 消费、p4 std 接池、p7final 自建 4 判据、p8 std 关闭、p9 vendor 接池 +
> p_stack2 / p_stack3 叠加面）。每个 trial 输出含：HEAD blob → 变异后 blob → 还原后 blob
> 与 HEAD 一致 = 是 + `git status` CLEAN。本机 `core.autocrlf=true` 需经 `GIT_CONFIG_*`
> 注入 `false`（`trial_env.ps1`），否则补丁新增行转 CRLF 导致 apply/还原双失败——**只影响
> 补丁机械性，不改被测行为**。
>
> **五、过程事件 / 环境教训（移交下轮必读）**
>
> 1. **core.autocrlf 注入**：本机 `core.autocrlf=true` 必须经 `GIT_CONFIG_*` 注入 `false`
>    才能保证补丁机械可逆；当时的注入脚本随工作区回收（判据已在上条列出，可据此重写）。**下轮不要复用**
>    任何不含注入的 trial 脚本。
> 2. **同树 trial 串行**：同一棵树上的 `trial` 与 `cargo build` **必须严格串行**。`trial`
>    内部 `git apply` 与 `cargo build` 都要触碰 worktree，并行会出现「修补未完成就被 build
>    看到」或「build 与 apply 互锁」两类竞态。
> 3. **trial 重建二进制需确认同步**：`trial` 完成后会重建树内二进制（cargo 的 fingerprint
>    含源码路径，**换树 = 全量重建**），后续量测前必须确认 `bin/yaoxiang-rs` 与 HEAD 同步。
>    **本轮曾用残留变异二进制测得假数据**，已重建重测修正。
> 4. **第二棵树 yx49-b 的建树理由**：对拍需要修复前二进制（62f6e1a59），批次内无现成品；
>    `scratch.py init yx49-b` 开了第二棵树（再吃一次全量构建）。**树已 release**（详见六），
>    删除留批次收口 `gc` 处置。
> 5. **cli_e2e 两条 Internal 用例夹具补成合法包（补 `[run].main`，断言未动）的理由**：
>    修复前 `cli_e2e` 中两枚 `Internal` 角色钉板夹具**未声明 `[run].main`** ⇒ 4.9.1 落地
>    后 `compile_project` 走的是 `ExplicitSurfaces` 根调路径，找不到 `main` ⇒ 整批 2 夹具
>    行为从「按 Script 兜底」漂移到「找不到入口」——是 4.9.1 引入的退路行为而非缺陷。
>    补 `[run].main` 字段后两夹具走正常包路径，**断言**（exit code / 诊断码 / 警告集合）
>    逐字未动。
> 6. **第二轮树删除的安全方式**：release 后用 `python scripts/dev/scratch.py rm yx49-b`
>    （不是 `rm -rf`），会校验「无挂载」后再删。本批收口前已 release，下一步 `gc` 自然清理。
>
> **六、批次树状态**
>
> 收口前：yx49 已 release（4.8 GB 释放），yx49-b 已 release。两棵树无挂载，状态可在 `scratch.py status` 复核。本任务输出末附 `scratch.py gc` 完整 status。
>
> **七、RFC-029f「已知缺陷」表翻转**
>
> 上一版（2026-10-09）以现在时描述「引用池的三处设计缺陷」（含 `collect_project_refs` / `orchestrator.rs:603-648` / `orchestrator.rs:393-396` 等已删代码位置）。本批收口同步改写为「**已知缺陷：已落地（2026-10-16，WBS 4.9 轮 1）**」：① 现池的根 = `collect_package_refs`（`helpers.rs:481`），按 `ExplicitSurfaces` 解析，不扫目录；② 现池类型 `HashMap<String, HashSet<String>>`（键 = 模块键），装配唯一入口 `collect_ref_pool`（`helpers.rs:429-449`），消费端 `arms.rs:603-609`；③ 现 Script 不接池，`arms.rs:611-623` 的 `match us.role` 默认臂。**所有 file:line 均在被引用 revision `3863a9ff0` 上重取**。`collect_project_refs` 与 `orchestrator.rs` 已随 P4 4.2.9 拆分（`c59fb751`）移入 `src/driver/{state,arms,helpers}.rs`，表中旧位置不再命中——这是「声明与事实不符」的修复（t3 复审备注①）。
>
> **八、最终门禁（收口 revision `3863a9ff0`）**
>
> fmt --check 0 / clippy -D warnings 0 / `cargo test --lib` 2754 passed / `--test integration` 287 passed /
> `yx_multifile_runner` 全绿 / 语料基线**仅 +2 行**（两条新夹具 behavior run，338a339,340；另 342 行
> 逐字节不变）/ IR 快照 204 条零漂移 / check-obligations PASS（13·13·0·8）+ selftest /
> check-docs-truth 0。**过程事件**：本批一次 cargo 构建因 `core.autocrlf` 把脚本行尾
> 转 CRLF，team scratch 路径下 `yaoxiang-rs` 启动报 `BadImage`，实测为编码与 git 索引不一致
> ——重建后通过（与 4.6 批「外部整树 discard」教训并行、原因不同）。
>
> **九、遗留（移交下轮）**
>
> 1. **#450 归属裁决**：证据表明属「过载解析后名字未传回引用计数」类，**仍在轮 1 已落地
>    形态下复现**——按用户裁决丙流程分派。**#450 探针**：双二进制同集合 `{W1002@2, W1005@4}`
>    （实测，rootless 形态），与轮 1 改造无直接关系，但 #450 的修法可能要等 4.9.4（消费细化）
>    一起做——因为根因是「过载解析后名字传引用计数」需要 use 项的精确消费面，4.9.4 的
>    「被 import 后真实使用」是同类判据的精确版。
> 2. **dead_code.rs:683 指路 nit**：注释写「两池的装配口径见 driver 侧 collect_compilation_refs」
>    ——实际 `collect_compilation_refs` 是退路路径，主路径是 `collect_package_refs`（`helpers.rs:481`）。
>    注释与代码主路径口径不齐（**不在本批写范围**），留给 4.9.4 / 4.9.5 落地时一起改正。
> 3. **4.9.4 / 4.9.5 前置已解锁**：4.6.1（`use` 路径解析）已由 `39546020` 交付；4.9.4 可立即
>    启动。4.9.5（测试池分离）需先定 RFC-029f 表格收编（已由本批「七、」翻转铺垫）。
> 4. **`yaoxiang check` 多文件 `run` 永不报死代码族警告**（WBS 3.4.6，2026-10-15 用户裁决丙缓议
>    至 4.9 后）——本批轮 1 未涉及；下一轮 4.9.4 / 4.9.5 落地时同批处理。

## P4 4.9 引用池正确实现（RFC-029f 修正）— 轮 2 收口（4.9.4 / 4.9.5 / 4.9.6）

> **4.9 轮 2 批实录（2026-10-16 实现 → 2026-10-17 收口）**：覆盖 4.9.4（消费判据 = 被 import
> 后真实使用）、4.9.5（测试池分离）、4.9.6（方法可达性口径，issue #450，用户 2026-10-16
> 裁决 A 并入轮 2）。收口修复（验证侧补救 M1/M2/M4 可编译重做 + F1 修复轮 + PA-2 删死分支）
> 与登记缺口 issue #464（WBS 4.9.7）一并落定。
>
> **一、判决链与提交（一律 push 后真值）**
>
> | 步 | 内容 | push 后提交 | push 前本地值 | 判定方式 |
> |---|------|------------|--------------|---------|
> | 1 | fix(util)：check 显式路径不被默认排除表拦截 + --exclude 恒空（issue #463，P0-0.4） | 366bc5a69 | ef959ca55 | 补丁 blob 相同 |
> | 2 | 阶段 A = 4.9.4 + 4.9.5（RefPool 按 import 归因 + Test 不贡献消费面） | e6cf1aeaa | 84a6f6978 | 补丁 blob 相同 |
> | 3 | 阶段 B = 4.9.6（#450 逐候选方法可达性；含生成物 warning-codes.md） | ddcbb12d0 | cd629b5fe | 补丁 blob 相同 |
> | 4 | docs(meta)：AGENTS 第 10 条⑦（提交期间 stash 窗口约定） | ec94fa9d8 | 9a4ed5f3b | 补丁 blob 相同 |
> | 5 | 修复轮 = F1（重载兄弟重载恢复报 W1005）+ F2（验证侧 M1/M2/M4 可编译重做）+ F4（快照代次注明） | b8380e6ab | 3d9ef6eab | 补丁 blob 相同（36962f318e42…，git range-diff 无差异） |
> | 6 | 收口修复 = PA-2（删 strip_unit_prefix 死分支 + 029f/CHANGELOG 三处失实表述纠正，登记 #464） | b940300ac | e13984465 | 补丁 blob 相同（patch-id 9bda3cfe4c…） |
>
> **哈希说明**：每笔推送前的 git pull --rebase 会重写本地哈希；上表「判定方式」列均为
> 「补丁 blob 相同」（`git diff <rev>~1 <rev> | git hash-object --stdin` 两侧一致 +
> `git range-diff` 无差异）。本篇不引用任何 push 前哈希作权威出处。
>
> **二、三条判据的红/绿实测口径**（验证者自建复现，不采信实现者判据）
>
> | 判据 | 红态（before 二进制 366bc5a69 实测） | 绿态（ec94fa9d8 及之后） |
> |---|---|---|
> | 4.9.4 import≠消费 | 声明面项目 check a.yx 仅 1 条 W1001（main 的 use b.{foo} + foo() 把 a.yx 同名 foo 救活；guard 只被 import 未使用也被救活） | 同夹具 2 条如实报（撞名不救活 + 未使用导入不算消费）；ref_pool 8 passed |
> | 4.9.5 测试池分离 | src/util.yx 的 debug_print 被 tests/ 引用救活（仅 1 条） | 同夹具 2 条（测试引用不外溢）；ref_pool 的 test_ref_pool_excludes_test_role_references passed |
> | 4.9.6 方法可达性 | 自建夹具（2 组消歧重载调用 + 1 未调用）3 条 W1005 误报；method_call_args.yx 3 条 / method_overload.yx 1 条（重载按 full name 覆盖只报 1 条） | 同夹具恰报未被调用的 orphan；method_unused 3 钉 passed；语料 call_args 3→0 / overload 1→0 |
>
> **修复轮（F1）再收紧**：未命中决议的兄弟重载恢复报 W1005（b8380e6ab——同一夹具
> 调用 ->Float 时报 ->String 兄弟、调用 ->String 时报 ->Float 兄弟，双向行号钉）。
> **回落边界（登记 #464）**：跨单元（registry 解析）方法调用不产生决议条目，回落
> 名字级豁免，跨单元同名同序此前不被报——登记 4.9.7。
>
> **三、过程事件与教训（四条硬要求对应）**
>
> 1. **PA-1 由 blocker 改判为既有能力缺口（medium）**：轮 2 复审曾把「跨单元同名同序不被报」
>    登记为 blocker（PA-1）；其后两次事实对账（实现者留档 + 验证者在 b8380e6ab/b940300ac 双二进制
>    复测同一夹具集，诊断集合逐字节一致）确认它是**轮 2 之前就存在的能力缺口**而非本轮回归——
>    机制 = 跨单元方法调用不产生决议条目，回落名字级豁免；处置 = 改判 medium、登记 issue #464、
>    WBS 4.9.7 排期下一轮。
> 2. **复审者公开更正 before 数据并撤回回归判定**：复审者最初引用的 before 数据（「−42 条中
>    含被调用方法」）与其原始输出（W1005 count=0）不符——验证者核对后确认 before 抓拍为 0 条、
>    复审者的回归判定据此**公开撤回**；−42 的正确口径 = 26 文件的被调用方法误报消除（阶段 B），
>    逐条可追溯到 behavior-run exit=0 的文件（方法真实被调用执行）。
> 3. **t13 因契约前提不成立终止（非实现缺陷）**：该任务的验收前提写的是「跨单元同名同序
>    会报 W1005 的回归修复」，而实测（t14 双二进制）确认该行为是已登记缺口（#464）而非回归——
>    契约前提不存在，任务终止；其探针价值由 t16 只读判定承接（确认 `strip_unit_prefix` 是死分支）。
> 4. **文档提交的真正判据是「真的构建一次」**：轮 2 收口的文档提交（`042c07860`）让 Deploy Docs 红在 VitePress 的 Vue 模板编译——实录里的 `<rev>` / `<before-exe>` / `<after-exe>` 等尖括号占位符没进反引号，被当成元素解析（报 `Element is missing end tag`，指向 `<rev>` 那一行）；markdownlint 抓不到这一类，且 pre-commit 的 markdownlint 钩子在仅含 docs 的提交里被判 `no files to check` 跳过（实测）。处置：占位符一律进反引号，推送前本地跑一次 `npm run docs:build`（修复提交 `0c837e1b4` 以本地构建 exit=0 佐证）；若用 `--no-verify` 提交，commit-msg 校验需手动补跑。
> 5. **契约描述未同步的流程疏漏**：t14 的任务描述文本残留 PA-1 旧假说（写「b.yx 报 W1005」
>    的期望），验证者在报告里主动指出「按 contract 字段判，描述文本不计」并请队长确认——处置 =
>    以任务契约字段为判决依据，描述文本留作流程教训（**改派/派工时任务描述必须与最新判决链同步**）。
>
> **四、变异试验（机械可逆，全部 scratch.py trial）**
>
> 轮 2 验证者共 9 份补丁（m1b/m2b/m4b 为 F2 后的可编译重做版——
> 初版 M1/M2/M4 把 if 条件行注释掉留下悬挂块体，被 t8 复审 F2 判「编译失败误记为判据翻红」
> 而作废；重做版经逐补丁 cargo check exit=0 + trial 翻红）。M3（撞名归因回潮）初版即为真变异、
> 实测 ref_pool 5 条判据翻红，保留原判。M1b 锚点在 b8380e6ab 上重取（F1 修复后 probe 段重构，
> 旧锚点失效）。全部 trial：HEAD blob → 变异后 blob → 还原后 blob 与 HEAD 一致 + git status CLEAN。
> 环境注入沿用 trial_env.ps1（core.autocrlf=false 经 GIT_CONFIG_*）。
>
> **五、警告面全量对拍（口径先讲清）**
>
> 仪器 = warn_surface_r2.py（自建解析器，(文件, 码, 行, 列) 四元组 symmetric diff，项目态通道
> cwd=文件父目录，全路径键）；命令 = `python warn_surface_r2.py corpus_listing_r2.txt <before-exe> <after-exe>`
> （337 文件 × 2 二进制）。**口径**：本轮 before 抓拍已含全部夹具 ⇒ matched set = 全集
> 337，不存在轮 1 的 +8（matched 增量）/+16（全集）双口径问题。结果：only_before = 42 条 W1005
> （26 文件，全部为被调用方法的误报消除 = 阶段 B 的 −42；逐文件核对其全部在 behavior-run exit=0
> 基线上，方法真实被调用执行——**无一条真死代码被救活**）/ only_after = 3 条 W1001（ref-pool-import-use
> 的 a.yx:4 检查面 ×2 + ref-pool-test-pool 的 ../src/util.yx:2 = 阶段 A 判据面）。**快照代次（F4）**：
> 阶段 A 时点的 after 抓拍（阶段 B 前的 57 条残留）与终版（阶段 B 后的 15+2=17 条）是**两个代次**，
> 引用时必须注明；另有 1 条例外（interface_member_conflict.yx 的 Dog.name@18）属字段访问形态、
> 不在阶段 B 的 W1005 移除集内，前后一致。
>
> **六、最终门禁（收口 revision b940300ac）**
>
> lib 2761 passed / integration 288 passed / IR 快照零漂移（snapshot 7 passed）/ 语料基线零漂移
> （字节数三方可比：基线 = b8380e6ab = b940300ac）/ clippy -D warnings 0 / docs-truth exit=0 /
> check_tracking exit=0 / 生成器已跑无产物改动。**门禁核对安排（第 13 条）**：该批推送的 CI /
> Deploy Docs / Auto Translate run 结论交由定时核对环节登记（run id + conclusion + 连推覆盖关系：
> 后代 run 覆盖前一笔），不在会话内等待。
>
> **七、批次树状态**
>
> yx49 与 yx49-b 均已 release（收口时 gc 统一删除）；仓库外无本批新增目录。

## P4 4.9 引用池正确实现（RFC-029f 修正）— 轮 3 收口（4.9.7）

> **4.9 轮 3 批实录（2026-10-17 实现 → 2026-10-17 收口）**：覆盖 4.9.7（跨单元方法调用的决议
> 登记与接收者归因，issue `#464`——WBS 4.9 的最后一个三级任务；本轮收口后 4.9 整个二级任务
> 看板勾选）。链路 = 产出侧 t19 → 独立验证 t20 PASS → 复审 t21 needs_revision（F1：注入面
> 误报）→ 修复轮 t24 → 独立验证 t25 PASS → 复审 t26 PASS（增量，verdict=pass）。全部提交号
> 为 push 后真值（判定方式 = 补丁 blob 相同；`git diff <rev>~1 <rev> | git hash-object --stdin`
> 两侧一致）。
>
> **一、判决链与提交（push 后真值）**
>
> | 步 | 内容 | push 后提交 | 判定方式 |
> |---|------|------------|---------|
> | 1 | 产出侧：跨单元方法调用产生 D20 决议条目（`feat(typecheck)`，t19） | `b8b3cc3f6` | 补丁 blob 相同 |
> | 2 | 驱动侧归因：方法可达性按定义单元给出标记、有决议记录的名字组停用池的名字级豁免（t20 验证 PASS / t21 复审抓 F1） | `d5c00235a` | 补丁 blob 相同 |
> | 3 | 注入面修复轮 F1：候选注入按「名字 ← 提供模块」**逐对**过滤，消除 E1107 误报（t24，含 F2 表述与 F3 落表单点） | `d5dd1a37b` | 补丁 blob 相同 |
> | 4 | 钉测试归位：`src/frontend/core/typecheck/tests/environment.rs` 的 F3 钉板迁入声明面 `src/driver/tests/overload_injection.rs` | `c2f887843` | 补丁 blob 相同 |
> | 5 | dev 顶端 | `e9675c9dd` | merge 提交，见「二」 |
>
> **二、哈希映射与 dev 顶端的两笔 merge（来源未确认，不许写成员所为）**：轮 3 的四笔提交
> （上表 1–4）均已推上 origin/dev；`git worktree list` 的主挂载在 `e9675c9dd`。dev 顶端存在两笔
> **非本团队命令面产生**的 merge 提交（`5da05b885` / `e9675c9dd`，reflog 显示为 `pull --tags` +
> push 形态），来源未确认——按第 10 条④（引用必须任何人可解析）与第 13 条（门禁交专门核对）
> 的同一次收口口径，此处只登记事实与 reflog 证据，不做成员归因；另见「五、」教训②（push 归属
> 约定必须可核）。
>
> **三、D20 契约扩展的实际形态（键形态与消费点）**：产出侧把「候选 / 混编 IR 名 / 声明期形参名」
> 三张平行表**同序**打包进 `ModuleInfo.method_overloads`（`src/frontend/module/mod.rs:157`）随单元
> 跨界，注册表按模块键升序确定性合并（`src/frontend/module/registry.rs:150`）；typecheck 臂按
> 本单元 import face 逐对注入（`src/driver/arms.rs:354` → `visible_method_overloads`，
> `src/driver/arms.rs:1521-1579`：名字 ← 提供模块逐对、同名冲突整名跳过、自有键位不外并），
> 落表单点 `append_overload_group`（`src/frontend/core/typecheck/environment.rs:399`：三表同序
> 追加 + 等长 debug 断言）；消费端把 D20 决议产物按**定义单元**归因（`MethodMarks`，
> `src/driver/helpers.rs:569-573`；`typed_owners` `:591`；`resolved_method_marks` `:621`），并让
> 有决议记录的名字组停用池的名字级豁免（`src/frontend/core/typecheck/passes/dead_code.rs:252-267`、
> `:825-841`）。键形态 = 条目名 `{Type}.{method}#k`（`{Type}` 为定义侧裸名；`#k` 声明序）。
>
> **四、键作用域与误报防线实测（修复轮 F1 与五条残余边界）**：F1 的红态是修复前 `d5c00235a` 的
> 注入面「名字集合 × 模块集合」组合把另一模块定义的同名类型候选并进同一键位 ⇒ `use a.{Temp}` +
> `use b.{Other}`（a 亦定义 Other、b 亦定义 Temp、方法同名同序）的夹具 `check` 报 2×E1107、退出 1
> （修前成功的程序编译失败）；修复后 `check`/`run` 均退出 0，4 条 W1005 如实报出两侧**未被调用**的
> 同名定义（反向漏报面也核了）。入库夹具 `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`；
> 驱动级钉 `src/driver/tests/overload_injection.rs` 3 条 + `src/driver/tests/method_unused.rs` 轮 2 的
> 5 条 #464 判据与 4 条 #450 旧判据全绿；主验收夹具 `tests/yaoxiang-multifile/ref-pool-cross-unit-method/`
> 诊断集在修复前后逐条一致（未回退）。五条残余边界与逐条实测（单独 `check` 定义文件不报 / 名字层
> 歧义 E5006 整名跳过 / 边界(c) 按 t20 实测改写 / 内联别名与 `ns.T` 不注入候选面 / 整体导入 `use M`
> 与 std 候选面不产条目）全文见 [029f](../../rfc/accepted/029f-target-semantics.md) §跨单元方法可达性
> 归因「残余边界」，此处不重复。
>
> **五、过程事件与教训（四类延续）**：
>
> 1. **契约措辞写窄会直接变成缺陷**：F1 的根因之一是契约只写「只注入能按裸类型名命名的模块面」，
>    没写「按名字逐对归属」——实现按字面取了「名字集合 × 模块集合」的组合，复审才抓出 E1107 误报。
>    契约里描述行为边界时必须把**组合学**写死（按名逐对 / 全组合），不能留给实现自行理解。
> 2. **约定必须可核：「push 由队长做」只写在派工消息里 ⇒ 出现第三方 push/merge 时无法归因**。
>    本轮 dev 顶端出现两笔来源未确认的 merge（见「二」），最终靠 reflog 反推并撤回一次错误归因。
>    可核形态 = 成员不运行 `git push`/`fetch`/`pull`、同步一律 `pull --rebase`、用 reflog 核对。
> 3. **基础设施不稳，先落 patch 再换人**：本批共 5 次不可恢复故障（3×TRANSPORT、1×INVALID_REQUEST（thinking 协议）、
>    1×后台子代理无消息退出）；处置惯例 = 已产出的证据与补丁先落到**工作区**批次目录（`target/` 下，
>    不入库），但**结论与数字必须就地落进本实录**——工作区路径不得作为文档引用出处（AGENTS 元规则 ①）。
>    再换人续做，同一会话连续两次失败不赌第三次。
> 4. **文档提交的真正判据是「本地真构建一次」**：轮 2 收口的文档提交（`042c07860`）让 Deploy Docs 红
>    在 VitePress 的 Vue 模板编译（尖括号占位符 `<rev>` 等没进反引号被当元素解析，报 `Element is missing
>    end tag`）；markdownlint 抓不到这一类，且 pre-commit 的 markdownlint 钩子在仅含 docs 的提交里被判
>    `no files to check` 跳过（实测两次）。修复提交 `0c837e1b4` 以本地构建 exit=0 佐证。本轮该教训的红→绿 run：Deploy Docs **39011946793**（红）→ **39014696984**（绿，详见「六」）。
>
> **六、轮 2 门禁结论补登记（第 13 条，定时核对环节回填）**：轮 2 收口提交 `b940300ac` 的推送——
> CI **39011946778** success / Auto Translate **39011946796** success / Deploy Docs **39011946793**
> **failure**（VitePress 构建红，即「五、4」的尖括号占位符；已由 `0c837e1b4` 修复）；修复提交
> `0c837e1b4` 推送后三绿：CI **39014697000** / Auto Translate **39014697044** / Deploy Docs **39014696984**。
> 连推覆盖关系：CI@`0c837e1b4` 的 **39014356642** 被后继 run 以 concurrency 取消（取消 ≠ 失败，后代 run
> 覆盖前一笔，与轮 2 §六的口径一致）。
>
> **七、最终门禁（收口 revision `e9675c9dd`，本地数字；CI 结论交定时核对，见「八」）**：
> lib 2770 passed / integration 289 passed / yx_multifile_runner 1 passed / IR 快照 7 passed 零漂移 /
> clippy `-D warnings` 0 / docs-truth 0 / 生成器幂等（重跑 `scripts/docs/gen-error-code-docs.py` 后
> `git status` 干净）；语料基线 346 → 347 → 348 行（每笔 +1 条 behavior 记录，其余行逐字节不变；
> 基线 `tests/baselines/corpus-parity.jsonl` 在 `e9675c9dd` 共 348 行）。
>
> **八、门禁核对安排与结论（第 13 条）**：本批推送后的 CI / Auto Translate / Deploy Docs 结论交由定时
> 核对环节登记（不在会话内等待，也不把「没等」写成「已绿」）。**回填（2026-10-17 定时核对）**：
> `d5c00235a` 的 CI `39023417670` = **failure**——Docs Truth 报「错误码文档落后于码表/locales」，
> 即该笔漏了生成物同提交；由其后 `d5dd1a37b`（再生生成物）修复：`5da05b885` CI `39027788246` /
> Deploy Docs `39027788268` / Auto Translate `39027788340` 均 success，覆盖前一笔；
> `e9675c9dd` CI `39028827871` / Deploy Docs `39028827896` success；`d0cfc4e62` CI `39033963240` /
> Deploy Docs `39033963278` / Auto Translate `39033963261` success。三笔的 run 各自跑到终态，
> 本轮无 concurrency 取消。教训：**「本地生成器幂等」不等于「生成物已同提交入库」**——收口前应
> 逐笔核 CI 结论，而不是只看最新一笔。
>
> **九、批次树状态（收口 gc，status 输出）**：收口前 `scratch.py status`——t25（holder=-）与 yx49b
> （holder=-，此前由已停用成员 verify-r3 陈旧持有，队长 `release --force` 释放）两棵树均无持有者；
> `gc` 删除两棵，释放约 12.7 GB；gc 后 `status` = 「没有批次树」，`git worktree list` 只剩主挂载。
> 另：`yx49r3` 与 `yx49r4` 不是批次树（无 meta.json），是成员放证据/探针的**工作区**目录，
> 不属「残留批次树」。仓库外无本批新增目录。

## P4 4.4 证明层与 wasm 收尾 — 收口（4.4.1 / 4.4.2 / 4.4.3）

> **一、批次构成与轮次（批次名 `yx44`）**：三条三级任务 4.4.1 / 4.4.2 / 4.4.3 与 wasm 属性可达性判定同批；批次验证树由 `scripts/dev/scratch.py init` 建在工作区 `target/` 下（首次全量构建；收口 gc 后已删除）。
> 角色与轮次：实现 4.4.2 = t1（12 文件；本地 `1fc135437` → 推送后 `24028ffe8`）｜实现 4.4.1 的层序重写 = t2（只文档/注释，零行为变化，基于 `b5e68b3c9`）｜实现 4.4.3 = t3（11 文件，含新码 W1082；本地 `2d5e610cd` → 推送后 `52d2276ca`）｜wasm 可达性判定 = t4（只判定、零清理，判定 revision `b5e68b3c9`）｜4.4.1 收口与一笔文档提交 = t8（7 文件 173+/43-；本地 `29ebd2145` → 推送后 `d07993157`）｜独立验证 = t5（冻结 `29ebd2145`）｜轮 1 复审 = t6（revision `1b593e993`）｜补口 = t9（3 文件 9+/10-；本地 `1ebddaf95` → 推送后 `1b593e993`）｜修复轮 = t10（3 文件 23+/23-；本地 `657c3d5e3` → 推送后 `e184d88b6`）｜轮 2 复审 = t11（`657c3d5e3` = 推送后 `e184d88b6`，patch-id 全等）｜收口 = t7（勾选 / 实录 / 批次树 gc / 文档真构建 / 一笔文档提交；本地 `c8412565c` → 推送后 `da724940b`）。
> **失败与重做**：t6 以 `needs_revision` 终态失败（复审轮按契约须 fail 并给 findings，非事故）；t7 首轮在批次树 gc 阶段中断，已完成产物留在主仓工作树未提交，由第二轮接手「补缺口 + 复核」——未重做已完成部分（本轮新增：4.4 勾选与证据注记、本实录、三处指向已删批次目录的路径改写、文档真构建与收口提交）。无取消轮次。
>
> **二、逐任务产出与量测数字（原始命令与输出见证据摘要与留档；此处只记结论）**：
>
> - **4.4.1**：`layers/README.md`（含 `layers/mod.rs` 顶部注释）的层序表重写为**实际调用序**（predicate → termination → ownership，调用点锚定 `checker.rs::check_module_impl`），`equivalence` 按真实身份描述（无生产调用点）；同源声明 7 处收口（`01-routing` / `02-stage-contract` / RFC-039 / `06` / typecheck README / layers 两文件）；机械对账脚本在批次树绿基线上 **11/11 PASS**（判据 11 条 = README 侧 4 条：实际调用序声明 / 两处入口锚 / 旧序声明消失；代码侧 7 条：5 个行锚 + `check_module_impl` 无短路 + `equivalence` 零生产调用；对账脚本为一次性脚本、随工作区回收，判据 11 条已逐条列出可据此重写），变异 (c) 回退旧声明 → 对账 2 FAIL 即红。
> - **4.4.2**：求解器获取面统一到 `proof/smt/backend.rs` 的进程级共享单例 + 唯一闭包口 `with_shared_solver(consumer, f)`；三消费点（predicate / termination / ownership）全部改走共享口，**编译生产链上 `default_solver()` 调用点 = 0**（唯一后端选择点下沉为私有 `new_backend()`）。`wasm/src/lib.rs` 的 `test_smt` 自检调用点是**既存**（早于本批，自 `b5e68b3c9` 起）的非测试调用点，属「测试与独立场景」面，不构成 D58 禁令违反。独立验证自构造读数（一次编译）：predicate acq=2 hit=1 miss=1 ｜ termination acq=1 hit=2 miss=2 ｜ ownership acq=2 hit=1 miss=1，三点 hit/miss 之和 (4,4) = 共享实例累计 `Some((4,4))`；变异 (a) 单例退化 → predicate hit=0 即红。**改动词面披露**：该笔提交含 `z3_backend.rs` 15 行——`cache_stats` 从固有方法迁入 `Solver` trait 块（返回 `Some(实例内计数)`）+ 1 行注释改指，无语义耦合。D58 禁令已机制化：同线程重入 → panic 含 D58，探针 `recv_timeout(120s)` 不挂死。
> - **4.4.3**：`body_checker` 缺失时的静默降级收敛为 `TypeChecker::ownership_inputs`（唯一判据点）+ 新码 **W1082**（与既有 W 码事实不重叠：W1080 约束降级 / W1081 求解器缺失 / W1090 方向相反）；钉测试 2/2 绿；健康路径语料 348:348 零 diff；**降级分支生产面不可达穷举成立**（CLI 四入口 + eval/repl + format + LSP + wasm playground + 全部 `ProgramKind` 汇入 `check_module_impl`：`init_body_checker` 无条件先于所有权块，函数内 0 个 return/break/continue，`body_checker` 无 `.take()`）⇒ W1082 无用户可见面。变异 (b) 删发射 → 钉测试即红。**W1082 属对外契约表变更（E 类），经用户追认**；`locales/zh.json` / `locales/en.json` 与代码同提交，其余语言由 i18n bot 于推送后补齐（run 见「九」）。
> - **wasm 属性可达性判定（t4）**：`orchestrator.rs` 现树 **10 条 = 4 正臂 + 6 反臂**（口径 = 同一行内 `#[cfg(...)]` 属性文本，正反都计；`cfg!` 运行期宏 0 命中）；4 条 wasm 正臂在 playground 运行期不可达但**编译期承重**（调用点未门控，单独删臂 ⇒ wasm 编译失败），6 条 `not(wasm32)` 反臂 native 三条阶段表承重 ⇒ D35 设想的「删不可达者」在该文件**无适用目标**，真实清理形态 = 整条 project-mode 子树在 wasm32 上门控（登记 P10 S7，三条未核实原样保留）。
>
> **三、独立验证（t5，冻结 `29ebd2145` = 推送后 `d07993157` 的内容——两 revision 逐笔 patch-id 全等，整树差异仅在 `docs/src/en/**`、`docs/src/.i18n-cache.json` 等 bot 产物面——`git range-diff` 三笔全 `=`）**：门禁 **8/8 全绿**——`cargo fmt --check` 0 ｜ `clippy --all --all-features --all-targets -- -D warnings` 0 ｜ `cargo test` **3128 passed / 0 failed**（7 个测试目标合计；基线 3126 + 本批 2）｜ 语料差分 **348:348** 逐行零差异（sort diff 0 行）｜ IR 快照 `UPDATE_SNAPSHOTS=1` 重跑后 `git status` 零漂移 ｜ `check-obligations.py` 21 = 13 + 8、containment 0 缺口 ｜ `check-docs-truth.py` 源码 156 / 文档 156、CLI 21 命令一致 ｜ 错误码生成器重跑前后树均干净。构建告警仅 4 条 bot 语言的 W1082 缺条目（已知态）+ Z3 下载噪音，零代码警告。
> **契约 verify 的口径更正（必须两个 JSONL 路径）**：语料门禁的调用口径 = `python scripts/ci/check-corpus-parity.py tests/baselines/corpus-parity.jsonl <本次生成>.jsonl`——**两个 JSONL 路径都要给**；漏掉第二个参数就是拿基线比基线（恒零差异，判据空转）。本批按两路径口径执行并落证（t5 的 `corpus-r44.jsonl` 产物）。
>
> **四、变异证据（3/3，机械可逆）**：全部经 `python scripts/dev/scratch.py trial`（`git apply` → 跑命令 → `git apply -R` → 逐文件 blob 与 HEAD 比对 → 要求 `git status` 干净）：
>
> - (a) 单例退化 → 共享读数 predicate hit=0 miss=2、测试 FAILED exit 101；`backend.rs` HEAD=`3e2b2d150b24` → 变异 `6d00bad4d4aa` → 还原 = HEAD。
> - (b) 删 W1082 发射 → 钉测试 FAILED exit 101；`checker.rs` HEAD=`70c04a41e020` → 变异 `db0a623c9b9a` → 还原 = HEAD。
> - (c) 回退旧层序声明 → 对账 2 FAIL exit 1；`layers/README.md` HEAD=`8b1ae875db32` → 变异 `e88f9af6c42d` → 还原 = HEAD。
> 三次 trial 还原后 `git status` 均 CLEAN。trial (c) 首跑因绝对路径被 WSL 解析失败 exit 127（非判据红），改相对路径重跑为准。
>
> **五、复审轮次与结论**：轮 1（t6，revision `1b593e993`）**verdict = needs_revision**——3 条 findings 均为文档行号锚失准（F-01 medium：`02` §「wasm 现状」4 组过期行号；F-02 low：`02` 与 `09` 的 4.4.3 交付锚 `checker.rs:1283-1293`；F-03 low：`06` 的对照陈述），四项实质交付全部 pass。→ 修复轮（t10，本地 `657c3d5e3` = 推送后 `e184d88b6`，3 文件 23+/23-，并附两条机械判据）→ 轮 2（t11，同 revision）**verdict = pass、新 finding = 0**。**最终勾选依据 = 轮 2 的 pass**。
> **轮 3（收口 tip `da724940b` 上）**：独立验证 t12 **verdict = pass**（绑定 `da724940b`：四处规则文本逐字节、3.19 / S7 / S8 字段与 scope 齐备、计数表与判决链复算、死引用零、`check-docs-truth` 0、`markdownlint` 0）；复审 t13 **verdict = needs_revision**，4 条 findings——F-01 medium（t10 / 轮 2 只给 rebase 前哈希、§十四 的清单与 origin/dev 实际不符）、F-02 low（`02:379` 的 09 4.2.8 锚被本批自身在 `09:144`/`:182` 各插一行顶到 `:274`）、F-03 low（`09a` 写入本机盘符路径）、F-04 low（三级计数行注记和 157 ≠ 表头 156）→ 修复轮 t14（本笔）逐条关闭。
> **t10 契约的口径裁决**：修复面以**任务记录的 5 条 F-01…F-05** 为准（F-04/F-05 为机械判据），派工简报不是判据。
>
> **六、用户裁决（2026-10-18）与规则固化**：
>
> 1. **D58 口名就地改写**：`with_shared_solver(f)` → `with_shared_solver(consumer: SolverConsumer, f)`，就地改正文、**不写修订注记**（t8 在 4.4.1 一笔提交内完成）。
> 2. **wasm 判定登记为 P10 S7**（裁决 B）：带「归属阶段：P10；是否阻塞主线：否」+ 三条未核实**原样保留**；`06` 的 F2「需要决策的」行改指该登记；t4 产出的「### F2 判定」小节未动。
> 3. **崩溃项登记为 P3 3.19**（裁决 A）：症状 / 成因指向 `proof/smt/translate.rs` 的排序推断 / D59 **A/B 类**（崩溃）/ 验收行（该形状不再 abort——保守降级或诊断，语料与 IR 快照零漂移）；**定性 = 预存在**（t5 两 revision 对照，见「七」⑤）。
> 4. **规则固化一**「更正就地写入正文，不留新旧对照」：落 `coding-rules.md`（新增条文）、`07-equivalence-oracle.md` §语义修正的分流判据第 3 条、RFC-039 **D59 行**第 3 条（同提交登记 = WBS 行标注错误类型 + 就地更正受影响正文 + PR 必填块「错误类型」一行）。
> 5. **规则固化二**「契约 `inScope` 一律写可被路径匹配器解析的路径前缀」：落 `AGENTS.md` 团队编排约定第 6 条。
> 6. **新增码 W1082 经追认**（对外契约表变更，E 类）。
> 7. **登记 P10 S8**「文档更正形式收敛（就地改写，删修订注记）」：含 7 条审计清单 scope 与验收行。
> 8. `09` 顶部计数表：二级 **63**（60 + 3.19 / S7 / S8）、三级 **157**（154 + 3），带 2026-10-18 注记；三级项的账目复核见「十五」。
>
> **七、须如实记录的进程事实**：
> ① 4.4.2 提交用 `SKIP=cargo-fix,cargo-clippy-fix` 跳过两个 `--fix` 钩子——共享工作树里有队友在制品，`--fix` 会顺手改写他人文件；**等价门禁已在最终树手工全跑**（fmt --check / clippy `-D warnings` 均 0）。
> ② `pre-commit` 的 stash 窗口确实发生（`Stashing unstaged files…` / `Restored changes from…`），提交后 `git status` 核对显示队友在制品完好。
> ③ **4.4.3 的 inScope 教训**：散文限定 `docs/src/（仅限错误码生成物）` 被 `changedPaths` 校验拒绝 ⇒ `inScope` 一律写可匹配路径前缀（已固化为规则，见「六」5）。该任务**实际 11 个文件**，其中 7 个因声明写法无法被路径匹配器解析而未列入 `changedPaths`，已在任务结果中逐一列出（`codes/w1xxx.rs`、RFC-013 码表、`reference/error-code/` 的 index 与各 E 段页等）。
> ④ **W1082 的 locale 窗口已闭合**：`locales/zh.json` / `locales/en.json` 与代码同提交，其余语言由 i18n bot 于推送后补齐（run 见「九」）。
> ⑤ **Z3 `Sort mismatch` 崩溃判定（t5 判据⑦）**：**预存在**——同一探针、同一输入在 `b5e68b3c9` 与冻结 revision `29ebd2145` 上 stdout **逐字节一致**（`Error: Sort mismatch at argument #1 for function (declare-fun not (Bool) Bool) supplied sort is Int`）、均 exit 1；静态归因一致（4.4.2 对 `translate.rs` 0 行改动、对 `z3_backend.rs` 仅 `cache_stats` 迁移）。**降级分支生产面不可达穷举成立**（W1082 无用户可见面）。探针三行源码：`NonNegative: (x: Int) -> Type = { x >= 0 }` / `AtMost: (n: Int, x: Int) -> Type = { x <= n }` / `f: (x: NonNegative(x)) -> AtMost(x, x) = { return x }`。
> ⑥ **t5 冻结面与 t9 的覆盖差异**：t9（三处注释/文档补口，`1b593e993`）提交晚于 t5 的冻结 revision `29ebd2145`，属零行为改动，其验收由 t9 自带的机械核对 + t6 复审覆盖（t6 在 `1b593e993` 上**重跑**了 t5 的关键量测，不转述）。
> ⑦ `docs/src/en/**` 不手改，由 push 后的 auto-translate 工作流再生（本批该 bot 落地两笔：`92d490695` i18n locales 补 W1082 其余语言、`63b6cdf37` auto-translate 文档）。
> ⑧ **t9 越界观察的三条裁决**：㈠ t9 顺手改正 `02` 同一句内的 `wasm/src/lib.rs` 行数与门控点行号（73 / 139·153·170 → 152 / 151·170·187）——**接受**（同口径同数字的必需面，提交信息已列明）；㈡ RFC-039 **D35 行**仍写「wasm 27 个文件的 cfg 分支」——**保持不动**（决议行是历史裁决记录，其前提修正已由 P10 S7 登记承载，不改写历史）；㈢ `docs/src/en/**` 的旧口名译文由 auto-translate 对齐。
> ⑨ `09` / `09a` 中原指向已删批次目录的两处路径在收口轮改为**就地内联**（证据摘要并入本实录本节，探针三行源码内联）。
> **2026-10-21 更正**：当时改指向的仍是工作区路径，等于把违规下移一层；本轮起该摘要的交付提交、门禁数字与
> CI run id 全部内联，文档不再引用任何工作区路径（AGENTS 元规则 ① / 第 10 条④）。
>
> **八、t8 验收④ 的口径裁决**：验收原文要求 `git diff | grep '修订注记\|原为\|现改为'` 为空。实测**默认上下文口径不可达**——D58 行邻行的 D59 / D61 决议行本身含「修订注记 / 原为」字样（非本任务改动）。裁决：按**新增行 / 删除行 / `-U0` 增量行**三口径全为空接受，并写明**后续同类验收只 grep 新增/删除行**——判据应落在本次改动行上，而不是整份 diff 的上下文。
>
> **九、CI 门禁结论（AGENTS 第 13 条①，定时核对 2026-10-18 17:26 回填；run id 与结论就地内联）**：
> 第一次推送（3 笔，tip = `d07993157`）：CI **39110908163** success ｜ Deploy Docs **39110908155** success ｜ Auto Translate Docs **39110908193** success ｜ Auto Translate I18n **39110908144** success；该 tip 覆盖 `24028ffe8`（4.4.2）/ `52d2276ca`（4.4.3）/ `d07993157`（4.4.1）三笔全部内容。
> 第二次推送（1 笔，tip = `1b593e993`）：CI **39112293391** success ｜ Deploy Docs **39112293062** success ｜ Auto Translate Docs **39112292903** success；覆盖含 t9 补口在内的全部四笔。**无 concurrency 取消**，后者覆盖前者。
> 第三次推送（2 笔，tip = `da724940b`）：CI **39121538555** success ｜ Deploy Docs **39121538562** success ｜ Auto Translate Docs **39121538541** success——三条 run 的 headSha 均 = `da724940b` 且**均已终态**（2026-10-18 第三轮定时核对回填）；轮 3 独立验证 t12 当时记为「CI 未终态」的观察就此关闭。
> 无关项：Nightly **39081392694** failure 的 headSha = `290f9ebc`，不属本批提交面。
>
> **十、三次 `pull --rebase`（推送期间的自动提交对端）**：对端分别为 auto-translate 与 i18n 提交；六笔哈希被重写且逐一 **patch-id 全等**——本地 `1fc135437` → `24028ffe8`、`2d5e610cd` → `52d2276ca`、`29ebd2145` → `d07993157`、`1ebddaf95` → `1b593e993`、`657c3d5e3` → `e184d88b6`、`c8412565c` → `da724940b`（第三次 rebase 后 origin/dev 头 = `da724940b`）；`git range-diff` 核过**逐笔内容不变**（三笔全 `=`）。成员未执行 `git push` / `fetch` / `pull`，同步与推送由队长统一做。**取证提醒**：此后引用提交号一律用**推送后哈希**（上列右值）；`09` 的勾选行按约定不写提交号。
>
> **十一、未覆盖面（不得把「没测」写成「已过」）**：① doc-tests 0/0 未承载判据；② **wasm32 目标实编未做**（该面由 CI 的 `_build-wasm.yml` 覆盖，本批的 wasm 判定为静态论证）；③ 变异仅覆盖契约点名的 3 处，其余门禁未做反向改坏验证；④ 语料差分未对 `b5e68b3c9` 单独跑（只对冻结 revision 跑）；⑤ i18n bot 补齐的其余语言 W1082 文案行为未验证（只核了 `zh.json` / `en.json`）；⑥ 轮 1 复审与修复轮报出的**同类但不在契约 findings 内的既有漂移**未修（清单见「十二」），归属待用户裁决。
>
> **十二、`02` 全篇既有漂移观察清单（用户 2026-10-18 同意登记；2026-10-21 落库为 S10）**：口径 = 均为 **pre-existing 漂移**（非本批引入），**不构成本批 finding**；清单由修复轮发现、轮 2 复审在 `657c3d5e3`（= 推送后 `e184d88b6`）上独立抽验并给映射，逐条照录（清单就地内联如下，原始文件随工作区回收）：
>
> 1. `02` §(a) 入口表 3a-3d / 5 / 5b：`02:72` 引 `orchestrator.rs:273-399` → 现 `:155`；`02:73` `:450` → `:250`；`02:74` `:1374` → `:985`；`02:78` `util/diagnostic/mod.rs:565` → `:681`；`lsp/handlers/diagnostics.rs:146` 的 `run_diagnostics` 已不在该处；**内容级**：`run_diagnostics` 与「否则手工 lex→parse」已被 4.6.1（删手工序列）取代。
> 2. `02:1016`：`compile_project :99-237` → 现 `:107-134`（28 行 Program 构造器）。
> 3. `02:360-361`：「缺失时降级为 warning」→ 现树 `_build-wasm.yml:50-51` 是 `::error` + `exit 1`（#435）；`build.rs:65-72` 已改 `panic!` 硬失败；`_build-wasm.yml:35-59` 区间需重取。
> 4. `02:127` 机制指针：机制实际在 `src/driver/arms.rs::typecheck_orchestrated`（`:288-289` 自述）；结论仍成立（多文件臂丢 warnings），失准的是「result 唯一的下游用途」一句（`type_result` 还经 `arms.rs:1389` → `helpers.rs:259/:636` 喂 `overload_resolutions`）；建议改法 = 指针改 `typecheck_orchestrated`、「唯一下游用途」改为「warnings 字段无读取点」。
> 5. `02:128`：`check_project:385-395` / `250-252` 需重取。
> 6. `02:334`（§S5 (d) 调查段）：引用的旧代码片段已被 4.4.3 的 `ownership_inputs` 判据点取代。
> 7. `02` 改动清单表其余行：`predicate.rs:34-36`、`checker.rs:1303-1314` / `:1244`、`backend.rs:67-72`、`wasm/src/lib.rs:30-36` / `:42-51`、`util/diagnostic/mod.rs:621-661`。
> **本次收口不预置 S9 的 WBS 条目**（用户未裁决）；获得裁决后再按登记形态落 `09`。
> **后续（2026-10-21 落库）**：用户 2026-10-18 同意登记，本清单已作为 `S10`「02 正文与事实对齐（行号锚 + 内容级过期）」落 `09`（二级 +1 / 三级 +1 = S10.1）——**原拟的 S9 号已被「发版前审计的抽象化收口」占用**（S9.1 = issue #473 / S9.2 = issue #474），按 09「编号在全文档内必须唯一」的登记约定顺延为 S10。**登记时机偏离原计划**（原写「随 P5 的收口文档提交一并落库」），按用户 2026-10-21 指示本批即落库。
>
> **十三、批次树回收（gc 与 status 原始输出；输出中的树路径按仓库相对路径记）**：
> `python scripts/dev/scratch.py gc` → `[scratch] 已删除 yx44-evidence（释放约 0.0 MB）` / `[scratch] gc 完成，共释放约 0.0 MB`；
> `python scripts/dev/scratch.py status` → `yx44-evidence            holder=-` 下一行 `<工作区>/yx44/tree`——注册表记录的路径**已不存在**，该名是注册表残留（`gc` 的 `_remove` 删的是 `meta["path"]` 的父目录即工作区下的 `yx44`，残留的 `meta.json` 仍落在扫描面内；其内容 batch=`yx44`、`base_rev=29ebd2145`、`lock=null`、含 t5 / t6 / t11 的持有历史）；
> 收口轮把该注册表文件改名为 `meta.retired.json` 使其退出扫描（**不删任何证据文件、不新建任何树**）→ `python scripts/dev/scratch.py status` = `[scratch] 没有批次树`；再跑 `gc` = `[scratch] 没有批次树，无需回收`；`git worktree list` 只剩主挂载（`dev`）。
> 批次原始证据目录（无 `meta.json`，**非批次树**）当时保留为留档；**2026-10-21 起工作区目录不再作为证据出处**——收口摘要已内联于本节，原始目录随清理删除（AGENTS 元规则 ①）。验收表述（可解析）：`git status` 干净；批次树 gc 后 `python scripts/dev/scratch.py status` = 没有批次树；`git worktree list` 只剩主挂载；仓库外无本批新增目录。
>
> **十四、文档真构建与提交卫生**：`npm run docs:build` 本地 **exit 0**（AGENTS 第 10 条⑨——markdownlint 抓不到 VitePress 的 Vue 编译错误，且仅含 docs 的提交会被判 `no files to check` 跳过 markdownlint）；本批的勾选 / 注记 / 登记 / 规则修改攒成**一笔**文档提交（第 8 条），落在批次收口；成员未执行 `git push` / `fetch` / `pull`。**本批最终两笔均已推送**（收口 tip 上 `git rev-list --count origin/dev..HEAD` = 0，origin/dev 头 = `da724940b`）：`e184d88b6`（轮 1 findings 的文档锚点修复，3 文件 23+/23-）+ `da724940b`（收口文档提交，7 文件 96+/10-；含勾选 / 实录 / 三处路径改写 / 登记与规则修改）。
>
> **十五、计数表账目复核（轮 3 修复 F-04；方法与结果均可复核）**：判定对象 = `09` 顶部「定位与范围」表的三级计数行。
> **方法**（两条独立口径，逐 revision 用 `git show <rev>:<file>` 取字节复算）：
> ① **逐二级条目求和**——取每一条二级看板行括号内的「N 个三级任务」（行首 `- [x]`/`- [ ]` 加粗标题起首、行内含「个三级任务」，含 S7 行内嵌写的「1 个三级任务」）：现树 70 条二级条目，**和 = 157**。
> ② **历史自洽性**——同一口径在 `366bc5a69^` 上得 66 条、**和 = 151 = 当期表头 151**（自洽）；`366bc5a69` 当日新登记 `4.9.6` 与 `0.4.1` 两条三级（和 151 → 153），而表头只写 151 → 152 ⇒ **表头自该笔起少记 1**；此后各 revision 均差 1（`92d490695` / `1b593e993` / `d07993157` / `e184d88b6`：和 154 对表头 153；`da724940b`：和 157 对表头 156）。
> **结果**：真值 = **157**，故更正表头（`09:13` 三级 156 → 157）；注记 144+2+2+2+2+1+1+3 = 157 本来正确，未改；本批之前的基数 = **154**（不是 153）。**二级计数行未动**——该行口径从未与看板条目数对齐（`366bc5a69^` 上是 66 条对表头 59），本次没有可复核证据推翻 53 + 10 = 63，不为凑数改数。
> 附带更正（同一轮）：§二 的 4.4.1 括注分解按对账脚本实际项数改为「README 侧 4 条 + 代码侧 7 条 = 11」，与 **11/11** 及脚本 11 次 `ck` 调用一致。
>
> **十六、4.4.4 观测账裁决与 doc 契约（2026-10-24 用户裁决 ① 保留；加框自 4.9 队）**：
>
> - **决策点**（issue [#475](https://github.com/ChenXu233/YaoXiang/issues/475)）：`ConsumerStats` / `CONSUMER_STATS` 这份观测账**生产路径每点都写、生产侧 0 读取**——写入在 `with_shared_solver` 每次获取求解器后调 `record_consumer_use`（原子累加 `acquisitions`/`hits`/`misses`），唯一读取口 `shared_solver_consumer_stats` 的调用点全仓只在 `src/frontend/core/typecheck/proof/smt/tests/shared.rs`；`SolverConsumer::ALL` 的使用点同样只在该测试文件。要求在「① 保留并把『唯一消费者 = 测试』写成 doc 契约」与「② 删掉」之间裁决。
> - **取舍理由**：选 ①——4.4.2（D58 进程级共享单例）的验收判据**以这份读数为载体**，「三点共享同一实例、命中率按消费点可观测」只能靠它证明；选 ② 则要同时删掉判据载体并改写 4.4.2 的验收数字（等于把一条已裁决、已验证的判据退回去）。① 的**唯一代价**是生产路径每次获取多一次原子累加（一把互斥锁的一次加法，不在求解关键路径的量级上），换来判据载体与可观测性不变。
> - **落地（doc 契约，零行为变化）**：契约三句写进 `src/frontend/core/typecheck/proof/smt/backend.rs` 的模块头与相关项文档注释——① 写入面 = 经共享口每次获取记一次原子累加；② 读取面**只有** `proof/smt/tests/shared.rs`（生产代码零调用）；③ `SolverConsumer::ALL` 是**测试专用常量**，生产代码不得依赖。
> - **零行为变化的证据**：该文件的改动面**只有注释**（`//` / `//!` / `///`），无任何代码行增删（`git diff -U0` 的新增/删除行全为注释行）；4.4.2 的验收数字（`cargo test` 3128 passed / 0 failed、语料 348:348 零差异、IR 快照零漂移、`check-obligations` 0 等）**未动**——判据载体保留正是本裁决的目的。
> - **登记**：`09` 的 4.4.4 行记为「2026-10-24 用户裁决 ① 保留并写 doc 契约」并写终态；4.4 二级看板随之勾选（4.4.1–4.4.4 四个三级任务全部完成；该行的「N 个三级任务」由 3 订正为 4——4.4 下确为 4.4.1–4.4.4 四个三级任务；09 定位表的三级表头本条**未动**（表头与「逐二级求和」口径的对账属 09a「十五」那条独立复核事项，不在本条范围））。本条**加框自 4.9 队**（用户 2026-10-24 批准），故 4.4 的收口落在 4.9 队的收口轮内。

## P3 欠账轮 1 收口（3.5 / 3.6 / 3.4.6 / 0.5）

> **一、批次构成与轮次**：本批收口 P3 侧剩余欠账——**3.5 高阶类型参数调用路径**（3.5.1–3.5.3）、**3.6 函数类型值调用路径**（3.6.1–3.6.4）、**3.4.6 compile 路径消费 warnings**（D 类，用户 2026-10-19 裁决「做」），并一并登记 **0.5 测试套件分层**（P0，issue #486 切片）与 **3.20 死代码分析的既有误报**（3.20.1 / 3.20.2）。角色与轮次：
>
> - 实现：3.5.1–3.5.3 = t1｜3.6.1 = t2｜3.4.6 = t6｜3.6.2 = t7｜3.6.3 = t8（主体 + 收尾两笔）｜3.6.4 = t9｜0.5.1 = t10（#486 切片）
> - 独立验证：轮 1 = t3（冻结 revision `81d5e414e`，覆盖 3.5 / 3.6.1–3.6.3 / 3.4.6）｜3.6.4 = t11（轮 2，冻结 `8e32e8636`）
> - 复审：轮 1 = t4（verdict = **needs_revision**，F1/F2/F3）｜修复轮 = t12（一笔）｜轮 2 = t13（verdict = **pass**）
> - 收口 = t5（勾选 / 登记 / 本实录 / 批次树 gc / 文档真构建 / 一笔文档提交）
>
> **失败与重做**：t4 以 needs_revision 终态失败（复审轮按契约须 fail 并给 findings，非事故）→ t12 真修 → t13 复审 pass。t8 内部经历一次 attempt 中断（进程崩溃），前一轮证据保留在批次目录、续作 attempt 补齐后两笔入库。无取消轮次。
>
> **二、逐任务产出与量测数字（结论口径；原始命令与输出见各批次证据目录）**：
>
> - **3.5.1**（合一器在头部位置匹配构造器变量）：判据收敛为**单一实现** `is_ctor_reference`（`pub(super)` 自由函数，全仓 1 定义 3 消费）——这是 t12 对 t4 的 F1 真修（原先 `inference/statements.rs` 另有一份同名方法副本，构成禁令三意义上的同名两份实现）。头部改名不再内联复制：`environment.rs` 与 `expressions.rs` 各自先遍历 TypeRef / 变量表构造 head_map，再统一交共享实现 `substitute_ctor_heads`（`types/substitute.rs`）执行——两处 head_map 构造判据不变，改名只按名作用于头部，空表原样返回（t12 的 F2 真修）。等价性论证：遍历不改 Generic 头部名字。
> - **3.5.2**（显式类型实参接受类型构造器，E1010 消失）：红态 E1010 / E1002，绿态 exit 0。
> - **3.5.3**（mono 产物替换构造器变量）：符号化类型实参判据按容器泛型参数名收窄；产物中构造器变量被替换为真实构造器（IR dump 实证：特化函数 `Function probe(Box, int64)`）。
> - **3.6.1**（局部函数类型值按名调用）：判据单一源 = `is_static_fn_name`（全仓 1 定义 1 调用），与变量求值路径的名字解析优先级（局部 → 全局）对齐，未新增第二份解析表；IR 实证：模块初始化段由 `LoadConst String("f")` + `CallNative` 变为 `LoadGlobal` + `CallDyn`。回归面：同名「函数绑定 + 值绑定」不可达（一律编译期 E2010），非函数值按名调用仍 E1065。
> - **3.6.2**（`use` 别名导入的函数值按名调用）：原报 `E6006 Native function not found: lib.f`（`use lib.{f}` 与 `use lib.{f as g}` 两形态）。修法 = 在 `is_static_fn_name` 内**加一条臂**（非复制），新增唯一实现 `use_alias_value_slot` 承担「use 别名 → 限定名 → 全局槽位」两表联查，消费点 2 处（调用形态判据 + 变量求值路径）。
> - **3.6.3**（嵌套函数体引用外层帧值）：**判据改判**——不是「让它跑对」，而是该形态按 RFC-009 §2.3 + SPEC §12.3 应**编译期被拒**（D59 C 类：把静默错值改成响亮；按 `09` §P3 的 D59 行，「静默给错答案」归 C 类——本批收口简报记作「A/B 类」，两者同指「旧行为非成功」，落地方式同为「先红后绿 + 编译期响亮拒绝」）。原形态 = **不报错、算错值（void）**：红态 stdout `void`、EXIT=0（静默错值留档）；绿态编译期 E1001 `Unknown variable`、EXIT=1，与既有「未知变量」路径**同码同文案**。判据单一源 = 捕获屏障（`enter_capture_barrier`），编译期名（类型参数）不挡；E1001 仍只在 typecheck 发出、注册唯一。**赋值路径由 E2010 兜住**（t8 的收尾提交：声明 / 赋值判定不套捕获屏障）——否则本次修复自身会新开一条「同名赋值静默错值」通道（c1 形态：只有屏障时退化为 exit 0 打印 10）。
> - **3.6.4**（`use lib` 整体导入后 `lib.f(...)`）：走 FieldAccess callee 的命名空间分支，原报 E6006 `lib.f`。修法 = 命名空间分支解出限定名后先判全局槽位布局 → 改走间接调用；限定名解析与槽位查询复用 `resolve_namespace_data_access`（与数据访问同源），未新增解析表。
> - **3.4.6**（compile 路径消费 warnings，D 类）：变更前后对外表现 = **多文件 `run` 此前恒不输出任何 W 码**（红态留档：多文件 [] vs 单文件 [(W1001,3),(W1002,5),(W1003,1)]），现四路径（单文件 run / 多文件 run / 单文件 check / 多文件 check）的 W 码集合恒为 {W1003@1, W1001@3, W1002@5}（非空集）。语料基线：348 条记录中 **46 条变化**，新增 W 码 **118 条**（W1001 34 / W1002 12 / W1003 13 / W1004 42 / W1005 17），**118/118 逐条归因到文件里真实存在的未使用定义（0 条未定位）**，纯增量（0 条删除、0 处 exit / stdout / check 变化）。**D59 四条兑现**：①先红后绿（红态留档 + 转钉测试）②不改变「原先成功程序」的对外表现（纯增量，0 处 exit / stdout / check 变化）③同提交登记（本实录 + `09` 的 3.4.6 行）④基线同提交再生并人工 review diff。**超集获准**：W1004 / W1005 / W1006 一并进入 `run`（用户 2026-10-19 批准），理由 = **一份判据、一个通道**——只放 W1001/W1002/W1003 等于再造第二份过滤判决。**对外契约面变更**：`orchestrator::compile_project` 返回值由 `Result<ModuleIR, _>` 改为 `Result<(ModuleIR, FileDiagnostics), _>`（仓内消费者 2 个 + 2 个测试直驱点同步适配）。
> - **0.5.1**（测试资源画像标记 + 独占锁 + fast/full 最小消费面）：新增 `scripts/dev/testsuite.py`（`markers --check` 与 `cargo test --list` 双向对账、`plan` 预算装箱、`run fast|full` 预算不足即报缺口不静默跳过、`manual` 档永不隐式执行）与 `src/util/test_support.rs`（跨进程文件锁，测试体内取锁 ⇒ 任何调用路径都互斥）；就地 `cost:` 标记 3 处（2600 档 exclusive，峰值由 5000 档实测线性外推、**未实测**；5000 档 exclusive，15.6 GiB / 72.6 s 实测；快照更新 manual，峰值与耗时均为外推）。自验数字：`markers --check` 核对 3149 个 `#[test]`、3 例机械变异各自 exit 1、`run fast` 批量波 2793 passed / 47.7 s + 独占波 1 passed / 14.6 s、预算 1 GiB 时 `run full` 列出缺口 7.11 GiB、两进程争锁实测 `held→blocked→held(waited 2.0s)`。**独立验证属 #486 后续排期**（本批只有实现者自验）。**0.5.2 待排期**（pre-commit / CI 接线 + 2600 档实测校准 + `test-specification` 细则节）。
>
> **三、独立验证（t3，冻结 `81d5e414e`；红态基线 `c599de7bb` / `1725d0215` / `ef2334de5` 均为 HEAD 祖先，`git merge-base --is-ancestor` 核实）**：探针全部为验证者自建（批次树由 `scripts/dev/scratch.py init` 建在工作区下，探针与补丁在其 `probes/` / `patches/` 子目录），不转述实现者结论；该批次树连同其树内证据已在本批收口的 gc 中删除（见「九」），故本节把量测结论与数字**原样落进实录**以留证。3.5 红态四条探针均报 E1003+E1002 / E1010，绿态 exit 0 且值正确（stdout 1 / 7 / 1），并有 IR 值级实证（特化函数 `Function probe(Box, int64)`）；3.6.1 绿态输出 6 / 10，对照「值位置调用」绿、「非函数值按名调用」编译期 E1065；3.6.2 红态两形态 E6006、绿态 exit 0（探针内 `assert` 失败即非 0，exit 0 即值正确）；3.6.3 红态 stdout `void` / EXIT=0（静默错值可核）、绿态编译期 E1001 / EXIT=1，而**表达式位置内联 lambda 捕获本函数参数**放行（EXIT=0、输出 6）——放行面未被误伤。3.4.6 的 C2 归一由验证者自建夹具独立复现（四路 W 集合恒等、非空），基线差分独立复算（changed=46、added=0、removed=0、expectation / run.exit / run.stdout / check 变化 = 0、新增 W 码头 118 条与实现者一致），归因抽样 10 条全部命中真实定义。门禁在冻结树上全绿：fmt / clippy（`-D warnings`）/ `cargo test --all`（**lib 2790 passed + 2 ignored、integration 297 passed**、0 failed）/ 语料差分 348:348 差异为空 / IR 快照 204 条零漂移 / 义务账本 / docs-truth。**t11（轮 2，冻结 `8e32e8636`）** 独立复现 3.6.4 的红→绿（红态父提交 16e2a5c88：E6006 `lib.f`、EXIT=1；绿态 EXIT=0、stdout 27，探针内 4 断言全过）、判据单一源（`resolve_namespace_data_access` 1 定义 2 消费，无第三份「限定名 → 槽位」联查）、不回归八形态全过，verdict = **pass**。
>
> **四、复审轮次与结论**：轮 1（t4，判 `20abd3ebc` 的父 revision）verdict = **needs_revision**，三条 findings 全部是真缺陷：F1 = `is_ctor_reference` 同名两份（`inference/statements.rs` 方法副本），F2 = 头部改名在 `environment.rs` 与 `expressions.rs` 各内联一份，F3 = 可见性越界（该私有的未私有、该 `pub(super)` 的未收敛）。→ 修复轮 t12（一笔，4 文件 +96/−85）→ 轮 2（t13，被复核 revision `20abd3ebc`）verdict = **pass**，新 finding = 0：8 条自建探针的值与退出码与修复前逐条相同、4 份 IR dump 与轮 1 绿态**逐字节一致**、语料 348/348 零漂移、IR 快照 204 零漂移、`cargo test --all` EXIT=0（lib 2794 passed + 2 ignored、integration 297 passed、0 failed）、fmt / clippy / docs-truth 全 exit 0、`check-boundary` 的 `pub(crate)` 计数 167 / frontend-core 26（= 3.5 前基线）、`check-fanout` 同名定义零命中。**最终勾选依据 = 轮 2 的 pass**。
>
> **五、变异证据（全部经 `python scripts/dev/scratch.py trial`，apply → 跑命令 → apply -R → 逐文件 blob 比对 + 要求 `git status` 干净）**：
>
> | # | 目标 | 变异后 blob | 抓取结果 |
> | --- | --- | --- | --- |
> | M1 | 3.6.1 `is_static_fn_name` 的全局查找臂 | `022986cbea65…` | 两条探针复现 E6006、EXIT=1（还原后 blob = HEAD，CLEAN） |
> | M2 | 3.5.2 显式类型实参分支 | `abbcc5ecf116…` | 探针 check 报 E1002、EXIT=1（对照的函数形态仍绿 ⇒ 归因精确到 3.5.1） |
> | M3 | 3.4.6 的 `MULTI_FILE_STAGES` 死代码臂 | `372f7b92755d…` | run 通道 W 码全灭（C2 红）；M3b 归因显示 check 通道判据健在，坏的正是 3.4.6 新开的 compile 外发通道 |
> | M4 | 3.6.3 捕获屏障入口 | `7a7204c60383…` | 捕获类转钉测试 4 failed、TEST_EXIT=101 |
> | M5 | 3.6.4 值槽位 → CallDyn 块 | `25853376639b…` | 探针复现 E6006、EXIT=1（blob 链 `0d837e9d30de…` → `25853376639b…` → `0d837e9d30de…`） |
> | M8 | 3.5.1 转钉测试 | — | 修复后复跑仍可失败（E1003、TEST_EXIT=101、机械可逆还原 CLEAN）⇒ 判据未失去可失败性 |
>
> **六、用户裁决与本批登记**：
>
> 1. **3.4.6 = D 类**（对外可观察：多文件 `run` 新增 W 码），用户 2026-10-19 裁决「做」；**超集（W1004 / W1005 / W1006）同获授权**，理由 = 一份判据 / 一个通道。原 2026-10-15 裁决丙「按决策 B1 缓议」的**前置条件**（4.9 池语义修正）已于 2026-10-17 收口。
> 2. **3.6.3 判据改判**：由「让它跑对」改为「嵌套函数体引用外层帧值 → 编译期 E1001」；赋值路径由 E2010 兜住。
> 3. **两条「先于本任务」的静默形态登记不改**：①同块兄弟嵌套函数引用——基线即 E1001（外层块局部家族），本批不扩大范围；②嵌套函数标注引用外层类型参数仍打印 `void`（t1 登记项）。两者都**不是**本批引入，均原样保留。
> 4. **3.20「死代码分析的既有误报」登记**（用户 2026-10-19 同意按丙处置后登记）：3.20.1 = W1001 不跟踪闭包内引用（`docs/src/reference/stdlib/test.md` 的 `t_add` 形态：在闭包内被引用却报未使用）；3.20.2 = W1003 把变体解构必需导入判成未使用（`docs/src/reference/stdlib/weak.md` 的 `use std.option` 形态：删除即 E1002）。两形态在 3.4.6 之前的基线上同样复现，**非 3.4.6 引入**。**回滚义务（写死）**：本组修好后必须把 `weak.md` / `test.md` 中被改写以绕开误报的示例**恢复为自然写法**并**再生基线**（基线会再动一次）。
> 5. **0.5 测试套件分层**（P0，issue #486）：0.5.1 已完成；0.5.2（pre-commit / CI 接线 + 2600 档实测校准 + `test-specification` 细则节）待排期。
> 6. **#489 ureq 3 迁移**：依赖面追踪项，**不进 WBS**；`.cargo/audit.toml` 的 `RUSTSEC-2026-0381` 豁免**带硬性移除条件**——ureq 迁移到 >= 3 之后删除该 ignore，届时若仍红不得延长期限、须先修依赖。0.26 线无修复版本（0.26.12 不存在），ureq 2.x 无更新版本。
> 7. **3.20 编号冲突的处置（本轮新增登记）**：3.20 号原由 2026-10-18 登记的「发版前审计（v0.8.4）补登缺口」占用（3.20.1–3.20.4），而用户 2026-10-19 裁决登记的「死代码分析的既有误报」需用 3.20。全文档编号唯一性规则不允许两个 3.20，故**把发版前审计组整体迁至 3.21**（三级改 3.21.1–3.21.4，issue 锚 #468–#471 不变、条目内容不变），3.20 号让给「死代码分析的既有误报」。迁移只改编号，不改任何一条登记的内容与归属；历史提交信息不改写（迁移事实记于本实录与 `09` 的计数表注记）。
>
> **七、须如实记录的进程事实**：
>
> 1. **两次 inScope 写窄各换来一轮返工**：①3.6 系列的实现者把 inScope 写窄到 `ir_gen.rs` 谓词层，未覆盖 `types/` 下的共享替换实现 → 复审轮 1 的 F2 落地时才发现头部改名在两个文件里各内联一份，须再开一轮收敛；②3.5 的修复轮在 `types/` 共享实现与两处调用方之间搬运改动时，再次撞上 inScope 与实际改动面不匹配（AGENTS.md 第 6 条：inScope 必须覆盖机械必需面，路径前缀可被匹配器解析）。两处返工均以「实现者 inScope 写窄」为因，**不是实现缺陷**——但代价是两轮返工与一次复审 findings。
> 2. **外部同步（2026-10-19 13:08 `pull --rebase --autostash origin dev`、13:19 `pull --tags origin dev`）与 V0.8.5 发版提交**：经用户 2026-10-19 确认为**正常发版流程、已完成、不影响本队**；成员提交完好无丢失。取证（`git reflog --date=iso` / `git reflog show origin/dev --date=iso`）：13:08:43 的 `pull --rebase --autostash` 拾取的是发版链的「补写 CHANGELOG 保留策略的判据与自检」，13:19:34 的 `pull --tags` 为快进；V0.8.5 的发版提交（`:twisted_rightwards_arrows: V0.8.5: …` 与其 `:bookmark:` 伴生提交、CHANGELOG 生成与保留策略两笔）均在 `v0.8.5` 标签可达范围内。
> 3. **三次 rebase 导致本地哈希重写（引用旧号会扑空）**：`b3245aac9` → `56ae469e1`（3.6.3 主体）、`415f0154b` → `ef2334de5`（3.6.2）、`bdbe859e1` → `8e32e8636`（3.6.4）。验证者以「`src/`、`tests/` 树 diff 为空」证明三者为**纯 rebase 等价**（内容未变、仅父链变），本实录一律用**当前可达**的哈希。
> 4. **F2 的不可修部分**：3.5.1 那一笔的提交信息写「唯一实现」属历史，**不改写**；更正只能以「共享实现收敛的那一笔 + 代码注释按事实更正」承载——本实录保留这一句，以免读者误以为当时无人发现该问题。
> 5. **三条观察项（已核、不阻断，但登记在案以免日后被当成未处置违规）**：**O1** 头部改名收敛到共享实现后覆盖面多下钻 `MetaType` / `AssocType` / `Literal`，但三条覆盖探针**全部解析失败**（`Pipe` / `Ampersand` / 旧 `Type[...]` 语法；`ast::Type::Union` 在 parser 无构造点）⇒ **设计上更强、当前不可达**，不是行为变化；**O2** `check-fanout.py` 在修复轮之后新报 `add_type_param: 2 个文件`，引入者是 3.6.3 的收尾提交（`inference/scope.rs`）**而非**本批修复轮，复审判为**启发式误报**（委托包装 + 唯一实现的分工，与既有 `add_param` 同型）；**O3** `09` 的 3.6 行原文仍写「1 个三级任务」而实际有 3.6.1–3.6.4、3.5 三行缺完成注记——由本收口轮一并补登（见 `09`）。
> 6. **`src/driver/arms.rs` 的「决策 B1」注释未随代码同提交改写（遗留项，本批未改）**：3.4.6 已把 `typecheck_orchestrated` 的 FailFast 分支改为**暂存** `result.warnings`（原为直接丢弃），但该函数上方的文档注释仍写着「`result.warnings` 丢弃——决策 B1」，与 3.4.6 后的代码事实相反；同源的另一处注释（`src/driver/tests/program.rs` 的阶段表说明）**已随 3.4.6 那一笔同提交改写**为「决策 B1『死代码族警告不进 compile 路径』就此撤销」。本批 `src/**` 出 scope，故**未改**该注释，登记待下一队（属 AGENTS.md 第 6 条「被裁决直接点名的声明载体」类欠账）。`02` 的运行时行为表 B1 行已就地更正为现状（死代码族警告现经同一判据进入 MultiFile `run` 路径）。
> 7. **成员未执行 `git push` / `fetch` / `pull`**：同步与推送由队长统一做（`git reflog show origin/dev` 中各笔均为「update by push」，成员侧无 push 记录）。
>
> **八、未覆盖面（原样抄录，不得把「没测」写成「已过」）**：
>
> - **t3 的 7 项**：①wasm32 目标实编未做；②性能影响未量测（3.4.6 新增两阶段、3.6.x 的间接调用开销均无基准数字）；③**W1005 逐条归因未独立复现**（验证者的消息形态正则未覆盖其文案形态，只核了新增 W 码总数 118 一致）；④3.6.3 收尾提交（声明 / 赋值不套屏障）未单独变异，c1 形态未独立重测；⑤t8 的 29 探针矩阵未全量复跑（只重测三个代表形态 + M4 转钉）；⑥3.6.4 属轮 2 范围，轮 1 未独立量测；⑦CI 门禁按约定不在会话内等，交收口环节统一核对。
> - **t11 的 4 项**：①带泛型实参的限定名 callee（`lib.g(Int)(3)`）与命名实参调用函数类型值——仓内无样本，本轮未构造复测；②`use lib` 整体导入下不带项导入的 `p.get_x()`（既有 E6006 边界，先于本任务）未复测变异态；③在 `8e32e8636` 上未重复全量门禁（由 t9 的门禁日志与 t3 在含该提交的 `81d5e414e` 上的全绿覆盖）；④CI 交收口环节核对。
> - **本收口轮新增登记**：本轮的文档改动**未重跑 `cargo test --all`**（改动面为 `docs/**` 三个文件，零代码面）；收口轮的门禁为 `npm run docs:build` + `check-docs-truth.py` + 批次树回收 + `git status` 核对。
>
> **九、批次树回收（gc 与 status 原始输出）**：
> `python scripts/dev/scratch.py gc` → `[scratch] 已删除 verify-p3-t3（释放约 13220.5 MB）` / `[scratch] 已删除 verify-p3-t4（释放约 14428.0 MB）` / `[scratch] gc 完成，共释放约 27648.5 MB`；
> `python scripts/dev/scratch.py status` → `[scratch] 没有批次树`；
> `git worktree list` → 只剩主挂载（`dev`）。
> **被 gc 一并删除的还有两棵批次树内的验证证据**（`verify-p3-t3/` 含独立验证的探针、补丁、原始日志与结论文本；`verify-p3-t4/` 含复审轮 2 的证据）——它们是**注册批次目录**，随树回收；本实录第三、四、五节已把其结论、变异表与门禁数字原样落账，结论不因文件删除而失效。**未受影响的留档**（非注册批次目录，gc 不动）：各批次的证据目录 `wbs35` / `wbs35repair2` / `wbs346` / `wbs36` / `wbs362` / `wbs363b` / `wbs364` / `wbs486`（位于工作区 `target/` 下，**不入库**；结论已逐条落进本实录，工作区路径不作为引用出处）。
> 验收表述（可解析）：`git status` 干净；`python scripts/dev/scratch.py status` = 没有批次树；`git worktree list` 只剩主挂载；仓库外无本批新增目录。
>
> **十、文档真构建与提交卫生**：本批的勾选 / 注记 / 登记 / 文档更正**攒成一笔**文档提交（AGENTS.md 第 8 条），落在批次收口；生成器再生的文档并入其功能提交。提交前本地跑 `npm run docs:build`（AGENTS.md 第 10 条⑨——markdownlint 抓不到 VitePress 的 Vue 编译错误，且仅含 docs 的提交会被判 `no files to check` 跳过 markdownlint）与 `python scripts/ci/check-docs-truth.py`。**提交号在 push 之后取**：本实录按历史实录的惯例记录各轮实际发生的提交与哈希（**注意与 `09` 勾选行的分工**——`09` 的勾选 / 完成登记只写「任务 id + 证据（门禁数值、issue 号、file:line）」不写提交号，因 `pull --rebase` 会重写本地哈希；本实录是当轮过程的历史记录，保留哈希并同时给出 rebase 前后的对照，见「七」3）。**CI / Deploy Docs 结论交「统一核对」环节**，不在会话里等（AGENTS.md 第 13 条）——本批的 CI run id 与 conclusion 由队长在统一核对时回填。
>
> **十一、计数表账目复核（本轮口径）**：`09` 顶部的二级 / 三级计数表沿用「**基数 + 增量**」口径（与 2026-10-18 的登记同）。本轮增量 = 二级 +2（0.5 / 3.20，其中 3.20 为迁号后新占号，发版前审计组改 3.21 属净零）、三级 +7（3.6.2 / 3.6.3 / 3.6.4 / 0.5.1 / 0.5.2 / 3.20.1 / 3.20.2）。**遗留的预存偏差（本轮不纠正，供后续账目复核取用）**：以「逐二级条目括号内『N 个三级任务』求和」为口径复算，本树得 162，与表头差 7——差口来自 2026-10-18 登记时新增三级但未同步改写父条目计数注记的 5 处（4.9 写 7 实为 8、3.14 写 1 实为 2、4.10 写 3 实为 5、4.4 写 3 实为 4、S2 写 1 实为 2）与 1 处反向偏差（7b 写 6 实为 5），另有 1.2 / 1.3 / 9.1 / S7 四个父条目本就不带该注记（不计入口径）。该差口**非本批引入**，本批不为他人的条目改数，只在此登记。
>
> **十二、遗留待办（本批登记，未修）**：①`src/driver/arms.rs` 的「决策 B1」注释与 3.4.6 后的代码事实相反（见「七」6，`src/**` 出 scope）；②3.20.1 / 3.20.2 两个既有误报（修好后须恢复 `weak.md` / `test.md` 的自然写法并再生基线）；③0.5.2（pre-commit / CI 接线 + 2600 档实测校准 + `test-specification` 细则节）；④#489 ureq 3 迁移（依赖面，豁免带硬性移除条件）；⑤3.8（native 对用户函数名的遮蔽）与 3.12（运行时模式间的执行器配置传播）待裁决、3.18 前置待 P6——**均不在本队范围**。

## P3 3.17 收口（轮 2：3.17.1 手注 FFI 命名实参 + 第二份绑定判据合并）

> **一、范围与轮次关系**：本节是**轮 2** 的收口登记，范围 = **3.17.1**（issue [#456](https://github.com/ChenXu233/YaoXiang/issues/456)），不含其它二级任务。轮 1（3.5 / 3.6 / 3.4.6）的收口见上一节「P3 欠账轮 1 收口」；两轮的关系在此写明：**轮 1 的复审轮 1（t4）那次 needs_revision（F1 / F2 / F3）已由 t12 修复轮 + t13 复审轮 2 覆盖（verdict = pass），不在本节重复**。本节自己的判据链 = 实现 **t15**（attempt 2 交付；**前一次 attempt 因 provider 超时、零产物**）→ 独立验证 **t16**（verdict = pass）→ 对抗性复审 **t17**（verdict = pass，**无未处置 findings**，登记 4 条观察项）。
>
> **二、逐条产出与量测数字（3.17.1）**：
>
> - **形参名表同源**：`add_native_signature` 改收**签名串**并返回 `Result`——`signature::parse_signature` 出类型，唯一出口 `signature::export_signature_param_info` 同灌「形参名 + 必需下限」；键与签名表同（同一 name）。仓内零调用者故可改签名；文档明写「只拿得到 `MonoType` 的调用方应在自己那侧解析出签名串后再走本 API」，且本函数**不得**自己解析形参名。
> - **先红后绿**（红态在父提交 `537b46b74` 量测）：手注面四形态 + 重复命名 + 合法调用共 6 条，**5 条失败、诊断全为 `[]`**（命名实参被静默丢弃）；绿态同文件 **12 条全通过**——四形态 E1014 / E1010 / E1111 / E1111、重复命名 E1015、合法命名调用与省略可选位放行。
> - **三调用面同一判据同一码**：手注 FFI（typecheck 阶段）/ 普通用户函数（IR 阶段）/ stdlib 原生导出（`assert.assert`）三面**逐码相等断言**（E1014 / E1010 / E1111；E1015 见手注面与用户面）。观测阶段不同只反映「谁先发现」；宿主 native 不进 registry，IR 阶段本就不可解析（**先于本任务的嵌入面限制**）。
> - **判据单一源**：`bind_args_to_params` **1 定义**（`inference/arg_binding.rs:35`）+ **6 调用**（typecheck 2 + ir_gen 4）；第二份判据 `build_arg_slots` **0 处**（已删，`ir_gen.rs:516` 仅剩解释其历史的注释）；`ir_gen` 只剩纯搬运 `slot_order_arg_indices`（`ir_gen.rs:521`，1 定义 2 调用，零判定）。行数：`ir_gen` **+58 / −92（净 −34）**；全仓 4 文件 **+448 / −102**（其中 353 行是新测试）。
> - **不回归**：8 条 CLI 探针（u1–u4 / d1 / d2 / n1 / n3）变异前后**逐码一致**；`ir_gen` blob `0d837e9d…` ↔ `ffe1af8e…` 核对。合并初版一处自伤已修：IR 侧重判「缺参」把 `t.pick2(a = 2)` 合法重载误报 E1010（named_args 单测抓到）⇒ 缺参判定归 typecheck、IR 只重放槽位绑定；修后 19 条 named_args + 12 条新单测全绿。语料 348-348 零漂移、IR 快照零漂移。
>
> **三、独立验证（t16，冻结 `c3082a160`；红态基线 `537b46b74`，两者均经 `git merge-base --is-ancestor` 核实为 HEAD 祖先；探针全部自建）**：
>
> - **红→绿独立复现**（手注 FFI 面）：自建独立 Rust 单测 `verify_host_ffi.rs`（4 条：未知名 E1014 / 争槽 E1111 / 命名后接位置 E1111 / 合法命名调用放行），经 `scratch.py trial` 临时挂入 `tests/mod.rs`（apply → 测 → apply -R → CLEAN）。两态各用**各自接口形态**的探针变体（父提交 `add_native_signature(name, MonoType)` 旧签名 / 修复态 `(name, &str) -> Result` 新签名——接口本身就是修复面）：父提交 **3 failed / 1 passed**，3 条失败诊断**全为 `[]`**（`got []`）、TEST_EXIT=101；修复态 **4 passed / 0 failed**、TEST_EXIT=0。红态另在修复树撤销修复的变异态复现（`M7v2` 合并补丁：撤销修复 + 撤实现者测试 + 挂自建旧 API 探针）——3 failed `got []`、TEST_EXIT=101，还原后四文件 blob 与 HEAD 一致、`git status` CLEAN，撤销后 `ir_gen.rs` blob = `0d837e9d30de`（即父提交原 blob）。CLI 面（用户函数 + stdlib）两态逐码一致——符合预期：这两面 3.16.1 已响亮，本提交的修复面只在手注 FFI。
> - **四形态逐条**（CLI 自建探针，`c3082a160` 实测）：形参名错误 E1014 `has no parameter named x (available: a, b)`、缺必需位 E1010、位置与命名争槽 E1111、命名后接位置 E1111；附加：重复命名 E1015、stdlib 未知名 E1014、stdlib 缺必需位 E1010（help 点名 cond）、stdlib 争槽 E1111、合法命名调用 EXIT=0 输出 3/3、省略可选位 EXIT=0。
> - **判据真删而非改名**：`git show c3082a160:src/middle/core/ir_gen.rs` 内 `build_arg_slots` **0 命中**（父提交同文件 3 命中、定义在 :521）；`slot_order_arg_indices`（修复态 :521）逐行核 = `sort_by_key(|&arg_idx| positions[arg_idx])` 纯排序，无位次/名字/可选位判定残留，文档注释明示「不判位、不判名、不判可选位」；被删的 71 行判定块（父提交 :521-593，六条规则：空表响亮 / 位置占槽 / 顺序非法 E1111 / 未知名 E1014 / 争槽 E1111 / 入槽）已不存在；`bind_args_to_params` 1 定义 6 调用（typecheck 2：`expressions.rs:2854` / `:5653`；ir_gen 4：:7828 / :7926 / :8021 / :8351），与实现者报告一致；形参名唯一出口 = `export_signature_param_info`（environment.rs 手注 / typecheck::mod stdlib / ir_gen 查表），无第二处自行解析。
> - **自伤回归钉住**（`t.pick2(a = 2)`）：`git show c3082a160 --name-only` **不含 `named_args.rs`**（4 文件：environment.rs / tests/mod.rs / native_param_names.rs / ir_gen.rs）⇒ 钉子测试未被改；独立复跑该用例 1 passed、named_args 全套 **19 passed / 0 failed**。
> - **不回归门禁**（`c3082a160` 树）：`cargo test --all --quiet`（4×4 口径）EXIT=0——lib **2806** + 2 ignored、integration **297**、52/44/9/2/1/9 全绿 0 failed；`cargo fmt --check` / `clippy -D warnings` 均 EXIT=0；语料差分（双参数）348/348 差异为空；`check-snapshot-drift.sh` PASS（204 快照零漂移）；`check-docs-truth.py` EXIT=0；3.16.1 四形态对照（stdlib 面）E1014 / E1010 / E1111 / 放行。
> - **未覆盖面（t16 五项，原样抄录）**：①`testsuite.py run full`（CI 口径资源画像通道）未复跑——实现者已按裁决回落（独占波峰值 8.11 GiB > 预算 7.51 GiB，exit 2 属工具按设计拒绝）；本轮以 4×4 与 2×1 两口径的 `cargo test --all` 覆盖。②手注 FFI 面（host-FFI）只能经 Rust 测试驱动（`add_native_signature` 仓内零调用者、无 CLI 面）——「宿主嵌入进程内真实 FFI 调用」未测。③变参（`...`）命名实参形态未构造样本（无仓内判据样本）。④`check-boundary.py` / `check-fanout.py` 未复跑（实现者已过；与行为判据无交叉）。⑤CI（GitHub Actions）按约定不等，交收口环节核对。
>
> **四、对抗性复审（t17，被复核 revision `c3082a16051cb1bfb8b84bdc62ec6bd29d90ff2d2`，父 = `537b46b7488a60b42cdc35110e0a413c80a58169`；全部内容用 `git show <rev>:<file>` 取、行号在被引 revision 重取）**：
>
> - **① 判据单一源反向核**：`slot_order_arg_indices`（ir_gen.rs:521-527）**不按名重排**（排序键是共享判据给出的槽位号，名字只出现在 `arg_binding::bind_args_to_params` 内）、**缺参不兜底**（缺席的槽位根本不产生实参下标）、**不分可选位**（IR 两处传全可选、另两处传真值 `i >= required_min`）。全仓「按形参名找位置 / 找成员」的 grep 逐条核过：**槽位计算只 `arg_binding.rs:76` 一处**，其余 7 处命中各属别的关注点（泛型类型参数的约束接口检查 / const 泛型实参替换 / 泛型类型变量绑定 / 泛型类型参数替换 / const 泛型参数查表 / 方法定义查表 / 结构体字段查表）。
> - **② 职责边界（缺参只在 typecheck 响亮、IR 只重放槽位绑定）**：typecheck 侧 `bind_named_args`（`expressions.rs:2854`）传全必需 ⇒ 缺任一值形参 E1010；原生导出臂（`:5653`）传真可选位 `i >= required_min` ⇒ 只对真必需位 E1010、`?msg` 省略放行。IR 侧本提交改的两处（`ir_gen.rs:7926` RFC-004 绑定路径 / `:8021` 重载决议方法路径）**一律传全可选、只重放槽位绑定**（注释明写「缺参归 typecheck」与「按声明表重判缺参会把合法决议误报 E1010（`t.pick2(a = 2)` 实测）」）。
> - **③ API 形状**：environment.rs:990-993 明写「本函数**不得**自己解析形参名」并指向唯一出口 `export_signature_param_info`；:997-999 写明「只拿得到 `MonoType` 的调用方应在自己那侧解析出签名串后再走本 API」。
> - **④ 行为变化面**：代码上四处差异（同一形参命名两次 E1111→E1015 码变 / 未知名 E1014 文案变 / 空表 E1014 文案变 / 位置实参超 IR 侧声明表：静默丢弃→照发），**实测可及形态上无诊断码 / 文案变化**（typecheck 先判、同码），IR 产物在 5 个自建探针上**逐字节一致**（p1 / p5 走本提交改的分支且产物不变、p2 / p3 / p4 两态同码、p9 不可构造、p6–p8 源码层不可达）；t15 的 8 条 CLI 探针两态逐码一致。差异全部落在**本来就失败**的程序上（程序仍失败）⇒ D59「不改变原先成功程序的对外表现」不被触及。
> - **⑤ 测试质量**（不只看红绿）：12 条新测试（`typecheck/tests/native_param_names.rs`）逐条对 `test-specification`——命名 `test_<what>_<scenario>`、三段式 AAA 注释、断言全带自定义消息、setup 重复 ≥3 次已抽助手、助手内 panic 带上下文；**无「只断 exit code」式弱断言**（先 `assert!(`诊断非空`)` 再 `assert_eq!(`逐码相等`)`，两面都空不会空洞通过）；3 条负向用例（合法命名调用放行 / 可选位省略放行 / 显式给可选位放行）在 M1 红态下仍通过 ⇒ 它们**只防误报**，正是其用途。**M1**（去掉 `native_param_names` 灌装）→ `cargo test --lib native_param_names` **9 failed / 3 passed**、TEST_EXIT=101 ⇒ 新测试**真钉住修复**。**M2**（整段回退 `ir_gen` 合并）→ `cargo test --lib` **2806 passed / 0 failed / 2 ignored**、exit 0 ⇒ 合并面**无测试覆盖**。`pick2` 钉子（既有 `named_args.rs:253-283`，本提交未改）走**真实全链路**并断言**运行期值** `Int(20)` ⇒ **真钉住重载决议**（不是「没报错」），且它对「IR 不得重判缺参」是活的守门。
> - **⑥ 限制登记**：宿主 native 只进 `TypeEnvironment` 的两张表、**不进 `ModuleRegistry`** ⇒ IR 生成按 registry 解析命名空间成员，故宿主 native 在 IR 阶段本就不可解析（E3006），**先于本任务的嵌入面限制**；该限制与「为何手注面必须在类型检查阶段观测」写在**测试文件文档注释**与 `environment.rs` 字段文档里（不只在报告里）。
> - **门禁八项在被复核 revision 全部复跑全绿**：fmt --check / clippy -D warnings / `cargo test --all`（**3220 passed、0 failed**；lib 2806 + 2 ignored、integration 297）/ 语料差分 348-348 零漂移 / IR 快照 204 零漂移 / docs-truth / check-boundary（0 豁免 0 违规）/ check-fanout（新符号无同名多定义条目）。
> - **四条观察项（原样登记：已核、判定不构成 finding、无需处置，但登记在案以免日后被当成未处置违规）**：
>   - **O1**：提交信息称「本次合并后**全仓只剩共享判据一处**」——就**槽位绑定**而言成立；但 typecheck 的**方法重载臂**仍有一份「按名查候选表（E1014）+ 元数（E1010）」的自有判定（`inference/expressions.rs:5578-5600`），它用的是**候选组代表**的形参表（`method_overload_param_names`），与共享判据用的**已决议候选**表不同。判定依据：该处**先于本提交存在、本提交未触碰**（`git show c3082a160 --name-status` 只有 4 个文件，无 expressions.rs），且属「候选匹配」关注点（表不同、语义不同），不是槽位绑定。**措辞偏宽，登记为观察**。
>   - **O2**：`add_native_signature` 改收签名串后，**无** `MonoType → 签名串` 的共享助手 ⇒ 未来只拿得到 MonoType 的调用方只能各写一份序列化。判定依据：形参名 / 必需下限已被文档锁死在唯一出口 `signature::export_signature_param_info`；当前**零调用者**；文档已写明「只拿得到 MonoType 的调用方应在自己那侧解析出签名串后再走本 API」⇒ 残余风险仅在「串怎么构造」一段，**低**。
>   - **O3**：本提交的**合并部分**（ir_gen 去重）**无任何测试覆盖**（M2：整段回退 ir_gen 后 `cargo test --lib` 仍 2806 passed 全绿）。判定依据：「去重」无新行为可钉，性质使然；「单一源」性质由 grep + `check-fanout.py` 门禁保证。**可选加强**：在 `native_param_names.rs` 增一条 **IR 层**用例（覆盖 ir_gen:7926 / :8021 的路径），让合并面也有护栏。
>   - **O4（归因更正）**：提交信息与报告把 d2 的 E1010 归因为「**方法臂**元数预检先触发」——实测该 E1010 的 help 是通用文案「check the argument count of the function call」，**不是**共享判据「缺参」的 help（「缺少必需形参：…」）⇒ 触发者是**原生（native）臂**的元数预检（`native_arity` 表）。**代码行为结论不变**（父提交既有、本次未改变，已复核），仅**归因措辞需更正**；且「`list.push` 重复命名即 E1010」须限定为「**元数不符 + 重复命名**」——只重复命名而元数相符时（`list.push(list = v, list = 9)`）两态都是 **E1015**（重复命名先于元数判）。
>
> **五、门禁与资源通道（一句外链）**：本任务**首次真用** `python scripts/dev/testsuite.py run full` → **exit 2**——独占波峰值 8.11 GiB > 本机预算 7.51 GiB（缺口 618 MiB），工具按设计**拒绝静默跳过** ⇒ 回落 `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 cargo test --all --quiet` → **exit 0**（3220 passed / 0 failed）。该通道状态链 **#486**：`full` 在本机因峰值外推偏保守而不可用，**校准（2600 档由实测校准）属 WBS 0.5.2**。
>
> **六、计数表：本轮不动**——`3.17.1` 已于 2026-10-15 登记并计入表头（「2026-10-15：+2 = 3.17.1 / 3.18.1」），本轮只勾选 + 注记，**三级条数不变** ⇒ `09` 顶部的二级 67 / 三级 176 **不动**。本轮**不新增**计数偏差（父条目 3.17 的「1 个三级任务」注记本就正确）；上一节「十一」记录的 6 处预存偏差（父条目计数注记滞后）**与本轮无关、不由本轮引入**，仍按该节口径留待账目复核。
>
> **七、批次树回收（gc 与 status 原始输出）**：
> `python scripts/dev/scratch.py gc` → `[scratch] 已删除 verify-p3-t16（释放约 10235.4 MB）` / `[scratch] 已删除 verify-p3-t17（释放约 10079.2 MB）` / `[scratch] gc 完成，共释放约 20314.6 MB`；
> `python scripts/dev/scratch.py status` → `[scratch] 没有批次树`；
> `git worktree list` → 只剩主挂载（`dev`）。
> **被 gc 一并删除的还有两棵批次树内的验证证据**（`verify-p3-t16/` 的 `EVIDENCE.md`、`verify-p3-t17/` 的 `EVIDENCE-t17.md` 与探针 / 补丁 / 原始日志）——它们是**注册批次目录**，随树回收；本节第三、四节已把其量测数字、门禁数值、变异表与四条观察项**原样落进实录**以留证，结论不因文件删除而失效。**未受影响的留档**（非注册批次目录，gc 不动）：`wbs317`（实现者证据：evidence-3.17.1.md / red-test.txt / trial-before-after.txt + fix-3171.patch / 归因表等；位于工作区 `target/` 下，**不入库**；结论已落本节，工作区路径不作为引用出处）。
> 验收表述（可解析）：`git status` 干净；`python scripts/dev/scratch.py status` = 没有批次树；`git worktree list` 只剩主挂载；仓库外无本批新增目录。
>
> **八、文档真构建与提交卫生**：本轮的勾选 / 注记 / 登记**攒成一笔**文档提交（AGENTS.md 第 8 条）；提交前本地跑 `npm run docs:build`（AGENTS.md 第 10 条⑨）与 `python scripts/ci/check-docs-truth.py`，markdownlint 零告警。**不 push**（成员不推送，见 AGENTS.md 第 10 条⑧）；**CI / Deploy Docs 结论交统一核对环节**回填，不在会话里等。
>
> **九、遗留项与登记**：
>
> 1. **轮 1 收口登记的遗留项 ① 已关闭**：`src/driver/arms.rs` 的「决策 B1」注释与 3.4.6 后代码事实相反一项，由 `537b46b74`（`:memo: docs(driver): 修正与事实相反的 B1 注释（3.4.6 收尾遗漏）`）就地改写为现状事实（行为中性、仅注释行；措辞与 `src/driver/tests/program.rs` 中同一事实的表述对齐）。
> 2. **本轮新登记**：O1–O4 四条观察项（已核、不阻断、登记在案）；0.5.2（`full` 通道的 2600 档实测校准 + pre-commit / CI 接线）；#489 ureq 3 迁移与 `.cargo/audit.toml` 豁免的硬性移除条件；3.8 / 3.12 待裁决、3.18 前置待 P6——**均不在本队范围**。
>
> **十、CI 门禁结论（AGENTS 第 13 条①，定时核对 2026-10-21 回填）**：该批最后一笔推送 tip = `1f5eb8665`（`:memo: docs(meta): verify 清单补三个 CI 独有门禁，并写明「发射点」口径`）→ CI **39604109534 success**、Deploy Docs **39603305398 success**；该 tip 覆盖 `e7a3fd4b1`（再生错误码生成物）与其前全部提交。`e7a3fd4b1` 自身那一轮 CI **39603305171 cancelled**——按第 10 条③「连推会取消前一轮」，其结论由后继 `1f5eb8665` 的 success 承接。失败链条供追溯：`c3082a160`（**39587784184 failure**）与 `190a0e7a3`（**39599653111 failure**）同为同一 job「Error-code docs up to date（generator idempotent）」，根因 = 3.17.1 删除判定 + 行号平移后未重跑生成器；由 `e7a3fd4b1` 再生、`1f5eb8665` 收口后转绿，累计红 5 轮，促成 AGENTS 第 10 条⑤ 的措辞修补（`1f5eb8665` 即该修补）。
>
> 核对命令与原始输出（2026-10-21 13:19Z 复跑）：`gh run list --workflow ci.yml --limit 4` → `39604109534 success`（`1f5eb8665`）/ `39603305171 cancelled`（`e7a3fd4b1`）/ `39599653111 failure`（`190a0e7a3`）/ `39587784184 failure`（`c3082a160`）；`gh run list --workflow docs-deploy.yml --limit 3` → `39603305398` / `39599653238` / `39587784210` 均 success。

## 文档引用治理与「不可解析引用」门禁（2026-10-21）

> **触发**：清理工作区证据目录时发现，`09` / `09a` / `CHANGELOG` / 分支维护指南里有 24 处把「工作区内、被 `.gitignore` 排除的目录」（批次树 / 探针 / 证据目录）当作**证据出处**——这类目录在 clone 后不存在，外部贡献者一条数字都复核不了（AGENTS.md 元规则 ① 与第 10 条④）。其中一处是 4.4 收口轮的修法：把旧路径改指向另一个本地路径，等于把违规下移一层而非消除。
>
> **一、引用就地内联（47 处 = zh 24 + en 23）**：结论、数字、CI run id、交付 commit 就地写进本文与 `09`，出处改为入库路径（实录章节锚）或 revision / run id / issue 号。CI run id 明细与 S9 七组清单**本就已内联**，被删掉的只是冗余的路径指针。en 镜像由 auto-translate bot 依 zh 源再生。
>
> **二、新增机械门禁** `scripts/ci/check-docs-refs.py`（挂进 `ci.yml` 的 docs job）：跟踪文本文件里反引号与 markdown 链接中的路径状 token，凡被 `.gitignore` 命中且带具体子路径者即红；占位符形态（规则陈述）与裸目录名不判；系统运行时目录（包管理器缓存、构建产物、playground 发布目录、本地 agent 工作台）走**逐条写明理由**的豁免表（26 条；明细写进门禁自己的运行报告——CI 里作为 artifact 上传，不入库）。**判据能红能绿**：自测 9 例（4 反例必红 / 5 正例必绿，含跨分批边界一例）；端到端变异验证——把一处违规引用打回去 → 门禁红 → 还原 → 绿 → 该文件与变异前逐字节一致。
>
> **三、规则措辞补口**：`AGENTS.md` 第 10 条④ 明确「『仓库内路径』指 git 跟踪的入库文件」，第 9 条补「工作区里的东西不是证据出处」，第 10 条⑤ 的本地 verify 清单补本门禁（用户 2026-10-21 批准）。
>
> **四、两平台一致性（首版门禁的缺陷与修法）**：首版门禁 **Windows 本地绿、CI（Linux）红**——`git check-ignore` 对「以 `/` 结尾的目录形态路径」两平台行为不同（Linux 报真实规则，Windows 报一条**空 pattern** = `.gitignore` 的空行），只认非空 pattern 时 Windows 侧漏判 `docs/public/wasm/`（playground 发布目录，RFC-037 引用）。改法：每个 token 查三形态（带斜杠 / 去斜杠 / 去斜杠 + 探针子项），任一形态命中**非空** pattern 即算命中。修后 Windows 与 Linux 双跑**同数**：1122 个跟踪文本文件 / 5533 个路径状 token / 0 违规 / 26 条豁免 / selftest 9/9。教训：门禁脚本**至少要在两种 OS 与检出形态上各跑一遍**才敢挂 CI（CI 的 job 日志要 admin 权限，取不到时用「Linux 真 clone 复现」定位）。
>
> **五、CI 证据（AGENTS.md 第 13 条①，定时核对回填）**：修复笔 `c7a0dd417` → CI run **39626722284 success**；前一笔 `fff9580f8` → CI run **39623038087 failure**（红在新增的「Check doc references are resolvable」，即上述平台差异，已由 `c7a0dd417` 修掉），同笔 Deploy Docs **39623038228 success**、Auto Translate Docs **39623038027 success**；更早各笔（`ec2893e0d` / `ff2104b0c` / `ffd43b1d0`）的 CI run 因连推被 concurrency **cancelled**（39621547581 / 39620372322 / 39619834870），按「以后代 run 的结论覆盖前一笔」取结论。
>
> **五之二、本节入库那笔的 CI 证据（2026-10-22 定时核对回填）**：本节入库笔 `612c65322` → CI run **39798235412 success**（7 个 job 全绿，含新增的「Check doc references are resolvable」与「Error-code docs up to date」两步）、Deploy Docs **39798235484 success**；同笔 Auto Translate Docs **39798235413 failure**，失败在最后一步「Commit translations」——翻译本身成功（Translate documents / Format / Lint 均 success），但推送时远端已被本批的代码提交与 bot 自己前一笔自动翻译提交推进，`git pull --rebase` + `git push` 被挡下，**结果是本文的 en 镜像尚未含本节**（zh 为唯一源；下一次触及 `docs/src/**/*.md` 的推送会再次触发该工作流补译）。并行的 4 笔 bugfix（`eb4124cf0` / `14e9b2704` / `96638ec9f` / `06ac6d687`）的 CI run **39797917813 cancelled**（连推被 concurrency 取消），按「以后代 run 的结论覆盖前一笔」取结论。
>
> **五之三、缺口闭合与本批收口（2026-10-23 定时核对回填）**：补推后 bot 的 Auto Translate Docs run **39904034074 success**（bot 提交 `449360a5a`）——**本文的 en 镜像已含本节**，上一轮的缺口闭合；同笔 CI run **39904034164 success**、Deploy Docs **39904034075 success**。上一轮 39798235413 的失败是 bot 自身的竞态：它前一笔翻译提交与本批的代码提交同时推进远端，`git pull --rebase` + `git push` 被拒。**操作教训**：推完不要紧接着再推——给 bot 留出落地窗口（本次间隔约 15 分钟即成功）。
>
> **五之四、一次提交带入并行会话改动的更正与计数表复核（2026-10-23）**：本节收口那笔 `8a83485ef` 的**提交信息只描述了本文的新增段，但该笔实际还一并带入了并行会话的两处文档改动**——① `09` 的三级计数表头 177 → 182（新增 3.6.5 / 3.6.6 / 4.10.6 / 4.10.7 / 4.10.8，并改了 4.10.4 / 4.10.5 的描述）；② `09a` 两处 `git worktree list` 输出里的本机路径（`E:/git/YaoXiang <sha> [dev]`）被清成「只剩主挂载（dev）」。成因：队长提交时对 `09a` 执行了 `git add`（把对方的未暂存改动一并吸进暂存区），而 `09-execution-wbs.md` 的那笔改动本就在暂存区里。
>
> **计数表复核结论：表头 182 不改**。按 09a §十五 ① 的口径（逐二级看板行括号内的「N 个三级任务」求和），改动前 `449360a5a` 上**求和 = 170 / 表头 = 177（差 +7）**，改动后**求和 = 177 / 表头 = 182（差 +5）**——该笔把两口径的差距**由 7 收敛到 5**，是朝自洽方向的修正，不是新引入的不自洽；残余的 5 属该行两口径历来不完全对齐的既有漂移（§十五 已记「二级计数行未动…不为凑数改数」）。**故本笔不对计数表做任何改数**。
>
> **六、登记口径**：本节是**文档治理**记录，不新增 WBS 看板行，`09` 顶部计数表不变。

## P4 4.10 多文件语义缺口 收口（轮 1：4.10.4 / 4.10.5 + 加框 3.6.5）

> **一、范围与轮次关系**：本节 = **4.10** 二级任务下的第一轮收口，范围 = **4.10.4**（issue [#467](https://github.com/ChenXu233/YaoXiang/issues/467)，rootless 多文件初始化按 `use` 依赖拓扑序）、**4.10.5**（issue [#476](https://github.com/ChenXu233/YaoXiang/issues/476)，rootless 定义 `main` 未调用时退出码 1）、**加框并入 3.6.5**（issue [#479](https://github.com/ChenXu233/YaoXiang/issues/479)，选择式导入的函数值在顶层重绑后静默 `void`；**加框自 4.10 队、用户 2026-10-21 批准**）。本批另含一笔**跨二级加框**的 std 修复（`src/std/json.yx` 的 `\uXXXX` 下标 + `src/std/tests/json_ops.yx` 长期零覆盖），它不属 4.10 的任务面，只在「九」登记并作为「测试假通过」实例留证。
>
> **二、四笔修复的落地内容与证据**（提交号取**推送后** `origin/dev` 上的哈希；`09` 的勾选行按约定不写提交号）：
>
> - **4.10.4（#467）**：根因 = 各单元 `init` 段按**源文件路径字典序**拼接（发现期的路径字典序排序，**不是**「发现顺序」——`09` 原行写「发现顺序」已于本轮更正；实测把依赖文件改名即可翻转结果，故「依赖先行」从来不是不变量）。改法 = 在 Linking 臂按 `use` 依赖求 **Tarjan SCC 拓扑序**（环内按路径字典序作确定性约定、SCC 间按最小单元下标优先出队）后重排单元顺序，`link_module_irs` 拆成「文件表序」与「初始化执行序」两个概念，**零 IR 形状变更**。先红后绿 = 把执行序改回恒等（退回发现序）后 4 条顺序用例全红、CLI 复现「入口读依赖顶层值 = `void`、exit 0」与「运行期 E6007、exit 1」两形态；还原后逐字节回位、7/7 绿。语料 = 新增 2 个多文件夹具，基线 **+2 行**且原 348 行逐字节不变；**IR 快照零漂移**。
> - **4.10.5（#476）**：运行入口改返回**结局枚举**（已完成 / 已定义但未调用），CLI 只据结局定退出码 **1**（2 保留给 CLI / 配置用法错）；两行提示文案**逐字不变**、不新增第二条诊断；值型 `main` 已在初始化期求值者不受影响。钉测试 2 条落 `tests/integration/cli_e2e.rs`。
> - **3.6.5（#479）**：值路径的别名兜底臂按「**模块来源 + 导出种类**」分叉——用户模块导出的函数 → 物化为闭包；std 常量 / 原生函数 → 保持原零参调用形态；**不新增任何诊断码、`src/std` 零改动**。钉测试 3 条（选择式重绑 / 选择式直接调用 / 整体导入重绑）。生成物：`docs/src/reference/error-code/E1xxx.md` 与 `E8xxx.md` 各 1 行行号平移（1789→1821 / 1976→2008），**生成器再生、未手改**。
> - **std 侧（跨二级加框，用户批准）**：`src/std/json.yx` 的 `parse_string` 在 `e == "u"` 分支把**反斜杠位置**传给 `parse_uchar`，而 `hex4` 的契约要求从 `'u'` 处起读 4 位十六进制 ⇒ 所有 `\uXXXX` 都报 “invalid \u escape”；改传「`'u'` 位置」后 BMP 情形 adv=6、代理对 adv=12 的算术不变，报错位置仍取反斜杠（用户可见位置不变）。同一笔修好 `src/std/tests/json_ops.yx` 这个**从未跑过的测试**，并连带修好夹具自身三处错误（低代理段多拼一个十六进制位、尾随 `\u0039` 从未拼进源串、错误定位断言写成「含 2」而实际消息是 “invalid literal at line 1, column 7”）。**IR 快照同提交再生**（`src/middle/core/tests/snapshots/src/std/tests/json_ops.snap`），语料基线 1 行变化。
>
> **三、复审判决（批次复审轮 1，异模型家族）与三条未覆盖分支的登记**：复审对两条已提交实现给出 **#467 = pass、#479 = pass**（两条实现本身经独立核对均成立），同时指出**两项 high 级未覆盖分支原先只存在于代码注释、未进 WBS**——本轮已按复审意见登记为 **4.10.6**（导入环无告警、仍报误导性 E6007）与 **4.10.7**（原生函数作值静默不动作），另登记 **4.10.8**（纯 yx 的 std 导出作值；复审标 medium、**未核实**）与 **3.6.6**（同族判据收敛，重构项）。复审另指出**延后理由的分类有误**：延后理由写的是「要新诊断码 ⇒ D 类待裁」，但 D59 的分流把「静默错配 / 静默不动作」列在 **C 类**（旧行为非成功，可随重构顺路修）⇒ 不得以未登记的 D 类标签静默延后。据此，三条已在 2026-10-23 获用户裁决（issue [#491](https://github.com/ChenXu233/YaoXiang/issues/491) / [#492](https://github.com/ChenXu233/YaoXiang/issues/492) / [#493](https://github.com/ChenXu233/YaoXiang/issues/493) / [#494](https://github.com/ChenXu233/YaoXiang/issues/494)）：**4.10.6 = 编译期 W 码告警、环不得静默**（告警面「存在环」还是「环内读未初始化值」、是否允许 `#[allow]` 抑制、是否与既有 W10xx 家族的「证据等级」对齐——三条**待斟酌**，落地前须定稿）；**4.10.7 = 真正支持**（把原生函数物化成值，**不加诊断码**，终态 = `use std.io.{println}` + `f = println` + `f("hello")` 真的打印 hello）；**4.10.8 = C 类、不加码、可直接修**；**3.6.6 = 重构项**（行为不变）。
>
> **四、已推翻的假设（留证免重复劳动）**：「同一文件两种 `use` 拼写 ⇒ 依赖边丢失」**不成立**——该形态报既有诊断 **E5001**（`use` 目标不在单元集），不是「静默按发现序跑」。复审把它列为已推翻的假设，故 4.10.4 的「无依赖信息三情形」各有确定性处置，不存在第四条静默通道。
>
> **五、任务清单与终态**：
>
> | 任务 | 主题 | 终态 |
> | --- | --- | --- |
> | t1 | 复现与定位：两个 P1 的红态取证 + #467 修法可行性裁决 | completed |
> | t2 | #467 初始化拓扑序（实现 + 钉测试 + 语料） | completed（第 3 次尝试收口） |
> | t3 | #467 独立验证 | completed，verdict = pass |
> | t4 | #479 选择式导入 + 顶层重绑（实现 + 钉测试） | completed（第 2 次尝试收口） |
> | t5 | #479 独立验证 | completed，verdict = pass |
> | t6 | #476 退出码 1 | **failed**（provider 侧断流累积，6 次尝试） |
> | t7 | #476 独立验证 | **cancelled**（交叉阻塞 + 加框时序，由队长接管后取消） |
> | t8 | 批次复审（#467 / #479 两条实现） | completed，verdict = pass ×2 |
> | t9 | 批次收口（勾选 + 3.6.5 加框 + 实录 + 门禁 + 回收批次树） | completed |
>
> **六、每角色试次账与 provider 侧失败处置**：
>
> | 角色 | 承担 | 试次账 |
> | --- | --- | --- |
> | 探针 | t1（只读取证）、t6（#476 实现接手） | t1 一次成；t6 **6 次尝试未成**——期间多次遇 provider 高峰断流（`PI_AI_ERROR`，2064「服务器短暂繁忙，通常 1-5 分钟内恢复」），任务终态判 failed |
> | 实现 A | t2 首派、t4 接手、t6 接手审阅 | t4 一次成（attempt 2）；t6 的实现最终由其审阅并补全、随 t4 的「干净树全量」方案一并落库 |
> | 实现 B | t4 首派 | attempt 1 因 provider 侧 **400 `invalid_request_error`**（thinking 模式下 `reasoning_content` 必须回传）判 failed → 改派 |
> | 实现 C | t6 期间的实现接手 | 连续两次同款 **400 `invalid_request_error`** ⇒ 按 AGENTS 第 11 条「换人」处置：**移出团队**，其任务退回共享池后改派 |
> | 实现 D | t2 接手 | attempt 2 |
> | 实现 E | t2 收口、t6 一次尝试 | t2 第 3 次尝试完成；t6 的一次尝试亦因同款 400 断流判 failed |
> | 验证 | t3、t5 | 各 1 次，verdict 均 pass |
> | 复审 | t8 | 1 次，verdict = pass ×2 |
> | 收口 | t9 | 1 次 |
>
> **provider 侧断流与换人的处置口径**（本轮实测可复用）：① **断流不是缺陷证据**——被断流打断的任务不得据「零产物」判失败结论，只能判「未完成」并改派；② 换人后**不重跑已有结论的轮次**（AGENTS 第 12 条：换人会把被移出成员的全部任务退回共享池，已判决的轮次不应被调度器重跑），本轮把退回的 t6 / t7 直接改派或取消，未重复劳动；③ **断流期间零产物**的成员一律先看盘上状态再决定接手方（本批多次出现「实现已在工作树、只是没人认领」的情形）。
>
> **七、仓库并发写入与 pre-commit stash 窗口（本轮两个实例）**：
>
> 1. **并发写入**：批次期间主工作树被**另一个并发会话**写入，其暂存区已含 `.github/workflows/ci.yml` 与 `scripts/ci/check-docs-refs.py` 等**他人文件**。处置 = **提交权收口到队长**：批次期间全体成员**不得执行 `git commit`**（`git add` 仍按显式路径），改动只留在工作树，提交由队长统一做——否则一方的提交会把别人的暂存内容卷进去。本批四笔修复最终由队长以四笔独立提交落库即源于此。
>    **收口笔的撞车实例**（本轮实测）：收口提交与队长的并发提交撞车——队长那次 `git commit` 读到了**本轮已暂存的两份文档**，把本轮内容一并带进它自己的提交（提交消息沿用了队长的表述，只描述了「治理一节」的 CI 回填）。处置 = 用 `git commit --amend` 把消息改成**同时覆盖两处内容**（仍是**一笔**提交、**一个**消息），并把本实例登记在此。**教训**：提交前必须确认索引里**只有自己的路径**（`git status --short` 首列全为 `M ` 且路径属本任务）——并发会话的暂存内容会被别人的 `git commit` 卷走（这正是本节第 1 条的处置方向：批次期间提交权收口到队长）。
> 2. **批次目录被第三方删除**：`scratch.py` 的 `rm` / `gc` 会对**整个批次目录**递归删除（该目录内的报告与探针一并消失）。本轮实测：批次目录连同其内的定位报告与全部探针日志在实施期间被删除（**归因未核实**：执行方不明）。处置 = 报告与探针改放到批次目录**之外**，且关键数字与结论**直接写进任务输出**（不依赖工作区文件作为出处）；批次树重建后由量测方按冻结 revision 接管。
> 3. **pre-commit stash 窗口**：本轮实例 = 提交 std 侧那一笔（amend 补入 IR 快照）期间，pre-commit 钩子把**全部未暂存改动** stash 走、跑完再恢复 ⇒ 并行读工作树的成员看到「只剩已暂存文件」的假象（本轮有成员据此报告「改动丢失」）。处置按 AGENTS 第 10 条⑦：①**提交前先给未暂存改动留 patch 备份**（本轮实例的三文件 sha256 = `02e89141…` / `3736961f…` / `98eee1d7…`，复原后逐一相同）；②提交期间他人只按**冻结 sha** 量测、不读工作树；③每笔提交完成后用 `git status` 核对剩余改动完整性。
>
> **八、本地哈希重写对照**（本批四笔在推送前经历 `git rebase` 与 `git commit --amend` 两类重写；`09` 的勾选行按约定不写提交号，本实录保留**推送后** `origin/dev` 上的哈希并给出重写链）：
>
> | 内容 | 推送前本地 → 推送后 `origin/dev` |
> | --- | --- |
> | 4.10.4（#467） | `950a5838e` →（rebase）→ `eb4124cf0` |
> | 3.6.5（#479） | `0ef2e8887` →（rebase + amend）→ `14e9b2704` |
> | std 侧（`\uXXXX` + `json_ops.yx`） | `48bc37af0` →（**amend 补入 IR 快照**）→ `c67de7e2f` →（rebase）→ `96638ec9f` |
> | 4.10.5（#476） | `47c2d5292` →（rebase）→ `06ac6d687` |
>
> `48bc37af0` → `c67de7e2f` 这次 amend 的内容 = 把**再生的 IR 快照**（`src/middle/core/tests/snapshots/src/std/tests/json_ops.snap`）补进同一提交：原提交只含 `src/std/json.yx` / `src/std/tests/json_ops.yx` / 语料基线三个文件，amend 后四个文件——即「改 IR 行为必须同提交再生基线」的红线在**同一次提交内**兑现，而不是留一笔「基线欠账」。取证 = `git reflog --date=iso`（`commit` → `commit (amend)` → `rebase (pick)` 三段可逐条对读）。
>
> **九、语料 / IR 基线在当前字节的复验 + 本轮两条新发现**：
>
> - **当前字节复验**（收口前在 `origin/dev` 头上重跑）：`tests/baselines/corpus-parity.jsonl` = **350 行**，sha256 = `76bd7a1cc82797fe0c2fb4f3406506ad65e7fe268d15b7d807448f4a30b5118b`；在该字节上重跑语料差分探针，产物与基线**逐字节相同**（差分为空）⇒ 当前字节零漂移。**说明**：两条验证轮（t3 / t5）的判决绑定的是**更早**的语料字节（`41cc0343…`），与当前的 `76bd7a1c…` 不同——复审已提示该差异，本轮按当前字节复验并落账；此后引用语料数字须先确认所引字节。
> - **新发现 ①：`src/std/tests/json_ops.yx` 长期零覆盖**——该文件定义了 `main` 却**从未调用**；script 模式下顶层语句即程序主体 ⇒ 15 个 `check_*` 的断言**一次都没执行过**、退出码恒 0，即**测试假通过**的典型实例。`std.json` 的 `\uXXXX` 缺陷因此长期未暴露。本轮已修（补 `main()` 调用 + 修夹具自身三处错误 + 修 `std.json` 的下标），修后该文件 15 个 check 的断言全部真跑并通过（exit 0）。**教训**：「有测试文件」不等于「判据被执行」——script 形态下「定义了 `main` 却未调用」的测试文件是**假绿**来源，应作为一类独立检查项（对照：#476 的「定义 `main` 未调用」在 rootless 形态下 exit 0，也让这类文件长期以 exit 0 充数）。
> - **新发现 ②：环导入无 W 码告警**——4.10.4 把环内顺序**确定性化**了，但环本身仍被静默容忍：环内单元读未初始化顶层值 ⇒ 误导性 E6007「type mismatch」。已登记为 **4.10.6**（issue [#491](https://github.com/ChenXu233/YaoXiang/issues/491)；用户 2026-10-23 裁决 A = 编译期 W 码告警、**不得静默**；W 的告警面待斟酌）。
>
> **十、门禁数值**：
>
> - **收口前在当前 `origin/dev` 头上本地重跑**：`npm run docs:build` → exit 0（文档提交前的**真实构建**，AGENTS 第 10 条⑨——markdownlint 抓不到 VitePress 的 Vue 编译错误）；`python scripts/ci/check-docs-truth.py` → exit 0；`python scripts/ci/check-boundary.py` → exit 0（违规 0 处，`pub(crate)` 计数守基线）；`python scripts/ci/check-fanout.py` → exit 0；`python scripts/ci/check-docs-refs.py` → exit 0（1124 个跟踪文本文件、5535 个路径状 token、无不可解析引用）；错误码生成器重跑后 `git diff --quiet` → exit 0（生成物零漂移）；语料差分重跑 → 与基线逐字节相同（350 行）。
> - **各修复面自身的门禁**（各验证轮在冻结 revision 上量测）：#467 —— fmt 0 / clippy 0 / `cargo test --all` exit 0（lib **2810 passed** + 2 ignored、integration 297 passed、0 failed）/ docs-truth 156/156 页 / boundary 0 / fanout 0 / docs-refs 0 / 生成器重跑后零漂移 / **IR 快照 diff 为空**；#479 —— 同口径 exit 0（lib 2810、integration **300** = 297 + 新 3 条钉测试逐名通过）；#476 —— **干净树**全量 exit 0（9 个 test target 全 ok、**3227 passed / 0 failed**，含 `tests/yx_runner.rs::test_all_yx_files_pass`）。
> - **门禁纪律的一条实例**：全量 `cargo test --all` 在「共享工作树带未提交改动」时曾红在 `tests/yx_runner.rs::test_all_yx_files_pass`（`src/std/tests/json_ops.yx` exit 1），一度被记为「本批缺陷」；在**干净冻结 revision** 上复测该用例通过 ⇒ 该红点属「树上带 #476 补丁状态」的测量，不是修复面的属性。**教训**：全量门禁必须在**隔离的干净树**上量测，否则会把「共享树状态」误判为「交付缺陷」。
>
> **十一、批次树回收（gc 与 status 原始输出）**：
>
> `python scripts/dev/scratch.py gc` → `[scratch] 已删除 wbs410（释放约 13268.2 MB）` / `[scratch] gc 完成，共释放约 13268.2 MB`；
> `python scripts/dev/scratch.py status` → `[scratch] 没有批次树`；
> `git worktree list` → 只剩主挂载（`dev`）。
> **被 gc 一并删除的还有批次树内的验证证据**（各验证轮的探针、变异补丁、原始日志与结论文本）——它们是**注册批次目录**，随树回收；本节第二、三节已把其结论、数字、变异表与门禁数值**原样落进实录**，结论不因文件删除而失效。**未受影响的留档**：非注册批次目录下的证据目录（不入库；结论已逐条落进本实录，工作区路径不作为引用出处）。
> 验收表述（可解析）：`git status` 干净（收口文档提交后）；`python scripts/dev/scratch.py status` = 没有批次树；`git worktree list` 只剩主挂载；仓库外无本批新增目录。
>
> **十二、文档真构建与提交卫生**：本轮的勾选 / 注记 / 登记**攒成一笔**文档提交（AGENTS 第 8 条）；提交前本地跑 `npm run docs:build`（AGENTS 第 10 条⑨）与 `python scripts/ci/check-docs-truth.py`，markdownlint 零告警。**不 push**（成员不推送，见 AGENTS 第 10 条⑧）；**CI / Deploy Docs 结论交统一核对环节**回填，不在会话里等（AGENTS 第 13 条）。另：本轮顺手把两处历史实录里残留的**本机绝对路径**（`git worktree list` 输出的引用）改写为不含本机路径的表述（元规则 ①）。
>
> **十三、遗留项与登记**：
>
> 1. **待落地（已获用户 2026-10-23 裁决，登记在 `09`）**：4.10.6（issue #491，编译期 W 码告警、不得静默；W 的告警面三条待斟酌）、4.10.7（issue #492，真正支持原生函数作值、不加码）、4.10.8（issue #493，C 类、不加码、可直接修）、3.6.6（issue #494，判据收敛重构项、行为不变）。
> 2. **已知不完美项**：① **t6 任务记录终态为 failed、t7 取消**（provider 断流 + 加框时序）⇒ #476 的**独立验证轮未单独跑**；其验证证据来自「干净树全量 exit 0（3227 passed / 0 failed）」与该轮内的退出码钉测试（`tests/integration/cli_e2e.rs` 两条）。② 两条验证轮（t3 / t5）的判决绑定的是更早的语料字节，本轮已按当前字节复验（见「九」），但**验证报告本身的字节绑定仍是旧值**——引用时须说明。③ 复审**未亲自重跑**两条验证轮的变异构建（批次树已由多人释放，重跑需独占树 + 一次增量构建），以「产物核对 + blob 逐字节核对 + 自建探针」折算。
> 3. **未覆盖面**（各验证轮自列，本轮不补）：① 环导入的 W 码告警未实现（已登记 4.10.6）；② `check` 与 LSP 形态未单独探针（与 `run` 共用 Linking 臂，库级钉测试间接覆盖）；③ 变异只做「退回发现序」一个方向——环内路径序 / 最小下标 tie-break 各自失效形态未做反证变异（其行为由钉测试断言覆盖）；④ 多包 / workspace 形态不在 CLI 探针内；⑤ #479 侧：原生函数作值、三段模块路径、不存在导出名、`ExportKind` 其他变体、变异轮未跑全量、LSP 路径；⑥ 复审自建的 7 个探针中，「纯 yx 的 std 模块函数作值」一条标 **未核实**（已登记 4.10.8）。
> 4. **提交信息 scope 被拒两次**：本批两笔提交的初稿 scope 取自**源码目录名**（`ir_gen` / `cli`），两次被 `scripts/hooks/check-commit-msg.py` 拒（其 scope 允许表无这两个词），改用**子系统名** `codegen` / `util` 后通过（允许表：`frontend` / `middle` / `backends` / `std` / `formatter` / `lsp` / `package` / `util` / `proof` / `driver` / `codegen` / `monomorphize` / `lifetime` 等）。**教训**：scope 取**子系统**而非源码目录名。
> 5. **本批未触及的既有欠账**（不在本队范围，原样转出）：3.8（native 对用户函数名的遮蔽）、3.12（运行时模式间的执行器配置传播）、3.18（跨模块类型身份，前置待 P6）、0.5.2（pre-commit / CI 接线 + 资源档实测校准）、#489 ureq 3 迁移。

> **十四、CI 门禁结论（AGENTS 第 13 条①，定时核对 2026-10-23 回填）**：末笔 push tip = `5c0a47570`（本批收口文档提交）→ CI **39907068941 success**、Deploy Docs **39907068812 success**，覆盖本批四笔修复（`eb4124cf0` / `14e9b2704` / `96638ec9f` / `06ac6d687`）与文档收口。覆盖关系（第 10 条③）：`06ac6d687` 自身那一轮 CI **39797917813 cancelled**、`8a83485ef` 那一轮 **39906194379 cancelled**，均按「连推取消前一轮、以后继结论计」处理，分别由 `43cc68ee6` 的 **39904034164 success** 与 tip 的 success 承接。**本批无失败 job。**

---

## P4 4.10 多文件语义缺口 收口（轮 2：4.10.6 / 4.10.7 / 4.10.8，2026-10-24）

> **轮次性质**：三个三级任务全部来自上一轮批次复审判决引出的「未覆盖分支」（登记于 `09`，issue [#491](https://github.com/ChenXu233/YaoXiang/issues/491) / [#492](https://github.com/ChenXu233/YaoXiang/issues/492) / [#493](https://github.com/ChenXu233/YaoXiang/issues/493)）。用户 2026-10-23 裁决：4.10.6 = 编译期 W 码告警且**不得静默**（W 的语义三条待斟酌）、4.10.7 = **真正支持**原生函数作值（不加码）、4.10.8 = C 类可直接修。本轮无新增三级任务，三级计数仍 182。

> **一、交付与判决（三笔提交，均已推送）**：
>
> | 任务 | 提交 | 规模 | 判据落点 |
> | --- | --- | --- | --- |
> | 4.10.8（#493） | `5a2fa8829` | 12 文件 +474/−24 | 判据从「模块来源」改为**导出粒度**：`ModuleInfo` 纯 yx 导出名集（写入点唯一在 `register_std_modules` 合并嵌入源处）+ `ModuleRegistry::is_yx_std_export`；`alias_targets_user_function` 更名 `alias_targets_code_function`（该判据函数在 #492 提交 `19deb87d3` 中进一步统一为 `qualified_export_is_code`，与发射点 `emit_module_export_value` 并作一处） |
> | 4.10.6（#491） | `28c70a518` | 24 文件 +619/−18 | **W1007**；检出复用 4.10.4 的 Tarjan SCC（出参 `InitOrder{order, import_cycles}`，不新增第二份环检测），Linking 臂每环一条诊断、锚真实 `use` 语句，值边信息进 help |
> | 4.10.7（#492） | `19deb87d3` | 26 文件 +649/−57 | `ConstValue::NativeFn(限定名)`（常量池 tag 7）+ `RuntimeValue::NativeFunction`；判据 `ModuleRegistry::is_native_fn_export`；发射收敛到 `ir_gen::emit_module_export_value` 一处、三条入口共用 |
>
> 批次整体：**53 文件 +1716/−73**。

> **二、取证轮（t1）推翻了什么**：本轮第一件事是只读取证，它把两条假设各打散一次。
>
> 1. **#493 不是单一形态**：非泛型纯 yx 导出作值（`use std.json.{parse}`）此前**静默** `unknown(void)` / exit 0 / 零诊断；泛型纯 yx 导出（`std.list.map`）此前**运行期 E6007**、check 仍零诊断；「作返回值」是**编译期 E1012**（纯 yx 与原生同形，非本条专有）。原契约只点名 `std.list.map`，据此在开工前补全为两条终态。
> 2. **判据不能按模块粒度**：嵌入的纯 yx std 模块**也带** `ModuleSource::Std`，且 `std.result` 是 native（`is_ok`/`unwrap`）与纯 yx（`Result` 类型 + Try 方法）的**混合面**——模块粒度必然判错，判据必须落到导出粒度。
> 3. **环信息当时取不到**：`init_topological_order` 只返回置换，SCC 在函数内被丢弃；warnings 通道零改动可用；W1007 空闲；**语料真实导入环 = 0 例**（t1 取证时点的语料规模：385 个 `.yx` / 82 条边 / 0 个多成员 SCC、0 自环；本批新增 8 条夹具后 `HEAD` 为 397 个 `.yx`）⇒ 新码误报面 0。
> 4. **「环内读未初始化值」可静态点名**：`GlobalSlot.name` 是限定名、`ModuleIR.init_file_ids` 逐条指令记所属单元 ⇒ merged IR 的 init 段单趟即可比出「读侧 `Load{src: Global}` 早于该槽首条 `Store{dst: Global}` 且跨单元」。**这条事实改变了 4.10.6 的落地形态**：原本按「只报环存在」定稿，改为「红线仍是环存在，但有能力就把被违反的边与槽名并入同一条码的 help」——这正是用户说的「W 的语义还得斟酌」的答案。实测 help 文案：`环内值依赖与 use 顺序相反：zlib 读取 zother.o_val 时 zother 尚未初始化（被违反的边：zother 导入 zlib）…`。

> **三、4.10.6 的语义三条（定稿，取代 `09` 原文的「待斟酌」）**：① 告警对象 =「**存在环**」（静态事实，含环路径）；「环内读未初始化值」不单独建码，只作精确信息并入同一条码的 help。② **不新增** `#[allow]` 一类抑制机制——现有出口只有 W 默认非阻断与 `check --deny-warnings` 升级；抑制等于把要消灭的静默请回来。③ 按「**确定事实**」级登记，与 W1001–W1006 同级；**查证结果**：码表只有 `span_exempt` 一个布尔（`src/util/diagnostic/codes/mod.rs`），RFC-013 无「证据等级」表——所谓证据等级是文档约定，不是数据结构，故本条不新增字段。

> **四、基线**：`tests/baselines/corpus-parity.jsonl` **350 → 356 行**，新增恰 6 行且与三笔提交**一一对应**（2 = #493、1 = #491、3 = #492），既有 350 行**零字段变更、零删除**，无重复行——这一条由验证轮在它被掐断前完成并留下原始输出。IR 快照：既有 211 条零漂移，新增 5 条（#493 两条、#492 三条），共 216 条——口径 = `src/middle/core/tests/snapshots/` 下的 `.snap` 条数（348 是同期的**语料基线行数**，两者不可混记；此条为收口复核时的更正）。语料夹具备注：`global_init_cycle_err.yx` 头部的「状态: 待测试」与基线不符（基线早已记录它产出 E3019），本轮改为与事实一致的措辞，并刻意写成单行以免行号平移。

> **五、验证（独立验证（t5 attempt 2，冻结 `73a9d3b6f`，新树 `wbs410r2-v2`）+ 事故期队长自查）**：
>
> **5.1 独立验证（t5 attempt 2，冻结 revision `73a9d3b6f`，批次树 `wbs410r2-v2`）**——该批次树是按 AGENTS 第 9 条建的一次性工作树（工作区 `target/` 下、收口时 `gc` 回收，不入库），其结论已原样落进本实录，量测锚点是**可解析的冻结 revision `73a9d3b6f`**：
>
> | 项目 | 结果 |
> | --- | --- |
> | `cargo test --lib` | **2826 passed / 0 failed / 2 ignored** |
> | `cargo clippy --all --all-features --all-targets -- -D warnings` | **exit 0** |
> | 四门禁（`check-docs-truth` / `check-boundary` / `check-fanout` / `check-docs-refs`） | 全 **exit 0** |
> | `gen-error-code-docs.py` + `git diff --quiet` | **exit 0**（生成器幂等：157 码注册 / E 140 / W 12 / 含 std 运行时 5） |
> | `testsuite.py run full` | **exit 0**（2 波） |
>
> **5.2 事故期队长自查**（编排事故后补做；事故经过与补救见第六节）：
>
> | 项目 | 结果 |
> | --- | --- |
> | `cargo test --lib`（跳过计划内的 2 条） | **2825 passed / 0 failed / 1 ignored** |
> | 独占测试 `test_multi_file_cross_unit_pathological_recursion_within_ci_budget_reports_e3005` | 单独跑 **1 passed** |
> | `cargo clippy --all --all-features --all-targets -- -D warnings` | **exit 0** |
> | 四门禁（`check-docs-truth` / `check-boundary` / `check-fanout` / `check-docs-refs`） | 全 **exit 0**（docs-refs：1130 个跟踪文件无不可解析引用） |
> | `gen-error-code-docs.py` + `git diff --quiet` | **exit 0**（生成器幂等） |
> | rebase 到 `origin/dev`（13 笔依赖升级）后复跑 | 同上，**2825 passed / 0 failed** |
> | `testsuite.py run full` | 本机**未跑成**：预算不足 **exit 2**（拒绝执行任何波、不静默跳过），改为「batch 波手跑 + 独占测试单独跑」（见第九节第 4 条） |
>
> **5.3 两套数字的差异（唯一一处，方向向好）**：队长自查报 **2825 passed / 0 failed / 1 ignored**，独立验证（t5 attempt 2）报 **2826 passed / 0 failed / 2 ignored**——**0 failed 两处相同**，差 1 条且方向向好（多通过 1 条）。差异来源 = **环境自忽略门控**：本机 lib 套件有两条常驻 `#[ignore]`（`test_multi_file_cross_unit_pathological_recursion_at_5000_reports_e3005`，`cost: mode=exclusive`、实测 15.6 GiB / 72.6 s；`test_update_ir_snapshots_via_env_guard`，`cost: mode=manual`、须 `UPDATE_SNAPSHOTS=1` 才可跑），被忽略条数随运行环境的自忽略门控而变，两次运行的用例清单也不同（总用例数 2826 vs 2828；这 1 条的确切归属未逐条追平，**未核实**）——两次运行**没有任何一条是失败**。收口环节在主工作树（代码状态 = 本批三笔修复 `5a2fa8829` / `28c70a518` / `19deb87d3`）复跑 `cargo test --lib`，实测 **2826 passed / 0 failed / 2 ignored**，与独立验证一致 ⇒ 本实录以**独立验证值**为准（口径 = 本批三笔修复的代码状态；其后 F3 修复轮 `8206aca0f`（推送前本地哈希 `40f8c0ec5`）又新增 1 条良性环钉测试，同一命令在含该提交的工作树上报 **2827 passed / 0 failed / 2 ignored**——lib 用例总数随入库内容变化，判据以冻结点为准）。
>
> **机械可逆变异（AGENTS 第 10 条②）**——判据能否失败，靠把实现改坏。**现有两套互不依赖的证据**：队长自查的 M1 / M2，与独立验证的 T-A / T-B / T-C：
>
> | 变异 | 出处 | 打点 | 结果 |
> | --- | --- | --- | --- |
> | M1 | 队长自查 | `src/driver/helpers.rs` 把出参 `import_cycles` 置空 | 4 条环测试 **3 红**（自环/锚点/被违反边），无环那条仍绿；`apply -R` 还原后 blob 与 `HEAD` 一致、树 CLEAN |
> | M2 | 队长自查 | `src/frontend/module/registry.rs` 的 `is_native_fn_export` 恒 `false` | 5 条 native 值测试 **2 红**（别名/成员物化），另 3 条仍绿；还原后 blob 一致、树 CLEAN |
> | T-A | 独立验证（t5 attempt 2） | `src/frontend/module/registry.rs` 的 `is_yx_std_export` 恒 `false`（#493 判据回退；blob `e459c11b5b14`→`fda2a4a52ac1`） | `pureyx_export_fn_value` 与 `pureyx_generic_export_fn_value` 双双 **E6006**「Native function not found: std.json.parse / std.list.map」、exit 1——判据「作值成功 + 值断言过」被破坏 |
> | T-B | 独立验证（t5 attempt 2） | `src/driver/arms.rs` 的 `warn_import_cycles` 首行 `return`（#491 发射点回退；blob `b49794fe19a0`→`4ebee1a6c8e7`） | `check` 的 **W1007 计数 = 0**（编译期静默——正是裁决 A 要防的失败面）；`run` 仍 E6007 / exit 1（运行期行为不随告警消失） |
> | T-C | 独立验证（t5 attempt 2） | `src/middle/core/ir_gen.rs` 的 `emit_module_export_value` 里 `is_native_fn_export` 分支前置 `return false`（#492 物化臂回退；blob `e824e34d2dc5`→`bfbf6bb346a5`） | `native_fn_value` 回到 **exit 0 且 stdout 全空**（恰为修复前「静默不动作」形态）、`native_fn_member_value` **exit 1**（成员 hello 丢失） |
>
> **变异执行方式与基线口径**：t5 的三处变异走 `python scripts/dev/scratch.py trial`（内部 `git apply` → 跑命令 → `git apply -R` → 逐文件比对 blob 与 `HEAD` 一致并要求 `git status` 干净），三 trial 均「`apply -R` 成功 + 还原 blob 与 `HEAD` 一致 = 是 + `git status` CLEAN」。基线比对取**集合级**口径：`tests/baselines/corpus-parity.jsonl` 共 **356 行**，三笔提交对基线的 numstat 分别为 **2/0、1/0、3/0**（只增不减）；**pre-batch 350 行集合级零变更、零删除**，差集恰 6 行且与三笔提交归属一一对应（#493 两条 / #491 一条 / #492 三条）。注：基线按夹具路径排序，新行**按序插入**而非尾追加 ⇒「head -N 前缀比对」不成立，集合级逐字节比对才是正确口径。
>
> **口径说明**：本轮的独立验证轮已在编排事故后**补做完成**（t5 attempt 2，冻结 `73a9d3b6f`，新批次树 `wbs410r2-v2`），故第五节不再是「只有队长自查」。队长自查的 M1 / M2 与独立验证的 T-A / T-B / T-C **互不依赖**（打点文件与打点位置都不同），两套都构成「判据能失败」的硬证据；实现者各自的先红后绿（机械可逆补丁 + 还原后 sha256 比对）另有留档。

> **六、编排事故（必记）**：本轮用 AgentTeams 编排，六个角色（取证 / 三个实现 / 验证 / 复审 / 收口，验证与复审各一）。**t1–t4 全部收口并推送后，编排层在 t5（独立验证）执行中途整体消失**——`agent_teams_*` 工具组从会话工具表移除、成员列表只剩队长、任务板清空；同时一个后台验证子任务报失败。**后果**：独立验证、异家族复审、收口三个环节全部缺失，改由队长自查补齐。**未受影响**：三笔提交在消失前已完成并留本地，工作树干净。**教训**：编排层是外部依赖，收口不能把它当唯一保障——**每个任务收口时就把证据落进任务 output / 提交信息**（本轮实现者做到了，所以事故后仍能复核），而验证证据如果只存在批次树与成员上下文里，事故一到就等于没有。**事故后的补救（2026-10-24，编排层恢复后）**：独立验证轮 **t5 以 attempt 2 补做完成**（冻结 `73a9d3b6f`，新批次树 `wbs410r2-v2`，不复用事故期旧树），异家族复审轮 **t6** 补做完成（verdict = pass，附 4 条非阻塞 finding：F1 归 #492、F2 / F3 归 #491、F4 归 #493 与收口），收口轮 **t7** 即本节这一笔（复核 + 更正 + 末位文档提交）⇒ 第五节的两套数字、两套变异证据与第十节的 CI 结论均已补齐；第六节保留事故经过本身作为教训记录。

> **七、契约缺陷两例（我方，均为 `inScope` 写窄）**：① `t3` 的 `outOfScope` 仍含 `tests/baselines/`，与加进去的 `inScope` 自相矛盾 ⇒ `update_task` 的 `changedPaths` 校验按出界拒收该路径，实现者只能把它从 `changedPaths` 摘掉、在 evidence 里说明。② `t4` 的 `inScope` 未覆盖 6 个**机械必需**路径：`src/backends/common/value.rs`（值枚举的家）、`src/lib.rs` 与 `src/std/io.rs`（穷尽 `match` 补臂）、三份 `docs/src/reference/error-code/*.md`（生成器行号平移）。**三个实现任务里犯了两回**，故已提出在 AGENTS 第 6 条追加两条模板项（新枚举/指令/常量池条目 ⇒ 定义家 + 全部穷尽匹配点；改 IR 或语料行为 ⇒ 基线两项），**待用户裁决后落文**。**登记准确性订正（t6 复审 F4，low，属登记文档缺陷、**非行为缺陷**）**：4.10.8 行与 09a 轮 2 交付表所记的判据函数名 `alias_targets_code_function` 在 #492 提交 `19deb87d3` 中二次更名为 `qualified_export_is_code`（`src/middle/core/ir_gen.rs:817`，消费点 `:860`）；09 的 3.6.5 / 3.6.6 / 4.10.8 三行引用的 ir_gen 行号（`813` / `6757` / `4779` / `6838` / `6872` / `6962`）经 #493、#492 两笔后平移为 `817` / `6811` / `6815` / `4840` / `6917` / `7009` / `878` 等现状——本轮收口已按**当前字节**订正这三行与交付表（只改 zh 源文件，译文目录交 auto-translate bot）。

> **八、队长自己的三处判断错误（留证免重复）**：① 断言「`check-commit-msg.py` 白名单里没有 `driver`」——**错**，`scripts/hooks/check-commit-msg.py` 的 `VALID_SCOPES` 明确含 `driver`；这是从上一批被拒的两个 scope（`ir_gen` / `cli`）外推、未查证的结果。② 初版契约只点名 `std.list.map` 一条终态，漏掉真正静默的非泛型形态（t1 纠正）。③ `inScope` 模板两次写窄（第七节）。

> **九、遗留与登记**：
>
> 1. **本轮已落地的登记（2026-10-24，三个 issue + 一个 WBS 行）**：① **W 码抑制机制缺失**——issue [#499](https://github.com/ChenXu233/YaoXiang/issues/499)（用户裁决：先做 CLI 级 `--allow=<CODE>`；issue 里同时列出「源级属性」等语法的设计空间供后续考虑；属 **RFC 级**，不进 WBS 排期）；② **运行期 E6007 文案误导**——issue [#500](https://github.com/ChenXu233/YaoXiang/issues/500) + WBS **3.11.3**（归属 P3、不阻塞主线；不改 W1007 告警面）；③ **E1012 返回位置括号化函数类型丢一层**——issue [#501](https://github.com/ChenXu233/YaoXiang/issues/501)；2026-10-24 取证后已归位：**新开二级 3.22「函数类型注解的分解语义」+ 三级 3.22.1**（二级 68 → 69、三级 184 → 185）。取证结论 = **parser 无责**（注解解析正确）、**typecheck 两条路径期望互相矛盾**：`= inc` 时期望 `int64`（注解被按柯里化读成「一个值参数、返回 Int」），`= () => { return inc }` 时期望 `fn(int64)->int64` 而 found 是 lambda 整体类型；结果是「声明返回函数类型的函数」没有任何能过的注解写法。④ **AGENTS 第 6 条追加 ⑤⑥**（新枚举/指令 ⇒ 定义家 + 全部穷尽 `match` 点；改 IR 或语料行为 ⇒ `tests/baselines/` 与 `src/middle/core/tests/snapshots/` 必进 `inScope`），已写入本仓根 `AGENTS.md`。
> 2. **本轮未做但已定性的两项**：`E1012` 与 `E6007` 都是**先登记不改**——前者归属待查、后者改文案会改变失败程序的输出（需按 3.11.3 的判据做，不夹带进 4.10 轮 2）。
> 3. **环的「正确性」问题（用户 2026-10-23 追问，本轮只取证不改）**：线性序在环上**必然**违反至少一条边（拓扑序存在的充要条件是无环），4.10.4 给的是**确定性**不是正确性。三条路的成本事实已实测：**禁环** = 小（但要先区分值边 / 类型边 / 函数边，否则误伤良性环）；**SCC 内惰性求值** = 跨层大改（IR + 字节码 + opcode + 降级器 + 解释器 + 新错误码 + 新运行期状态），是唯一能「构造上正确」且环外行为不变的方案；**显式初始化序** = 语法层零新关键字（`use` 的书写顺序信息已在手但未被使用）、语义层中—大。三者都属 D 类，须单独裁决。
> 4. **未覆盖面**：事故期报废的那轮验证原本还要做的「变异轮全量」「LSP / check 双形态分别探针」「多包 workspace 形态」仍**未完成**（补做的 t5 attempt 2 覆盖了三条修复的变异与门禁，未覆盖这三项）；本轮变异共 5 个方向：M1 / M2（队长自查）+ T-A / T-B / T-C（独立验证）。
> 5. **环境事实**：本机 `testsuite.py run full` 在 `full` 通道下因**预算不足**（独占测试需 8.11 GiB、当时预算 7.17 GiB）**拒绝执行任何波**（exit 2，不静默跳过）——满足设计意图；改为「batch 波手跑 + 独占测试单独跑」，两条都给结果。
> 6. **复审 findings 的处置状态（t6，4 条**非阻塞**，均不影响本批勾选）**：
>
>    | finding | 归属 | 级别 | 处置状态 |
>    | --- | --- | --- | --- |
>    | F1 | #492（`ir_gen.rs:7009` 命名空间常量臂仍直发零参 `Call`、绕过 `emit_module_export_value` 三路判据 ⇒「发射收敛到一处」的表述对该臂不成立；实测该臂对原生函数**不可达**，无用户可见缺陷） | low | **已在案**：`09` 的 3.6.6 行 / issue [#494](https://github.com/ChenXu233/YaoXiang/issues/494) 已登记该残余点，下一轮 3.6.6 收敛时改走新判据（或写明它只服务 native 常量） |
>    | F2 | #491（W1007 的告警面 =「存在环」，宽于裁决 A 点名的触发面 ⇒ **原先成功的良性环**（函数互导、无值边）也新增可见告警；W 默认非阻断、退出码不变，但 `check --deny-warnings` 无抑制口） | medium | **部分已库 + D 类面待用户裁决**：告警面定稿已由第三节登记（不判实现违规），但「改变原先成功程序的可观察行为」须与 E6007 文案 / E1012 一并提交用户裁决 |
>    | F3 | #491（W1007 模板对良性环附带「读取尚未初始化的顶层值会在运行期报 E6007」从句——**该从句为修复前的模板文案**，修复后改为「环内先后只按路径字典序作确定性约定（非语义保证）」；而无值边的环该后果本就不发生——实测良性环 run exit 0、零 E6007） | low | **已入库**（提交 `8206aca0f`——推送前的本地哈希 `40f8c0ec5`，`pull --rebase` 后重写；模板改写落 `locales` 六份 + 生成器重跑 `docs/src/reference/error-code/index.md` 与 `docs/src/reference/warning-code/warning-codes.md`，两条生成物随代码同提交入库（AGENTS 第 10 条⑤，禁手改生成物）；另带 1 条良性环钉测试 `src/driver/tests/import_cycle.rs` 与语料基线 1 行 numstat 1 增 1 删） |
>    | F4 | #493 + 收口（登记文档的符号名与行号过期：`alias_targets_code_function` 二次更名、`ir_gen.rs` 六个行号平移） | low | **本轮收口已订正**（见第七节末的「登记准确性订正」） |
>    | F3 计数注记 | 同上（F3 修复新增 1 条 lib 测试） | — | 本表 **5.1 的 2826** 是冻结 revision `73a9d3b6f` 上的实测值；F3 修复（`8206aca0f`）新增 1 条 lib 测试，故其后 tip 的 lib 计数为 **2827 passed / 0 failed / 2 ignored**（两处 0 failed 一致） |
>
>    F1 / F2 / F3 的定位与实测证据见复审轮 t6 的任务记录；**本节只登记处置状态，不重写其证据**。

> **十、CI 门禁结论（AGENTS 第 13 条①，定时核对 2026-10-24 回填）**：核对对象 = 收口文档提交所在 tip `73a9d3b6f` —— CI **40074735616 success**、Deploy Docs **40074735655 success**、Auto Translate Docs **40074735608 success**。该 tip 覆盖本批三笔修复（`5a2fa8829` / `28c70a518` / `19deb87d3`）、翻译 `7e599c62b` 与文档收口 `73a9d3b6f`。覆盖关系（第 10 条③）：`19deb87d3` 自身那一轮 CI **40074348818 cancelled**（连推被后继 run 取代），按「以后继结论计」由 tip 的 success 承接；同轮 Deploy Docs **40074348888 success**、两条翻译 job 亦 success。**本批无失败 job。** 本节回填提交（推送后 `56354d0c1`）自身的 run 亦已核对：**CI 40076493209 success**、**Deploy Docs 40076493240 success**、**Auto Translate Docs 40076493199 success**。
>
> **补记（2026-10-24，定时核对第二轮）**：后续两轮 `229dfae42`（哈希订正）与 `f6da37c6f`（登记 #499 / #500 / #501 + AGENTS ⑤⑥）的 CI **均失败**，失败 job = **Test → Corpus parity vs baseline**（run `40080094899` / `40080885899`），差分 1 处、正是本批夹具 `import-cycle-reads-before-init/main.yx` 的 W1007 行。**根因不是代码回归**：auto-translate 的 i18n 提交 `9ad0636be` 改写了 `locales/en.json` 的 W1007 `template`（`…initialization order inside the cycle is undecidable…` → `…initialization order within the cycle cannot be determined…`），而语料基线**逐字**记录 run 的 stderr ⇒ 译文一变，基线必红。**处置**：按当前文案再生基线（提交 `076833cc4`：numstat **1 增 1 删**、356 行不变、复跑 `check-corpus-parity` 差分空 exit 0），tip `076833cc4` 的 CI **40082151891 success**。另注：`f6da37c6f` 的 **Auto Translate Docs** job 亦 failure（翻译 bot 自身作业，非代码门禁）。**暴露的结构缺陷（待裁）**：语料基线把**本地化文案**也钉死了，译文一改就是一次「看起来像回归」的红；候选修法 = 基线只记 `code | file | line`（文案交给 locales 单测）或固定探针 locale。
>
> **末笔结论（2026-10-24，定时核对第三轮）**：`636d1dbe9`（本节补记的文档提交）→ CI **40083597051 success**、Deploy Docs **40083597135 success**、Auto Translate Docs **40083597070 success**。**本批至此全部门禁绿，仅 `229dfae42` / `f6da37c6f` 两轮因上述基线-译文耦合而失败（已由 `076833cc4` 修复并记录覆盖关系），`f6da37c6f` 的翻译 bot 作业失败亦已重跑为 success。**
---

## P0 0.5.3 语料基线只记结构化事实（2026-10-24）

> **触发**：4.10 轮 2 的两轮 CI 红（run `40080094899` / `40080885899`）——失败 job 都是 Corpus parity vs baseline，差分仅 1 处，根因是 auto-translate 的 i18n 提交 `9ad0636be` 改写了 `locales/en.json` 的 W1007 `template`。语料基线**逐字**保存渲染后的诊断文案，于是译文一变基线必红，且红的样子像代码回归。用户 2026-10-24 裁决：保留基线，字段压成结构化事实（C1）。

> **改法**（提交 `69a22c4d1`）：`examples/corpus_probe.rs` 的 `capture_run` 把 `stderr` 从「逐字原文」改为 `diagnostics: [[码, 文件, 行], …]` + 仅保留**崩溃标记行**（`panicked at` / `RUST_BACKTRACE` / `internal compiler error` / `INTERNAL ERROR` / `stack backtrace:`）；新增 `strip_ansi`（run 侧 stderr 带 ANSI 颜色码，不剥则诊断头匹配不中）与 `parse_diagnostics` 的位置前瞻（诊断头与 `-->` 之间夹着源码片段时仍能取到位置——multifile 的 E6007 就是这种形态）。`exit` / `stdout` / `check.*` / `expectation` 一律不变。

> **证据（本地实测，2026-10-24）**：
>
> | 判据 | 结果 |
> | --- | --- |
> | 基线重生后 `check-corpus-parity` | 356 / 356，差分空，exit 0 |
> | **改 `locales/en.json` 的 W1007 文案后重跑探针** | **差分仍为空**（本项的存在理由：基线不再随译文漂移） |
> | 变异核对（把某条 `run.exit` 改 99） | 报 1 处差分（比对路径未失效） |
> | 基线规模 | 356 行不变；230 条因字段形状变化而变（机械 diff） |
>
> **代价（已接受）**：语料层不再承担「运行期文案措辞」的回归；该职责回到 `locales` 单测、`yx explain` 展示快照与错误码生成物对账。**新增码时若只改文案，语料基线不再变动**——这是设计目标，不是覆盖退化。

> **登记**：issue [#502](https://github.com/ChenXu233/YaoXiang/issues/502)；WBS `0.5.3`（P0，不阻塞主线）；三级计数 183 → 184。

---

## P4 4.9 引用池正确实现（RFC-029f 修正）— 轮 4 收口（4.9.8，issue #465）

> **一、批次构成与轮次**：WBS 二级任务 `4.9`（引用池正确实现，RFC-029f 修正）下的第 8 个三级任务 `4.9.8`，issue [#465](https://github.com/ChenXu233/YaoXiang/issues/465)——跨单元同名类型经**内联别名** / 模块限定引入时，方法派发落到**裸名归属单元**（静默错值，D59 **C 类**）。轮次链 = 实现 → 独立验证 → 异家族复审（**needs_revision**，5 条 findings）→ 修复轮（R1–R4）→ 复审轮 2（**pass**，无新增可达同族漏修）。本轮收口后 WBS `4.9` 的八个三级任务全部完成，二级看板**整体勾选**。

> **二、最终口径（取代修复前的表述）**：
>
> - **判别实验定死机制 = D20 决议覆盖 + 裸名前缀**（不是 `type_bindings` 短名键碰撞）：入口单元方法调用点的 `type_bindings` 在本形态为空，两处调用都命中 `overload_resolutions` 且值为**定义侧裸名** `Temp.tag`，随后由 `qualify_names` 的别名前缀把裸名前缀解析到**裸名提供者**——与 issue 的 8 组对照一致（赢家 = 裸名属主，与 import 顺序、a 侧有无调用点无关）。
> - **归属判定唯一在 `imported_type_owner`**（`src/middle/core/ir_gen.rs:1232`）：三条判据按序 = ① 权威限定名**精确命中即返回、不再解析**（注册表模块键是权威身份）② 本地名 → `use_aliases` ③ 命名空间前缀 → `Resolver::resolve_namespace`；**调用点只有一处** = `method_dispatch_name` 内的两处（接收者两种形态）。
> - **`method_dispatch_name`（`:1265`）的生产调用点 5 处**：绑定臂（`:8318`）/ 决议臂（`:8379`）/ 裸名臂（`:8532`）/ **Index 臂（`:6444`）** / **二元接口臂（`:7858`）**——`:7858` 是**全部「接口驱动二元运算」的共享臂**（Equal + Add/Subtract/Multiply/Divide/Modulo）：另建「同名 `Temp` 各实现 `Add`」的夹具实测 `u + u` 由修复前 `3.0` 转 `20.0`，即算术族此前同样被本修复关闭。
> - **写出名来源** = 注解原文 / 构造器名（局部与顶层绑定同链记录）；`constructor_written_type_name`（`:1188`）已改为**纯语法提取**（只记录、不判定）。
> - **029f 侧表述已按实测更正**：`:7858` 在 029f 的对应表格里不再只标注「Equal 臂」，而是写明它是**全部「接口驱动二元运算」的共享臂**（Equal + Add/Subtract/Multiply/Divide/Modulo）。
> - **钉测试**：驱动级 **10 条**（`src/driver/tests/method_dispatch.rs`，结构层读合并 IR 的调用名 + 行为层读解释器全局槽位值）+ 集成 **5 条**（`tests/integration/multifile.rs`），**全为值级断言**；issue 的 8 组对照现已全部入库可复跑。
> - **基线**：语料 356 行**零 diff**、IR 快照 216 文件**零漂移**（只改调用名归属，不改 IR 形状）；错误码生成物（`E1xxx` / `E8xxx`）为纯行号平移，由生成器再生、**未手改**。

> **三、门禁（收口 revision 的冻结树实测）**：`cargo test --lib` **2837 passed / 0 failed / 2 ignored**；语料基线 356 行零 diff；IR 快照 216 文件零漂移；`cargo fmt --check` 与 `clippy -D warnings` 全 0；`check-docs-truth` / `check-docs-refs` / `check-boundary` / `check-fanout` 全 0；错误码生成器重跑后工作树干净。**如实声明**：`python scripts/dev/testsuite.py run fast` 在批次验证树内**结构性空跑 0 波**——其源文件扫描器跳过路径含 `target/` 的目录段，而批次树按规程就建在仓库内的 `target/` 产物目录下；本轮的实质覆盖由 `cargo test --lib` 承担，**不得**把该次 `run fast` 写成「全绿波次通过」。

> **四、修复轮 R1–R4（异家族复审 findings 的处置）**：
>
> - **R1（high）算子接口两臂只修了半边**：Index / Equal 此前用 typecheck 过来的**短类型名**拼调用名、不经归因 ⇒ 两单元同名类型的算子实现互相落错单元（实测 `u[0]` 得 `3.0`、`u == t` 得 `true`，应 `20.0` / `false`）。修复后两臂同经 `method_dispatch_name`；**本地类型两形态逐字节不变**（钉 `test_local_type_operator_arms_keep_own_dispatch` + 语料 `operator_index.yx` / `struct_equality.yx` 全绿）。
> - **R2（medium）判据非单一源 + 已限定名被二次解析**：上一版 `constructor_written_type_name` 产出**权威限定名**，`imported_type_owner` 又把它过一遍 `resolve_namespace`，首段被命名空间别名表改写 ⇒ 类型导出判定失败、回落裸名（实测 `y = 3.0`）。现在构造器只做纯语法提取、归属判定收敛到一处且**权威限定名精确命中即返回、不再解析**；钉 `test_multifile_qualified_module_key_not_rewritten_by_namespace_alias`（模块键 `p.q` + 未使用的 `use other as p`）。
> - **R3（medium）文档表述与实测不符**：修复前记录的「`check` 0 error / 0 warning」与实测不符——issue 给出的清单形态（`[package]` 只有 `name`、缺 `version`）下 `check` 实为 **2 条 `W1002`**（`a.yx:1:1` / `b.yx:1:1`），补上 `version` 才是 0 warning。已按实情改写，并**如实写明根因在清单解析路径**（不在死代码判据本身）。
> - **R4（low）8 组对照只钉了 3 组**：补钉 5 组值级用例——`use` 顺序对调 / 两单元方法体对调（值跟单元走）/ **裸名属主改为字典序靠后的 `z.yx`**（证明赢家来自 `use` 面而非排序）/ 只调别名侧 / 同文件两类型阴性对照 ⇒ 8 组对照全部入库。
> - **R5（low）契约面矛盾**：修复轮的契约把错误码生成物划到 `outOfScope`，而实际改动包含生成物（纯行号平移）——属**契约缺陷**而非实现缺陷，登记备查。

> **五、变异核验（机械可逆）**：关闭 `method_dispatch_name` 的归因 ⇒ 7 条跨单元用例翻红（含算子两臂与 4 组对照）、3 条本地 / 阴性对照仍绿；恢复「旧构造器产限定名 + 去掉精确命中」⇒ R2 的钉用例翻红。两次补丁 `git apply -R` 后与被变异文件**逐字节一致**，`git status` 干净。

> **六、残余边界（登记，不在本轮修）**：① 写出名不可得时（函数返回值 / 字段 / 容器元素流入的接收者）回落裸名；② 内联别名 × 同名重载沿用 4.9.7 的候选收窄规则（`#k` 的跨单元精确性不变）；③ 跨单元值绑定转手（`use M.{v}`）无本地写出名，同样回落裸名；④ **清单解析失败 / 缺必填字段时静默退化**——已登记为 `3.23.1` / issue [#503](https://github.com/ChenXu233/YaoXiang/issues/503)；⑤ 算子接口方法仍被报 `W1005`（死代码分析未把算子派发计为调用）——已登记为 `3.20.3` / issue [#505](https://github.com/ChenXu233/YaoXiang/issues/505)；⑥ `Try`（`?`）四方法派发面当前被 typecheck 的 `E1082` 挡住（硬错非静默），上游一旦放开「导入的用户 `Try` 类型」即成静默错派；⑦ `to_string` 零开销面当前不可达。**⑥⑦ 已合并登记为 `3.18.3`** / issue [#506](https://github.com/ChenXu233/YaoXiang/issues/506)（终态要求：放开任一前置之前，两处必须同经 `method_dispatch_name` 或给出书面理由，并补不可达性钉测试）。

> **七、批次验证树清理（收口 gc）**：收口时按规程 `python scripts/dev/scratch.py gc` 回收本批三棵批次验证树（另有两名成员遗留的持有锁，持有者任务已结束，按 `release --force` 先解锁再回收），共释放约 15.9 GB；回收后 `status` = 「没有批次树」，`git worktree list` 只剩主挂载，仓库外无本批新增目录。
>
> **八、CI 门禁结论（AGENTS 第 13 条①，定时核对 2026-10-25）**：核对对象 = 本批 tip `d0df6d5a2`（覆盖 `247d8b503` 及其余 8 笔）——CI **run 40178906143 success**、Deploy Docs **run 40178906130 success**；**Auto Translate Docs run 40178906097 failure** 属 bot 侧 rebase 冲突（其翻译提交 rebase 到并发落地的 `e6cb85011` 时撞上 `docs/src/en/reference/error-code/E1xxx.md` / `E8xxx.md` 的 content 冲突），而译文已由 `e6cb85011` 成功落地（含 `029f` 与两份错误码文档），失败的是重复触发的一次，**非代码门禁**。**覆盖关系**（第 10 条③）：`247d8b503` 自身那轮 CI 为 cancelled（连推被后继 run 取代），按「后继结论计」由本 tip 的 success 承接。

---

## P3 本轮新增登记（2026-10-25 用户批准）：3.23.1 / 3.18.2 / 3.20.3 / 3.18.3

> **一、3.23.1 清单解析失败 / 缺必填字段的静默退化**（WBS 二级 `3.23`「清单解析失败 / 缺必填字段的静默退化」下的唯一三级任务，issue [#503](https://github.com/ChenXu233/YaoXiang/issues/503)）：`yaoxiang.toml` 解析失败（非法 TOML）或缺必填字段（如 `[package] version`）时，CLI **不报清单错误**、静默退化为单文件语义 ⇒ 跨文件引用池失效 ⇒ 两个都被入口引用的类型被误报 `W1002`×2，且 `check` 仍退出 0；pre-fix 与 4.9.8 修复后逐条相同 ⇒ **先于 #465、与本修复无关**。根因在清单解析路径（`src/driver/helpers.rs:727` 的清单反序列化以 `.ok()` 吞错；`src/package/manifest.rs:28` 的 `version` 为必填字段），不在死代码判据本身。D59 **D 类**，**语义待裁**（甲 报错并退出 / 乙 继续但发 W 码 / 丙 维持现状）——终态判据待裁决后定稿。证据细节留在 issue 里。
>
> **二、3.18.2 导入类型的注解身份**（WBS 二级 `3.18` 下的第二个三级任务，issue [#504](https://github.com/ChenXu233/YaoXiang/issues/504)）：`make_b: () -> TempB` + `u: TempB = make_b()` ⇒ `E1012` + `E1002`、退出码 1，双二进制（修复前 / 4.9.8 修复后）逐条一致；归因同 `3.18` 的 MonoType 裸名义名族；分类待队内定（疑 D 类）。证据细节留在 issue 里。
>
> **三、3.20.3 算子接口方法的 W1005 误报**（WBS 二级 `3.20` 下的第三个三级任务，issue [#505](https://github.com/ChenXu233/YaoXiang/issues/505)）：算子接口方法被 `u[0]` / `u == t` 实际调用，`check` 仍报 `W1005 Unused method`——合法清单 + 类型体内 `Index(Temp, Int, Float)` / `Equal(Temp, Temp)` 声明下，`check` 为 0 error + 4 条 W1005，而 `run` 值正确（20.0 / false）⇒ 死代码可达性**不计算子派发**；本地单文件夹具 2 条、跨单元 4 条；与 #465 无关（修复前后同样存在）。改的是**成功程序**的警告面 ⇒ D 类，登记待排期；证据细节留在 issue。
>
> **四、3.18.3 三条未收敛的短名派发臂**（WBS 二级 `3.18` 下的第三个三级任务，issue [#506](https://github.com/ChenXu233/YaoXiang/issues/506)）：`src/middle/core/ir_gen.rs:8769` 的零开销 `to_string` 派发与 `:6828-6870` 的 `Try` 四方法派发仍按**短类型名**拼调用名；当前分别**不可达**（用户结构体的 `to_string` 实测仍走兜底 repr）与被 typecheck `E1082` 硬错挡住 ⇒ 无用户可见缺陷，但**放开任一前置即成静默错派**；终态要求「放开之前两处必须同经 `method_dispatch_name` 或给出书面理由」+ 不可达性钉测试。
>
> **五、计数同步**：`09` 定位表「二级」69 → **70**（追加 `3.23` 的来源说明）、「三级」185 → **189**（`3.23.1` / `3.18.2` / `3.20.3` / `3.18.3`）。

## P3 3.21 收口（轮 1：3.21.1 一元取负 i64::MIN 溢出，2026-10-25）

> **触发**：issue [#468](https://github.com/ChenXu233/YaoXiang/issues/468)（2026-10-18 发版前审计登记，WBS 3.21.1）：`-9223372036854775808` 字面量或对持有 `i64::MIN` 的变量取负，解释器直接 panic（exit 101）而非诊断；二元溢出已由 #281 走 checked 报 E6007，一元漏在通道外。D59 **C 类**（崩溃）——顺路修通道，无需语义裁决。

> **改法**（提交 `cfb3442d`，2026-10-25 推送）：`src/backends/interpreter/executor/ops/arith.rs` 的 `UnaryOp::Neg` / `RuntimeValue::Int` 臂由裸 `-n` 改 `checked_neg`，溢出经 `ExecutorError::runtime("integer overflow in \`-{n}\`", capture_stack())` 报 **E6007**——与 #281 二元样板（`executor.rs` 的 `int_op`）同码同构造、带栈回溯。`tests/integration/execution.rs` 新增五形态测试：i64::MIN 字面量 / 变量两形态断言「E6007 + 栈帧 + 无 panicked」（先红后绿），MAX 取负、普通值、MAX+1 二元三对照锁定既有行为。**编译期同类路径不改动**：`eval/evaluator.rs:471` 与 `eval/const_eval.rs:423` 的取负操作数是 `ConstValue::Int(i128)`，字面量路径值域内取负在 i128 域永不溢出；经 saturating 算术链触达 `i128::MIN` 的理论形态另立 [#507](https://github.com/ChenXu233/YaoXiang/issues/507) 备案（未实测，先实证可达性再修）。

> **证据（三段判据链，2026-10-25）**：
>
> | 环节 | 判据 | 结果 |
> | --- | --- | --- |
> | 实现轮门禁 | `cargo fmt --check` / 全量 `clippy -D warnings` / `testsuite run fast` / `markers --check` / `check-docs-truth` / `check-boundary` / `check-fanout` / `check-docs-refs` | 8 项全 exit 0（脚本以命令退出码裁决，一轮通过） |
> | 独立验证 | 批次树冻结 `cfb3442d` 五形态复测（E6007 + 栈 + 非零退出 + 无 panicked / 三对照不变） | PASS（重建二进制上复跑确认） |
> | 独立验证 | 变异可逆证明：`checked_neg` 打回 `-n`（`scratch.py trial` 机械 apply→跑→还原） | 新测试 2 FAILED + panic 形态 exit 101（判据能失败）；还原后 blob 与 HEAD 逐字节一致、`git status` 干净 |
> | 独立验证 | 语料差分 / IR 快照 / 错误码生成器再生 | 356/356 零差异、快照测试绿、生成器再生 `git diff --quiet` = 0 |
> | 复审 | 冻结 revision 只读复审（正确性对照 #281 样板 / issue 验收三条对账 / 测试合规 / 红线 / inScope / D59 分类） | APPROVE，2 条 low 备案（见下） |
> | pre-commit | 全钩子链 | `cargo audit` 因网络故障跳过（内部 git 库与系统 git 网络栈不同），按既定模式离线补扫：1352 条 advisory 扫 268 依赖，仅 1 条已 allow 的 yanked 警告 |

> **过程注记**：① 验证轮发现 `scratch.py trial` 的变异跑测会把批次树 dev 档**二进制**重编为变异版（源码机械还原不含构建产物）——本次已重建后在正确二进制上复跑全部量测；**建议固化为批次约定：trial 之后必须 `scratch.py build` 再做后续量测**（是否写入 AGENTS 待用户裁决）。② 复审 low 备案之一：溢出文案在 `n == i64::MIN` 时渲染为 `--9223372036854775808`（双负号）——与二元样板「操作数原样回显」惯例一致，纯观感不阻塞，留待文案统一时一并处理。③ 编排注记：实现 / 验证 / 复审三轮均为独立子代理实例（同模型家族 GLM-5.3-Flash——AGENTS 第 7 条「不同模型家族」在当前环境不可满足，以冻结 revision 量测 + 从零读码替代独立性，如实登记）。

> **登记**：issue [#468](https://github.com/ChenXu233/YaoXiang/issues/468) 已关闭；遗留 [#507](https://github.com/ChenXu233/YaoXiang/issues/507)（编译期 i128 取负理论形态，P3）；WBS `3.21` 二级下余 3.21.2（#469，D 类待裁）/ 3.21.3（#471）/ 3.21.4（#470）。

## P3 3.21 收口（轮 2：3.21.3 mono 配置贯通，2026-10-25）

> **触发**：issue [#471](https://github.com/ChenXu233/YaoXiang/issues/471)（2026-10-18 发版前审计登记，WBS 3.21.3）：`mono.max_depth` 无法从 `yaoxiang.toml` 配置——Run 臂只读 `runtime.*`，`[mono]` 节被 serde 静默忽略，3.10.1 的抬高闸门档在 `run` 上不可复现。D59 **C 类**（静默不动作）。

> **改法**（提交 `a8ad0530`，2026-10-25 推送，19 文件 +334/−53）：`util::config::ProjectConfig` 加 `#[serde(default)] mono: frontend::config::MonoConfig`（schema 唯一家不复制默认值，util 引 frontend 已有先例不构成新反向依赖）；`compile_project` / `check_project` / `analyze_source_in_project` / `compile_embedded_module` 四编排入口扩签名收 `CompileConfig`（未新增第二入口），全部调用方显式迁移——生产链路仅 run 链注入 toml 配置，其余传 `default` 保持行为不变；check 命令链与 LSP 的 mono 贯通明确不在本轮（接线点留注释）。新增 `tests/integration/mono_config.rs`（150 档闸门平移先红后绿 / 无节默认 100 守卫 / 显式 100 与省略节逐字节一致）；5000 档由手动 CLI 探针实测（`(5000，当前 5001)`），不进默认套件（72.6s / 15.6GB，与 `mono_cross_unit` 的 ignore 档同一取舍）。

> **证据（三段判据链，2026-10-25）**：
>
> | 环节 | 判据 | 结果 |
> | --- | --- | --- |
> | 实现轮 | 红态复现缺陷本体：toml 写 `max_depth = 150` 被静默忽略、报 `(100，当前 101)`；修后同测试绿（3 passed） | 先红后绿 |
> | 实现轮门禁 | fmt / 全量 clippy / `testsuite run fast` / markers / docs-truth / boundary / fanout / docs-refs | 8 项全 exit 0（一轮过） |
> | 独立验证 | 批次树三形态（150 档平移 / 无节默认 / 显式 100 逐字节一致 md5 相同）+ 选做 5000 档 | PASS（非零退出无 panic，重建二进制上复跑） |
> | 独立验证 | 变异可逆：Run 臂 mono 透传打回断供（`scratch.py trial`） | 新测试红（闸门回 100）、机械还原 blob 一致；trial 后按第 9 条新规重建再量测 |
> | 独立验证 | 双 revision 默认档对照（`a8ad0530^` vs `a8ad0530`）无节探针 stderr/stdout | 逐字节一致 |
> | 独立验证 | 语料 / IR 快照 / 错误码生成器 | 零差异 / 绿 / 重跑零新增 diff |
> | 复审 | 九项只读审查（通路 file:line 闭合、调用点迁移无漏网、D0、禁令三、生成物平移、验收对账、测试合规、D59 分类） | APPROVE |
> | pre-commit | 全钩子链 | scope 首选 `cli` 不在合法表被 check-commit-msg 挡下，按 main.rs 惯例改 `util` 后全过；`cargo audit` 网络故障按既定模式 SKIP（本批零依赖变更） |

> **过程注记**：①**契约缺陷追认**（AGENTS 第 6 条同型复发）：契约 inScope 未圈进 `src/lib.rs`、`src/lsp/analysis.rs` 与四个直驱测试文件（orchestrator 三个 + bytecode_load 等），均为签名扩链 E0061 的机械必需面——「扩签名」类任务的 inScope 应先 grep 全部调用点再写。②复审 low 备案归位 [#503](https://github.com/ChenXu233/YaoXiang/issues/503)：`main.rs` 的 `toml::from_str::<ProjectConfig>(...).unwrap_or_default()` 在清单解析失败时静默回退全默认（含 `runtime.*`），与 #503 登记的「解析失败静默退化」同族，既有债非本提交引入，不另立 issue。

> **登记**：issue [#471](https://github.com/ChenXu233/YaoXiang/issues/471) 已关闭；WBS `3.21` 下余 3.21.2（#469，D 类待用户裁决——裁决四件套已呈）与 3.21.4（#470）。

## P3 3.21 收口（轮 3：3.21.2 诊断外显保真三处，2026-10-25）

> **触发**：issue [#469](https://github.com/ChenXu233/YaoXiang/issues/469)（2026-10-18 发版前审计登记，WBS 3.21.2）。D59 **D 类**（对外文本面）——用户 2026-10-25 裁决四件套后批准**三处全修各取甲案**（help 入 locales / E4018 两小点一起修 / 绑定失败错误类型登记）。

> **改法**（提交 `df89d6ee`，rebase 前本地 `c2701f3c`，30 文件 +616/−172）：①**help 全量入 locales**——新增 per-site 参数化 help 键机制：builder 加 `with_help_key`（键驱动，与成品串 `with_help` 区分）与 `param_from_key`（段标题类用户可见文字随 message 同语言渲染），复用 `I18nRegistry` 同一管线与回退链（请求语言→en→zh），`render_str` 收 `&str` 避免运行时键查询的 `Box::leak` 累积；全仓 grep `with_help` 实迁 **6 处**（issue 点名 E1010 缺必需形参 / E1110 两处，补查 E5001 vendor / E1002 借用与二元两形态 / W1007 值边四处），zh 用原硬编文案、en 真翻译、ja/ru/zh-classical/zh-x-miao 英文占位（回退链生效，真翻译待补）。②**E4018**——六语言 template 改 `{constraint}{counterexample}`（反例段标题移出主模板入 `E4018.counterexample.section` 键），空 assignments 段整体缺席、有值经键渲染；`ConstExpr::UnOp` 的 Neg/Int 字面量按值打印（`-3`），其余保持结构直印（未扩散）。③**级联抑制**——`check_var_stmt` 包装在 Err 路径把被绑名以既有底部类型 `MonoType::Never` 登记（守卫：scope 无此名才登记防覆盖重赋值、排除方法绑定），配三处 Never 短路（`infer_binary` 结果取另一侧 / `FieldAccess` 短路 / `classify_mono` 记 ValueCopy 防 E2014 顶替级联）——CLI 实测 E1014 形态 4 errors 收敛 1。

> **证据（三段判据链，2026-10-25）**：
>
> | 环节 | 判据 | 结果 |
> | --- | --- | --- |
> | 实现轮 | 三处各红测试（help 无硬编中文 / 无空反例段 + `-3` 渲染 / 级联总数 1） | 三红转三绿（红态输出留档实现报告） |
> | 实现轮门禁 | fmt / 全量 clippy / `testsuite run fast` / markers / docs-truth / boundary / fanout / docs-refs | 8 项全 exit 0（一轮过） |
> | 独立验证 | 三处验收复测（en 无中文字符计数 0、zh 中文含缺失形参名、双语插值正确；空段缺席 / `-3` 按值 / 有值段保留；c4g 总数 1、真未定义引用仍报） | PASS |
> | 独立验证 | 变异可逆（删毒化登记块） | 级联测试红（`[E1014,E2014]` left 2 ≠ right 1）、还原 blob 逐字节一致、trial 后按第 9 条新规重建 |
> | 独立验证 | 语料 / 快照 / 生成器 | 356 行零字节差异（再生后基线复测零漂）/ 7 绿 / 重跑 diff 静默 |
> | 复审 | 九项（机制无第二套、毒化守卫、六文件键集机械比对、基线双向再生无夹带、既有测试期望为强负断言同步、D 类批准范围对照） | APPROVE（批准范围外仅 zh.json 缩进修正与 E4018 probe 记录连带两处无害项） |
> | 基线 | `corpus-parity.jsonl` 同批再生，9 条目漂移逐条人工 review | 全部正向（7 级联 E1001 消失主错误保留 + 2 行号归因修正），主预期码零丢失 |

> **过程注记**：①毒化静默边界代价（复审实测确认）：毒化变量参与**与根因无关**的类型错误运算也被静默（`b + "str"` 无诊断）——甲案（毒化 + vacuous 短路）已批设计的内在代价，记录于 [#509](https://github.com/ChenXu233/YaoXiang/issues/509) 供后续权衡。②Never 短路判据在两个既有遍历器各一份拷贝（注释互指，两遍历器并行是既有架构）——t8 F1 同型漂移风险，连同 message 位中文残留 4 处、help 键回退链护栏缺口、四语言占位待译均归 [#509](https://github.com/ChenXu233/YaoXiang/issues/509)。③复审过程留档：其前两次 trial 因在批次树 cwd 内嵌套调用 `scratch.py build`（相对路径找不到批次）产生假阳性「判据抓到了」，改 cargo 直构建重做并以第三次结果为准——**批次树内跑命令用 cargo 直调或回主仓调 scratch.py** 的操作教训一并记入。④复审收尾 `scratch.py gc` 顺带回收三棵已 release 的存量批次树（3.21.1/3.21.2/3.21.3，约 19.7 GB），status 终态干净。

> **登记**：issue [#469](https://github.com/ChenXu233/YaoXiang/issues/469) 已关闭；遗留 [#509](https://github.com/ChenXu233/YaoXiang/issues/509)；WBS `3.21` 下仅余 3.21.4（#470，生成物散文与 W1090 事实矛盾）。

## P3 3.21 收口（轮 4：3.21.4 生成器散文修正 + 整组归档，2026-10-25）

> **触发**：issue [#470](https://github.com/ChenXu233/YaoXiang/issues/470)（2026-10-18 登记，WBS 3.21.4）：生成的警告码页断言「W 族不会阻止编译」与同页 W1090 条目自述的 error 级矛盾——生成器写死散文未随 W1090 同码升 E（2026-10-13 裁决，WBS 4.3.2）同步。

> **改法**（提交 `3b6df553`，3 文件 +10/−6，零 Rust 改动）：生成器**三处**总述散文修正——issue 点名的开篇句与族表行 + **队长复审追加的第三处**（「与错误码的区别」小节「可选择修复」对 W1090 不成立，t1 首轮登记、同根因同文件同批修）；统一口径「码族与发射级别分开 + 例外点名 W1090（error 级、计入 error_count、改变退出码）」，事实基准为 W1090 条目自述；生成物再生 3 行行内替换，**无行号平移、无计数变化**；W1090 条目与生成逻辑未动；全 docs 面复查无「无例外版本」残留。

> **证据（三段判据链，2026-10-25）**：
>
> | 环节 | 判据 | 结果 |
> | --- | --- | --- |
> | 实现轮 | 红相（HEAD 态生成物含矛盾断言）→ 绿（三处含例外、与 W1090 条目逐项一致） | t1 以 stash + 指纹（`git diff \| sha1sum` 前后一致）在 HEAD 态取红相，既有改动零丢失 |
> | 实现轮门禁 | docs-truth / docs-refs / 生成器重跑 | 全 exit 0（工作流脚本的 `genDiff` 门禁因 dirty 树语义误报 exit 1，队长实测重跑幂等后修正脚本删除该门禁——generator 幂等判定归验证轮干净树） |
> | 独立验证 | 三处断言逐项对照 + W1090 条目逐字未变 + explain 输出复核 + 残留扫描 | PASS |
> | 独立验证 | 变异可逆（生成器散文打回旧断言重跑） | 生成物复现矛盾态（判据能失败）、还原 blob 一致、trial 后重建 |
> | 独立验证 | 干净树生成器幂等 | 重跑零 diff（exit 0） |
> | 复审 | 事实准确性 / 边界（W1090 条目与其余 9 生成文件未动）/ 生成物可由生成器复现（未手改） | APPROVE（2 条 low：族表简表表述较另两处简——无错误事实仅一致性观察；量测环境偏离说明见下） |

> **过程注记**：①t2 在主工作树量测而非批次树——替代路径依据：HEAD 恰为冻结 revision 且树干净（量测前后各核一次）、所跑为纯脚本与 npm 构建不依赖 cargo 产物、生成器重跑 diff 空证明未改文件；如实报告非隐瞒，注记认可但**后续批次仍默认走批次树**（零 cargo 依赖的纯文档任务可援引本例）。②第四处同族矛盾（`docs/src/dev/architecture/01-routing.md:264` 手写「不计入错误数」对 W1090 为假）超出「只改生成器三处」范围未修，遗留登记 [#509](https://github.com/ChenXu233/YaoXiang/issues/509)；`013-error-code-specification.md` 经查已自洽（例外同段登记，无需动）。③`cargo audit` 钩子全程网络故障 SKIP（本组四轮一致，零依赖变更）。

> **3.21 整组归档（2026-10-25）**：四个三级任务同日全部完成——3.21.1（`cfb3442d`，#468 取负溢出）、3.21.2（`df89d6ee`，#469 诊断外显三处，D 类经用户批准）、3.21.3（`a8ad0530`，#471 mono 配置贯通）、3.21.4（`3b6df553`，#470 生成器散文）；每轮均「实现门禁全绿 → 独立验证 PASS（含变异可逆）→ 独立复审 APPROVE」三段判据链，遗留集中 [#507](https://github.com/ChenXu233/YaoXiang/issues/507) / [#509](https://github.com/ChenXu233/YaoXiang/issues/509) / [#503](https://github.com/ChenXu233/YaoXiang/issues/503)（关联证据）；本地门禁全绿，各批 CI 结论按覆盖关系核对登记（执行面在跑的并入发版前统一核对，见各 issue 收口评论）。批次树已 gc 清零。

## P3 3.20 收口（轮 1：3.20.1 W1001 闭包引用 + 3.20.2 W1003 变体解构导入，2026-10-25）

> **触发**：WBS 3.20.1 / 3.20.2（2026-10-21 登记，来源 = 3.4.6 施工期暴露的两个既有 W 码误报；修复方向经用户 2026-10-19 裁决丙预先批准，回滚义务绑定两任务——修好后恢复 test.md / weak.md 被改写绕行的示例并再生基线）。

> **改法**（提交 `4c3c8260`，10 文件 +190/−37，两误报双修一笔提交）：①**W1001**——根因比登记描述深一层：`Expr::Lambda` 臂本就递归闭包体，漏点在 parser 对单表达式体闭包 `() => expr` 合成的 `StmtKind::Return` 糖语句（`pratt/led.rs:410-419`）在 `collect_from_stmt` 的六臂 match 中无对应臂、落入 `_ => {}` 致表达式体闭包内引用整体丢失（实测佐证：最小形态 `() => helper()` 报、块体 `() => { helper() }` 不报）；修法 = `dead_code.rs:735` 补 `Return(Some(expr))` 收集臂（+8 行）。②**W1003**——根因：match 变体名走 `check_pattern` 的 Union 臂逐变体比对 sum_types、从不经 `Expr::Var` 解析，语义层唯一导入使用打点 `note_import_use` 接不上，语法兜底收到的 some/none/v 与导入本地名 option 集合成员连不上；修法 = Union 臂解析出和类型后 `note_import_use(&sum_name)`（`expressions.rs:1390`，+7 行），沿既有成员监视集回流链（`watch_import_members` → `imported_used` → `drain_import_tracking` → 三层并集）算作消费。

> **回滚义务（已执行）**：`weak.md` 删 `probe = Option(Int).some(7)` 绕行，与 3.4.6 前形态（`git show 1725d021^`）**逐字节一致**；`test.md` 删主函数直调绕行恢复闭包引用自然形态。**一处有据偏离**：`test.md` 的 `use std.assert` 不恢复——实测 3.4.6 前原文跑 check 报 `W1003 unused import: 'assert'`（该示例只经 `use std.test` 用 `test.assert_eq`，std.assert 真未被消费），3.4.6 删它是**真修**而非误报绕行；恢复会重引入真 W1003 并打破文档示例门禁（`stdlib_docs.rs:116` stderr 非空即红）。复审两半论证均实测复核同意。

> **证据（三段判据链，2026-10-25）**：
>
> | 环节 | 判据 | 结果 |
> | --- | --- | --- |
> | 实现轮 | 两处各红转绿 + 守卫 + 负控 | W1001 红测试修前 FAILED（输出含实际 W1001 诊断）修后绿，真未使用守卫保留；W1003 红测试修前 FAILED 修后绿且 diagnostics 空（改判后仍可编译），真未使用守卫保留，删导入仍 E1002（前置未削弱） |
> | 实现轮门禁 | fmt / 全量 clippy / testsuite fast / markers / CI 四脚本 | 8 项全 exit 0（一轮过；首轮工作流因 testsuite stdout 262KB 超 world.run 256KB 上限 errored——改 `python -c` 包装只回传尾部 3000 字符后修正，实现者工作零成本缓存复用） |
> | 独立验证 | 批次树两处三支复测（误报消失 / 守卫真阳性 / 负控）+ 文档示例端到端（7 块 0/0） | PASS |
> | 独立验证 | 变异可逆（两修复合一补丁 revert） | 恰好两个红转绿测试 FAILED、apply -R 还原 blob 与 HEAD 逐一一致、trial 后重建 |
> | 独立验证 | 零漂四项（语料 356 零差异 / 快照绿 / 生成器干净树 diff=0 / stdlib_docs 门禁绿） | 全过 |
> | 复审 | 修复语义 / 回滚核实 / 基线对账 / 红线 / 漏报风险 | APPROVE（3 条 low，见下） |

> **基线与过程注记**：①语料再生 4 条记录 8 行全部为误报 W 码消失（`test_suite.yx` W1001×3、`test_suite_fail.yx` W1001×2、`weak_ops.yx` W1003、`sum_type_native_match.yx` W1003），**`test_suite_fail` 的真阳性 W1003（`use std.result` 确实无人用）保留在基线**——修复不误伤真阳性的活证据；文档示例不在语料范围（语料 = `tests/yaoxiang/**`、`src/std/tests/**`、multifile main，示例走 stdlib_docs 门禁——口径澄清实现轮已核）。②复审 3 条 low：Union 臂失败路径（field_not_found 等）在打点之后才 return Err——失败时已标使用，但伴随 E 码、非阻断通道、且「写了变体名即尝试消费」语义可辩护；新增 4 测试的判据引用（WBS 号/文档路径）可解析但未直引 warning-codes.md 章节（轻微规范缺口）；提交信息「闭包表达式体」未完整陈述 Return 臂顺路覆盖显式 `return expr;` 语句的同源历史漏收（漏报方向无夹带）——本注记即补述。③`spawn/analysis.rs` 的同名收集器经核实为独立读写分析且已处理 Return，无同类缺陷。

> **登记**：WBS 3.20.1 / 3.20.2 已完成（无独立 issue 锚，判据落本表与 WBS）；3.20 组仅余 3.20.3（issue [#505](https://github.com/ChenXu233/YaoXiang/issues/505)，算子方法 W1005）。

## P3 3.20 收口（轮 2：3.20.3 算子派发可达性 + 登记 3.20.4，2026-10-26）

> **触发**：issue [#505](https://github.com/ChenXu233/YaoXiang/issues/505)（2026-10-25 登记，WBS 3.20.3）：算子接口方法被算子表达式**实际派发调用**（run 值正确 20.0/false）仍报 W1005——可达性不计算子派发。

> **改法**（提交 `ef4cbbd8`，7 文件 +308/−13）：根因实证——算子表达式的具名决议走 `OperatorDispatch` 义务通道（`TypeCheckResult::operator_dispatches`，RFC-011b span 键控，产点 `expressions.rs` 四处 + `statements.rs` 两处），与死代码分析消费的 D20 重载决议表（`overload_resolutions`）是 `obligations.rs` 里的**两条独立义务**、互不相通；方法组无决议记录回落短名共判，而 AST 只有 `u[0]`/`u==t`、无 `index`/`equal` 标识符 ⇒ 误报。修法 = driver 解析 `operator_dispatches` 按既有 `typed_owners` 归因到定义单元，analyzer 新增「派发组整组命中」（派发名不分重载候选 ⇒ 组内全部声明按活）；方法名映射复用 Layer 0 SPECS（`OperatorDispatch.method` 的产物），不生造第二份；Script 与 Check/MultiFile 双臂接线。

> **证据（三段判据链，2026-10-26）**：
>
> | 环节 | 判据 | 结果 |
> | --- | --- | --- |
> | 实现轮 | 单文件（2 条 W1005）与跨单元（4 条）双形态红转绿 + run 值不变（20.0/false）+ 守卫（接口声明但算子未作用仍报） | 红态机械还原实测（t1 自查修正过一次 forbidden 行号错位导致的假绿并在红态复验翻红） |
> | 实现轮门禁 | fmt / 全量 clippy / testsuite fast / markers / CI 四脚本 | 8 项全 exit 0（一轮过） |
> | 独立验证 | 双形态值级+诊断集双断言、守卫、泛型残留行为与登记一致、变异可逆（还原派发接线后恰好主验收测试红）、零漂四项 | PASS（一次 trial 后测试二进制复用变异期产物假红，强制重编后全绿、树内容 blob 核实始终为冻结 rev） |
> | 复审 | 断链三环逐点核实、两臂接线完整、整组按活有据、4.9.6/4.9.7 逐候选零改动、基线无夹带、变异实验独立复核红绿真实性 | APPROVE（3 条 low 收口提醒，见下） |
> | 基线 | 语料 4 处 W1005 消失逐夹具归因（Vec3.equal / Box.index / Grid.index / Point.add+multiply——按条目计 5 条、按夹具计 4 处），同提交再生；IR 快照零漂移 | 全部真实派发夹具 |

> **遗留与过程注记**：①**3.20.4 新登记**（issue [#510](https://github.com/ChenXu233/YaoXiang/issues/510)）：泛型约束形参的算子派发（`T: Add` 形参经 mono 具名化，typecheck 后的死代码分析不可见）仍误报——修它需消费 mono 改写产物（第三条通路），注入点设计需先裁决；t1 未用放宽手法（把 `T.add` 也算命中）掩盖，夹具 `constrained_generic_ops.yx` 的现存 W1005 被基线钉住。②**整组豁免 × D20 逐候选叠加边界**：同一 `(Type, method)` 组既有普通重载调用又有算子派发时，死候选被整组豁免掩盖——与运行时按名 Call 的 RFC-011a 历史债同源（派发通道与运行时都无法分辨「候选是否被调用」），宁漏报方向合规、非判据放宽；何时收敛取决于 RFC-011a 签名分发债的清偿。③**测试基建观察（待用户裁决是否登记改进）**：`src/driver/tests/ref_pool.rs` 的 `assert_warnings` helper（4.9 系列既有，非本批引入）——`forbidden` 参数是 `(code, line)` 精确对的存在性断言，**行号写错 ⇒ contains 永假 ⇒ 断言永真 ⇒ 静默空转**（t1 首跑假绿即踩此坑）；改进方向 = forbidden 语义升级为「该文件不得含该 code 的任何条目」或 forbidden 行号与诊断集反向对账。④跨单元「不同名」类型（`a.Temp`/`b.Box`）的守卫形态未写（t1 仅单文件守卫 + analyzer 级验证）——同名近似（`typed_owners` 可能归属都豁免）与 4.9.7 既有口径一致故不阻断，随 3.20.4 一并考虑。

> **登记**：issue [#505](https://github.com/ChenXu233/YaoXiang/issues/505) 已关闭；新三级 **3.20.4**（#510）登记待排期（三级计数 189 → 190）；3.20 组至此 3.20.1–3.20.3 完成、3.20.4 在册。

## P5 checker 文件内拆分收口（5.1 / 5.2 / 5.3，2026-10-26）

> **触发**：WBS P5 三个二级任务（原缺口 G1「P5 无任何施工步骤」的补齐）：`include!` → 真 `mod`、拆出 `refinement`、拆出 `annotations`。判据 C1 zero-diff。**本批按 AGENTS 第 3 条「加框」把三个二级任务并进一支队**——同一文件、同一条依赖链（5.1.1 → 5.2.1 → 5.2.2 → 5.3.1）、同一判据；用户 2026-10-26 批准。若按「一队一二级」拆三队，5.1 那两行改动要单独养一支队，而真正的工作是「把约 3,700 行搬进三个子模块」这一个连续动作。

> **轮次构成（14 个任务）**：t1 勘察（只读）→ 5.1 / 5.2.1 / 5.2.2 / 5.3 各「实现 + 独立验证 + 异家族复审」→ 收口。实现 `deepseek-v4.1-flash-oc`；验证 `glm-5.3-flash`；复审先后为 `MiniMax-M3.1-Flash` → `glm-5.3` → `kimi-k3.1-pro`（前两者分别遇 TRANSPORT 与 429，见「执行环境事故」）。

> **终态形状**：`checker.rs` 5736 → **4167** 行（余 3 个 `mod` 声明与编排方法）；新增 `checker/refinement.rs` 1107、`checker/annotations.rs` 470；`checker/semantic_tokens.rs` 1553（原 1535 + 作用域头 + rustfmt 重排）。四文件合计 7297 行。

> **四轮判据（每轮独立在冻结 revision 上量测）**：
>
> | 轮次 | 形状 | 生成物 | 反向对照 |
> | --- | --- | --- | --- |
> | 5.1 | `include!` → `mod semantic_tokens;` + `use super::*` + 入口 `pub(super)`；B1 库存清零 | 零漂移 | 删 `use super::*;` → build exit 101（136 errors） |
> | 5.2.1 | 迁 22 项（方法 15 + 类型/常量 3 + 自由函数 4）；保留面 13 项与共享 6 项原地 | 3 文件 4 行 / 6 个发射点 token | `self.env` → `self.envv` → build exit 101（E0609） |
> | 5.2.2 | `&mut self` → `&self` + 返回诊断；7 插入 / 7 删除 | 42 行，`:N` 归一残留 0 | 漏传 `module` 实参 → build exit 101（E0061） |
> | 5.3 | 迁注解族 10 项；`collect_used_in_type` 不迁、零再导出 | 19 行（17 纯平移 + 2 路径行号） | t12 报告文本丢失，未留存（见「执行环境事故」） |
>
> 每轮共同：语料 356 行 / 76950 字节 sha256 `48c2b40a…` 逐字节等值；IR 快照 204 张零漂移且脚本输出含「1 passed」；`cargo test --lib` 2845 passed / 0 failed / 2 ignored（四轮同数）；`check-boundary` 的 `pub(crate)` 全程 **167 未增**、B1 库存 0；fmt / clippy / docs-truth / docs-refs / fanout（默认模式）/ 生成器零漂移 全 exit 0。子模块入口一律 `pub(super)`（`collect_semantic_tokens` / `collect_refined_binding_checks` / `check_annotation_type_names` 三处），不放大到 `pub(crate)`。

> **5.2.2 为什么不是 WBS 原文的形态**：原文写「`&mut self` → `(module, &mut TypeEnvironment, &mut Vec<ProofFunctionCall>)`，调用点仅 1235」。实测：该 impl 块 78 处 `self.`、入口生产调用点 3 处 + 测试 1 处；「脱 impl」要求整片走查方法一并转自由函数，连带 `resolve_type_annotation`（生产 8 处）与 `collect_param_refinements`（生产 2 处 + 测试 1 处）外移，并制造「保留面（签名收集）→ refinement」反向依赖——与该步要消除的耦合方向相反。故改取 **B′**：入口改 `&self` + `-> Vec<Diagnostic>`，诊断写入点搬到唯一调用方 `check_module_impl`，7 插入 / 7 删除即消除「以 `&mut self` 隐式写错误通道」。`#263` 的缓冲注释即该隐式通道的旁证。

> **t1 勘察对 WBS 前提的三处推翻**（判据一律改用符号）：① WBS 行锚全失准（`:5618` 已是 `RefinedWalkCtx` 字段、`3441–3661` 已属类型定义面、5.2.2 的「1235」实为 `:1240`）；② 混装块 `3843–5571` 里「应归 annotations 的项 = 0」——`resolve_type_annotation` 的调用方是 6 处注解路径 : 1 处走查 : 1 处走查，属跨界共享；③ 5.2.1 不能整块搬——该块 1729 行含 **437 行保留代码**（`auto_derive_traits`、`collect_refined_var_names` 族、`collect_termination_measures` 族、`terminates_measure` 等）。最终边界：annotations 10 项 / refinement 22 项 / 共享 6 项 / 保留 79 项。

> **三条契约缺陷（队长写错，记录在案）**：① t11 的 acceptance 要求更正 09 的 5.3.2 行，`inScope` 却没圈 `09-execution-wbs.md` ⇒ 改文档与 `changedPaths` 校验互斥（实现者停在原地提问，没有硬凑）；② t11 任务书正文照 WBS 旧行锚列了 7 个自由函数，与 objective/acceptance 的「10 项、`collect_used_in_type` 不迁」冲突（实现者按契约执行，判断正确）；③ 5.2.2 原文的调用点计数失真。共同教训：**行锚与调用点计数不能凭旧文档写契约，必须先量**。

> **两条文档事实缺陷（同样是我方）**：① `01-routing.md` 一度写入 `semantic_tokens.rs` 1537 行——它是「格式化前 1535+2」的中间态，**任何 revision 都未持有该数字**，由复审用 `wc -l` 抓出后改为 1553；② 5.2.2 那笔提交（`:recycle: refactor(typecheck): 精化入口收口为 &self + 返回诊断`）的信息写「check-fanout 0（13 处均既有登记项）」，实测该 revision 为 report-only、库存 9 / 违规 9、全输出无「13」——不改写历史，改由收口提交的信息加一行更正并在此登记正确数字。

> **文档行锚清算（32 处 / 7 文件）**：P5 把代码搬出 `checker.rs` 后，`checker.rs:<行>` 形式的引用**失去对象**（例：`02` 的六处 `Unproven` 分支锚、`01` 的 layers 调用点、`layers/README` 的三个 `check_predicate` 站点与 `substitute_const_expr` 站点）。一律改成**符号**（函数名 / 模块名），不再写新数字——P5 本身就是三次连续搬运，数字锚必然再漂。`check-docs-truth` 与 `check-docs-refs` 都不校验源码行锚，这一批全靠复审发现。

> **执行环境事故（不粉饰）**：本批三次进程级失败（两次收尾阶段上下文/步数耗尽、一次三例 `TRANSPORT: Stream ended without finish_reason`），以及 GLM 路由 5 小时额度耗尽（`429 [1308]`，当日 15:37:53 重置）。后果：t12（5.3 独立验证）任务侧 completed 但**报告文本丢失**，改由队长在收口时重跑同一套门禁复现数字（与 t6/t9 同值），而不是假装报告还在。所有失败均未污染工作树：`HEAD` 与提交始终干净；成员不推送、不清理，清理与收口归队长。另：`pull --rebase` 时与上游 `3.20.3/3.20.4` 的提交在**生成物**上冲突，逐笔以「重跑生成器」解冲突（而非手改行号），故每笔提交的生成物与当时代码自洽。

> **遗留**：① `check-fanout.py --strict` 9 处（全在 `driver/`，父提交同判据同 9 处 ⇒ 既有欠账，非本批引入），是否登记 WBS 行待用户裁决；② `check-snapshot-drift.sh` 文案写「204 快照」而实际 216 个 `.snap`（早于本批）；③ `02-stage-contract.md` 中「仅行号平移、指向仍留在 `checker.rs` 的逻辑」的历史引文未逐一重取（本批只修**已指错对象**的那些）；④ 批次树 `gc` 后清零。

> **计数**：本批不新增二级/三级任务 ⇒ 09 顶部计数表不变。

## P9 判据 C 接线点重定基收口（9.12.1 / 9.12.2，2026-10-26）

> **登记误标（我方）**：P5 收口实录把 `check-fanout.py --strict` 的 9 处违规列为「遗留」，同日登记为 9.12，标题写成「清存量同名多定义」——**写错了**。`--strict` 的违规只来自判据 C（入口接线）；A 判据（同名函数多定义，181 条）是 report-only，永远不会让 `--strict` 红。收口时更正标题与正文。

> **根因**：`STAGE_MARKERS` 是 2026-10-06 按 P4 之前「六条路径各自手工接线」的现状标定的，注释明写「P4 统一 Driver 后本表随库存一起删除」。P4 落地后词表与 19 条 `KNOWN_WIRING_SITES` 都没退休 —— 于是判据把 Driver **自己的阶段表**判成违规（`Stage::DeadCodeAnalysis` 变体名、`slug` 字符串表、`state.rs` 依赖表），与禁令三判据 C 的规则文本（「新增第 N 个入口**而非登记进声明式阶段表** → 违规」）正好相反。这是「门禁长期红、被人为绕过」的典型形态，故处置是**重定基判据**，不是补白名单。

> **9 处的实际构成（逐行查了命中行）**：4 处是真接线（`driver/arms.rs` 的 `parsing_single_file` / `parsing_check` / `parsing_lsp` 与 `driver/helpers.rs` 的 `collect_package_refs`，各一次 `tokenize` + `parse`）；4 处命中的是**阶段词汇表本身**（`driver/mod.rs` 的 `dispatch` 臂、`driver/stage.rs` 的 `scope` 谓词与 `slug` 表、`driver/state.rs` 的依赖表）；1 处是 `frontend/module/orchestrator.rs` 的 `#[cfg(test)] mod` 单元测试（原 `is_test_path` 只豁免 `/tests/` 目录，不覆盖 `src/` 内嵌测试模块）。

> **四项判据修正（口径变化，逐项有依据）**：① **LEX 并入 `Parsing`** —— `src/driver/stage.rs` 与 02 §目标设计 1 都写明 `Parsing` =「词法 + 语法分析（lex → parse）」，词法不是独立阶段，同一函数内 tokenize + parse 只算一个阶段；② **DEADCODE / MONO 只认调用形态** —— 原正则 `dead_code|DeadCode` 会把 `Stage::DeadCodeAnalysis` 这类词汇表当成阶段调用；③ **`#[cfg(test)] mod` 豁免** 以**花括号配对**求模块行范围（不用关键字回看）；④ **12 个变体的臂形态改为派生** —— 从 `Stage::ALL` 取变体、从 `dispatch` 的穷尽 match 取 `self.<臂>(`，单一源 + 三条 fail-loud 守护（变体源失效 / 变体在 match 里找不到臂 / 派生臂名在 `src/driver/*.rs` 里没有 `fn` 定义），`dispatch` 另单列 `DECLARED_DISPATCH_POINTS`（输出标记 `[调度点]`，不计违规）。

> **附带修掉的虚报**：`fn_ranges` 把函数签名行算进被测体，于是 `fn check_project(` 的函数名自匹配 TC 正则、凭空多出一个阶段（`check_project` / `check_source_in_project` 两条退役理由因此不实）。新增 `fn_body` 切掉签名段；实测「≥2 阶段 → <2 阶段」的函数减少量为 **0**，即这次修根因没有顺手放掉任何真接线（由复审用四种口径独立重算确认，非复述实现者）。

> **13 条退役库存的分类（实测，非推断）**：函数已删除 5 条（`frontend/pipeline.rs` 的 `run_typecheck` / `run_ir_generation`、`orchestrator` 的 `collect_project_refs`、`lsp` 的 `update_semantic_db` / `run_diagnostics`）；函数仍存但已不再直接调用阶段原语 5 条（`pipeline.rs run`、`orchestrator compile_project` / `compile_embedded_module` / `check_project` / `check_source_in_project`）；仅因 LEX 并入 `Parsing` 而降为单阶段 3 条（`lib.rs eval_code`、`orchestrator parse_file`、`lsp/handlers/completion.rs document_symbol_items`）。最终库存 = 6 条，接线点实测 = 7 处（6 库存 + 1 调度点）。

> **复审链四轮，每轮都抓到真问题**：轮 1 `needs_revision`（medium）——词表不认 driver 阶段臂的调用形态，`self.parsing(` + `self.typecheck(` 的手工两阶段接线**实测为绿**（真逃逸，不是措辞问题）；轮 2 `needs_revision`（medium）——「其余 6 个变体除臂形态外没有第二个可调形态」不实：VendorConsistency / Discovery / Registry / RoleClassification 的阶段实现是 Driver **之外**的共享原语；轮 3 `needs_revision`（low）——盲区段把「活接线」与「死代码候选」并列成同一处置菜单；轮 4 **pass**，无 finding。轮 2 另独立验证了「派生失效必须 fail-loud」（四种破坏各造一次全部 exit 1，不存在「退化空表却仍全绿」）。

> **变异证据**：独立验证侧 5 条负控（driver 外新接线 / driver 内非 dispatch / 删调度点登记 / 删库存条目 / 登记失效），复审侧逐条用自己的补丁复跑全部 exit 1 并机械还原（`git status` 干净 + blob 与 HEAD 一致）；复审另自造 3 个实现者与验证者都没用过的形态。判据没有变成「改绿就算」——每轮都要求「你能不能再造一个应红的」。

> **队长自己的四处缺陷（不粉饰）**：① 派工写「13 条里 10 条函数已不存在」是从脚本 `[进展] … 已消失` 标签反推的，而该标签只表示「不再被判为接线点」，实测分类是 5 / 5 / 3；发现后 amend 了验证任务的 acceptance，没有让验证者背假 finding。② 自动生成的 repair 任务 `inScope` 里混进了 findings 文本被切出的**伪路径**（`PROOF/DEADCODE/MONO`、`PARSE/TC`、`Codegen/Exec`）与禁改文件，直接下发会让 `changedPaths` 校验当场失败——由队长重写为单一脚本路径。③ 派工文本里写了「用户裁决 A′」这种**会话私有代号**，成员如实抄进脚本 docstring，违反元规则①（仓库内不可解析）——复审抓出后改为四处可解析依据（禁令三判据 C / 02 §目标设计 1 / `stage.rs` 的 `Parsing` 注释 / 09 的 9.12 行）。④ **9.12 登记提交只追加了「+1 = 9.12 / +1 = 9.12.1」的日期注记，没有同步行首总数**（仍留 70 / 190），收口时一并补正（含本批新增的 9.12.2 ⇒ 71 / 192）。前三条都是「凭标签与旧文档写契约」的同一个病，与本批要修的判据错误同源。

> **流程偏差（记录，不掩饰）**：一次变异试验没走 `scratch.py trial` 批次树，而是在主仓工作树用 difflib 生成补丁 → `git apply` → 跑门禁 → `git apply -R` → blob 对账。复审判定「机械可逆的实质等效」并接受，同时记两条注记：绕开「探针/独立检出一律走批次树」的约定、且无「变异前树必须干净」的前置检查与 `apply -R` 失败兜底。其余各轮均在批次树的冻结 revision 上量测。

> **环境事故**：复审成员的路由两次 `TRANSPORT: Stream ended without finish_reason`（收尾轮与后续消息），用户手动复活成员后继续；两次均未产生未收口的 attempt、未污染工作树。另有一次评审循环撞运行时上限（三轮 `needs_revision` 后由系统升级），队长按「先向用户报、不自行再开一轮」处置，用户复活成员后才继续。

> **门禁（最终字节上）**：`check-fanout.py --strict` exit 0（违规 0、库存 6、接线点 7）、report-only exit 0；`check-boundary.py` exit 0（`pub(crate)` 167、违规 0）；`check-docs-truth.py` / `check-docs-refs.py` exit 0；重跑 `gen-error-code-docs.py` 后 `git diff --quiet` exit 0；A 段 181 条同名定义、B 段六族文件数（BinOp 58 / UnOp 31 / MonoType 120 / ConstValue 78 / Literal 54 / bytecode 运算符 10）与改动前一致。Rust 侧门禁（fmt / clippy / test / 语料 / 快照）本批不适用——四笔改动只碰 `scripts/ci/check-fanout.py`，`git show --name-only` 可证。

> **遗留（登记为 9.12.2，未决）**：① 4 个变体的共享原语未收录进词表（是否收录属**未决裁决**，实测收录会新暴露且仅暴露 2 处）；② `build_registry` 是零调用者的死代码重复（`driver/arms.rs` 的 Registry 臂自建表、不调它），但它是 lib 目标公开项，删除属**对外 API 面变更**，须单独裁决；③ `collect_package_refs` 是活代码的真接线，收录后需登记库存或重构。

> **计数**：本批新增三级任务 9.12.2 ⇒ 三级 190 → 192（含补正的 9.12.1 漏计 1 条）；二级 70 → 71（同上，9.12 登记时漏计）。

## P9 判据 C 覆盖面扩张与 `build_registry` 清账（9.12.2，2026-10-26）

> **裁决**：用户 2026-10-26 就两个互不耦合的决策给出 **D1-A（收录 4 个变体的 Driver 外阶段共享原语）+ D2-A（删除零调用者的 `build_registry` 链）**。裁决书写明 D1 只改门禁的看法、不动代码与 API，D2 才是对外 API 面变更（`build_registry` 因 `pub mod frontend` → `pub mod orchestrator` 属 lib 目标公开项），因此两者独立评估、执行顺序为**先删后收**（这样新暴露只剩一处，库存落在 7 而不是 8）。

> **删除面（D2-A）**：删 `orchestrator::build_registry`（原 `:379-382`）与私有 `build_registry_from`（原 `:384-396`）。**连带删 `discover`**（原 `:416-418`）——它唯一的调用点就在 `build_registry` 体内，删后即零调用者，`-D warnings` 会因 dead_code 直接报错；这一点在契约里事先写明，不是实现时临时扩大范围。保留：`DiscoveredFile` / `Discovery` 类型 / `extract_module_info` / `parse_file` / `is_vendor_path`（各有其它使用者）。Driver 的 `registry` 臂（`src/driver/arms.rs`）自建表、不调被删函数 ⇒ 行为中性。

> **收录面（D1-A）**：删除后仍存在的 Driver 外阶段原语——VendorConsistency ← `ensure_vendor_consistency`、Discovery ← `discover_with_used`、RoleClassification ← `roles::classify`、Registry ← `extract_module_info`。两条口径要点：① `roles::classify` 必须用**限定形态**，裸名 `classify` 会误伤其它模块的同名助手（复审用「无关 `classify` + `tokenize`」探针实测复现过误判）；② 不用裸 `.register(`（仓内多处同名），`ModuleRegistry::with_std` 这类精确形态可用。收录后新暴露**恰一处**：`collect_package_refs`（`driver/helpers.rs:1057`，活代码、`driver/arms.rs:626` 调用），登记为库存 **6 → 7**，接线点 **7 → 8**。

> **判据自身的诚实性修正**：轮 1 复审抓到「Registry 变体零覆盖 + docstring 三处断言不实」——`build_registry` 删掉后，脚本却断言「Registry 已没有 Driver 外原语」，而 `extract_module_info` 与 `ModuleRegistry::with_std` 仍在。处置取**补标记**而不是改断言（改断言只是把洞写进文字）：只补键不加探针时 `--strict` 逐项不变（无副作用），补键后「Driver 外重建 registry」的探针由 exit 0 转 exit 1。同一轮还把「收录口径」改写成可判定的 R1–R3，并**删掉那条被表内 5 项证伪的规则**（「凡构造该阶段数据的函数就不收」——PARSE 收 `parse_file`、TC 收 `TypeChecker::new`、DEADCODE 收 `DeadCodeAnalyzer`、MONO 收 `Monomorphizer`、IRGEN 收 `AstToIrGenerator` 都是构造器形态）；反例换成 `is_vendor_path` / `scan_use_refs`。

> **覆盖面数字**：判据头部改为派生计算后的 **14 键 = 共享原语 12 + 2 个变体专有臂键（GlobalSlotAlloc / Linking）**，另 10 个变体的臂形态并入共享原语键；开头段与「覆盖范围」段统一为 **10 + 2**（原先「6 个变体另有共享原语 / 其余 6 个变体分三类」的旧划分已删除）。

> **失实注释与陈旧数字（同一批清掉）**：`orchestrator.rs:7` 模块头不再声称本模块构建 Registry（改为「构建由 Driver 的 `registry` 臂承担」+ 保留删除日期）；`statements.rs:737` 改指 `discover_with_used`；`orchestrator.rs:403` 的 `[discover]` 悬空 intra-doc 链接改为文字引用；脚本里 `orchestrator.rs:1312 → :925` 的陈旧行号（经核：父提交时该注释在 `:952`，`1312` 早已越界，**非本批引入**）与 `ModuleRegistry::with_std` 的「30+ 处」未实测数字一并改为实测值（**26 处 / 21 文件；`/tests/` 16 处 13 文件；其余 10 处 8 文件里 3 处落在生产文件的 `#[cfg(test)] mod` 内 ⇒ 生产 7 处 6 文件**）。`docs/src/en/**` 的翻译副本不手改，由 auto-translate bot 在推送后再生。

> **复审链**：轮 1 `needs_revision`（1 medium：Registry 零覆盖 + 三处断言不实；2 low：裸名 `classify` 过宽、悬空 doc 链接）→ 修复轮 → 轮 2 **pass**（复审用**自造探针**独立复现，未复用实现者的探针；F1 补键后违规行 `[Discovery/PARSE/Registry]` 由它自己构造）。随后把两处失实注释与脚本里三处失真表述（F4/F5/F6/F8）并成一轮清理 → 窄复审 **pass**：该轮用**剥掉 docstring 后比对 AST** 的强口径证明判据逻辑零夹带（非注释语句 254/254 逐条相同、词表 pattern 值级全同、运行期 `stage_markers` / `wiring_sites` / `dispatch_points` 结果全等），并独立复算了 F6 的四个数字。

> **行为中性证据**：语料差分 356 行**逐字节一致**（sha256 与父提交相同）、IR 快照 216 张**零漂移**、`cargo test --lib` **2855 passed / 0 failed / 2 ignored**（测试数不减）、`cargo fmt --check` 与 `clippy --all --all-features --all-targets -D warnings` exit 0、`check-boundary` exit 0（`pub(crate)` **167 → 166**，即删掉 `discover` 的必然结果）、`check-docs-truth` / `check-docs-refs` / `gen-error-code-docs` 零漂移全 exit 0。A 段同名定义 **181 → 180**：删掉 `build_registry` 后「`build_registry`: 2 个文件」这条同名条目退出名单，是删除的直接结果（父提交实测 181、当前 180，差值恰为该条）。

> **队长自己的两处契约缺陷（不粉饰）**：① 我在实现契约里写「A 段 181 条不变」，但删除必然让「`build_registry`: 2 个文件」退出同名名单 ⇒ 正确预期是 180；验证者按实测报 180 并给出对账，没有迁就我的错误预期。② F4（两处失实注释）的归属在**看板 acceptance 与派工消息之间自相矛盾**（一边算给本修复轮、一边说归另外两个任务），实现者按「修复面超出 inScope 就停下报告」处理——判断正确，是我的契约缺陷；随后单独开窄任务补做，并要求**原地替换不增删行**，以免把上一轮刚校正的行号再次推漂。

> **偏差登记（不改写历史）**：修复轮那笔提交的信息里写 `extract_module_info（:1057…）`，而该 commit **自身 revision** 上是 `:1058`（同一笔里在 `orchestrator.rs` 插入一行所致）——违反「行号必须在被引用的 revision 上重取」。处置：**不改写该提交**（改写会换掉复审已绑定的 sha、使其 `pass` 失效），改为在此登记；脚本 docstring 内的行号已同步为 `:1058`。

> **未修的两条非阻塞观察（如实留着）**：① `orchestrator.rs:7` 新写的「本模块只提供 `extract_module_info` 等构件」作用域是第 3 条（Registry），字面可能被读成模块级断言，复审建议改成「在 Registry 一节…」；② 同一模块头第 1 条「**发现**：从入口文件所在目录递归收集所有 `.yx` 文件」是**本批之前**就与实现（沿 `use` BFS）不符的旧描述。两条均非阻塞、未在本批处理。

> **过程偏差**：本轮无成员进程级失败（复审成员此前两次 `TRANSPORT` 是上一批的事）；实现者首次 amend 被 `commit-msg` 钩子拒（scope 不接受 `docs(frontend,ci)` 多值），改为 `docs(ci)` 通过，**未跳钩子**。

> **计数**：本批不新增二级/三级任务（9.12.2 已于前批登记）⇒ 09 顶部计数表不变（二级 71 / 三级 192）。另记一处自查：上一批把 9.12 勾成完成时，其三级 9.12.2 尚未做，存在过一段「二级已勾、三级未收口」的不一致，本批收口后该不一致自动消解。

## P3 3.20 收口（轮 3：3.20.4 泛型派发具名化 + 整组归档，2026-10-26）

> **触发**：issue [#510](https://github.com/ChenXu233/YaoXiang/issues/510)（2026-10-26 登记，WBS 3.20.4）：`sum: (T: Add) -> (a: T, b: T) -> T` 以 `Int`/`Vec3` 实例化（run 值正确）时 `Vec3.add` 仍报 W1005——typecheck 记录的派发名是约束形参名（组 `"T.add"`），具名化在 mono 特化，死代码分析不可见。

> **通路裁决（用户 2026-10-26 批准甲案）**：裁决前调查推翻了登记文案「修它需消费 mono 改写产物」的前提——typecheck 期已持有全部实例化：`InstantiationRequest.type_args: Vec<MonoType>`（instance.rs:20）+ `GenericFunctionId.type_params: Vec<String>`（instance.rs:345）。三案比选（甲 = driver 死代码臂内映射展开；乙 = 消费 mono 产物/管线后移——违 3.4.8 ③「CHECK 形态 mono 成功产物不消费不外发」+ 跨层红线；丙 = typecheck 内即时具名化——时序不可能，实例化请求来自调用点、晚于泛型体检查），甲案胜出：零管线改动、零跨层、复用 3.20.3 注入缝。

> **改法**（提交 `c94bb869`，7 文件 +329/−17）：driver 死代码臂建「形参名 → 具体类型名」映射，把派发组 `"T.add"` 展开成具体组（`Vec3.add`/`int64.add`…，原组保留）再喂 `set_operator_dispatched_groups`；只收值形态实参（Struct/Enum/Int/Float/Bool/Char），符号化实参（Generic/TypeRef）不入映射。三个单点函数（`generic_instantiation_map` / `declared_type_param_names` / `expand_dispatch_groups`，helpers.rs）两臂共用：编排臂 `resolved_method_marks` 两遍扫（先跨单元聚合建映射，派发循环展开后逐组走既有 owners 归因），单文件臂同构接线。**机制缺口修补**：跨单元兜底路径的请求 `params=[]`（t1 探针实测）⇒ 消费侧扫定义单元 AST 补名（复用 `ast::classify_generic_params` 单点，is_trait 与 checker.rs:1520-1526 同源：`has_trait || operator_interfaces::spec`）；上游登记口缺陷（checker.rs:466-477 Assign 臂只认 `(T: Type)` 不认 `(T: Add)` ⇒ `Export::type_params` 为 None）属 D/E 类**未修**——修它会让跨单元请求切到声明名路径、mono 特化匹配随之改变。

> **证据（判据链，2026-10-26）**：
>
> | 环节 | 判据 | 结果 |
> | --- | --- | --- |
> | 实现轮 | 先红后绿三组：单文件（夹具形态，红态 W1005@19 留档）、跨单元（临时项目）、守卫（sum 未调用 ⇒ 无请求 ⇒ 仍报）+ 值级×2（Int(3)/Float(5.0)）；3.20.3 既有 18 测试逐字节不变（纯追加 147 行） | 修前 2 FAILED（输出留档）→ 修后 18/18 绿；真二进制双夹具 check 0 warning、run exit 0 |
> | 实现轮门禁 | fmt / 全量 clippy / testsuite fast / markers / CI 四脚本 | 8 项全 exit 0（一轮过） |
> | 基线 | 语料再生恰 1 行（constrained_generic_ops 的 W1005@19 → 空，exit/stdout/stderr 不变）；IR 快照零漂移 | 逐条核对无夹带 |
> | 独立验证（t2） | 批次树三组复测（值级+诊断集双断言）、守卫、深链边界与登记一致、变异可逆（还原展开接线恰红 + blob 机械还原 + trial 后重建）、零漂四项 | **FAIL——唯一失败项 = 错误码生成物欠账**（见下）；其余全过 |
> | 复审（t3） | 数据链只用前端产物、两臂闭合、单点无复制、同源判据核实、守卫边界、过滤谓词对照 MonoType 全变体、归因口径与 3.20.3 一致、红绿独立复现、基线对账、测试合规 | **REQUEST_CHANGES（high = 同款欠账）**；主体判定成立：`c1adca7b^`+新测试补丁 2 条未修先红、trial blob 机械还原、`pub(crate)` 167 未增 |
> | 队长补修 | 重跑生成器 → 5 处行号平移（E0010/E0011 `arms.rs:92→93`、E8001 `helpers.rs:68→69`、W1006 `653→660`、W1007 `1752→1759`，计数无更正）→ amend 并入 → 幂等复核 | `git diff --quiet` exit 0；rebase 后复验仍幂等；t2/t3 判定迁移依据 = amend 仅追加生成物、代码与被验 revision 逐字节一致 |

> **六条边界登记（不改码，全部宁漏报方向）**：① 嵌套泛型深链——符号化实参不求值，`g(x) = sum(x, x)` 形态仍可能误报；② 短名尾段误配——补名按函数名尾段查表且 first-wins（`or_insert` 不合并），跨模块同名泛型错配方向 = 展开多余组 ⇒ 至多漏报；③ 兜底路径实参序——反推 type_args 按位去重，声明序与位序不一致的签名可错挂（同前，漏报方向；arity 不齐 zip 截断）；④ 块内 FnDef 形态——补名只扫顶层 Assign（当前无块内泛型派发场景证据）；⑤ Ref 形态实参不入映射——t3 探针（`sum(&a, &b)`）实测 check 过、run **E6006**（mono 按形参名改写与登记名 `Vec3.add` 不符）＝既有深层缺陷，伴生 W1005 残留 → [#512](https://github.com/ChenXu233/YaoXiang/issues/512)；⑥ 同名形参跨函数并集漏报——t3 探针（union_leak：`f` 体内派发但从不调用 + `g` 无派发但以 Vec3 实例化）实证从未派发的方法可被并集豁免——与 3.20.3「整组按活」同构（派发通道无法分辨个体），非判据放宽。

> **契约缺陷（队长写错，记录在案）**：t1 契约写「不改诊断发射点 ⇒ 不用重跑错误码生成器」——误。AGENTS 验证⑤原文即含「行号平移」：import/注释引起的发射点行号平移同样必须再生生成物。本批 t1 门禁清单漏「重跑生成器 + `git diff --quiet`」一项，导致 t2 抓到时已提交（本地 `c1adca7b`，amend 为 `17201560`，rebase 后终值 `c94bb869`——三值代码逐字节一致，差异仅生成物与提交位）。**后续批次 t1/t2 门禁固定含此项**。与 #470 批次的教训合并：生成器幂等必须作为独立门禁项存在，不能依附于「是否改了发射点」的主观判断。

> **登记**：issue [#510](https://github.com/ChenXu233/YaoXiang/issues/510) 已关闭（实现 `c94bb869`）；新遗留 [#511](https://github.com/ChenXu233/YaoXiang/issues/511)（上游登记口 D/E 类，修前须用户裁决）与 [#512](https://github.com/ChenXu233/YaoXiang/issues/512)（Ref 形态 E6006 既有深层缺陷）备案不占号；**3.20 组四任务（3.20.1–3.20.4）全部收口，2026-10-26 归档**（回滚义务已于轮 1 执行）——归档条件满足：全部三级收口、提交已推送、CI 按第 13 条交由后代 run 覆盖并入发版前统一核对。

## P6 6.0 门禁先行收口（6.0.1 / 6.0.2，2026-10-27）

> **触发**：P6 的设计文档写于 2026-10-05，此后 P4 统一 Driver、P5 拆分 checker，行锚已漂；09 的 P6 表与 03 文档的数字互相矛盾（13 / 12 / 11）。用户 2026-10-26 裁决「一队做 6.0–6.8」后，首轮先做 **6.0 门禁先行 + 全量锚点重测**，不先动业务代码。

> **裁决登记（用户 2026-10-26，三项）**：① **团队范围 = 一队做 6.0–6.8**（P5 先例；收口时登记偏差）；② **硬验收限 6.1–6.7**——`git diff --stat tests/ src/std/` 必须在 6.1–6.7 的**每轮**为空；**6.8.2（D54 行为修复，新诊断码 + 基线更新）单列一轮并单独裁决**，不适用本硬验收；③ **3.18 / 3.18.2 留 P3**，P6 之后另立设计/批次，并更正 09 中「正确修法属 P6」的措辞（该措辞已改为「归属阶段：P3（前置待 P6）」）。

> **本轮两条裁决**：① **计数 = 删 13**：03 §5.2 的处置表逐行数出 删 13 / 迁 1 / 留 12（26 → 13），**§5.2 表头「删 12、迁 1、留 13 / 26 → 14」与 09 的 6.3.1「删 11 个变体」都写错**，已按表体更正为 13。② **`AssocType` 的处置**：03 §5.2 第 18 行与 §5.7 的原触发条件（「若报告显示 `AssocType` 存在生产构造点，说明本节核实有误，正确动作是回到 RFC 决议表改 D8」）**确实命中**——实测在 `src/middle/passes/mono/function.rs` 的两个 AST→AST 替换函数里各有一个生产重建点。但**命中的是口径问题、不是核实有误**：那些点是把**已有 AST 类型**重造一份以替换类型变量，不是 parser 的产出。因此处置是 **D8 不变、口径改为「parser 前向零构造」、两个函数的重建点登记进 6.3.2**，**不塞白名单**。**用户裁决 A：就地更正 D8**（不附日期注记）——把「生产零构造」改为「**parser 前向**零构造」、补两处 AST→AST 重建点、把「其余 11 个」改为「其余 12 个」；删除结论不变。

> **轮次**：本批 = 6.0 的两个三级任务（6.0.1 新建 `tools/type-tables` 实现 T1/T2/T3 且 T1 先只报告不失败；6.0.2 在业务代码零改动上取全量基线并人工核对），共 6 个工作单元：全量锚点勘察 / 门禁实现 / 基线取样 / 独立验证 / 复审与修复轮 / 本收口。

> **判据**：C3（诊断码 / 消息 / 顺序不变）。本批是**纯新增门禁**（首版只发 `cargo:warning`、不 `panic!`），故实际判据是三条：**门禁在未改动代码上零漂移**、**故意引入零构造变体必须被门禁抓到**、**修复后基线与修复前逐点相同**。

> **门禁数字（实测）**：T1 扫 `src/` 全量 **287 个非测试 `.rs`**，**构造点 74 = 前向 25 + AST→AST 28 + 反向桥 21**（反向桥 A `From<MonoType> for IrType` 9 变体/12 点、反向桥 B `mono_to_ast_type` 8 变体/9 点；AST→AST 15 变体/28 点）。**前向零构造 13**（`Int`/`Float`/`Char`/`String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`AssocType`）、**全局零构造 1**（仅 `Enum`）。03 §2.2 名单内的 AST→AST 重建 = **5 变体 / 10 点**（`Union`/`Option`/`Result`/`Sum`/`AssocType` 各 2）——**这就是 6.3 的实际改写面**。T2 三表 13 臂（38 写法 / 25 别名）/ 15 项 / 15 项，差异 14 条（C1 10、C2 2、C3 2）全部登记处置、未登记 0；T3 差异 14 条（文档独有 3 + 代码独有 11）全部登记处置、未登记 0。

> **过程注记（两条）**：① **勘察任务的 `output` 落成占位串**：该字段写入时因工具读取失败写成占位串 `REPORT_READ_FAILED`，真实报告在其补交正文中；实现者据此对齐 26/26 行，结论一致。② **批次树里的 workspace 认领伪影**：验证轮在 `scripts/dev/scratch.py` 管理的批次树（worktree）内跑 `cargo run --manifest-path tools/type-tables` 时，外层 workspace 的 `exclude` 不匹配嵌套路径，cargo 报 workspace 认领冲突。量测改用**主仓同内容二进制 + `--root <批次树>`**（工具按 root 参数扫描目标树）完成，**未在批次树内构建**；工具文件逐字副本的 sha256 全一致。

> **复审链**：复审轮 1 `needs_revision`（F1 通配臂豁免泛化 / F2 03-09 口径载体 / F3 `cfg` token 子串判定 / F4 文件级文本包含判别名）→ 修复轮（豁免改逐条登记、`cfg` 改谓词语义、别名改从 `use` 声明 AST 解析并带作用域与泛型形参遮蔽、03 与 09 的口径载体同步）→ 复审轮 2 **pass**。修复前后基线**零漂移**：74 个构造点逐点相同、26 变体五项计数无差异、前向零构造 13 与全局零构造 1 不变。

> **遗留（转入下一轮契约输入，不在本批处理）**：① **T1 的 `Void` 兜底豁免登记须随 6.3 同步删除**——`Type::Void` 变体删除后该登记转为 stale，门禁随即红；② **T3 的比对对象待 6.6 裁决**——RFC-011 **没有**内置类型名表，当前口径是取表格 inline-code token（5 个），6.6 转硬前须在「改文档源 / 降级 report-only / 删除并并入 T2」之间三选一，**不得直接转硬**；③ `substitute_type_ast` / `substitute_type_in_ast` 的行号区间**由 AST 派生**，文档与散文不得写死行号；④ 09 的 P6 行锚已按本轮实测改为**符号优先**（类型 / 函数名 + 入库路径），数值行号只保留仍有效的若干处。

> **计数**：本批不新增二级/三级任务 ⇒ 09 顶部计数表不变（二级 71 / 三级 192）。6.0 的两个三级任务已在 09 勾选完成，证据取**实测数字**而非提交号（`pull --rebase` 会重写本地哈希）。

## P10 S11 登记（DateTime 迁出内置类型名，2026-10-27）

> **触发**：P6 6.1 的内置类型名定稿（15 名：`Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`）把 `DateTime` 排除在外。`DateTime` 是 `std.time` 的语义别名（`Int`），最终由标准库提供，不能留在 `MonoType::from_builtin_name` 里。

> **为何 6.1.2 不删**：6.1–6.7 的硬验收要求 `git diff --stat tests/ src/std/` 为空，而 `src/std/interfaces/time.yx` 与 `src/std/time.rs` 仍使用 `DateTime`；同批删除会改 std + 文档 + 基线。故 6.1.2 只保留**临时兼容别名**（仅大写 `DateTime`；小写 `datetime` 随 22 个别名同批删除），SPEC 内置表不写它并注明「待迁出」。

> **登记**：P10 S11（二级 +1、三级 +1 = S11.1），出处 issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)。**删除保证**：issue #513 是唯一出处；S11.1 未完成前不得关闭。验收 = `grep -rn DateTime src/std` 归零 + 旧写法明确未知类型诊断 + std 编译/测试通过 + 语料/快照按新签名再生并人工 review。

> **计数**：09 顶部二级 71 → 72、三级 192 → 193（本条登记即两处 +1）。

## P6 6.1 名称规范化收口（6.1.1 / 6.1.2 / 6.1.3，2026-10-28）

> **触发**：P6 6.0（门禁先行）收口后，6.1 的三个三级任务待收口——6.1.1 统计语料里的非规范类型名（决定能否**直接删同义词**、是否需要再生基线）、6.1.2 收敛 `MonoType::from_builtin_name` 的同义词表、6.1.3 核实 `classify_generic_params` 消费的是哪个参数列表（**必须逐行核实，不能假定等价**）。6.1.2 的名字清单属「登记/不登记 + 破坏性变更」决策点，须用户拍板，故本批先取定稿再动实现。

> **定稿（用户 2026-10-27）**：内置类型名 **15 个**——`Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`；删除 **22 个**实现内拼写——`int` / `int64` / `i64` / `datetime` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never`；**不设 lexer 别名表**（语料实测真实类型位 0 处，见 6.1.1）。`DateTime` **不属内置名**：它是 `std.time` 的语义别名（运行时就是 `RuntimeValue::Int`，#338 连带修复），最终由标准库提供；但 6.1–6.7 的 `src/std` 零差异硬验收不允许同批迁出（`src/std/interfaces/time.yx` 与 `src/std/time.rs` 的签名串仍写 `DateTime`），故 6.1.2 只留**大写 `DateTime` 的临时兼容别名**（小写 `datetime` 随 22 拼写同批删除），SPEC 内置表不写它并注「待迁出」。**迁出由 P10 S11 负责**（issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)）。

> **轮次**：本批 = 6.1 的三个三级任务（6.1.1 语料统计 / 6.1.2 同义词表收敛 / 6.1.3 参数列表核实），共 5 个工作单元：6.1.1 统计与口径复核 → 6.1.2 实现（定稿 15 名 + 删 22 拼写 + 删第二份别名表 + SPEC §2.1 自包含）→ 独立验证 → 对抗性复审 → 本收口登记。

> **判据**：C3（诊断码 / 消息 / 顺序不变）。实际落到三条可机械复核的判据：① **接受面**——15 个内置名在类型位置一律接受，22 个实现内拼写一律报未知类型（不得 panic、不得静默放行）；② **零漂移**——语料差分与 IR 快照相对基线**零差异**；③ **文档自包含**——SPEC §2.1（`docs/src/reference/language-spec/type-system.md`）本节即权威表，不再把名字清单委托给实现文件。

> **门禁数字（实测）**：① **接受面**——15 名全接受（`Never` / `Void` / `Bool` / `Int` / `Int64` / `Float` / `Float64` / `Char` / `String` 走变量注解形态 rc=0；`Int8` / `Int16` / `Int32` / `Float32` / `Float64` / `Bytes` 走参数注解形态 rc=0，绕开字面量宽度收敛；`()` 走 `mut u: () = {}` rc=0；`DateTime` 临时别名 rc=0）；22 个拼写全拒：21 个报 `E1003 Unknown type: '<名>'`，`void` 在**词法层**被拒（`E0010 expected a type, found VoidLiteral`——`void` 是 `Void` 的字面量关键字，不是类型名）；负控 `Uint` / `Int128` / `Point` 均 `E1003`（与 SPEC §2.1 的无符号注记一致）。② **语料差分 356/356、差异为空**（基线 `tests/baselines/corpus-parity.jsonl` 356 行）；IR 快照 `test_ir_snapshots_stable` 1 passed / 0 failed；`cargo test --lib` 2863 passed / 0 failed。③ **T2 报告** `tools/type-tables --report` RC 0——T2.1 权威清单 = **15 臂 / 16 写法**（`DateTime` 别名臂在 `src/frontend/core/types/mono.rs:883`）、T2.3 差异 11 条**全部已登记**（C1 大写缩写留 6.2、C2 `DateTime` 留 P10 S11）、**T2.5「已删除拼写哨兵」= 检查 22 条 / 被重新接受 0 条**；`tools/type-tables` 单测 16 passed / 0 failed（含「22 名口径自校验」与「哨兵能红」两个用例）。④ **6.1.1 语料重测**（命令见下）= 393 个 `.yx`（`tests/yaoxiang` 316 + `tests/yaoxiang-multifile` 39 + `src/std` 38）里 22 个拼写命中 **308 行**，**真实类型位 0 处**。⑤ **硬验收**——`git diff --stat tests/yaoxiang tests/baselines src/std/` 为空；`tests/` 侧仅 `tests/formatter_integration.rs` 改了 **3 行**（2 处 `assert_format_eq` 的 `i64`→`Int64` + 1 行注释），属**机械要求**（被改的拼写要求期望类型名同步），未删测试、未放宽断言。

> **6.1.1 重测命令（可复跑）**：

```bash
# ① 22 个定稿拼写在语料中的全部命中行（308 行）
grep -rEn --include=*.yx -w 'int|int64|i64|datetime|int32|i32|int16|i16|int8|i8|float|float64|f64|float32|f32|bool|char|string|str|bytes|void|never' tests/yaoxiang tests/yaoxiang-multifile src/std | wc -l

# ② 类型位候选（只有 2 行，且逐行看全在 // 注释内 ⇒ 代码里的真实类型位 0 处）
grep -rEn --include=*.yx '(:|->|<)[[:space:]]*(int|int64|i64|datetime|int32|i32|int16|i16|int8|i8|float|float64|f64|float32|f32|bool|char|string|str|bytes|void|never)([^A-Za-z0-9_]|$)' tests/yaoxiang tests/yaoxiang-multifile src/std
```

> **6.1.1 逐条裁决（308 行）**：字符串字面量 + 注释 + `std.string` 模块名 / 命名空间访问（`use std.string`、`string.chars(...)`）+ `src/std/interfaces/convert.yx` 里 `int.to_string` / `float.to_string` / `bool.to_string` / `char.to_string` 的**命名空间限定方法名前缀** + `Json` 的变体名与变体构造（`bool` / `str`）+ `void` **字面量**（`Result(Void, String).ok(void)`）——**无一处是类型标注**。另：仓库里余下的 4 个 `benches/shootout/src/**/*.yx` 同扫，命中只有注释与模块名，类型位同为 0 处。**2026-10-27 旧记的「原始词命中 16 处 = 字符串 12 + 注释 4」是窄口径**（只数整数/浮点族 14 个拼写、不计 `src/std` 的命名空间用法），本次按定稿的 22 个拼写全量重测，**结论一致**（真实类型位 0 处 ⇒ 无需再生基线）。

> **6.1.3 逐行核实结论**：`classify_generic_params`（`src/frontend/core/parser/ast.rs:1009`）消费的是 **`signature_params`**——`StmtKind::Assign::signature_params`（同文件 `:263`）与 `StmtKind::TypeDefinition::signature_params`（同文件 `:283`）两个字段；消费点 `src/frontend/core/typecheck/inference/statements.rs:1799`、`src/frontend/core/typecheck/checker.rs:1520` 与 `:2866`、`src/driver/helpers.rs:1160`。**不是** `extract_generic_param_names`（同文件 `:932`）的产物（后者消费点 `src/middle/core/ir_gen.rs:1919` / `:2052` / `:3684`、`src/frontend/core/typecheck/checker/semantic_tokens.rs:472` / `:706`）。**二者不等价**，四条可证差异：① trait 约束来源不同——`extract` 只认 `operator_interfaces::spec(name)`（`ast.rs:971`，静态封闭集合），`classify` 走 `is_trait` 回调（`ast.rs:1038`；调用点传入「trait 表有该名」**或**「内建算子接口有该名」，`inference/statements.rs:1800-1801`）⇒ `classify` 覆盖面更大；② `CONST_PARAM_TYPES` 判定不同——`extract` 另需「同一签名内被当类型引用」的启发式（`ast.rs:953-965`）才认，`classify` 直接判为 `Const`（`ast.rs:1029-1037`）；③ 元组多约束不同——`extract` 对 `Type::Tuple` 恒不认（`ast.rs:995`），`classify` 在「全部元素是 trait」时认作多约束 Type 参数（`ast.rs:1056-1069`）；④ 返回类型不同（`Vec<GenericParamName>` 无 kind vs `Vec<GenericParam>` 带 `GenericParamKind::Type` / `Const`）。

> **硬验收口径修订（用户 2026-10-27，承 2026-10-26 裁决②）**：P6 的零差异硬验收口径由 `git diff --stat tests/ src/std/` 修订为 **`git diff --stat tests/yaoxiang tests/baselines src/std/` 必须为空**，并**新准**：`tests/` 下的**直驱测试文件**（驱动被改代码的测试，如 `tests/formatter_integration.rs`）在**改动机械要求**时可修改，须逐项登记 **file / 行数 / 理由**，且**不得删测试、不得放宽断言**。**修订关系**：2026-10-26 裁决② 定的原口径把整个 `tests/` 一刀切为零差异，与「直驱测试文件须随被改拼写同步更新」的现实冲突；本修订**只放宽直驱测试文件一档**，**语料层（`tests/yaoxiang`）、基线层（`tests/baselines`）与标准库（`src/std/`）的零差异要求不变**，6.8.2 行为修复批单列一轮并单独裁决的例外也**维持不变**。三处同步登记：09 的 P6 硬验收条、03 §6.4 硬性验收条件、本实录。首次适用 = 6.1.2（`tests/formatter_integration.rs` 3 行，见门禁数字⑤）。

> **过程注记（三条，验证与复审轮实测）**：① **`scratch.py trial` 内的 bash 探针落到了 WSL**——trial 走 `git apply` 加命令执行，命令里的 bash 探针被 WSL 接管并以 **exit 127** 结束，判据**根本没触发**，首轮据此得出的「绿」是假绿；改用 **python 探针**后两门禁才真实转红。② **trial 之后必须先 `scratch.py build` 再量测**——`trial` 的机械还原只覆盖源码文件，变异版测试跑测已把批次树二进制重编，不重建则后续量测全部出自变异产物（AGENTS.md 第 9 条的固化条款）。③ **首轮 trial 未在补丁后重建**得出假绿（PASS），识别为方法错误后补「先 build 再探针」的脚本重做，两条负控才**真实转红**：负控 1 = 把 `int` 臂加回 → 该拼写重新被接受（rc=0）；负控 2 = 删掉 `Int` 臂 → `Int` 报 `E1003 Unknown type: 'Int'`。两次均 `git apply -R` 还原、还原后 blob 与 HEAD 逐字一致、`git status` CLEAN。

> **遗留（转入下一轮契约输入，不在本批处理）**：① **`DateTime` 迁出**（P10 S11，issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)）——6.1.2 的临时兼容别名臂（`src/frontend/core/types/mono.rs:883`）在 S11.1 完成前不得删除；S11.1 的验收 = `grep -rn DateTime src/std` 归零 + 旧写法明确未知类型诊断 + std 编译/测试通过 + 语料/快照按新签名再生并人工 review。② **诊断文案的内部显示名**（复审轮的非阻断观察）——`E1002` / `E1012` 的文案里仍出现 `int8` / `int64` 这类**内部显示名**（`MonoType` 显示名呈现层的既有事实，`operator_type_display` 只覆盖部分路径），不在 6.1.2 的判据与改动面内，留 **6.2 及之后的名称统一轮**考虑。③ **「类型名解析须单源」的约束仍在**——`NameKind`（03 §5.3）与两份 `name_used_as_type*` 的合并属 6.4 / 6.5，本批不动；6.1.2 只删掉了 `add_builtin_types` 里那份**小写别名副本**。④ **门禁登记随 6.3 / 6.6 同步**——6.0 收口实录的遗留①（T1 的 `Void` 兜底豁免登记须随 6.3 删变体清账，否则门禁报 stale）与遗留②（T3 的比对对象待 6.6 裁决）**仍在**，本批不动；6.1.2 新增的 T2.5「已删除拼写哨兵」在 6.1–6.7 各轮持续生效。

> **计数**：本批不新增二级/三级任务 ⇒ 09 顶部计数表不变（二级 **72** / 三级 **193**）。6.1 的 3 个三级任务为既有登记，勾选不改变计数；P10 S11 已在前一笔（2026-10-27）登记。

## P6 6.2 字节码类型统一收口（6.2.1 / 6.2.2 / 6.2.3，2026-10-28）

> **触发**：P6 6.1 收口后，6.2 的三个三级任务待收口——6.2.1 改字节码签名与模块类型表的载体、6.2.2 删有损桥 `From<MonoType> for IrType` 与 `ir::Type` 再导出别名、6.2.3 改解释器镜像的 `type_table` 载体。6.2 是 6.3 删变体的前置（03 §实施要点 阶段 0→6 严格串行）：载体不先统一，阶段 3 删 `ast::Type` 变体时就没有唯一的改写面。

> **轮次**：本批 = 6.2 的三个三级任务（6.2.1 字节码签名与模块类型表改载 MonoType / 6.2.2 删有损桥与 `ir::Type` 再导出 / 6.2.3 解释器镜像类型表改载 MonoType），工作单元依次为：实现（6.2.1 + 6.2.3 一笔）→ 实现（6.2.2 一笔）→ 独立验证 → 复审 → 欠账清偿（6.1.2 遗留，一笔独立提交）→ 验证（第二轮）→ 本收口。

> **判据**：C3（诊断码 / 消息 / 顺序不变）+ 03 §6.4 的硬验收（`git diff --stat tests/yaoxiang tests/baselines src/std/` 必须为空）。另加本批自身的两条载体判据：① 四处载体（`BytecodeFunction.params` / `return_type`、`BytecodeModule.type_table`、`Image.type_table`）统一载 `MonoType`，解码期**原样搬运**、不得再经任何降级桥；② 删桥后签名与类型表**不得丢结构**——此前塌缩为 `Void` 的 8 类 `MonoType` 必须保留真实形态。

> **结论（改动落点）**：① **四处载体改载 `MonoType`**——`BytecodeFunction.params`（`src/middle/core/bytecode.rs:814`）与 `return_type`（同文件 `:816`）、`BytecodeModule.type_table`（同文件 `:856`）、`Image.type_table`（`src/backends/interpreter/image.rs:44`）。② **解码三处原样搬运**——`params: func.params`（`bytecode.rs:2315`）、`return_type: func.return_type`（`:2316`）、`type_table: file.type_table`（`:2346`）；此前的 `.into_iter().map(|t| t.into()).collect()` 与 `.into()` 两处有损转换随 6.2.2 一并消失。③ **有损桥 `impl From<MonoType> for IrType`（改前位于 `bytecode.rs:2353-2390`）整个 impl 删除**。④ **`ir::Type` 再导出别名删除**——`src/middle/core/ir.rs:3` 的 `pub use crate::frontend::core::parser::ast::Type;` 降为私有 `use crate::frontend::core::parser::ast;`（同文件 `:7`）；`ir::Type` 这个名字在 IR 层不再对外存在。

> **提交（本地 dev，未推送）**：`:recycle: refactor(middle): 字节码签名与类型表改载 MonoType`（2026-10-28，14 files / +109 / −38）、`:recycle: refactor(middle): 删除有损桥与 ir::Type 再导出`（2026-10-28，4 files / +116 / −88）、`:wrench: chore(meta): 清偿 6.1.2 遗留的三笔预存欠账`（2026-10-28，4 files / +12 / −6）；全批 **20 files / +220 / −115**。按仓库约定提交号在 push 之后取（`pull --rebase` 会重写本地哈希），故此处只记提交主题与日期。

> **实测数字（冻结 revision 上）**：① **硬验收**——6.2 的父提交（`:memo: docs(meta): 固化 WBS 与实录的记录边界`，2026-10-28）到本批 tip 之间，`git diff --stat -- tests/yaoxiang tests/baselines src/std/` **为空**（语料层 / 基线层 / 标准库零差异）。② **语料差分 356/356、差异为空**（基线 `tests/baselines/corpus-parity.jsonl` 共 356 行），两文件逐字节相同。③ **IR 快照 216 条零漂移**（`src/middle/core/tests/snapshots/` 下 216 个 `.snap`，非 `manual` 再生）。④ **`dump_bytecode` 定向对拍 12/12 逐字节一致**（3 个程序 × 4 类产物；md5：p1 `2c93d78f` / `a7bca9a6` / `94f667d4` / `d013ebe6`，p2 `3af1d390` / `b21f40e0` / `4e0b2c99` / `d013ebe6`，p3 `a0617e15` / `006604ad` / `1e0636d3` / `03220d59`）。⑤ **`cargo test --all` exit 0**——lib **2863 → 2865 passed**（+2 = 两条新的解码保真测试）、0 failed / 2 ignored；integration **314** passed。⑥ **变异验证**——把「解码期把 `MonoType` 降级重建为 `ast::Type`」的旧桥以补丁形式装回，**4 条保真测试转红**（exit 101）；`git apply -R` 还原后该文件 blob `004bd15e422b…` 与 HEAD 一致、`git status` CLEAN（证明判据能失败，不是恒绿）。

> **门禁（实测，冻结 revision 上 9/9 绿）**：① `cargo fmt --check --all` 0；② `cargo clippy --all --all-features --all-targets -- -D warnings` 0；③ `cargo test --all` 0；④ 语料差分 0（356/356）；⑤ IR 快照 0 漂移（216 条）；⑥ 义务账本 0（13 必填 / 13 全填 / 交接字段 0 未归零 / 登记字段 8）；⑦ 边界 + 扇出 0（`pub(crate)` 计数 166、fanout 0）；⑧ 文档三门禁 0（docs-truth 157=157、docs-refs 1145 文件 / 5978 token〔本收口小节落地后该计数为 6012，因本节新增了引用 token〕、docs-examples 258 块 0 失败）；⑨ 错误码生成器幂等 0（重跑 `python scripts/docs/gen-error-code-docs.py` 后 `git diff --quiet` 0）。另 `check-ir-verifier.sh` PASS。

> **直驱例外（`tests/` 下两处机械改动，逐项登记 file / 行数 / 理由）**：① **`tests/integration/backends.rs`，+4 / −3 行，3 处**——导入行 `use yaoxiang::middle::{ConstValue, Type};` 拆为 `use yaoxiang::frontend::core::MonoType;` + `use yaoxiang::middle::ConstValue;`（`ir::Type` 别名删除后 `Type` 不再从 `middle` 再导出）；`params` 的两个字面量 `Type::Int(0)` → `MonoType::Int(0)`；`return_type` 的 `Type::Int(0)` → `MonoType::Int(0)`。**理由** = 载体类型由 `ir::Type` 改为 `MonoType`，构造点必须同步；该文件**无类型断言，断言一行未动**。② **`tests/integration/codegen_extended.rs`，+25 / −11 行，1 处**（`test_ir_types`）——导入 `use yaoxiang::middle::Type as IrType;` 改为 `use yaoxiang::frontend::core::MonoType;`；五个类型字面量改 `MonoType`（`Int(64)` / `Float(64)` / `Bool` / `Void` / `make_string()`）；**断言只增不减**——原函数只构造不判等，现补 5 条 `assert_eq!` + 2 条 `assert_ne!`（均带自定义消息）。两处均**未删测试、未放宽断言**。

> **塌缩口径（8 类 `MonoType` 不再塌缩为 `Void`）**：改前 `From<MonoType> for IrType` 把 **8 类 `MonoType` 变体**（`Struct` / `Enum` / `Ref` / `TypeVar` / `TypeRef` / `Union` / `Intersection` / `AssocType`）**全部塌缩为 `IrType::Void`**，外加一条 `_ => IrType::Void` 兜底（任何非 `String` / `Bytes` / `Tuple` 的 `Generic` 亦塌成 `Void`）——`IrType::Void` 当时是**合法的类型值而非错误信号**，下游无法区分「确实返回 `Void`」与「返回类型在桥接时丢失了」。改后解码原样搬运，**这 8 类保留真实类型**。**为何无行为变化**：运行时不消费函数签名的类型信息（`src/backends/` 对 `.params` / `.return_type` 的引用数为 **0**，类型表只被克隆与按下标寻址），故「恢复真实类型」不改变任何可观察行为——语料差分 356/356 逐字节相同即为证据。**唯一的可观察差异是 REPL 的 `{:?}` 展示文本**（`src/repl/eval.rs:360` 打印 `func.return_type`、`:370` 打印 `module.type_table[…]`，此前显示塌缩后的类型、现在显示真实类型），该处**无断言**，方向与 03 §6.4「`dump_bytecode` 输出变化属 C3 允许范围」一致。

> **T1 门禁的预期告警（反向桥 A 9→0，须在 6.6 转硬前刷新基线）**：`tools/type-tables` 的 T1 报告按三类分列统计构造点，其中「反向桥 A」= `From<MonoType> for IrType`。6.2.2 删掉该 impl 后，**反向桥 A 由 9 变体 / 12 点变为 0 变体 / 0 点**——T1.4 分列实测：反向桥 A「变体 0 个 / 点 0 个」，反向桥 B（`mono_to_ast_type`）仍是「变体 8 个 / 点 9 个」。T1 报告自带的**冻结基线**（`tools/type-tables/src/baseline.rs` 的 `SCOUT_BASELINE_REV` = `bfe2f6ba366ff0c45f517a4bec8d7e87b0a59696`，6.0 勘察基线，已推送）在 T1.6 对账时**必然对反向桥 A 列打 ✗**——实测「**17 / 26 行一致**」，不一致的 9 行恰是反向桥 A 重建过的那 9 个变体（`Int` / `Float` / `Char` / `String` / `Bytes` / `Bool` / `Void` / `Tuple` / `Fn`），而**前向列与 AST→AST 列全部仍 `=`**。**这是预期差异而非回归**：基线冻结于 6.0（那时反向桥 A 尚在），代码在 6.2 后确实没有这些构造点了。T1 当前是 report-only（报告仍 exit 0、不拦构建），但**一旦 6.6 把它转硬成 `panic!`，这个 ✗ 就会把「已删除的构造点」当成退化报出来**——故**转硬前须先刷新基线口径**。已登记进 09 的 6.6 行「下一轮契约输入」。另：T1.3 前向零构造 = **13**、T1.3b 全局零构造 = **1**（仅 `Enum`），与 6.0 基线一致——**6.2 未改变 6.3 的删除名单**。

> **IR 侧残留的三处 `ast::Type` 载体（留 6.3 决定）**：`src/middle/core/ir.rs` 的 `Instruction::Cast { target_type: ast::Type }`（`:325`）、`Instruction::TypeTest { ty: ast::Type }`（`:330`）、`FunctionBody::TypeDecl { definition: ast::Type }`（`:651`）**仍持 `ast::Type`**。6.2 只删了 `ir::Type` 这个**再导出别名**、把三处改写成全名 `ast::Type`，**未改 IR 的形状**（这三处仍是语法层类型，IR 层仍留一处指向 parser 的类型引用）。三处如何处置（随 6.3 删变体一并改载体，还是另立一轮）**留给 6.3 的契约决定**；它们落在 IR 指令 / 函数体的类型标注上，改动会动 **IR 快照（216 条）与语料基线（356 行）**。已登记进 09 的 6.3 行「下一轮契约输入」。

> **6.1.2 遗留的 E4xxx 生成物欠账（已清偿）**：6.1.2 删掉 `src/frontend/core/types/eval/const_eval.rs` 里 `eval_builtin` 的 `"Uint"` 死臂时**改动了该文件的行数**，使 `E4012`（常量递归过深）的发射点由 `const_eval.rs:540` 平移到 `:546`；而错误码文档由生成器**位置敏感**地产出（`python scripts/docs/gen-error-code-docs.py`），生成物 `docs/src/reference/error-code/E4xxx.md` 里的发射点行号因此变成**陈旧值**——这正是 AGENTS.md 第 10 条⑤「改动诊断码或其发射点后必须重跑生成器」要防的情况。**现状**：该欠账与另两笔预存欠账一并按**用户裁决 A**在**一笔独立提交**（`:wrench: chore(meta): 清偿 6.1.2 遗留的三笔预存欠账`，2026-10-28）里清偿：① 重跑生成器（`E4xxx.md` 的发射点行号 540 → 546）；② 对 `tools/type-tables` 补 `cargo fmt`（`tables.rs` 三处超长 `push_str` 换行，9 / 3 行，**纯格式化无语义变化**——6.1.2 实现时本地 `cargo fmt` 未带 `--all` 才漏）；③ 打包指南中英两侧示例的返回类型由已删除的小写别名 `string` 改为规范名 `String`（`docs/src/guide/packaging.md` 与 `docs/src/en/guide/packaging.md`）。**处置后**：生成器幂等（重跑后 `git diff --quiet` 0）、`cargo fmt --check --all` 0、docs-examples 258 块 0 失败。

> **注记（须更正的一处措辞）**：该笔提交的说明把这处发射点写成「E4013」，**实为 `E4012`（常量递归过深）**——`E4013` 在错误码注册表（`src/util/diagnostic/codes/e4xxx.rs`）与生成物 `docs/src/reference/error-code/E4xxx.md` 中**均不存在**。复核命令：`grep -rn 'E4013' src/ docs/src/reference/ scripts/`（零命中）、`grep -n 'E4012' src/util/diagnostic/codes/e4xxx.rs docs/src/reference/error-code/E4xxx.md`。

> **过程注记（三条）**：① **t1 首轮被 `changedPaths` 拒收**——契约的 inScope 漏了 `tests/integration/backends.rs`（载体改型后该直驱测试文件必然要改，属 AGENTS.md 第 6 条契约规则②「机械必需面」），经**契约追认**后按同一提交重报并复测。② **t6 的 outOfScope 前缀遮蔽了 inScope 的精确路径**——该轮契约把 `tools/type-tables/src/tables.rs` 写进 inScope，却又在 outOfScope 列了前缀 `tools/`，校验器按 out-of-scope 优先判定该文件不可声明；**下一批的 outOfScope 须收窄**（写成 `tools/code-tables/` 之类，或删掉 `tools/`）。③ **本批未改 IR 形状**——`ir.rs` 三处 `ast::Type` 载体只改全名、不改形状（见上）。

> **遗留（转入下一轮契约输入，不在本批处理）**：① **T1 冻结基线须在 6.6 转硬前刷新**（反向桥 A 9→0 的对账差异，见上；已登记进 09 的 6.6 行）。② **`ir.rs` 三处 `ast::Type` 载体的处置留给 6.3**（删变体时须一并决定载体，改动会动 IR 快照与语料基线；已登记进 09 的 6.3 行）。③ **T1 的 `Void` 兜底豁免登记随 6.3 同步删除**（6.0 收口实录遗留①，仍在）。④ **T3 的比对对象待 6.6 裁决**（6.0 收口实录遗留②，仍在）。⑤ **`DateTime` 迁出**（P10 S11，issue [#513](https://github.com/ChenXu233/YaoXiang/issues/513)，6.1 收口实录遗留①，仍在）。

> **计数**：本批不新增二级/三级任务 ⇒ 09 顶部计数表不变（二级 **72** / 三级 **193**）。6.2 的 3 个三级任务为既有登记，勾选不改变计数。

## P6 6.3 死变体删除收口（6.3.1 / 6.3.2，2026-10-29）

> **触发**：P6 6.2 收口后，6.3 的两个三级任务待收口——6.3.1 删 `ast::Type` 的 13 个死变体并清掉全部匹配臂与构造点（含 `src/frontend/core/parser/statements/types.rs` 的 `Type::Void` 错误兜底）；6.3.2 **必须与 6.3.1 同批**重写反向桥 B（`mono_to_ast_type`）与两份 AST→AST 替换（10 点）。**两者必须同批**：变体一删，两座桥上的重建臂就不可编译；只改桥不改变体，回写形态会静默改变。

> **轮次**：本批 = 6.3 的两个三级任务（6.3.1 删变体 / 6.3.2 重写两座桥），工作单元依次为：勘察（改写面清单 + 规范名往返策略）→ 实现（6.3.1 + 6.3.2 一笔提交）→ 独立验证 → 对抗性复审 → 契约追认（错误码生成物随删随清）→ 本收口。

> **判据**：C3（诊断码 / 消息 / 顺序不变）+ 03 §6.4 的硬验收（`git diff --stat tests/yaoxiang tests/baselines src/std/` 必须为空）。另加本批自身的两条载体判据：① **变体零残留**——13 个旧变体名在 `src/` 内零引用（含 `AstType` / `T` 两个 `use` 别名）；② **回写形态唯一**——反向桥 B 的产物必须是 `from_builtin_name` 认得的**规范名**（宽度显式），不得再落回诊断展示用的小写内部名。

### 结论：`ast::Type` 26 → 13

**删掉的 13 个变体**：`Int` / `Float` / `Char` / `String` / `Bytes` / `Bool` / `Void` / `Union` / `Enum` / `Option` / `Result` / `Sum` / `AssocType`。

**现存的 13 个变体**：`Name` / `Struct` / `NamedStruct` / `Tuple` / `Fn` / `Generic` / `Literal` / `Ptr` / `Ref` / `MetaType` / `ConstExpr` / `Paren` / `NamedParen`。

**零残留**：13 个旧变体名在 `src/` 内**零引用**，含两个 `use` 别名（`AstType` ← 2 文件、`T` ← 1 文件，即 `src/frontend/core/typecheck/checker/annotations.rs`）。复核命令：`git grep -n -E '\b(Type|AstType|T)::(Int|Float|Char|String|Bytes|Bool|Void|Union|Enum|Option|Result|Sum|AssocType)\b' -- src/`（零命中，`git grep` exit 1）。全仓其余命中只在 `docs/`（历史叙述）与 `tools/type-tables/`（门禁自测夹具里的模式字符串），**不是 `ast::Type` 引用**。

**改写面（17 个 match 块，全部同批处置）**：删变体后，**穷尽 match**（无 `_` 兜底）由编译器强制补臂（不补即编译失败）；**带 `_` 兜底的 match** 则不报编译错、通配臂会把新形态静默吃掉——故两类都必须显式处置。

| 兜底 | 站点 | 删前命中 | 处置 |
| --- | --- | --- | --- |
| 穷尽（编译期强制） | `format_type`（`src/formatter/handlers/types.rs`） | 13 臂 | 删 13 臂，内建标量一律走 `Type::Name` |
| 穷尽（编译期强制） | `From<ast::Type> for MonoType`（`src/frontend/core/types/mono.rs`） | 13 臂 | 删 13 臂 |
| 穷尽（编译期强制） | `collect_type_tokens`（`src/frontend/core/typecheck/checker/semantic_tokens.rs`） | 13 臂 | 删 13 臂 |
| 穷尽（编译期强制） | `substitute_type_ast`（`src/middle/passes/mono/function.rs`） | 10 点 | 删 10 点（见下表） |
| `_` 兜底（静默面） | `collect_name_refs`（`src/frontend/core/parser/ast.rs`） | `Union` / `Sum` / `Option` / `Result` / `AssocType` | 删臂（`Tuple` / `Ptr` 臂去掉 or-pattern 里的旧名） |
| `_` 兜底（静默面） | `name_used_as_type_in`（`src/frontend/core/parser/ast.rs`） | `Option` / `Result` / `Sum` | 删臂 |
| `_` 兜底（静默面） | `name_used_as_type`（`src/frontend/core/parser/statements/declarations.rs`） | `Option` / `Result` / `Sum` | 删臂 |
| `_` 兜底（静默面） | `extract_fn_type_info`（`src/frontend/core/parser/statements/types.rs`） | `Void` 兜底 | `_ => (Vec::new(), Type::Void)` 改 let-else 显式返回 `Type::Name{"()"}` |
| `_` 兜底（静默面） | `detect_sum_type` 的 `match ret`（`src/frontend/core/typecheck/checker.rs`） | `Result` / `Option` | 删臂 |
| `_` 兜底（静默面） | `collect_used_in_type`（`src/frontend/core/typecheck/checker.rs`） | `Option` / `Result` / `Sum` | 删臂 |
| `_` 兜底（静默面） | `collect_unknown_type_names`（`src/frontend/core/typecheck/checker/annotations.rs`，别名 `T`） | `Sum` / `Option` / `Result` / `AssocType` / `Union` | 删臂 |
| `_` 兜底（静默面） | `const_param_kind`（`src/frontend/core/typecheck/const_param.rs`） | `Int` / `Float` / `Bool` | 删臂 |
| `_` 兜底（静默面） | `extract_const_param_info`（`src/frontend/core/types/eval/const_eval.rs`） | `Int` / `Float` / `Char` / `String` / `Bytes` / `Bool` / `Void` | 删臂 |
| `_` 兜底（静默面） | `satisfies_ast_type`（`src/frontend/core/types/trait_data.rs`） | `Int`…`Void` / `Option` / `Result` | 删臂 |
| `_` 兜底（静默面） | `type_from_ast`（`src/middle/passes/codegen/mod.rs`） | `Int`…`Void` / `Option` / `Result` | 删臂 |
| `_` 兜底（静默面） | `mono_to_ast_type`（反向桥 B，`src/middle/passes/mono/function.rs`） | 7 个内建标量臂 | 改写为 `canonical_name` + 结构回写（见下节） |
| `_` 兜底（静默面） | `substitute_type_in_ast`（`src/middle/passes/mono/function.rs`） | 10 点 | 删 10 点，**保留原 `_ => ty.clone()`**（不新增兜底） |

**未新增任何通配臂**：逐文件 `_` 臂计数改前与改后逐字一致，唯 `src/frontend/core/types/mono.rs` +1 = 新增 `canonical_name` 的 `_ => None`（`MonoType` 查表语义，**不是** `ast::Type` 构造兜底）。

### 规范名往返口径（为何不能用 `type_name()`）

反向桥 B 的职责是把 `MonoType` 回写成 `ast::Type`。6.3 之前它对 7 个内建标量各有一条专用臂（`Int` / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes`）；变体删除后这些臂没有了，**回写必须落到 `ast::Type` 的某个变体上**——只剩 `Name` 可用。

**为何不能用 `type_name()`**：`MonoType::type_name()` 是**诊断展示用**的内部名（`int64` / `void` / `string` / `float64` …）。6.1.2 已把 22 个实现内拼写从 `from_builtin_name` 删除（含全部小写名），因此按 `type_name()` 回写出的 `Name{"int64"}` 在下游会被解析成**用户类型引用**（`TypeRef`）而不是内置标量——**类型身份静默丢失**（`Int64` 变成某个未声明的用户类型）。这不是格式差异，是语义降级。

**采用的口径**：新增 `MonoType::canonical_name()`（`src/frontend/core/types/mono.rs`），作为 `from_builtin_name` 的**严格逆向**——

- **覆盖范围 = `from_builtin_name` 的像**：15 个规范名对应的 `MonoType`；实现为 12 条臂（`Never` / `Void` / `Bool` / `Int8` / `Int16` / `Int32` / `Int64` / `Float32` / `Float64` / `Char` / `String` / `Bytes`）。
- **宽度显式**：`Int` / `Float` 是**入参别名**（`from_builtin_name("Int")` → `Int(64)`、`"Float"` → `Float(64)`），回写恒取宽度显式形式 `Int64` / `Float64`，保证产物唯一；`()` 恒回写为 `Void`。
- **`DateTime` 只入不出**：它是 std 层语义别名（`from_builtin_name("DateTime")` → `Int(64)`），`canonical_name` 永不产出（用例以 `assert_ne!` 钉死）。
- **返回 `None` = 该类型没有规范名**（容器泛型 / `Tuple` / `Ref` / `Fn` / `Struct` / `TypeVar` …），调用方须按**结构**回写。

**回写结构**：`mono_to_ast_type` 改为「先 `canonical_name`，再按结构回写」——`Generic { name, args }`（覆盖 `Option` / `Result` / `Tuple` / `List` / 任意泛型实例，逐字往返）、`Ref`、`Fn`；其余（`Struct` / `Enum` / `TypeVar` / `Literal` / `Union` / …）保持既有有损兜底 `Name { type_name() }`，**不扩面**（6.3 只删死变体，不改这些类型的回写形态）。**未改 `type_name()` 本身**。

**往返覆盖**：① **leg A（15 名逐名往返）**——`from_builtin_name` → `canonical_name` → `from_builtin_name` 还原同一 `MonoType`，由仓内用例 `src/frontend/core/types/tests/mono.rs` 的 `test_canonical_name_round_trips_all_builtin_names` 覆盖（15 名全列）；另有 `test_canonical_name_uses_width_explicit_spellings`（宽度显式）与 `test_canonical_name_rejects_structural_types`（结构类型返回 `None`）两条。② **leg B（`mono_to_ast_type` 产物再 lower）**——回写出的 `ast::Type` 再经 `From<ast::Type> for MonoType` 还原，与原 `MonoType` 全等，见「残留风险」②。

### 10 点处置表（AST→AST 重建）

03 §2.2.2 登记的 AST→AST 重建在 6.3 名单内有 **5 变体 / 10 点**（`Union` / `Option` / `Result` / `Sum` / `AssocType` 各 2）。**全部删除**——变体已不存在，臂不可达。

下表行号为**删前**位置，取自 6.3 的父提交（`:pencil: docs: auto-translate documentation`，2026-10-28；该 revision 亦记在 `tools/type-tables/src/baseline.rs` 的 `SCOUT_BASELINE_REV`）。变体删除后行号平移，故按 09 §6.3.2 的口径**以函数名定位、不在散文里写死行号**。

| 变体 | `substitute_type_ast` | `substitute_type_in_ast` | 处置 |
| --- | --- | --- | --- |
| `Union` | 377 | 606 | 删臂 |
| `Option` | 427 | 600 | 删臂 |
| `Result` | 431 | 602 | 删臂 |
| `AssocType` | 456 | 623 | 删臂 |
| `Sum` | 467 | 632 | 删臂 |

**两处替换函数的兜底口径不同**：`substitute_type_ast` 对 13 个现存变体**穷尽匹配、无 `_` 兜底**（删变体由编译器强制补臂，不存在「静默吃掉」的可能）；`substitute_type_in_ast` 保留其原有的 `_ => ty.clone()`——**本批未新增任何兜底**，删臂后「不再递归」也无行为差异（该节点已不可能出现）。

**下游可达性（删臂不改可观测行为的依据）**：① `substitute_type_ast` 仅由 `substitute_instruction` 的 `Cast` / `TypeTest` 调用，而 `ir_gen` 从不构造 `Instruction::Cast` / `Instruction::TypeTest`（全仓只有 IR 定义、mono 改写与 normalize / verify 消费）；② `substitute_type_in_ast` 仅由 `specialize_type`（`src/middle/passes/mono/mod.rs`）调用，语料 393 个 `.yx` 中 `类型名(...)` 特化 `TypeDecl` **0 例**。故 10 点当前**无生产可达路径**。

### T1 门禁清账

6.0 收口时登记在 T1 门禁上的死变体名单与兜底豁免，随 6.3 的删除**同批清账**（否则门禁立即报 stale）：

| 清账项 | 处置 |
| --- | --- |
| `VOID_FALLBACK`（`tools/type-tables/src/t1.rs` 的 `FallbackExemption` 常量） | **删除**——`Void` 变体已不存在，兜底点改为 `Type::Name{"()"}`（不再是通配臂构造点） |
| `REGISTERED_ZERO_FORWARD`（13 条 `RegisteredZeroVariant`） | **清空为 `&[]`**——13 个死变体已删，实测前向零构造为 0 |
| `DEAD_VARIANTS_03`（03 §5.2 的 13 个死变体名单） | **清空为 `&[]`** |
| `SCOUT_BASELINE` / `SCOUT_BASELINE_REV`（`tools/type-tables/src/baseline.rs`） | **刷新**——基线由 26 行收敛为现存 13 变体行，冻结 revision 换到 6.3 的父提交 |
| `tools/type-tables/README.md` 与 `t1.rs` 的口径文案 | 同步改为「6.3 之后登记表为空；机制保留且可注入（`*_with(registry)`）供单测覆盖」 |

**清账后的 `--report` 实测**（`cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`，exit 0）：

| 项 | 实测值 |
| --- | --- |
| T1.1 扫描面 | 非测试 `.rs` **287** 个；`ast::Type` 变体 **13**（定义行 445–528）；别名解析 2 个（`AstType` ← 2 文件、`T` ← 1 文件） |
| T1.3 前向零构造 | **0** |
| T1.3b 全局零构造 | **0** |
| T1.4 反向桥 A | 变体 **0** 个 / 点 **0** 个 |
| T1.4 反向桥 B | 变体 **4** 个 / 点 **6** 个（`Name` 3 点 / `Generic` / `Ref` / `Fn` 各 1 点） |
| T1.5 AST→AST | 变体 **10** 个 / 点 **18** 个（**剩余**重建点）；其中「WBS 6.3 删除名单」= 0 个变体 / 0 点 |
| T1.6 基线对账 | **13 / 13 行一致**（扫描口径与冻结基线完全一致） |
| T1.7 白名单核对 | 已登记 0 / 实测前向零构造 0 / 未登记 0 / 登记但已不成立 0 / 登记豁免点 0 条（命中 0）/ 未登记的通配臂兜底点 0 → **名单一致** |

### 实测数字

① **硬验收**——`git diff --stat -- tests/yaoxiang tests/baselines src/std/` 在 6.3 的父提交与落地态之间**为空**（语料层 / 基线层 / 标准库零差异）；`tests/` 侧同样零差异（见「直驱例外」）。

② **语料差分 356/356、差异为空**——基线 `tests/baselines/corpus-parity.jsonl` 共 356 行；`cargo run --example corpus_probe` 的产物与基线逐字节相同，`check-corpus-parity.py` exit 0。

③ **IR 快照 216 条零漂移**——`src/middle/core/tests/snapshots/` 下 216 个 `.snap`，全部在库未改。

④ **dump 对拍 8/8 md5 逐字一致**——4 个泛型替换程序的 dump（`77dd7cf4` / `9131dd36` / `82f43780` + 对照 `c2a2de8d`）+ 4 件 `.42` 产物（`8ce5007f` / `99f2dce8` / `67cb94f4` / `41b22441`），与改动前逐字相同。

⑤ **`cargo test --all` exit 0**——lib **2865 → 2868 passed**（+3 = 三条 `canonical_name` 用例）/ 0 failed / 2 ignored；integration **314** passed；其余测试二进制全绿；全仓 `#[test]` **3394 → 3397**（+3，**未删测试**）。

⑥ **变异验证（判据可失败性）**——把 `canonical_name` 的 12 条臂小写化（生产变异），往返用例转红（exit 101）且全量 lib 4 failed；`git apply -R` 机械还原后该文件 blob 与 HEAD 逐字一致、`git status` CLEAN。

> **提交（本地 dev，未推送）**：`:recycle: refactor(parser): ast::Type 26→13 变体并重写两座类型桥`（2026-10-28，28 files / +677 / −818）；本收口文档另起一笔 `:memo: docs(plan): 6.3 死变体删除收口`（2026-10-29，3 files）。按仓库约定提交号在 push 之后取（`pull --rebase` 会重写本地哈希），故此处只记提交主题与日期。

### 门禁（实测，全部 exit 0）

| # | 门禁 | 实测 |
| --- | --- | --- |
| ① | `cargo fmt --check --all` | exit 0 |
| ② | `cargo clippy --all --all-features --all-targets -- -D warnings` | exit 0 |
| ③ | `cargo test --all` | exit 0（lib 2868 passed / 0 failed / 2 ignored；integration 314 passed） |
| ④ | 语料差分 | 356 / 356，差异为空（`check-corpus-parity.py` exit 0） |
| ⑤ | IR 快照 | 216 条零漂移（`check-snapshot-drift.sh` PASS） |
| ⑥ | 义务账本 | PASS（13 必填 / 13 全填 / 红名单 0 / 非义务登记 8）+ 自测 5 项全 PASS |
| ⑦ | 边界 + 扇出 + 概念 | 违规 0（`pub(crate)` 计数 **166**）；`check-boundary` / `check-fanout` / `check-concepts` 全 exit 0 |
| ⑧ | 文档门禁 | `check-docs-truth` 诊断码 **157 / 157**；`check-docs-orphan` 文档站 md 176 篇 / 索引覆盖 173 篇；`check-docs-refs` 1145 个跟踪文本文件、6076 个路径状 token、无不可解析引用；`check-docs-examples` 抽取 258 个代码块、失败 0 |
| ⑨ | 错误码生成器幂等 | 重跑 `python scripts/docs/gen-error-code-docs.py` 后 `git diff --quiet` exit 0 |

另：`check-ir-verifier.sh` PASS（全语料 `verify_loose` 绿，校验面非空）；`cd docs && npm run docs:build` exit 0（VitePress 真构建，AGENTS.md 第 10 条⑨）。

### 错误码生成物（随删随清）

`src/frontend/core/typecheck/checker.rs` 删臂使该文件行数平移，两个精化类诊断码的发射点随之漂移；错误码文档由生成器**位置敏感**地产出，故必须重跑生成器并随提交再生（AGENTS.md 第 10 条⑤ 的机械必需面）：

| 诊断码 | 删前发射点 | 删后发射点 |
| --- | --- | --- |
| `E1092`（精化类型实参形态非法） | `checker.rs:3618` / `:3666` / `:3694` | `checker.rs:3603` / `:3651` / `:3679` |
| `E1093`（精化实参个数不匹配） | `checker.rs:3601` / `:3691` | `checker.rs:3586` / `:3676` |

再生范围 = `docs/src/reference/error-code/E1xxx.md` **2 行**（两个发射点行）；生成器复跑幂等。

### 直驱例外

**无**。`tests/yaoxiang` / `tests/baselines` / `src/std` 三段零差异，`tests/` 侧同样零差异；全仓 `#[test]` 由 3394 增至 3397，变化全部落在 `src/` 内的 10 个测试模块（`src/frontend/core/types/tests/mono.rs` 等）——**未删测试、未放宽断言**。

### 残留风险（转入下一轮契约输入）

① **`Name{"()"}` 兜底语法可达 + `semantic_tokens` 的 dummy token**——`src/frontend/core/parser/statements/types.rs` 的非 Fn 签名兜底现为 let-else 显式返回 `Type::Name{"()"}`（formatter 输出逐字不变）；该路径**语法可达**（非 Fn 类型出现在绑定位置），`semantic_tokens` 的 `Name` 臂会比旧的 `Void` 臂（空操作）多记一枚 dummy-span token（`add_token` 无 span 过滤）。语料 393 个 `.yx` 中该形态 **0 命中**，formatter / 诊断 / dump **零漂移**。**随 6.4 的错误路径改造一并消除**（已登记进 09 的 6.4 行「下一轮契约输入」）。

② **leg B 往返覆盖未入库**——反向桥 B（`mono_to_ast_type`）的产物再 `From` 回 `MonoType` 的**全链往返**，目前只有**验证树内**的探针测试覆盖（随验证树清理一并删除、未入库）；仓内只覆盖 `Int64` 单点。**转硬前须另立实现侧小任务把该往返测试正式入库**（已登记进 09 的 6.6 行「下一轮契约输入」）。

③ **值位 `.yx` 探针的 `E1002`**——值位探针在 6.3 的父提交与落地态**同报 `E1002`**，诊断逐字节一致 ⇒ 属**预存语言限制**（非本批引入）；该形态的往返覆盖因此改用 IR 层探针完成（15 个规范名 + 结构类型）。

④ **`ir.rs` 三处 `ast::Type` 载体未动**——6.3 只删变体、**未改 IR 形状**（IR 快照 216 条与语料基线 356 行零漂移）；`Instruction::Cast { target_type }`（`src/middle/core/ir.rs:325`）、`Instruction::TypeTest { ty }`（同文件 `:330`）、`FunctionBody::TypeDecl { definition }`（同文件 `:651`）仍持 `ast::Type`。载体处置留 6.4（改动会动 IR 快照与语料基线），已登记进 09 的 6.4 行「下一轮契约输入」。

### 过程注记（三条）

① **实现轮的契约把 `docs/` 划进 outOfScope，漏了生成物**——错误码生成物 `docs/src/reference/error-code/E1xxx.md` 随 `checker.rs` 删臂的行号平移而必须再生，属 AGENTS.md 第 6 条契约规则①「机械必需面」；经**契约追认**后并入同一提交（生成器复跑幂等）。

② **勘察清单漏了别名 `T`**——`src/frontend/core/typecheck/checker/annotations.rs` 用 `use …::ast::Type as T`，其 5 处 `T::` 命中不在勘察阶段的「201 行」清单内；实现轮已一并处置。**后续契约的「死变体清单」口径须含别名扫描**（T1 门禁的别名解析早已登记「`T` ← 1 文件」）。

③ **提交 amend 时改写了正文**——实现轮在 amend 时删掉了正文中「未纳入本提交」这类与事实不符的表述。

### 计数

本批不新增二级/三级任务 ⇒ 09 顶部计数表不变（二级 **72** / 三级 **193**）。6.3 的 2 个三级任务为既有登记，勾选不改变计数。

## P6 6.4 parser 数据流收口（6.4.1–6.4.5，2026-10-29）

> **触发**：P6 6.3 收口后，6.4 的五个三级任务待收口——6.4.1 新增 `probe.rs`（类型环境探针的**接口**定义在 L2）、6.4.2 改 `ParserState`（增 `probe` 字段与访问器）、6.4.3 删 `operator_interfaces::spec()` 调用（parser 侧 2 处 + middle 侧 1 处）、6.4.4 合并两份 `name_used_as_type*`、6.4.5 让 `CONST_PARAM_TYPES` 经 `NameKind::Builtin` 接线。

> **轮次**：勘察（改写面清单）→ 实现（6.4.1–6.4.5 一笔提交）→ 独立验证 → 对抗性复审 → **静态审计抓到真回归** → 修复轮 → 二次验证 → 二次复审 → 契约追认 → 本收口。

> **判据**：C3（诊断码 / 消息 / 顺序不变）+ 03 §6.4 的硬验收（`git diff --stat tests/yaoxiang tests/baselines src/std/` 必须为空）。另加本批自身的两条载体判据：① **parser 内零类型层引用**——`grep -rn 'typecheck' src/frontend/core/parser/` 与 `grep -rn 'operator_interfaces' src/frontend/core/parser/` 均须 0 命中；② **判据可失败性**——三条变异试验各自使判据转红并机械还原。

### 结论：parser 的类型名依赖改为注入式探针

parser（L2）此前**直接调用**类型检查侧（L3）的运算符接口表来判定「约束名 / 谓词名」，构成 L2→L3 反向依赖（`scripts/ci/check-boundary.py` 的 B2 库存 2 处）。本轮按 03 §5.4 改为**注入式探针**：接口定义在 L2，实现由 L3 提供——依赖倒置。

| 产物 | 位置 | 内容 |
| --- | --- | --- |
| 探针接口 | `src/frontend/core/parser/probe.rs` | `trait TypeEnvProbe`（`is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types` 四个纯查询方法，全部带默认实现）、`enum NameKind`（`Builtin` / `Constraint` / `Predicate` / `UserType`）、`NullProbe`（全 false）、`BuiltinProbe`（编译器内置封闭集） |
| L3 实现 | `src/frontend/core/typecheck/probe.rs` | `SemanticProbe`（以运算符接口表 / 内置谓词判定 / `MonoType::from_builtin_name` 为权威来源）+ 两个窄入口 `is_constraint_name`（`:33`）/ `is_builtin_predicate_name`（`:25`） |
| 注入点 | `src/frontend/core/parser/parser_state.rs` | `ParserState` 增 `probe` 字段（`:27`）+ `probe()`（`:60`）+ `kind_of()`（`:68`）两个访问器；构造器 `ParserState::new(tokens, probe)`（`:44`）显式收探针 |
| 入口 | `src/frontend/core/parser/mod.rs` | `parse`（`:35`）= `parse_with_probe(tokens, &BuiltinProbe)`；`parse_expression`（`:94`）= `parse_expression_with_probe(tokens, &BuiltinProbe)`；注入缝 `parse_with_probe`（`:40`）/ `parse_expression_with_probe`（`:99`） |

**其余五项改动**：

1. parser 内两处 `operator_interfaces::spec()` 调用**删除**，改走探针（`probe.is_constraint(name)`）。
2. parser 内 8 处提及 typecheck 的注释按事实改写（跨层描述改为「L3 的类型检查」等表述）。
3. 两份 `name_used_as_type*` 合并为 `src/frontend/core/parser/ast.rs:848` 的**单一实现**（详见下节差异表）。
4. `CONST_PARAM_TYPES` **15 → 5 名**（删 10 个 6.1 已废的实现内拼写），并经 `NameKind::Builtin` 接线（详见下节）。
5. `Terminates` 的逐点字符串比较收敛为单一判定口：`typecheck/` 内 9 个判定点改调 `is_builtin_predicate_name`（`checker.rs` ×3、`checker/annotations.rs` ×2、`checker/refinement.rs` ×1、`environment.rs` ×1、`inference/statements.rs` ×2）；该目录内 `"Terminates"` 字面量命中由 **16 行降为 8 行**（新增的 1 行是判定表 `BUILTIN_PREDICATES`，其余 7 行是诊断文案、既有常量 `TERMINATES_PREDICATE` 与 2 个未归一的判定点 `checker.rs:4128` / `inference/statements.rs:1868`）。

### probe 设计（trait / 三个实现 / 接入点）

**trait 的四个方法**（`src/frontend/core/parser/probe.rs:45`）：

| 方法 | 语义 | 默认实现 | L3 语义探针（`SemanticProbe`） |
| --- | --- | --- | --- |
| `is_constraint` | 该名字是否为**约束名**（运算符接口，`(T: Add)` 的 `Add`） | `false` | `typecheck::is_constraint_name` → 运算符接口表 |
| `is_predicate` | 该名字是否为**编译期谓词**（`Terminates`） | `false` | `typecheck::is_builtin_predicate_name` → 内置谓词表 |
| `kind_of` | 名字定种（`NameKind`） | `NameKind::UserType` | `MonoType::from_builtin_name` 命中 → `Builtin`；否则 `is_constraint_name` → `Constraint`；否则 `is_builtin_predicate_name` → `Predicate`；其余 `UserType` |
| `knows_builtin_types` | 本探针是否**持有内置类型名表** | `false`（保守） | `true` |

**三个实现**：

- `NullProbe`（`parser/probe.rs:93`）——全 false；单元测试与「无类型环境」入口的基线。
- `BuiltinProbe`（`parser/probe.rs:113`）——编译器**内置封闭集**的镜像：约束名 7 个（`CONSTRAINT_NAMES`，`:116`）+ 内置谓词名 1 个（`PREDICATE_NAMES`，`:121`）。**不是第二份权威表**：7 个约束名与 L3 运算符接口表逐名对账、内置谓词名与 L3 的内置谓词判定双向对账，由 L3 侧的对账用例守着（`typecheck/tests/probe.rs` 两条用例）。
- `SemanticProbe`（`typecheck/probe.rs:41`）——L3 实现，权威来源 = 运算符接口表 / 内置谓词判定 / `MonoType::from_builtin_name`。

**接入点**：

- **字段**：`ParserState.probe: &'a dyn TypeEnvProbe`（`parser_state.rs:27`）——借用而非 `Box`，不引入所有权与生命周期外的额外约束。
- **访问器**：`probe()`（`:60`）返回探针本身；`kind_of(name)`（`:68`）= **本遍已声明的谓词优先**（`predicate_names`）→ `NameKind::Predicate`，其余交探针——**定种的唯一入口**（`probe.kind_of` 不再被 parser 直接调用）。
- **构造点 40 处** = 生产 **4** + 测试 **36**：

| 类别 | 处数 | 位置 |
| --- | --- | --- |
| 生产 | 4 | `parser/mod.rs:44`（`parse_with_probe` 透传注入的探针）、`parser/mod.rs:103`（`parse_expression_with_probe` 同）、`parser/pratt/nud.rs:450`（f-string 插值重解析，透传外层探针）、`parser/statements/bindings.rs:107`（`parse_binding` 传 `&NullProbe`——该入口不接收环境） |
| 测试 | 36 | `pratt/tests/led.rs` ×9、`pratt/tests/nud.rs` ×23、`statements/tests/types.rs` ×3、`tests/parser_state.rs` ×1 |

> `parse_binding`（`parser/statements/bindings.rs:97`）虽在生产模块内，但**全仓唯一调用方是测试**（`parser/statements/tests/bindings.rs:310`）⇒ 其 `NullProbe` 构造点无生产影响。

### spec() 处置表

改前全仓 `operator_interfaces::spec()`（含别名形式 `ops::spec(`）的**调用点共 14 处**（不含 `operator_interfaces.rs:253` 的模块内自用与 `driver/helpers.rs:1138` 的文档注释提及）。本批逐处去向：

| # | 改前位置 | 层 | 本批处置 | 替代判据 |
| --- | --- | --- | --- | --- |
| 1 | `src/frontend/core/parser/ast.rs:923`（`extract_generic_param_names`） | L2→L3 | **删除调用**，改走探针 | `probe.is_constraint(name)`（`parser/probe.rs:49`） |
| 2 | `src/frontend/core/parser/statements/declarations.rs:508` | L2→L3 | **删除调用**，改走探针 | `state.probe().is_constraint(n)`（现 `declarations.rs:461`） |
| 3 | `src/middle/core/ir_gen.rs:696`（`is_constraint_param_ann`） | L3→L3 | 改调窄入口 | `typecheck::is_constraint_name(name)`（现 `ir_gen.rs:697`） |
| 4 | `src/driver/helpers.rs:1150`（`generic_instantiation_map` 的 is_trait 闭包） | L1→L3 | 改调窄入口 | `typecheck::is_constraint_name(name)`（现 `helpers.rs:1151`） |
| 5–9 | `src/frontend/core/typecheck/checker.rs` ×5（改前 `:1525` / `:2228` / `:2341` / `:2451` / `:2870`） | L3→L3 | **本批未动** | 仍是 L3 内部直接查表（合法） |
| 10 | `src/frontend/core/typecheck/inference/expressions.rs:1585`（`ops::spec(iface)` 别名形式） | L3→L3 | **本批未动** | 同上 |
| 11–14 | `src/frontend/core/typecheck/inference/statements.rs` ×4（改前 `:1443` / `:1781` / `:1801` / `:2932`） | L3→L3 | **本批未动** | 同上 |

**窄入口的等价性**：`is_constraint_name`（`typecheck/probe.rs:33`）的实现**逐字**是 `super::operator_interfaces::spec(name).is_some()` ⇒ 改调窄入口不改变任何判定结果，只把「摸表」收敛为「问一个名字是不是约束名」。

**新增的窄入口只增不减**：L3 侧的 `is_constraint_name` 是本批**新增**的（第 3、4 处改调它）；`is_builtin_predicate_name`（`:25`）把 9 处 `"Terminates"` 逐点比较收敛为内置谓词名的**单一判定口**。

### 「全仓 13 处」与实测数的对账

WBS 的 6.4.3 行写「**全仓 13 处**」——**该口径少算 1 处**：

| 分项 | WBS 计划口径 | 实测 | 差额 |
| --- | --- | --- | --- |
| parser 侧 | 2 | 2 | — |
| typecheck 侧 | 10 | 10 | — |
| middle 侧 | 1 | 1 | — |
| **driver 侧** | **0（未计）** | **1**（`src/driver/helpers.rs:1150`） | **+1** |
| 合计 | 13 | **14** | +1 |

**差额来源**：`src/driver/helpers.rs:1150` 的 `operator_interfaces::spec(name).is_some()`（L1 编排层 → L3 类型层）——原计划的 grep 口径只搜了 `operator_interfaces::spec(` 的全限定写法，漏了经 `use` 引入后的短名调用。**该处本批已一并处置**（改调窄入口），故落地态 parser 内 0 处、driver 1 处改窄入口、middle 1 处改窄入口、typecheck 10 处保持。

### name_used_as_type* 合并差异表

改前有两份**同名同义、行为已分叉**的实现（03 §2.4 登记）：

- `src/frontend/core/parser/ast.rs` 的 `name_used_as_type_in`（`pub`，无回调参数）
- `src/frontend/core/parser/statements/declarations.rs` 的 `name_used_as_type`（模块私有，带 `is_predicate` 回调）

**分叉三处 + 共同缺口**（下表即合并依据）：

| 分叉点 | ast.rs 版 | declarations.rs 版 | 合并后的**正本**（`ast.rs:848` 的 `name_used_as_type`） |
| --- | --- | --- | --- |
| `Generic` 的**头部**（`F(A)` 的 `F`） | 判头部是否被当类型用 | 不判 | **采 ast.rs 版**（头部判定） |
| **谓词短路**（`Terminates(b)` 的实参位是值） | 无 | 对整个 `Generic` 节点短路 `false` | **采 declarations.rs 版**（短路读 `NameKind::Predicate`） |
| `Struct` / `NamedStruct` 的字段类型递归 | 有 | 落 `_ => false` | **采 ast.rs 版**（递归） |
| `MetaType` / `ConstExpr` / `Paren` / `NamedParen` | 不处理 | 不处理 | **两版共同缺口，本批刻意不动** |

**合并方式**：以 ast.rs 版为正本（其头部判定与 Struct 递归较完整），叠加上 declarations.rs 版的**谓词短路**；短路判据由「字符串回调」改为**读 `NameKind::Predicate`**（探针问答，非名字表）。declarations.rs 侧那份实现**删除**，改为调用 ast.rs 的合并版（`declarations.rs:466`）；L3 侧 `checker/annotations.rs` 也改调同一实现（`:87` / `:90` / `:93`），并按 L3 原口径传恒 `false` 的谓词闭包（`annotations.rs:69-93`）。

**合并后的判定面扩大一处**（A 项）：类型位 lambda 形参的判定由 `[F, g]` 变为 `[g]`——合并版把 `Generic` 头部也算类型引用（这正是 ast.rs 版原有的更完整语义）。**对外零差异**：5 个 `.yx` 程序的 CLI 诊断在两个 revision 上逐字相同（判定面扩大不产生新的对外诊断）。

**为何不补共同缺口**：补 `MetaType` / `ConstExpr` / `Paren` / `NamedParen` 会改变「const 参数是否被当类型用」的判定（进而改 IR 的 `generic_params` 与语料行为），属**语义修正**，须按 07 的分流判据另立一轮，不在 6.4 的 C3 面内。缺口已登记在 03 §5.3 与本节「残留风险」。

### CONST_PARAM_TYPES 接线

| 项 | 改前 | 改后 |
| --- | --- | --- |
| 常量名 | `CONST_PARAM_TYPES`（`ast.rs`） | 同名保留（**不新增概念**） |
| 表内容 | 15 名：`Int` / `Bool` / `Float` / `Char` / `String` + `I8` / `I16` / `I32` / `I64` / `U8` / `U16` / `U32` / `U64` / `F32` / `F64` | **5 名**：`Int` / `Bool` / `Float` / `Char` / `String`（`ast.rs:825`） |
| 删掉的 10 名 | 6.1.2 已删除的实现内拼写 | 留在表里会产生**无法 lower** 的 const 参数名（`Uint` 族在 `MonoType` 里没有变体，位宽缩写也不是内置名） |
| 判定方式 | 直接查本表字符串 | **双知识接线**：`CONST_PARAM_TYPES.contains(name) && (probe.kind_of(name) == NameKind::Builtin \|\| !probe.knows_builtin_types())`（`ast.rs:961-964`） |

**双知识的含义**：名字必须既在「const 参数可接受集合」内，又由探针确认为**内置名**（权威表在 L3 的 `MonoType::from_builtin_name`）。只查本表会绕开探针的定种；只查探针则会把 `Int8` 等规范名也当 const 参数候选（**拓宽登记 = 行为变更**）。规范名（`Int8` …）自始不在 const 参数的可接受集合里，**不进本表**。

**`knows_builtin_types` 的存在理由**：L2 的默认探针（`BuiltinProbe` / `NullProbe`）拿不到内置名表，`kind_of` 永不返回 `Builtin`——若不放行，默认 `parse()` 入口对 `(N: Int)` 会**静默漏登记**。`knows_builtin_types() == false` 时按本表单表判定（= 6.4 前行为），L3 语义探针持有名表（覆写为 `true`），双知识仍全强度生效。

**连带清账**：`tools/type-tables` 的 T2 门禁 dispositions 表删 **10 条 `T2.C1`** 登记（这 10 个拼写已在 6.1.2 清账，本批把 const 参数表的同 10 名一并删除 ⇒ 登记随之兑现）；清账后该表只剩 `DateTime` 的一条 `T2.C2`。

### ir_gen 那处「跨层」的处置（层号更正）

WBS 的 6.4.3 行把 `src/middle/core/ir_gen.rs` 的 `is_constraint_param_ann` 标为「**L3→L2 跨层反向依赖**」——**该层号标注有误**。

- `scripts/ci/check-boundary.py` 的 `L3_DIRS` **同时含 `typecheck` 与 `middle`** ⇒ middle → typecheck 是 **L3→L3 同层**，RFC-039 四层模型下不构成反向依赖；实跑该门禁**违规 0 处**，CI 不旗标。
- 该处本批的处置是**改调窄入口** `typecheck::is_constraint_name`（`ir_gen.rs:697`），并让 3 处 `extract_generic_param_names` 调用传 `SemanticProbe`（否则 `(T: Add)` 不登记 → E3018）。

**门禁基线同步**：`scripts/ci/check-boundary.py` 的两张基线表随本批同步——B2（`L2→L3/L4` 反向依赖）库存清零；B6（parser 内硬编码类型/谓词名）由 `declarations.rs` 移到 `probe.rs`，库存仍 **2 处**（`ast.rs` 的 `CONST_PARAM_TYPES` 与 `probe.rs` 的 `PREDICATE_NAMES`）。豁免 **1 处**（L3 实现 L2 的判定接口，reason 已注明于 `typecheck/probe.rs:12-15`），违规 **0 处**，`pub(crate)` 计数 **165**。

### grep = 0 证据

改后（落地态）实测，**两条命令均 0 命中**（`grep` 无输出 = exit 1）：

| 命令 | 结果 |
| --- | --- |
| `grep -rn 'typecheck' src/frontend/core/parser/` | 无输出（exit 1） |
| `grep -rn 'operator_interfaces' src/frontend/core/parser/` | 无输出（exit 1） |

**对照改前**：同一口径在 6.4 的父提交上命中 **10 处**（`ast.rs` 6、`pratt/nud.rs` 2、`statements/declarations.rs` 2——其中 **2 处代码调用 + 8 处注释**）。改前 parser 内的全部 `use crate::` 路径中亦含 typecheck 引用；改后枚举 parser 内全部 `use crate::` 路径，**无 typecheck / middle 引用**。

### 回归与修复轮（首版真回归 → 方案 B）

**首版（6.4.1–6.4.5 一笔提交）的真回归**：const 参数的 guard 要求 `probe.kind_of(name) == NameKind::Builtin`，而 L2 的默认探针（`BuiltinProbe` / `NullProbe`）**不持有内置类型名表**（`MonoType::from_builtin_name` 在 L3），`kind_of` 永不返回 `Builtin` ⇒ 默认 `parse()` 入口对 `(N: Int)` 这类 const 参数**静默漏登记**，与 6.4 之前不一致。

**修法（方案 B）**：`trait TypeEnvProbe` 增 `knows_builtin_types()`（默认 `false` = 保守）；const 参数的 guard 改为 `CONST_PARAM_TYPES.contains(name) && (kind_of == Builtin || !knows_builtin_types())`——**不持表的探针回退单表判据**（= 6.4 前行为），L3 的 `SemanticProbe` 覆写为 `true`，双知识在全强度生效。

**未选方案 A**（让 `BuiltinProbe.kind_of` 对内置名返回 `Builtin`）：L2 没有内置名表，A 只能拿 `CONST_PARAM_TYPES` 反推 `Builtin`——等于把 const 可接受集合冒充内置名全集（`Int8` / `Bool` 仍会是 `UserType`），在 L2 造第二份半个内置名源，且 `kind_of` 的语义继续失真。

**先红后绿证据**：新增回归用例在修复前**实测红**（期望 `["N"]` 实得 `[]`），修复后绿；新增双探针同源对账用例绿。新增用例 7 条（`parser/tests/probe.rs` 3 条 + `typecheck/tests/probe.rs` 4 条）。

### 实测数字

① **硬验收**——`git diff --stat -- tests/yaoxiang tests/baselines src/std/` 在 6.4 的父提交与落地态之间**为空**（语料层 / 基线层 / 标准库零差异）；`tests/` 侧同样零差异（见「直驱例外」）。

② **语料差分 356/356、差异为空**——基线 `tests/baselines/corpus-parity.jsonl` 共 356 行；`cargo run --example corpus_probe` 的产物与基线逐字节相同。

③ **IR 快照 216 条零漂移**——`src/middle/core/tests/snapshots/` 下 216 个 `.snap` 全部在库未改。

④ **`cargo test --all` exit 0**——lib **2868 → 2875 passed**（+7 = 本批新增的 7 条探针用例）/ 0 failed / 2 ignored；integration **314** passed；其余测试二进制全绿。测试集合对账：`git grep -c '#\[test\]' <rev> -- 'src/**'` 求和 **2871 → 2878**（+7，**removed 0**），全仓口径 **3399 → 3406**（+7）——**未删测试、未放宽断言**。

⑤ **grep 判据**——见上节，两条命令均 0 命中。

⑥ **变异验证（判据可失败性）**——三条变异各自使判据转红并**机械还原**（`git apply -R` 后逐文件 blob 与 HEAD 逐字一致、`git status` CLEAN）：

| 变异 | 转红的判据 |
| --- | --- |
| M1 撤销 `knows_builtin_types` 回退（回到修复前的 guard） | 探针用例 2 FAILED（`parser::tests::probe` 期望 `["N"]` 实得 `[]`；`typecheck::tests::probe` 双探针对账 left `["N"]` / right `[]`） |
| M2 在 parser 内插入 typecheck 引用 | grep 判据红（`grep -rn 'typecheck' src/frontend/core/parser/` 命中） |
| M3 把 `SemanticProbe` 的 `knows_builtin_types` 改回 `false` | 1 FAILED（双探针对账用例） |

> **提交（本地 dev，未推送）**：`:recycle: refactor(parser): 注入类型环境探针，parser 去除类型名依赖（P6 6.4）`（2026-10-29，32 files / +660 / −295）与 `:bug: fix(parser): 默认探针恢复 const 参数收编（6.4 修复轮）`（2026-10-29，6 files / +185 / −7）；本收口文档另起一笔 `:memo: docs(plan): 6.4 parser 数据流收口`。按仓库约定提交号在 push 之后取（`pull --rebase` 会重写本地哈希），故此处只记提交主题与日期。

### 门禁（实测，全部 exit 0）

| # | 门禁 | 实测 |
| --- | --- | --- |
| ① | `cargo fmt --check --all` | exit 0 |
| ② | `cargo clippy --all --all-features --all-targets -- -D warnings` | exit 0 |
| ③ | `cargo test --all` | exit 0（lib 2875 passed / 0 failed / 2 ignored；integration 314 passed） |
| ④ | 语料差分 | 356 / 356，差异为空 |
| ⑤ | IR 快照 | 216 条零漂移（`check-snapshot-drift.sh` PASS） |
| ⑥ | 义务账本 | PASS |
| ⑦ | 边界 + 扇出 + 概念 | 违规 0（`pub(crate)` 计数 **165**；B6 库存 2 处；豁免 1 处）；`check-fanout` 违规 0、库存 7 处（接线点实测 8 处）；`check-concepts` exit 0 |
| ⑧ | 文档门禁 | `check-docs-truth` / `check-docs-orphan` / `check-docs-refs` / `check-docs-examples` 全 exit 0 |
| ⑨ | 错误码生成器幂等 | 重跑 `python scripts/docs/gen-error-code-docs.py` 后 `git diff --quiet` exit 0 |

**错误码生成物（随行号平移再生）**：本批改动了诊断码发射点所在文件的行数，故错误码文档由生成器**位置敏感**地产出，必须重跑生成器并随提交再生（AGENTS.md 第 10 条⑤ 的机械必需面）——`docs/src/reference/error-code/E0xxx.md`、`E1xxx.md`、`E2xxx.md`、`E8xxx.md` 与 `docs/src/reference/warning-code/warning-codes.md` 共 **5 个生成物**，逐项 diff 仅为发射点行号平移，**诊断码数量与文字未变**；生成器复跑幂等。

### 直驱例外

**无**。`tests/yaoxiang` / `tests/baselines` / `src/std` 三段零差异，`tests/` 侧同样零差异；全仓 `#[test]` 由 3399 增至 3406，变化全部落在 `src/` 内的两个新增测试模块（`src/frontend/core/parser/tests/probe.rs`、`src/frontend/core/typecheck/tests/probe.rs`）——**未删测试、未放宽断言**。

### 残留风险（转入下一轮契约输入）

① **`NullProbe` + 谓词应用路径无测试覆盖**（低危）——`NullProbe` 下 `ParserState::kind_of` 只看本遍已声明谓词，环境侧谓词不可见 ⇒ 谓词应用短路在纯 `NullProbe` 下不生效。生产入口不受影响（`parse` / `parse_expression` 默认 `BuiltinProbe`，`parse_binding` 的 `NullProbe` 构造点全仓仅测试调用）。**若未来把 `NullProbe` 用于生产或等价基线，先补谓词应用用例。**

② **`NameKind` 未字段化**——03 §5.3 的目标形态是给 `Type::Name` 增 `kind` 字段、在构造点一次定死；本批只引入**定种枚举 + 探针查询**，不动 `Type::Name` 的结构（字段化会牵动全部构造点，且要求反向桥 `mono_to_ast_type` 保真回写 `kind`，IR 快照与语料基线随之再生——与本批 C3 面冲突）。当前定种信息只有一个消费者（`extract_generic_param_names` 的 const 参数判定），**字段化留待出现第二个消费者时再评估**。

③ **`name_used_as_type` 的四个变体缺口**——`MetaType` / `ConstExpr` / `Paren` / `NamedParen` 两版原先都落 `_ => false`，合并版保持该缺口。补它们会改变「const 参数是否被当类型用」的判定（进而改 IR 的 `generic_params` 与语料行为），属**语义修正**，须另立一轮（07 的分流判据），不在 6.4 的 C3 面内。

④ **两处陈旧注释**——`src/frontend/core/parser/probe.rs:44` 写「三个方法都带默认实现」，实为**四个**（增 `knows_builtin_types`）；`src/frontend/core/parser/statements/declarations.rs:450` 的注释仍称 `ParserState::is_predicate_app`，实际方法名是 `kind_of`。二者在 `src/` 内（不在本批收口文档的改动面内），留待下一轮实现侧一并修正。

⑤ **9 个默认 `parse()` 入口尚未注入 `SemanticProbe`**——`parse()` 的 9 个生产调用点为：driver 的三个解析臂（`src/driver/arms.rs:82` / `:166` / `:205`）、前端编排的两处逐文件解析（`src/frontend/module/orchestrator.rs:1041` / `:1247`）、前端门面（`src/frontend/compiler.rs:173`）、库入口（`src/lib.rs:100`）、LSP 补全（`src/lsp/handlers/completion.rs:136`）、`src/std/` 的源码解析（`src/std/yx_sources.rs:51`）。它们目前走默认 `BuiltinProbe`（编译器内置封闭集的镜像），故 B6 仍有 2 处库存。注入缝已就位（`src/driver/helpers.rs:1246` 的 `parse_with_probe` 是生产用例），**后续批次可把这 9 个入口全部改为注入 `SemanticProbe` 后删除 `BuiltinProbe` 的内置封闭集**——但 `src/std/` 的源码解析属本批硬验收的**零差异**面，改签名须同批再生语料基线。

⑥ **parser 的非 Fn 签名兜底仍在**——`src/frontend/core/parser/statements/types.rs` 的 `_ => (Vec::new(), Type::Void)` 兜底在 6.3 已改为 let-else 显式返回 `Type::Name{"()"}`（语义与 formatter 输出逐字不变），但该路径**语法可达**：`semantic_tokens` 的 `Name` 臂会比旧的 `Void` 臂（空操作）多记一枚 dummy-span token（语料 393 个 `.yx` 中该形态 0 命中，formatter / 诊断 / dump 零漂移）。**6.4 未接手该路径**（本批只做类型名依赖的注入化），须在 6.5 一并消除（不再由 parser 兜底产出伪类型）。

⑦ **`ir.rs` 三处 `ast::Type` 载体未动**——`Instruction::Cast { target_type }`（`src/middle/core/ir.rs:325`）、`Instruction::TypeTest { ty }`（同文件 `:330`）、`FunctionBody::TypeDecl { definition }`（同文件 `:651`）仍持 `ast::Type`；6.3 只删变体、6.4 只做注入化，两批都**未改 IR 形状**（IR 快照 216 条与语料基线 356 行零漂移）。载体处置须另立一轮（改动会动 IR 快照 / 语料基线）。

### 过程注记（四条）

① **生成物再一次落在 outOfScope**——本批实现轮的契约把 `docs/` 划进 outOfScope，却含 5 个错误码生成物（随诊断码发射点行号平移必须再生，属 AGENTS.md 第 6 条契约规则①「机械必需面」）。**6.2 / 6.3 / 6.4 三批连续踩同一个坑**——后续契约须把「`docs/src/reference/error-code/` 与 `docs/src/reference/warning-code/` 的生成物」显式圈进 `inScope`（按路径前缀写，不得用散文限定）。

② **首版提交里有假陈述**——首版 `parser/probe.rs` 的注释写「`is_constraint` / `is_predicate` 的判定与 6.4 之前逐字一致」，但该表述被当时的审计轮判为**假陈述**（`kind_of` 不含内置名，const 参数判定靠回退才一致），修复轮已改为按事实分述。

③ **一次审计轮因 revision 变更而中止**——审计轮的结论（真回归）已被修复轮与二次验证吸收；其中止原因是冻结的 SHA 变更，**不是判据失败**。真回归本身由静态审计（读代码 + 读判据）发现，不依赖运行期探针。

④ **本次收口对 09 的 6.4 行做了三处口径更正**——6.4.2 的字段清单（6 → 7 字段）、6.4.3 的调用点口径（「全仓 13 处」→ 计划口径 + 实测 14 处）与**层号**（middle → typecheck 是 L3→L3 同层，原写「L3→L2 跨层反向依赖」）、6.4.5 的行号锚点。这些是**事实性更正**，不是实测数字回填；数字与判据细节仍在本文。

### 计数

本批不新增二级/三级任务 ⇒ 09 顶部计数表不变（二级 **72** / 三级 **193**）。6.4 的 5 个三级任务为既有登记，勾选不改变计数。

## P6 6.5 AST 死变体与专用字段收口（6.5.1 / 6.5.2 + 并入的 6.8.1，2026-10-30）

> **触发**：P6 6.4 收口后，6.5 的两个三级任务待收口——6.5.1 删 `Expr::FnDef`（生产零构造的表达式变体）；6.5.2 删 `StmtKind::Assign` / `StmtKind::TypeDefinition` 的并行形参表 `signature_params`，并把 `Type::NamedParen` 的 binder 语义归位。**6.8.1（`Type::Fn.params: Vec<Type>` → `Vec<Param>`）按用户 2026-10-29 裁决 B 跨二级任务加框并入本批**：6.5.2 只删旧表示、替代表示由 6.8.1 给出，二者必须同批收口——只删不替代必然改行为（初稿设想的「改用 `Lambda.params`」已被核实不可行，见「拍平 vs 过滤」）。
>
> **轮次**：勘察（改写面清单 + 语义风险清单）→ 实现（6.5.1 两笔 + 6.5.2/6.8.1 三笔）→ 独立验证 → 对抗性复审（needs_revision，6 条 finding）→ 修复轮 → 二次复审（needs_revision，2 条 finding）→ 二次修复轮 → 三次复审（pass）→ 本收口。
>
> **判据**：C3（诊断码 / 消息 / 顺序不变）+ 03 §6.4 的硬验收（`git diff --stat tests/yaoxiang tests/baselines src/std/` 必须为空）。另加本批自身的四条载体判据：① **变体零残留**——`Expr::FnDef` 与 `signature_params` 在 `src/` 内零引用；② **单一形参表示**——`Type::fn_params_flat` 是「语句声明里的原始签名形参」的唯一实现，无并行形参表；③ **NamedParen 等价**——节点保留、binder 解释点、五处透明下钻与渲染形态逐字不变；④ **零行为差分**——语料 356/356 与 IR 快照 216 条零漂移。
>
> **本节含五类内容**：① **实测数字**；② **探针结论**；③ **登记①：6.5.1 的 A/B 契约偏差**；④ **登记②：修复轮的 `inScope` 契约缺陷追认**；⑤ **登记③：F2 的处置结论**（裸标识符形参的运行期槽位口径）。后三类各占一节。

### 结论：三条主线各归其位

| 主线 | 终态 | 载体 |
| --- | --- | --- |
| `Expr::FnDef` | **变体已删**，`src/` 内零引用；函数定义统一走 `Expr::Lambda` + `StmtKind::Assign` | `src/frontend/core/parser/ast.rs` |
| 形参表示 | **单一表示** = `Type::Fn.params: Vec<Param>`（名字与类型同栖）；`Assign` / `TypeDefinition` 的 `signature_params` 已删，载体改为 `type_annotation: Option<Type>` | 同文件 `:463-466` / `:258` / `:280`；唯一取用入口 `Type::fn_params_flat`（同文件 `:985`） |
| `Type::NamedParen` | **节点保留**（纯语法节点）；返回位 binder 由类型检查器**显式**解释 | 变体同文件 `:533-540`；解释点 `src/frontend/core/typecheck/checker/refinement.rs:421` |

`Expr` 枚举由 **31 变体收敛为 30**（删 `FnDef` 一个）；`ast::Type` 保持 6.3 收口后的 **13 变体**不变——本批只改 `Type::Fn` 的**字段类型**，不动变体集合。

### 交付清单（7 笔提交，本地 dev，未推送）

| # | 主题 | 日期 | 规模 |
| --- | --- | --- | --- |
| 1 | `:white_check_mark: test(parser): 函数定义测试迁移到 Lambda 解构` | 2026-10-29 | 2 files / +72 / −11 |
| 2 | `:fire: refactor(parser): 删除零构造的 Expr::FnDef 变体及全部消费臂` | 2026-10-29 | 20 files / +46 / −346 |
| 3 | `:recycle: refactor(parser): Type::Fn.params 改为 Vec<Param>（表示重塑）` | 2026-10-29 | 24 files / +383 / −103 |
| 4 | `:fire: refactor(parser): 删除 Assign/TypeDefinition 的并行形参表` | 2026-10-30 | 35 files / +314 / −234 |
| 5 | `:recycle: refactor(parser): 形参表示可维护性收敛` | 2026-10-30 | 22 files / +228 / −168 |
| 6 | `:bug: fix(middle): 恢复裸标识符形参的运行期槽位口径 + 注释/守卫订正` | 2026-10-30 | 8 files / +175 / −14 |
| 7 | `:fire: refactor(parser): 删除两个与既有谓词精确互补的死谓词` | 2026-10-30 | 1 file / +0 / −13 |

前 2 笔 = 6.5.1；第 3–5 笔 = 6.8.1 与 6.5.2 同批（顺序：先替代表示 → 再删旧表示 → 再可维护性收敛）；第 6 笔 = 二次复审的修复轮；第 7 笔 = 三次复审的修复轮。本收口文档另起一笔 `:memo: docs(plan): 6.5 AST 死变体与专用字段收口`（2026-10-30，3 files）。

按仓库约定提交号在 push 之后取（`pull --rebase` 会重写本地哈希），故此处只记提交主题与日期。

**基线**：本批 7 笔落在 6.4 收口提交之后的自动翻译机器人提交之上；机器人提交只改 `docs/src/en/**` 与翻译缓存，`src/` 逐字相同 ⇒ 本批的 `src/` 基线 = 6.4 收口态。

### `Expr::FnDef` 处置表（基线 25 处 / 16 文件）

6.4 收口态上实测 `git grep -n "FnDef" -- src/` = **25 行 / 16 文件**（其中字面 `Expr::FnDef` 18 处）。拆解：

| 类别 | 处数 | 说明 |
| --- | --- | --- |
| 变体定义 | 1 | `src/frontend/core/parser/ast.rs:49`（`Expr` 枚举体） |
| 生产消费点 | 15 | 见下表 |
| 测试构造点 | 3 | `src/frontend/core/parser/tests/ast.rs:874` / `:887`、`src/frontend/core/typecheck/tests/checker.rs:121` |
| 注释 | 6 | `src/frontend/core/typecheck/inference/expressions.rs:69`、`src/frontend/core/typecheck/layers/ownership.rs:2318`、`src/frontend/core/typecheck/layers/termination.rs:2311`、`src/frontend/core/typecheck/tests/checker.rs:119` / `:478`、`src/frontend/module/orchestrator.rs:1155` |

**15 处生产消费点**（基线位置）：

| # | 位置 | 形态 | 处置 |
| --- | --- | --- | --- |
| 1 | `src/formatter/handlers/expr.rs:43` | match 臂 | 删；连带删唯一调用它的 `format_fn_def`（否则 `clippy -D warnings` 报 dead code） |
| 2 | `src/frontend/core/parser/ast.rs:1121` | `Expr::span()` or-pattern（**穷举**） | 删 |
| 3 | `src/frontend/core/parser/pratt/mod.rs:47` | `expr_end_line()` 臂（**穷举**） | 删 |
| 4 | `src/frontend/core/spawn/analysis.rs:912` | match 臂（闭包捕获） | 删 |
| 5 | `src/frontend/core/spawn/placement.rs:122` | match 臂 | 删 |
| 6 | `src/frontend/core/typecheck/checker.rs:1359` | `if let`（非穷举） | 删（else-if 链头改 `if`） |
| 7 | `src/frontend/core/typecheck/checker/semantic_tokens.rs:1372` | match 臂（token 收集） | 删 |
| 8 | `src/frontend/core/typecheck/inference/existential.rs:55` | `expr_span()` or-pattern | 删 |
| 9 | `src/frontend/core/typecheck/inference/expressions.rs:3874` | match 臂（推断） | 删（含 1 处 E1002 死发射点） |
| 10 | `src/frontend/core/typecheck/layers/ownership.rs:1263` | `expr_span()` or-pattern | 删 |
| 11 | `src/frontend/core/typecheck/layers/termination.rs:763` | match 臂 | 删（连带删仅此臂调用的 `check_fn_body`） |
| 12 | `src/frontend/core/typecheck/passes/dead_code.rs:505` | match 臂（引用收集） | 删 |
| 13 | `src/frontend/module/orchestrator.rs:1153` | `if let`（非穷举，整臂仅此内容） | 删整臂 |
| 14 | `src/middle/core/ir_gen.rs:5005` | `get_expr_span()` 臂（**穷举**） | 删 |
| 15 | `src/middle/core/ir_gen.rs:5812` | `expr_span` or-pattern | 删 |

**「生产零构造」的一手证据**：基线的 `src/frontend/module/orchestrator.rs:1155-1156` 注释明写「当前 parser 也不产此变体——仅测试手工构造」；函数定义实际走 `Expr::Lambda` + `StmtKind::Assign`。⇒ 删净不改变任何 parser 产出。

**测试迁移（未删测试）**：三处测试构造改写断言方向，不删用例——

| 原用例 | 终态 |
| --- | --- |
| `test_expr_fndef`（`src/frontend/core/parser/tests/ast.rs`） | 拆为 `test_expr_callable_parts_lambda_returns_params_and_body`（同文件 `:874`）与 `test_expr_callable_parts_block_returns_body_without_params`（`:906`） |
| 嵌套函数定义夹具（`src/frontend/core/typecheck/tests/checker.rs`） | `Expr::FnDef` → `Expr::Lambda`，用例保留 |
| ——（新增） | `test_fn_block_body_with_fn_annotation_parses_to_lambda`（`src/frontend/core/parser/statements/tests/functions.rs:125`）：`f: () -> Int = { 5 }` → Assign + Lambda 的端到端断言 |

### `signature_params` 处置（基线 168 行 / 31 文件 → 0）

6.4 收口态上实测 `git grep -n "signature_params" -- src/` = **168 行 / 31 文件**；落地态 **0 命中**（字段、构造、读取、注释一并清零；旧包装函数 `signature_fn_params` 亦删除）。

真·字段面只有 **2 处属主字段**：`StmtKind::Assign` 与 `StmtKind::TypeDefinition` 各一个 `signature_params: Vec<Param>`；其余命中是同名遮蔽（函数形参名写死 `&[Param]`）、函数名与注释。落地态两处字段均已删，载体改为 `type_annotation: Option<Type>`（`src/frontend/core/parser/ast.rs:258` / `:280`）——`TypeDefinition` 与 `Assign` 同一载体。

**拍平 vs 过滤（为什么不能改用 `Lambda.params`）**：初稿设想「就地重算 `Lambda.params`」，但 `Lambda.params` 是**已过滤**的值参数表（去掉类型位与约束位），而 `classify_generic_params` / `extract_generic_param_names` 需要**未过滤**的原始签名列表。实际做法是先把名字搬进唯一载体再删旧字段：拍平结果与旧 `signature_params` **逐位同源**（同一批 `Param` 节点），故泛型分类结果不变；改用 `Lambda.params` 的方案**会改行为，未采用**。

**单一拍平入口**：`Type::fn_params_flat`（`src/frontend/core/parser/ast.rs:985`）是「语句声明里的原始签名形参」的**唯一实现**（全仓 1 处定义 + 31 处调用，16 文件）；curry 各组依次展开，`Paren` / `NamedParen` 视为透明继续内层。六条语义消费点全部改调它：`classify_generic_params`（`src/driver/helpers.rs`）、`extract_generic_param_names`（`src/frontend/core/typecheck/checker/semantic_tokens.rs`、`src/middle/core/ir_gen.rs`）、`resolve_const_candidates`、`add_method_overload_with_params`、`resolve_return_refinement`（`src/frontend/core/typecheck/checker/refinement.rs`）、`enter_measured_fn`（`src/frontend/core/typecheck/layers/termination.rs`）。

**`Type::Fn.params` 重塑**（6.8.1，`Vec<Type>` → `Vec<Param>`）：复用既有 `Param` 结构——**不新增第二结构、不引入平行名字数组**（`param_names: Vec<Option<String>>` 一类被明令禁止）。无名形参（纯类型位 / 类型反解）以空名字合成，由 `Param::is_named()` 判定，空名不参与任何按名查找。基线 `Type::Fn` 共 **331 行 / 63 文件**，逐处处置：**改 23 文件**（含 `params` 元素的读取点改取 `.ty`）、**无需改 40 文件**（命中全部是 `MonoType::Fn`，不是 `ast::Type::Fn`）。

**为什么手写 `Param` 的 `Debug`**：规范化 IR 快照的 `type_decl` 行打印 AST 类型的 `Debug` 文本，且 216 条快照已入库。若靠 `UPDATE_SNAPSHOTS=1` 再生基线，就会把「形参携带名字」伪装成一次 IR 行为面变化（违反 07 §128「漂移必须能说清来源」）。故把一致性收在**打印边界**：无名形参按其所载 `Type` 打印（与重塑前 `Vec<Type>` 的文本逐字一致）＋ IR 载体入 IR 前由 `Type::without_fn_param_names`（`src/frontend/core/parser/ast.rs:858`）剥掉类型位形参名——**基线因此零漂移**。

### `NamedParen` 迁移等价性

**节点保留、语义归位**：`Type::NamedParen { param, param_span, inner }`（`src/frontend/core/parser/ast.rs:533-540`）保留为**纯语法节点**；`param` 仍是返回位 binder 名，但**解释权显式归语义层**——`src/frontend/core/typecheck/checker/refinement.rs:421` 直接取 `param` 作 binder。

等价性逐条可对：

| 判据 | 落点 | 状态 |
| --- | --- | --- |
| binder 名的唯一来源 | `src/frontend/core/typecheck/checker/refinement.rs:421` | 不变 |
| binder **不进入函数体作用域** | 同文件的作用域名单判定 | 不变 |
| 五处透明下钻（只递归 `inner`） | `src/frontend/core/typecheck/checker/refinement.rs:519` / `:987`、`src/frontend/core/typecheck/checker/semantic_tokens.rs:124`、`src/frontend/core/types/mono.rs:968`、`src/middle/core/ir_gen.rs:1198` | 不变 |
| 柯里化链终止（括号声明「完整类型」） | `src/frontend/core/parser/statements/declarations.rs:347`、`src/middle/core/ir_gen.rs:661` | 不变 |
| 渲染形态 `({param}: {inner})` | `src/formatter/handlers/types.rs:18`、`src/formatter/handlers/expr.rs:477` | 不变 |
| AST→AST 重建仍携 binder | `src/middle/passes/mono/function.rs:401-405` | 不变 |

`ast.rs` 的注释改写为「参数位的形参名归 `Type::Fn.params`；本节点只承载**返回位**单具名括号的 binder」——**参数位与返回位是两回事**，混同即语义错误。

**为什么不能简单删节点**：`(r: P(m))` 中若丢掉 `r`，类型检查器只能猜「约束里不在作用域的自由变量即返回值形参」，会把未声明的 `m` 也当 binder 静默代入（`ast.rs` 注释标注的实测缺陷）。⇒ 迁移 = 责任声明 + 注释改写，**不是删除**。

### 实测数字

① **硬验收**——6.4 收口态与本批落地态之间，`git diff --stat -- tests/yaoxiang tests/baselines src/std src/middle/core/tests/snapshots` **为空**（语料层 / 基线层 / 标准库 / IR 快照四段零差异）；`git diff --stat -- tests/` 同样**为空**（`tests/` 侧零直驱例外）。

② **语料差分 356/356、差异为空**——基线 `tests/baselines/corpus-parity.jsonl` 共 356 行；`cargo run --example corpus_probe` 的产物与基线逐字节相同（`check-corpus-parity.py` exit 0）。

③ **IR 快照 216 条零漂移**——`src/middle/core/tests/snapshots/` 下 216 个 `.snap` 全部在库未改（`check-snapshot-drift.sh` PASS）。

④ **变体零残留**——落地态 `git grep -n "Expr::FnDef" -- src/` = 0 命中；`git grep -n "signature_params" -- src/` = 0 命中；`git grep -n "signature_fn_params" -- src/` = 0 命中；`git grep -n "is_untyped" -- src/` 与 `git grep -n "is_type_position" -- src/` = 0 命中（末轮修复删掉的两个死谓词）。裸词 `FnDef` 在 `src/` 内只剩 2 处**说明性注释**（`src/frontend/core/parser/statements/tests/functions.rs:123` 与 `src/frontend/core/typecheck/tests/checker.rs:121`，均写明「不构造已删除的 FnDef 变体」）。

⑤ **`cargo test --all` exit 0**——lib **2878 passed** / 0 failed / 2 ignored；integration **314** passed；`tests/integration/` 下 22 个 `.rs` 模块全绿。`src/` 内 `#[test]` 计数 **2878 → 2883（+5，removed 0）**：3 个用例改名（`test_expr_fndef` → 两个 callable_parts 用例；`test_signature_params_stored_verbatim` → `test_type_fn_params_carry_signature_verbatim`；`test_named_function_type_params_stay_anonymous_types` → `test_named_function_type_params_carry_names_and_types`，断言方向由「类型位形参名丢弃」反转为「名字与类型同节点」，属加强）、5 个新增（`test_fn_block_body_with_fn_annotation_parses_to_lambda`、`test_type_fn_params_carry_name_and_type_on_same_node`、`test_anonymous_param_never_yields_generic_name`、`test_bare_identifier_param_takes_no_runtime_slot` 等）。**未删测试、未放宽断言**。

⑥ **单一拍平入口**——`git grep -n "fn fn_params_flat" -- src/` = 1 处定义（`src/frontend/core/parser/ast.rs:985`）；`git grep -n "fn_params_flat" -- src/` = 32 行 / 16 文件（1 定义 + 31 调用点）。

⑦ **错误码生成物（随行号平移再生）**——本批改动了诊断码发射点所在文件的行数（`ast.rs` 少 13 行等），错误码文档由生成器**位置敏感**地产出，必须重跑生成器并随提交再生：`docs/src/reference/error-code/E1xxx.md` / `E2xxx.md` / `E8xxx.md` / `index.md` 与 `docs/src/reference/warning-code/warning-codes.md` 共 5 个生成物，逐项 diff 仅为发射点行号平移，**诊断码数量与文字未变**；生成器复跑幂等（`git diff --quiet` exit 0）。

### 探针结论

**定向探针（13 组 × 2 类产物 = 26 份）**：在基线与本批的 4 个中间 revision（删变体 / 表示重塑 / 删旧表示 / 可维护性收敛）上分别产出 `check --json` 与 `format --stdout` 两类产物，**26/26 逐字节一致**。覆盖的同类路径含：lambda 头顺序与签名不一致（既有 E0012「signature has (a), lambda has (b)」）、类型参数与值参数混排（curry 分组）、纯类型位无名形参、TypeDefinition 泛型、const 泛型 + curry、命名实参重载、返回位 binder。

**返回位 binder 的正反对照**：正例 `f: (b: Int) -> (r: IsPositive(r + 1))` 代入后约束成立 ⇒ 零诊断；反例触发 E4018 且诊断文本含**代入后**的约束（证明 binder 真被代入而非丢弃）；AST 级探针显示返回位 binder **不入** `Type::Fn.params`（拍平结果为空）。

**formatter 只打类型**：类型位 `Type::Fn` 臂只渲染形参的 `ty`（无名形参不渲染）；声明签名位仍渲染 `name: type`。实测 `S: Type = { f: (a: Int) -> Int }` → `{ f: (Int) -> Int }`（名字确实被丢，与既有断言一致）；**全语料 formatter 对拍 355 个 `.yx` 逐字节一致**（基线二进制 vs 落地态二进制）。既有断言未改（`tests/formatter_integration.rs`）。

**变异试验（判据可失败性）**：① 把 `Type::fn_params_flat` 的拍平改成跳过首参（`.skip(1)`），不变量测试 `test_type_fn_params_carry_signature_verbatim` 转红（exit 101），语料差分门禁同步转红（321 处差异）；② 把形参构造改成「非首参类型强制错配」，不变量测试 `test_type_fn_params_carry_name_and_type_on_same_node` 转红，断言消息点名第 2 个形参的类型应为 `Int`。两次变异均以 `git apply -R` **机械还原**，还原后被改文件的 blob 与 HEAD 逐字一致、`git status` CLEAN。

**IR dump 对拍（F2 的判据）**：探针 `f: (Int64) -> (a: Int64) -> Int64 = 1` 的 `dump` 输出——表示重塑后（未修复）外层 `Params: (i64)`、修复后外层 `Params: ()`，与**基线**逐字相同（见「登记③」）。

### 登记①：6.5.1 的 A/B 契约偏差（变体定义与补臂必须原子）

6.5.1 的契约要求「**分 2 commit**」，落地为两笔：

1. `:white_check_mark: test(parser): 函数定义测试迁移到 Lambda 解构`——把两处测试构造迁到 Lambda（**准备**）；
2. `:fire: refactor(parser): 删除零构造的 Expr::FnDef 变体及全部消费臂`——删变体定义 + 全部消费臂（**删除**）。

**偏差**：契约把「定义删除」放在第 1 笔，实测**不可行**——Rust 的穷举 `match` 下，变体定义与补臂删除必须**原子**：任一侧先删即编译不过（定义先删而臂未删 ⇒ 臂指向不存在的变体；臂先删而定义还在 ⇒ 变体仍零构造、删净判据不成立）。故「定义删除」随第 2 笔落地，第 1 笔只承载测试迁移。

**可 revert 口径（实测）**：

- 第 2 笔（删变体）**可独立 revert**，revert 后可 build（源码全 auto-merge 干净，仅 3 个生成物文档冲突；重生成后构建通过、`cargo check --tests` 通过，`Expr::FnDef` 变体回到定义处）；
- 第 1 笔（测试迁移）**不可单独 revert**——须与第 2 笔**成对**回退（顺序：先第 2 笔后第 1 笔）。单独回退第 1 笔会把「构造已删变体」的旧测试带回来，编译报 `no variant named FnDef`。

**判定**：准备—删除型拆分**天然不可单独回退**，是该类拆分的固有性质，不是执行缺陷；「分 2 commit」是契约明写要求，可回退单元是**两笔合起来**。此事由本节登记，不作为 finding。

### 登记②：修复轮的 `inScope` 契约缺陷追认

二次复审的修复轮（`:bug: fix(middle): 恢复裸标识符形参的运行期槽位口径 + 注释/守卫订正`）实际改动 8 个文件，其中 **4 个是机械必需面却不在契约声明内**：

| 文件 | 为什么是机械必需面 |
| --- | --- |
| `docs/src/reference/error-code/E1xxx.md` | 诊断发射点行号随 `ast.rs` 平移，生成器产物（AGENTS.md 第 10 条⑤ 要求随提交入库） |
| `docs/src/reference/error-code/E8xxx.md` | 同上 |
| `src/middle/core/tests/fn_param_slots.rs` | F2 的行为面锁定测试；按仓库测试规范必须放 `src/middle/core/tests/` |
| `src/middle/core/tests/mod.rs` | 上一条测试的模块登记（否则测试树成孤儿，既不编译也不运行） |

**追认**：这 4 个文件属机械必需面，**按机械必需面接受**。成因是契约的 `inScope` 写窄——**不是实现者越界**，也不是「顺手多改」。

**由此固化的规则建议（待用户裁决后写入 `coding-rules.md`）**：**repair 类任务的契约必须预置生成物与测试家目录**——凡修复会移动诊断发射点行号，`inScope` 必须含 `docs/src/reference/error-code/` 与 `docs/src/reference/warning-code/` 的生成物**路径前缀**；凡修复会新增测试文件，`inScope` 必须含该测试的**家目录**（如 `src/middle/core/tests/`）。理由：这两类是「不写进契约就必然被契约校验拒收」的机械必需面，而本仓已连续多个批次（6.2 / 6.3 / 6.4 / 本批）反复踩同一个坑；09a §P6 6.4 的「过程注记①」已就生成物一侧登记过同一条建议，本条把它补齐到「测试家目录」一侧。

### 登记③：F2 的处置结论（裸标识符形参的运行期槽位口径）

**问题**：`Type::Fn.params` 装的是**全部**形参位条目，其中含「参数位裸标识符」这一形态——`name` 是标识符、`ty` 为空（可能是类型名、不是形参）。表示重塑后，`split_curry`（`src/middle/core/ir_gen.rs:615`）的游标按**参数条目个数**推进，于是裸标识符也占了一个运行期形参位 ⇒ 柯里化外层 IR 形状变化。

**实测**（探针 `f: (Int64) -> (a: Int64) -> Int64 = 1` 的 `dump`）：

| 形态 | 外层 `Params` | 内层 `Params` |
| --- | --- | --- |
| 基线 | `()` | `(i64)` |
| 表示重塑后（未修复） | `(i64)` | `(i64)` |
| 修复后 | `()` | `(i64)` |

⇒ 修复后与基线**逐字相同**。同形状的既有错误行为亦恢复一致：`= a` / `= (a) => a` / `= { a }` / `f: (a) -> (b: Int64) -> Int64 = (b) => b` 在落地态与基线同为 E3006（运行期变量解析失败），不再被意外编译通过。

**口径**：裸标识符形参**不占运行期形参位**——此口径由 **WBS 8.9.1**（参数位形态归一，D53）重新裁定；本批不越权改口径，只**恢复基线口径**并把口径写进 API：`Param::occupies_runtime_param_slot()`（`src/frontend/core/parser/ast.rs:814`）= `ty.is_some()`，`split_curry` 按它计位（`src/middle/core/ir_gen.rs:637`）。AST 表示**未动**（`Type::Fn.params` 继续承载带名 / 裸标识符的全部形参），也没有用「退回旧表示」绕过。

**锁定测试**：`src/middle/core/tests/fn_param_slots.rs` 的 `test_bare_identifier_param_takes_no_runtime_slot`（`:42`）断言外层 `Params` 为空、内层带类型形参仍占 1 位；测试注释写明「此口径由 8.9.1 重新裁定，改它必须同时改这条测试」。**改口径必须改这条测试**，防止静默漂移。

**同批另两条 finding 的处置**：

- **F1（注释与事实不符）**——`Param` 的名字约定改写为**三分形态**的事实描述：① 有名形参；② 无名（空串）；③ 参数位裸标识符（`name` 是标识符、`ty` 为空、可能是类型名、不是形参）。`is_named()` 的语义定死为「`name` 槽是否非空」，**不代表**「携带真实形参名」。parser 认不出用户类型名（更彻底的方案）不在本批范围，留 6.6 / 6.8。
- **F3（空串不参与按名查找无守卫）**——`classify_generic_params` / `extract_generic_param_names` 两个入口显式跳过无名形参，并加 `test_anonymous_param_never_yields_generic_name`（`src/frontend/core/parser/statements/tests/declarations.rs:309`）以三种可分类形态（类型名 / const 名 / 约束名）断言两入口均返回空表。
- **末轮修复**——删除两个与既有谓词**精确互补**的死谓词 `Param::is_untyped()` 与 `Param::is_type_position()`（各 0 引用、无桩、无替代方法，13 行删除）；保留 `is_named()`、`occupies_runtime_param_slot()`、`anonymous()` / `from_type()` 与手写 `Debug`。

### 门禁（实测，全部 exit 0）

| # | 门禁 | 实测 |
| --- | --- | --- |
| ① | `cargo fmt --check --all` | exit 0 |
| ② | `cargo clippy --all --all-features --all-targets -- -D warnings` | exit 0 |
| ③ | `cargo test --all` | exit 0（lib 2878 passed / 0 failed / 2 ignored；integration 314 passed） |
| ④ | 语料差分 | 356 / 356，差异为空（`check-corpus-parity.py` exit 0） |
| ⑤ | IR 快照 | 216 条零漂移（`check-snapshot-drift.sh` PASS） |
| ⑥ | 义务账本 | PASS + `--selftest` 全 PASS |
| ⑦ | 边界 + 扇出 + 概念 | 违规 0（`pub(crate)` 计数 165）；`check-boundary` / `check-fanout` / `check-concepts` 全 exit 0 |
| ⑧ | 文档门禁 | `check-docs-truth` 诊断码 157 / 157；`check-docs-orphan` / `check-docs-refs` / `check-docs-examples` 全 exit 0 |
| ⑨ | 错误码生成器幂等 | 重跑 `python scripts/docs/gen-error-code-docs.py` 后 `git diff --quiet` exit 0 |

各量测轮次：① ② ④ ⑤ ⑦ ⑧ ⑨ 在本批每一轮提交后复跑；③ ⑥ 在实现批的落地树上完整跑通；两轮修复轮各复跑其自身判据子集，均 exit 0。

另：`check-ir-verifier.sh` PASS（全语料 `verify_loose` 绿，校验面非空）；`npm run docs:build` exit 0（VitePress 真构建，AGENTS.md 第 10 条⑨——文档提交前的强制项）。

### 直驱例外

**无**。`tests/yaoxiang` / `tests/baselines` / `src/std` 三段零差异，`tests/` 侧同样零差异（6.4 收口态与本批落地态之间 `git diff --stat -- tests/` 为空）；`src/` 内 `#[test]` 由 2878 增至 2883，变化全部是「改名 + 新增」，**removed 0**，无放宽断言。表单驱测试 `tests/formatter_integration.rs` 未改。

### 残留风险（转入 6.6 契约输入）

① **parser 的非 Fn 签名兜底仍在**——`src/frontend/core/parser/statements/types.rs:823` 的 `extract_fn_type_info` 仍以 let-else 返回 `Type::Name{"()"}`（6.3 改自 `Type::Void` 兜底；语义与 formatter 输出逐字不变，但该路径**语法可达**：`semantic_tokens` 的 `Name` 臂比旧 `Void` 臂多记一枚 dummy-span token；6.3 收口时实测语料中该形态 0 命中，formatter / 诊断 / dump 零漂移）——**6.5 未接手**，须在 6.6 消除（不再由 parser 兜底产出伪类型）。已登记进 09 的 6.6.1 行。

② **`ir.rs` 三处 `ast::Type` 载体未动**——`Instruction::Cast { target_type }`（`src/middle/core/ir.rs:322`）、`Instruction::TypeTest { ty }`（同文件 `:328`）、`FunctionBody::TypeDecl { definition }`（同文件 `:651`）仍持 `ast::Type`；6.3 只删变体、6.4 只做注入化、6.5 只做表示重塑，三批都**未改 IR 形状**（IR 快照 216 条与语料基线 356 行零漂移）。载体处置须在 6.6 决定（改动会动 IR 快照 / 语料基线）。已登记进 09 的 6.6.1 行。

③ **表达式位置当参数列表仍在**——`src/frontend/core/parser/pratt/led.rs` 的 `expr_to_params` 臂仍把「空 body 的 `Expr::Lambda`」捞回来当参数列表（03 §6.3 #17 计划的「随 5.5 改为直接读 Lambda」**未做**，5.5 的改动清单里本就没有这一项）。处置随 6.8.2 的声明驱动检查模式一并决定。

④ **参数位裸标识符是否算参数，仍待 8.9.1 裁决**——本批只恢复基线口径并写进 API 与锁定测试；「裸标识符按类型命名空间解析、解析不到报新 E 码」属 8.9.1。

⑤ **parser 内两处内置封闭集仍在**——`CONST_PARAM_TYPES` 与 `PREDICATE_NAMES`（`scripts/ci/check-boundary.py` 的 B6 库存 2 处），9 个默认 `parse()` 入口尚未注入 `SemanticProbe`；转硬 T1 前须评估（承接 09 的 6.6.1 行第 ⑥ 项）。

### 过程注记（四条）

① **「删变体」不可能只删定义**——Rust 穷举 `match` 使「定义 + 补臂」成为原子单元；契约若把两者拆到不同 commit，就必须接受其中一笔不可单独 revert（见「登记①」）。

② **表示重塑的顺序不能反**——先删 `signature_params` 再给替代表示，中间态必然改行为；本批按「先 `Vec<Param>` → 再删两处旧字段 → 再可维护性收敛」三笔推进，每笔单独可 build。

③ **零行为差分靠「把一致性收在打印边界」而不是靠再生基线**——手写 `Param` 的 `Debug` 正是为此（见「为什么手写 `Debug`」）；若改走基线再生，216 条快照会被「形参携带名字」污染成一次假的 IR 行为变化。

④ **本批有 4 个机械必需面文件曾落在契约外**（见「登记②」）——本仓连续多个批次重复踩同一个坑，规则建议待用户裁决后固化。

### 计数

本批不新增二级/三级任务 ⇒ 09 顶部计数表不变（二级 **72** / 三级 **193**）。6.5 的 2 个三级任务为既有登记，勾选不改变计数；跨二级并入的 6.8.1 亦为既有登记。
