# Changelog

## :bookmark: V0.8.5: 跨层义务账本与引用池归因收口（RFC-039 进行中）

> 发布日期: 2026-10-19

### 📦 版本信息

| 项目     | 值                |
| -------- | ----------------- |
| 发布日期 | 2026-10-19        |
| 版本变更 | `0.8.4` → `0.8.5` |
| 提交数   | 153 个 commit     |

### 📋 本次更新概要

本次发版是 **RFC-039 编译器架构重构进行中**的一次迭代，**不是重构收口**。两条主线落地：

其一，**跨层契约从「位置键」升级为「结构身份键」，并接入可断言的义务账本**——`ReleasePlan` 与 `overload_resolutions` 改按 `PlanId` 键控（WBS 4.5），类型检查产出的跨层义务有了清单、交付标记与唯一结算点（WBS 4.3.1 / 4.3.2），未交付的义务在编译期响亮拒绝（W1090，error 级）。

其二，**死代码分析的归因修正**（WBS 4.9 引用池正确实现，分三轮收口）——包内引用池的来源与身份、消费判据、方法可达性的逐候选与跨单元归因依次落地，消除了大批「被调用的方法被误报未使用」，并让跨单元同名同序的未调用方法恢复被报。

对外可观察的行为变化集中在本版「⚠️ 行为变化」一节：**死代码告警集合出现收缩**（删误报方向）与**跨单元重载决议开始生效**（修好方向）；LSP 的项目内跨文件跳转 / 悬停 / 重命名 / 查找引用自静默落空转为可用；方法绑定 panic（exit=101）转为编译期错误 E1110；命名实参被整体静默丢弃的形态转为按签名形参名绑定并对冲突形态报错；命令行 `check` 的显式路径与 `--exclude` 不再静默失效。

### 🏗️ RFC-039 重构进展（进行中，本版本非收口）

**本版本已落地：**

- **P3 正确性修复**：方法绑定 panic 消除（3.13 / issue #451，转 E1110）、命名实参绑定判据统一（3.14 / 3.16，RFC-039 D60，新增 E1111）、单调化深度守卫爆栈消除（issue #447）、解释器深度守卫可达与错误码跨任务边界保真（issue #448）、所有权层输入降级加 warning（4.4.3，新增 W1082）。
- **P4 阶段契约与统一 Driver**：跨层契约 PlanId 化（4.5）、义务账本与运行期结算及静态门禁（4.3.1 / 4.3.2）、引用池正确实现三轮（4.9.1–4.9.7）、LSP 语义数据管线统一（4.6.1 / 4.6.2）、standalone run 统一走多文件编排（4.10.3）、证明层与 wasm 收尾（4.4.1 / 4.4.2 / 4.4.3）。
- **P2 等价性判据**：IR 快照与语料行为差分基线随每笔行为变更同提交再生——本版语料基线 340 → 348 行、IR 快照 204 条零漂移。
- **P9 防反弹门禁**：新增 `scripts/ci/check-obligations.py`（含正例 / 反例自检与陈旧检测）接入 CI 独立 job；错误码文档生成器幂等门禁（重跑生成器后 `git diff` 必须为空）。

**尚未收口：**

- **P5 checker 文件内拆分**（`include!` → 真 `mod`、拆出 refinement / annotations）、**P6 类型表示单一化**、**P7 中间表示 SSA 化**、**P8 前端范式变更**：未开工。
- **P9** 多数防反弹门禁仍为 report-only；**P10** 清理项余量见 [09-execution-wbs](docs/src/dev/architecture/09-execution-wbs.md) 看板。
- 2026-10-19 的发版前审计另登记 12 条三级任务（挂 issue #465–#477），并重开 4 个二级任务（4.9.8 / 3.14.2 / 4.4.4 / 4.10.4 / 4.10.5）、新增二级 3.20 与 S9，均不阻塞主线。

进度以 [09-execution-wbs](docs/src/dev/architecture/09-execution-wbs.md) 三级任务看板为准（本版计数：二级 65 / 三级 169）；总纲与决议登记见 [RFC-039](docs/src/rfc/accepted/039-compiler-architecture.md)。

### ⚠️ 行为变化

#### 跨层契约与义务账本

- **跨层契约改按结构身份键控（RFC-039 D20 / WBS 4.5）**：`ReleasePlan` 与 `overload_resolutions` 的键由 `Span`（源码位置）改为 `PlanId`，消费「读与记账同一次调用」，差集非空即在唯一结算点报 `error[W1090]`——原先两侧位置推导不一致时的「静默丢 Drop / 静默回落默认绑定」不再存在。

- **未交付的跨层义务在编译期响亮拒绝（WBS 4.3.1 / 4.3.2，新增码 W1090）**：RFC-039 P4 4.3 把「字段被产出但无人消费」从**不可检测**变成**可断言**——新增义务清单（逐条带定义 / 生产点 / 交付点 / 消费点的 file:line）、交付标记（ProofExecution 一处、IrGeneration 四处、Monomorphization 两处）、唯一结算点（`Driver::run` 阶段表尾部）与 `DriverOutcome` 审计面。终轮把红名单归零（义务 14 → 13）并新增非义务字段登记（8 条）：`TypeCheckResult` 字段集 = 义务清单 ∪ 非义务登记（21 = 13 + 8），「加字段忘登记 / 删字段忘同步 / 登记缺理由 / 两表重叠」四种形态当场红。**对外可观察变化**：命中未交付义务时渲染 `error[W1090]`（码号仍属 W 族但显式按 error 级发射，触发即计入 `error_count`、退出码非零）——属 D59 **D 类**破坏性变更，已获用户 2026-10-13 裁决并在 02 登记。

#### 死代码分析归因（包内引用池）

- **跨单元同名同序方法按定义单元判定：未被调用的定义恢复报 W1005（WBS 4.9.7 / issue #464 收口；含跨单元重载决议重新生效）**：方法可达性的逐候选归因此前只在**同单元**成立——跨单元（registry 解析）的方法调用不产生 D20 决议条目 ⇒ 该名字组回落池的**名字级**豁免（`RefPool::field_names` 短名全局平铺）⇒ **跨单元同名同序的未调用方法不被报**。本批两笔收口：① 产出侧（提交 `b8b3cc3f6`）让跨单元方法调用产生决议条目——候选组（候选 / 混编 IR 名 / 声明期形参名三张同序表）随 `ModuleInfo.method_overloads` 跨单元传递，注册表按模块键升序确定性合并，Typecheck 臂按本单元 import face **逐对**取候选（只注入「键的提供模块 == 该**类型名前缀**在本单元 scope 里的提供模块」的候选组；自有键位不外并、同名冲突整名跳过）——**修复轮实测**：逐对过滤前该面按「名字集合 × 模块集合」组合，会把另一模块定义的同名类型候选并进同一键位，`use a.{Temp}` + `use b.{Other}`（a 亦定义 Other、b 亦定义 Temp、方法同名同序、每个名字只由一个模块带进作用域）下 `check` 报 2×E1107、退出 1（修前成功的程序编译失败），逐对过滤后 `check`/`run` 均退出 0，入库夹具 `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`；② 驱动侧按**定义单元**归因——`resolved_method_marks` 产出 `MethodMarks { by_unit, groups }`：条目名 `{Type}.{method}#k` 的 `{Type}` 是**定义侧**裸名且不带单元，故用记录方单元的**类型名面**（花括号项 `use M.{T}`、内联别名目标名、模块限定访问 `ns.T`；全部复用 `collect_module_names` + `resolve_import_module` 单点，不另起路径解析）算「该单元能拼出这个名字的模块集合」，条目归入集合内各单元的 `by_unit`；有决议记录的**名字组**（全局 `groups`）停用池的名字级豁免——命中本单元声明的活、未命中的如实报 W1005。**对外可观察变化**：① 跨单元重载决议此前不生效、退回跨单元单值绑定表（同名定义 last-wins）⇒ 多候选同名方法的调用报 `error[E1002]`（实测 `Expected type 'string', found type 'int64'`），修复后按实参逐候选决议（同夹具实测 `Type check passed`；integration 用例 `test_multifile_cross_unit_method_overload_resolves_by_argument_type` 钉之，关掉注入面即翻红于同一条 E1002）；② 跨单元同名同序的**未调用**方法在**可达形态**（调用方与定义方同处编译集）下开始如实报 W1005——三文件夹具实测 `a.Temp.to`（有调用点）不报、`a.Temp.other` 报、**`b.Temp.to` 报**（修复前同一夹具只报 `a.Temp.other`）。**残余边界（登记，宁漏报方向，不得宣称全域）**：单独 `check <定义文件>`（调用方不在单元集 ⇒ 无决议条目 ⇒ 回落名字级豁免）**不报**；同一裸类型名被多个模块带进作用域（名字层已歧义，入口另报 E5006）不报；调用方拼出类型名却从不调用其方法（如只写 `b.Temp(..)` 构造）不报；内联别名 / 模块限定构造不注入候选面（调用点键仍是定义侧裸名 ⇒ 同名类型的 `#k` 精确性以「记录方能拼出的模块集合」为界）；整体导入 `use M` 不让裸类型名进作用域、std 模块候选面未接入，两形态都不产条目。驱动侧读取为**纯读**（经 `OverloadResolutions::iter`，不触达 `get`、不计入消费台账 ⇒ 不掩盖 W1090「消费必达」结算；驱动级钉 `test_resolved_method_marks_read_leaves_consumption_ledger_untouched`）。IR 快照 204 条零漂移；语料基线 346 → 348 行（两笔各多 1 条 behavior 记录：`tests/yaoxiang-multifile/ref-pool-cross-unit-method/` 与 `.../ref-pool-cross-unit-name-owner/`，其余逐字节不变）。

- **重载方法逐个候选择优：未被重载决议命中的兄弟重载恢复报 W1005（WBS 4.9.6 修复轮 / issue #450；作用域边界见 WBS 4.9.7 / issue #464）**：上一版把方法可达性改成**短名探针**，同名重载共享短名 ⇒「一个被调用，兄弟全活」——`Temp.to` 两个重载只调用 `->Float` 时 `->String` 兄弟不再报 W1005。现改为**逐候选归因**：可达性消费 D20 决议产物（`TypeCheckResult.overload_resolutions` 的**纯读**枚举，不触达 `get`、不计入消费台账 ⇒ 不掩盖 W1090「消费必达」结算；`test_resolution_iter_is_read_only` 钉之）；决议名形如 `{Type}.{method}#k`（不带模块前缀，唯一拼接限定名的 `SymbolTable::qualify` 不喂决议表），在 L1 解析成 `(基名, 声明序)` 交给分析器（L2 不知道 IR 名，无 L2→L3 依赖）。实测（同一夹具双二进制）：冻结版 = 0 条 W1005，修复后 = 恰 1 条（调用 `->Float` 时报 `->String` 兄弟 :7；调用 `->String` 时报 `->Float` 兄弟 :3——行号方向即声明序方向）；驱动级钉 `test_partially_called_overload_group_reports_the_unhit_sibling`。**作用域边界（WBS 4.9.7 / issue #464）**：本修复只在**同单元**内逐候选成立——**跨单元（registry 解析）的方法调用不产生决议条目** ⇒ 该名字组回落到池的名字级豁免（`RefPool::field_names` 全局平铺）⇒ **跨单元同名同序的方法此前不被报**，该缺口由 WBS 4.9.7 / issue #464 收口（见本小节上一则）；另有回落边界（本单元无任何决议记录时回落短名共判，宁漏报）。语料基线零漂移、IR 快照零漂移。

- **死代码分析：被调用的方法不再误报 W1005；同名重载各自独立判定（WBS 4.9.6 / issue #450；2026-10-16 用户裁决 A）**：可达性扫描按**限定名**查引用（`Point.render`），而引用点收集到的是**短名**（`obj.render()` 只出现 `render`）——于是被调用的方法永远落不进可达集：无项目根路径下实测 `tests/yaoxiang/02-type-system/functions/method_call_args.yx` 三个方法全被调用却各报 1 条 W1005、`interfaces.yx` / `generic_method_binding.yx` / `interface_dynamic_dispatch.yx` / `05-ownership/*` 同类；同一批定义表还按限定名键控，**同名重载互相覆盖**（`method_overload.yx` 两个重载只报 1 条、`method_overload_ambiguous_err.yx` 亦然）。现改为：定义表**逐定义一条**（方法追加；同名非方法绑定按名字去重保留最后一次——重载是各自独立的定义，顶层重赋值是同一个绑定），可达性扫描对方法改用**短名探针**。自建警告面对拍实测：**−42 条 W1005、0 条新增**（全部是被调用的方法），未被调用的方法照旧如实报（如 `interface_dynamic_dispatch.yx` 的 `Dog.age` / `Cat.age`）；语料基线 9 条 compile-error 条目的 check 诊断各少 1 条 W1005、IR 快照 204 条零漂移。本项属 D 类（删误报即改变诊断对外面），已按用户 2026-10-16 裁决 A 授权。

- **引用池消费判据收紧为「被 import 后真实使用」+ 测试池分离（WBS 4.9.4 / 4.9.5，RFC-029f 改进 4+5）**：包内引用池此前按「名字在别的模块出现过」记账（只剔除模块自有名），两个方向都过宽——① 别的模块 `use b.{foo}` 并调用 `foo()` 时，**同名撞名**会救活 a.yx 里无关的 `foo`；② 只出现在 `use` 项里、从未有使用点的名字（未使用导入，另报 W1003）也算消费。现改为：一条消费记录 = 某模块用 `use` 把名字带进作用域**且在使用点真的引用了它**，并且**只记到该 `use` 解析出的那个模块**（解析复用发现期同一份模块键集合，不另起一套路径解析）；`use lib` + `lib.foo()` 的命名空间形态同样按 import 归因。另：`FileRole::Test` 的引用不再贡献消费面——测试引用单独成池，不豁免生产代码（src/）的定义。对外可观察变化（自建警告面对拍仪器逐条带定义 file:line）：被撞名遮蔽的定义开始如实报 W1001、只被测试引用的生产定义开始如实报 W1001；字段/方法位引用仍按名字维度保守计入（`obj.render()` 只有短名，逐候选归因见 WBS 4.9.6 / issue #450）。语料基线 344→346 行（两条新夹具 behavior run）、IR 快照 204 条零漂移。

- **引用池的来源与身份收敛到 RFC-029f 改进 1+2（WBS 4.9.1 / 4.9.2 / 4.9.3，issue #334）**：池的根按判定三层解析——manifest 声明的 `[[bin]]` / `[lib]` / `[exports]` / `[run].main` 面包目标优先（`collect_package_refs`，`src/driver/helpers.rs:481`，按 `roles::ExplicitSurfaces` 根调 `discover_with_used`，**不扫目录找入口**）；无声明面退化为被检文件自身的编译集（`collect_compilation_refs`，`helpers.rs:454`，与之前 `collect_project_refs` 的退路同形）。池身份由裸名并集升级为 `(module, name)`：类型为 `HashMap<String, HashSet<String>>`（键 = 模块键），装配唯一入口 `collect_ref_pool`（`helpers.rs:429-449`），消费端在 `src/driver/arms.rs:603-609` 的 `other_module_refs` 把「除本单元外」的全部模块条目并集后送 `DeadCodeAnalyzer`，模块自有名（顶层定义与局部绑定，`use` 导入名例外）不再算作对别的模块同名定义的消费。`FileRole::Script` / `Test` 不接池（`arms.rs:611-623` 的 `match us.role` 默认臂，按文件内定义-使用图判定）。**对外可观察变化**：`check <非入口文件>` 时该文件作为检查面、其消费面之外的同模块定义会如实报 W1001 / W1002 / W1004 / W1005——`cross-file-method/lib.yx` 增 3 条（Point@2 W1002、get_x@4 W1005、norm_sq@8 W1005）、`use-type-and-function/lib.yx` 增 2 条（distance@4 W1001、Point@2 W1002）、`use-constant/lib.yx` 增 1 条（value@2 W1004）、`subdirectory-module/util/math.yx` 增 1 条（double@2 W1001）、`type-error-in-lib/lib.yx` 增 1 条（value@2 W1004）——共 **8 条**（按夹具聚合口径，对拍仪器 `warn_surface.py` 331 文件 symmetric diff 在 `target/yx-scratch/yx49/warn_compare_v8.json`）。成因 = 这 8 条夹具均**未声明包目标** ⇒ 4.9.1 根退化为被检文件自身编译集，消费者 main.yx 不在其中（before 时这些文件的 check 面本就零警告，非豁免路径——是设计内的「无包目标」退化行为）；仓库根 `yaoxiang.toml` 为空文件 ⇒ 同一退化路径实测成立。`check <入口文件>` 与多文件夹具（RFC-029 项目编排、`yaoxiang check` 入口）行为不变。`yaoxiang run` 入口不进入 `compile_project` 的池装配路径（仅 standalone 退路），其对单文件 Script 的 W1001 报告集合与 4.9.1 前一致。**与 v0.8.4（RFC-029g）衔接**：v0.8.4 删 `pub` 后已改走「引用池判定」豁免口径；本轮把池的来源与身份两轴收为可独立裁决，4.9.4（消费细化）/ 4.9.5（测试池分离）亦于同批次收口（见上一条）。

#### 类型检查与实参绑定

- **命名实参绑定判据统一（WBS 3.14 / 3.16，RFC-039 D60，issues #452 / #454）**：方法级重载决议与 stdlib 原生导出收敛为同一套六条绑定规则——位置实参按声明顺序占值形参槽位、命名实参按形参名绑定（与书写顺序无关）、位置与命名落同一槽位 / 命名写在位置之前 / 同一形参重复命名三类冲突一律编译期报错、命名实参计入重载候选匹配。原先打到原生导出（如 `io.format_fallback(value = 42, type_name = "int")`）的命名实参被整体静默丢弃（输出 `()`、未知形参名零诊断）；未知名 → E1014（附可用形参表）、缺参 → E1010、重复命名 → E1015、同槽/顺序非法 → 新对外码 **E1111（argument_order_invalid）**。纯位置实参调用零影响；语料 342 行与 IR 快照零漂移。

- **方法绑定 panic 消除（WBS 3.13 / issue #451）**：方法组当值再绑定、无签名标注两形态由编译器 panic（exit=101）转为编译期响亮拒绝 E1110。

- **方法重载 + 命名实参混合调用：从「静默算错」变为编译期报错**。`Type.method(位置实参, 名字 = 值)` 此前会丢弃 typecheck 已决议出的候选、回落到定义序最后的绑定（实测：`r: Int = t.pick(1, a = 2)` 编译通过但输出 `1` 而非 `20`）；现由跨层契约的**消费必达结算**拦下，编译期报 `error[W1090]`（该码自 4.3.2 起为 error 级）。健康程序零输出、零影响；终态语义（命名实参参与候选匹配）见 WBS §3.14 / issue #452。

- **所有权层输入降级加 W1082 诊断（WBS 4.4.3 / D59 C 类）**：`body_checker` 为 None 时所有权层拿到空类型账本 + 空调用点所有权表，分析退化为「什么都不冲突」——此前**无任何警告**。现收敛为唯一判据点 `TypeChecker::ownership_inputs`：缺失时返回空输入（保守方向不变）并经既有 warnings 通道发射 **W1082（所有权分析降级）**——不阻断编译、不混入 diagnostics。W1082 与既有 W 码事实均不重叠（W1080 是「约束降级为运行时检查」、W1081 专指 SMT 求解器缺失、W1090 方向相反），W1xxx 族计数 10 → 11。D59 **C 类**（静默不动作）登记：降级分支在生产路径上不可达（`init_body_checker` 先于所有权阶段执行），健康程序零新增诊断——语料 348/348 行零 diff 为证。

#### LSP 语义数据

- **LSP 语义数据与诊断同一次编译，项目内跨文件跳转/重命名/引用查找自静默落空转为可用（WBS 4.6.1，RFC-039 D55 收口）**：此前 didOpen/didChange 触发两趟编译——诊断走统一 Driver 管线，语义数据走 LSP 的手工 lex→parse→单文件 typecheck（无模块注册表，项目内 `use` 解析失败、跨文件引用事件不产生，跳转/重命名/引用查找静默返回空）；现两趟并为一趟（`analyze_source_in_project` / `lsp/analysis.rs`），跨文件引用的 `resolves_to` 指向定义文件的真实位置，未打开的文件的顶层定义经注册表签名面注入（`external_definitions`）使跳转照样命中；std/native 导入无源码定义位置，维持落 use 行的既有语义。编辑器内附带改进：词法/类型错误的文件不再丢失代码染色与跳转（残缺 AST 染色保留）。命令行编译与 `yaoxiang check` 的诊断集、语料 342 行、IR 快照全部零漂移。

- **LSP 跳转/悬停/重命名对标识符任意光标位置生效（WBS 4.6.2，2026-10-16 用户裁决）**：此前查库键直接用光标位置，而 `resolve_reference` 按引用起点精确匹配——光标落在标识符中部时跳转定义/悬停/查找引用/重命名全部静默不动作；现四个 handler 统一改用光标下标识符**自身的起点**作查库键（与语义事件同坐标系，不经 UTF-16 换算），起点与中部同效。匹配语义仍是 D55 precise-only（数据缺失返回空，不设按名兜底），只修传参。

- **LSP 的 POSIX 绝对路径换算修正（Linux 上语义功能整体落空）**：`path_to_uri` 把 POSIX 绝对路径（`/tmp/a.yx`）的前导 `/` 与 `file:///` 前缀叠加，产出四斜杠非法形式 `file:////tmp/a.yx`，归一后的 db_key 与客户端 URI 全不匹配 ⇒ Linux runner 上 LSP 语义功能整体落空（CI run 38852956374 的 Test / MSRV 红，21 枚既有钉板挂；Windows 因 `uri_to_path` 的去斜杠分支不现形）。现剥除 POSIX 绝对路径的前导斜杠，并补 `cfg(not(windows))` 的 POSIX 换算与幂等断言（此前 `path_to_uri` 只被 `cfg(windows)` 测试引用，Linux 上 import 未用、`clippy -D warnings` 报 error）。

#### 运行期、命令行与资源守卫

- **standalone `run` 与 `check` 统一编排（WBS 4.10.3）**：无 `yaoxiang.toml` 的文件直接 `yaoxiang run` 时，相对 `use`（如同目录 `use b.{f}`）由报 E5001 改为与 `yaoxiang check` 一致解析并真实执行（4.2.5 裁决 A 的 run 半边）；运行期错误的栈帧函数名按单元键限定（如 `main` → `main.main`、`f` → `stack_overflow_e6004.f`），与多文件项目 run 同口径。Script 语义不变：顶层语句即程序主体、不自动执行 `main`、#413 提示保留（键控改由项目探针驱动）。已知缺口：约束泛型运算符派发在**类型与泛型定义不同模块**时仍运行期 E6006（MultiFile 预存在缺陷，登记 WBS §3.18，待 P6 类型身份收敛）。

- **`check` 的显式路径不再被默认排除表拦下，`--exclude` 不再恒空（issue #463）**：此前 `yaoxiang check` 指向 `target/` 下的显式路径一律报「No .yx files found in provided paths」并以 exit=2 退出——默认排除表把用户**显式给出**的路径也一并排掉。收窄为「默认排除表只作用于遍历到的子项」（新增 `is_default_excluded_child`，只看子项自己的 file_name、不看祖先路径），显式路径只套用户给的 `--exclude`；顺路修 `--exclude` 的**静默失效**（D59 C 类）——`normalize_exclude_paths` 改走 `safe_canonicalize`，与 `is_user_excluded` 的比对基准同源；此前 `cwd.join` 拼出的平面路径与 canonicalize 产出的 verbatim 前缀路径 `starts_with` 永不成立，排除表恒空。判据 `tests/integration/cli_e2e.rs::test_e2e_check_explicit_path_under_target_is_honored` 覆盖四形态（显式文件可跑 / 显式目录可遍历 / 嵌套 `target/` 子目录仍跳过 / `--exclude` 优先零发现），两半各自先红后绿。
- **测试发现认工程目录，工程模块不再当独立测试（issue #457）**：`collect_yx_recursive` 此前按扩展名递归收集 `tests/` 下全部 `.yx`，含 `yaoxiang.toml` 的**工程目录**因此把模块文件也当独立测试目标——模块文件按定义没有 `main`，当独立测试跑必然报「缺少 main 绑定」。现改为：目录含 `yaoxiang.toml` 时只取约定入口 `main.yx`，不递归、也不收集同目录其余 `.yx`，缺入口则不发现任何文件；入口名与 manifest 名收敛到单一来源（`package::manifest` 新增 `ENTRY_FILE` 常量）。实测（`yaoxiang test --parallel`，默认 patterns）：改前 321 passed / 10 failed，改后 321 passed / 0 failed；`tests/yaoxiang`（CI 显式路径口径）311 passed 前后持平。
- **单调化深度上限抬到病态档后仍报 E3005 而非爆栈（issue #447）**：`MonoType` 的渲染、深拷贝与哈希此前每层做一次整请求深拷贝，深度闸门放开后编译器进程自身爆栈。现改为显式工作列表，使 `mono.max_depth` 抬高后仍产出 E3005 并报出真实上限（配置上限 / 当前深度），不再让进程崩溃；Drop 派生与 substitute 递归登记为残留边界。
- **解释器深度守卫可达，错误码跨任务边界保真（issue #448）**：解释型调用改为**显式栈驱动**（压帧后交还外层循环、返回值写回调用者槽位），此前不可达的深度守卫现可触发；`StackOverflow` 携带配置上限，**E6004 不再恒报 limit 为 0**，回溯按首尾加计数省略回显。另新增 `TaskFailure` 承载任务错误，跨任务边界按值还原 `ExecutorError`，Standard 模式错误码保真（此前任务边界会吞掉错误码）。

#### 依赖与供应链

- **依赖安全**：rustls 升级 0.23.45 → 0.23.46（RUSTSEC-2026-0346/0347/0348/0349 四条公告，`cargo audit` 恢复 exit 0，issue #455）；基准运行器的配置格式由 YAML 改为 TOML（`benches/shootout/bench.yaml` → `bench.toml`），解析改用本就在依赖树内的 `toml`——`serde_yaml` 0.9（deprecated）与它的维护分叉 `serde_yaml_ng` 都依赖 `unsafe-libyaml`（RUSTSEC-2024-0449，unmaintained），此链随之整体移除，`cargo audit` 由「1 allowed warning」转为零告警；迁移前后两种格式解析出的清单经单元测试逐字段比对一致。

- **依赖告警清零（2026-10-16 补记，承上条）**：docs 站工具链的三条 GitHub 告警与 `tools/yx` 的解包依赖一并收口。① katex 0.16.47 / js-yaml 5.2.3 / smol-toml 1.7.2（前一条是已污染原型下的 `trust` 绕过，后两条是二次方解析）——`mermaid` 抬到 12.2.0（该版已把 katex 约束改为 `^0.18.0`），其余由 `docs/pnpm-workspace.yaml` 的 overrides 拉到 katex 0.18.2 / js-yaml 5.4.1 / smol-toml 1.9.0；`markdownlint-cli` 已是当前最新版且用 `~` 把解析锁在补丁位，无从靠升级上游解决。② `zip` 8.6.0 → 9.0.3：**8.x 全线已被 crates.io yank**，`cargo update -p zip` 直接报 `failed to select a version for the requirement zip = "^8"`——锁里钉着的旧版只能靠既有 Cargo.lock 存活，重生成锁或新环境无锁构建都会失败；且 zip 9 的 `enclosed_name` 由「拒绝 `..`」改为「规范化掉 `..`」（实测 `pkg/../evil.txt` 得 `Some("evil.txt")`，条目被静默改写到另一个落点），故解包路径改用与 tar 侧同一个 `entry_path_is_safe` 判据拒绝，并补两条此前完全缺失的 zip 解包测试。

### ✨ 新功能

- **跨层义务账本、运行期结算与静态门禁（WBS 4.3.1 / 4.3.2）**：新增 `src/driver/obligations.rs` 的义务清单（定义 / 生产 / 交付点 / 消费点的 file:line）、`DeliveryLedger` 交付标记与唯一结算点的 `assert_drained` 三档结算、`DriverOutcome` 审计面；配套 `scripts/ci/check-obligations.py` 接入 CI 独立 job（含正例 / 反例自检与陈旧检测），「加字段忘登记」「删字段忘同步」「登记缺理由」「两表重叠」「登记锚点失效」五类失配当场红。门禁的锚点存在性校验在首跑即抓到登记里裸写文件名的真实缺陷。
- **跨单元方法调用产生 D20 决议条目（issue #464）**：重载决议此前只在**同单元**成立——跨单元（registry 解析）的方法调用不产生决议条目 ⇒ 候选组回落名字级豁免 ⇒ 跨单元同名同序的未调用方法不被报、跨单元重载退回单值绑定。现让候选组（候选 / 混编 IR 名 / 声明期形参名三张同序表）随 `ModuleInfo.method_overloads` 跨单元传递，注册表按模块键升序确定性合并，Typecheck 臂按本单元 import face **逐对**取候选（只注入「键的提供模块 == 该**类型名前缀**在本单元 scope 里的提供模块」的候选组；自有键位不外并、同名冲突整名跳过）。

### 🐛 Bug 修复

- **CI 被 OOM 杀掉：慢用例降档并隔离 5000 档（issue #449）**：实测 5000 档 72.6 s / 峰值 15.6 GB，16 GB 托管 runner 被 OOM 杀（CI run 38266710895 与 38332939256 的 Test / MSRV Check 两次都死在这条用例上，表现为作业「被取消」，其余四个作业全绿）。默认套件档改为 `PATHOLOGICAL_CI_DEPTH = 2600`（越过修复前约第 2500 层的原生栈崩点，判据不缩水；实测 6.4 s / 4.9 GB），5000 档保留为 `#[ignore]` 显式慢速用例并在测试头写明运行命令与代价。两档断言等价：都断言 E3005 报出「配置上限 / 当前深度」且 mono 失败不产 IR；只改测试，无生产代码语义变化。
- **VS Code 语法高亮副本再生（`pub` 不再是关键字）**：docs 语法副本中残留 `pub` 关键字高亮，与 canonical source 不一致（canonical source 已在 v0.8.4 修正，本次补齐当时漏再生的生成物）——`pub` 落到普通标识符着色，匹配 RFC-029g 的「`pub` 不再是关键字」。
- **基准图表页改放 VitePress 真正的 public 目录**：图表页此前落在非 `public` 的目录，产物不随站点拷贝，页面取不到图。

### ♻️ 重构优化（无行为变化）

- **SMT 求解器获取面统一为进程级共享单例（WBS 4.4.2 / RFC-039 D58）**：`predicate.rs`、`checker.rs`、`ownership.rs` 三个 SMT 消费点的获取面收敛到 `proof/smt/backend.rs` 的进程级共享单例与唯一闭包口 `with_shared_solver`，`default_solver()` 在生产路径的调用点降为 0（仅留测试与独立场景）。`with_shared_solver` 检测同线程重入并 panic（消息含 D58），把「Mutex 不可重入、嵌套即死锁」变成可诊断失败；新增 `shared_solver_cache_stats` / `shared_solver_consumer_stats` 观测面（一次编译实测三点命中 / 未命中之和 4/4 与共享实例累计 4/4 一致）。
- **实参绑定判据收敛为单一实现（WBS 3.16，RFC-039 D60）**：方法级重载决议与 stdlib 原生导出共用 `arg_binding::bind_args_to_params` 单一判据（四个调用点），原生臂与方法臂不再各持一份副本；原生导出的**可选位**判据与形参名同源（`signature.rs:236` 唯一出口），修掉「原生可省略位」回归——原先成功程序被误报 E1010。
- **清账 3.4.4 / 3.4.5 零消费者跨阶段表示（红名单 3 → 1）**：`variant_ctor_calls` 与 `method_overload_ir_names` 两个跨阶段镜像删除——前者改由 IR 侧就地做同一 AST 形态检测（span 键跨系统不可靠：两端对 Call 节点 span 语义不同），后者改由 `overload_resolutions` 承载、定义侧混编名由 `method_def_ordinals` 按同一 AST 序自算。零读者直证：`variant_ctor_calls` 在 `src/middle` 0 命中、`.variant_ctor_calls` 全仓 7 处全为写侧，成员访问清零；`method_overload_ir_names` 在 `src/middle` 搜索为空。类型推断与诊断不变，两处用例改写为同等强度的类型级断言。
- **删死 API 与零消费者转发面**：`infer_expression`（全仓零调用点，且不回流 `overload_resolutions`，接线即成 D20 同类缺口，issue #453 / WBS 3.15.1）、`ReleasePlan` / `OverloadResolutions` 上的 `consumed_len` 转发（零调用）、`ObligationSettlement::is_drained`（唯一读取面在测试内，5 处断言改写为 `settlement.findings.is_empty()`，与原实现逐字等价）、`ExecutionState::call_depth` 派生镜像与其唯一写入口 `sync_depth`（BREAKING：仓内无经 `Executor::state()` 读取该字段的调用点），以及 `src/driver/helpers.rs` 的 IR 名前缀剥离死分支（决议名永不带定义单元前缀，删除前后同一夹具集的诊断集合逐条相同，Compare-Object = 0 行）。

### 📝 文档与治理

- **层序声明按实际调用点收口（WBS 4.4.1 / RFC-039 D7）**：`src/frontend/core/typecheck/layers/README.md` 的调用序表按现树复核（精化走查 `predicate` → `termination` → `ownership` → 类型约束求解），行号全部重取，定位改以**函数名 / 驱动点**为锚（不再引用提交号）；`02` / `06` / `01` / RFC-039 四份文档中「声明的层序与实际相反」这类现在时陈述改写为事实，RFC-039 D58 行的口名就地写为 `with_shared_solver(consumer: SolverConsumer, f)`。
- **三处「声明与事实不符」按现树改写（WBS 4.4）**：`typecheck/mod.rs` 的「有序证明层」注释、`proof/smt/backend.rs` 的消费点标签段、`02-stage-contract.md` 的 wasm 现状计数，统一改写为与事实一致的表述（`src/` 29 个文件出现 `wasm32` 字面量 / 22 个带实际 `#[cfg(...)]` 属性 / 7 个仅文档提及），并把过期的 file:line 锚点按现树重取。
- **WBS 勾选与批次实录登记**：4.3 / 4.4 / 4.5 / 4.6 / 4.9 / 4.10 各批次的 09 勾选与 09a 实录、错误码分族页的逐轮再生（补 E1110、清发射点计数漂移）、RFC-036 发现语义补点、RFC-039 D61 登记与三处审计登记。
- **仓库治理条目固化**：AGENTS.md 团队编排约定新增第 10 条 ⑦（提交期间的 stash 窗口操作约定）⑧（推送与同步归属可核）⑨（文档提交必须本地真构建一次）、第 11 条（写入方负载上限，含 compact 失效条件）、第 13 条（不等 CI）与两条元规则（文档不得出现私有内容；先查证再断言）；「更正就地写入正文，不留新旧对照」与「契约 `inScope` 一律写可被路径匹配器解析的路径前缀」两条规则同步落到 `coding-rules.md` / `07-equivalence-oracle.md` / RFC-039 决议登记。
- **发版前审计登记（v0.8.4..HEAD）**：12 条三级任务登记，逐条带 D59 错误类型、归属阶段与「是否阻塞主线」两个必填字段，挂 issue #465–#477；4 个二级任务重开（新增三级后该二级不再完成），新增二级 3.20 与 S9。本批只登记，不改任何 `src/` 代码。

### 🔧 工具链、CI 与测试基建

- **错误码文档生成器幂等门禁**：docs-truth job 新增步骤——跑生成器后 `git diff` 必须为空，覆盖生成器声明的全部 11 个输出文件（error-code / warning-code / package）。动机：`check-docs-truth.py` 只对账「码有无页面」，对不到每码发射点计数，故另立一条门禁。
- **批次验证树唯一入口 `scripts/dev/scratch.py`**：验证与复审的独立树统一由该脚本管理（`use` / `release` / `build` / `status` / `gc`），并新增 `trial` 子命令——「故意改坏」写成补丁、执行后机械可逆还原（`git apply` → 跑命令 → `git apply -R` → 逐文件比对 blob 与 HEAD 一致并要求 `git status` 干净），报告里给补丁路径与还原前后 blob。
- **语料夹具按类别分层**：02-type-system 与 06-compile-errors 的 212 个夹具拆入按类别划分的 23 个子目录，运行期失败夹具归入 `runtime` 子目录；集中搬运 88 个 IR 快照并改写全部引用点，同提交再生语料基线与受路径长度影响的快照（逐行核对只有 span 偏移变化）。
- **依赖与工具链升级**：docs 站 npm 依赖组 9 项、生产依赖组 4 项由 Dependabot 统一抬升；`source-map-js` 1.2.1 → 1.2.2。

### 🛠️ 基准与性能追踪

- **跨语言对照定口径**（2026-10-15）：同一问题、同一算法、同一操作序列，**复杂度必须对齐**——否则量到的是复杂度差异而不是语言差异。据此把 `list_ops` / `string_concat` 补到 3 门语言（yaoxiang / python / rust），并把 yx 侧 `list_ops` 的建表由 `nums = nums + [j]`（每次复制整个列表 → O(n²)）改为 `list.push`（摊还 O(1)）：本机 input=100 平均值 1.25s → 中位数 488ms。对照实现里有两处刻意为之的「反惯用」写法（python 不用 `+=`、rust 显式 `.clone()`），理由写在各自文件头注——不这么做，rust/python 的读数会凭复杂度取胜。

- **性能历史入库**：nightly 的两套基准结果（Criterion 自身微基准、shootout 多语言横比）每次追加进 `benches/history/*.jsonl` 并推回 `dev`。此前只挂 artifact（保留 30 天），而 nightly 的 Pre-release 每晚被删掉重建——超过 30 天就没有「上一夜是多少」可比。数据带环境归因（工具链版本 / CPU / 分支 / 提交），同一 (run, attempt) 幂等（nightly 重跑是替换而非重复追加），趋势图由 `scripts/ci/bench-history.py --render-chart` 派生到文档站 `/perf/`（数据落 `docs/src/public/`——VitePress 真正会拷贝的 public 目录）。

- **基准 job 改测 `dev`**：nightly 的 schedule 事件走默认分支 `main`，而代码落在 `dev`（`main` 上次变更是 2026-10-12，其后八个 nightly 测的是同一棵树，趋势图会是一条直线）。基准与 shootout 两个 job 现显式 `checkout dev`，报告头写明「本次基准测于 dev@<sha>」；release 构件仍来自默认分支，报告里显式区分这层归属。

- **横比专业化**：头条数字由均值改为**中位数**（墙钟样本右偏，一次调度抖动就能把均值拉高）；报告新增「相对最快」倍数与跨测点的 geomean 比值；每门语言记工具链版本、每次运行记机器与 CPU；单语言测点显式列出（无横比对象）。

- **正确性判据**：输出与配置金标不符、或同输入下跨语言输出不一致，一律以**非零码失败**——算错的程序再快也不是读数，不该混进报告与历史。既无金标又只有一门语言的测点显式标为「未校验」，不默认通过。
