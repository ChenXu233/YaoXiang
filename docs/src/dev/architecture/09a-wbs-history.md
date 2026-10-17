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
> `target/yx-scratch/yx49/EVIDENCE.md`（含 8 份变异补丁与 warn_compare_v8.json）。
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
> `target/yx-scratch/yx49/EVIDENCE.md` 的 C 节写 `+16/−0`，但漏了「**新夹具记录**」这一
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
> 8 份变异补丁（`target/yx-scratch/yx49/*.patch`，p1 目录扫描、p2b use 导入名剔除、p3b script
> 接池、p3 flat 消费、p4 std 接池、p7final 自建 4 判据、p8 std 关闭、p9 vendor 接池 +
> p_stack2 / p_stack3 叠加面）。每个 trial 输出含：HEAD blob → 变异后 blob → 还原后 blob
> 与 HEAD 一致 = 是 + `git status` CLEAN。本机 `core.autocrlf=true` 需经 `GIT_CONFIG_*`
> 注入 `false`（`trial_env.ps1`），否则补丁新增行转 CRLF 导致 apply/还原双失败——**只影响
> 补丁机械性，不改被测行为**。
>
> **五、过程事件 / 环境教训（移交下轮必读）**
>
> 1. **core.autocrlf 注入**：本机 `core.autocrlf=true` 必须经 `GIT_CONFIG_*` 注入 `false`
>    才能保证补丁机械可逆；详见 `target/yx-scratch/yx49/trial_env.ps1`。**下轮不要复用**
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
> 4. **--no-verify 会连带跳过 markdownlint 与 commit-msg 校验**：轮 2 收口的文档提交曾因此被推送前补跑钩子拦下（表格列数 / 行尾空格 / 登记字段缺失）；文档提交若要用它，至少手动补跑这两个钩子。
> 5. **契约描述未同步的流程疏漏**：t14 的任务描述文本残留 PA-1 旧假说（写「b.yx 报 W1005」
>    的期望），验证者在报告里主动指出「按 contract 字段判，描述文本不计」并请队长确认——处置 =
>    以任务契约字段为判决依据，描述文本留作流程教训（**改派/派工时任务描述必须与最新判决链同步**）。
>
> **四、变异试验（机械可逆，全部 scratch.py trial）**
>
> 轮 2 验证者共 9 份补丁（target/yx-scratch/yx49/ 下 m1b/m2b/m4b 为 F2 后的可编译重做版——
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
