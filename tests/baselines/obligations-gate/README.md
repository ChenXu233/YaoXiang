# 反例/正例夹具（check-obligations.py 自检用）

本目录是 `scripts/ci/check-obligations.py`（WBS 9.2 义务字段静态门禁）
自带的**七类夹具**：门禁必须对六个反例判红、对正例判绿
（`python scripts/ci/check-obligations.py --selftest`）。没有这些夹具，
「门禁能抓到零消费字段 / 清单不同步」就只是实现者的自述。

| 夹具 | 形态 | 期望 | 命中的判据 |
| --- | --- | --- | --- |
| `negative/` | 幽灵义务：定义 + 写入但零读取、未登记红名单 | 红 | 判据 3（缺交付标记）+ 判据 2（消费者文件无真读取） |
| `positive/` | 正例：有交付标记 + 真实读取 | 绿 | — |
| `stale-manifest/` | **代码删了清单没同步**：结构体已无该字段，INVENTORY 仍登记 | 红 | 判据 1（INVENTORY ⊆ TypeCheckResult） |
| `stale-struct/` | **清单删了代码还在**：INVENTORY 少登记，结构体与文档表仍在 | 红 | 判据 1（文档表 ↔ INVENTORY 对账 + 表头条数） |
| `stale-red-list/` | **红名单陈旧**：登记为「零消费者」的字段长出了真读取 | 红 | 判据 4（红名单陈旧检测） |
| `unregistered-field/` | **结构体新增字段未登记**：既不在义务清单也不在非义务登记 | 红 | 判据 1 反向 containment（字段普查必须闭合，F2） |
| `stale-anchor/` | **锚点失效**：登记理由里的 `file:line` 指向不存在的符号 / 行号越界 | 红 | F1 锚点校验（文件存在 + 行号在册 + 符号锚） |

`negative` 与 `positive` 的差别只在两处（其余逐字相同）：消费者文件里有真读取、
交付点有交付标记——任一缺失都必须红（这正是「摘掉任一交付标记必红」的静态侧证据）。

`stale-manifest` 与 `stale-struct` 钉的是**清单同步**这一施工流程缺口
（t4 R3 / t3 F2 的部分面）：删字段或加字段时忘了同步 INVENTORY / 文档表，门禁必须当场红。
`unregistered-field` 与 `stale-manifest` 一起把**字段普查的两个方向**都钉住：
「结构体有而清单无」（反向 containment）与「清单有而结构体无」（正向 containment）。
`positive` 同时覆盖**红名单为空**这一形态（义务清单非空、红名单零条、全部有交付标记与真读取）。

> 注 1：本目录的 mini 结构体都必须自带一条「非义务字段登记」（`fixture_meta`）——门禁判据 1
> 现要求 `TypeCheckResult` 字段集 = 义务清单 ∪ 非义务登记，缺一侧即红。
>
> 注 2：每个夹具的 mini `02-stage-contract.md` 里「定义行」列必须是该夹具
> `types.rs.txt` 的**字段声明行**——门禁 F4 会逐行对账（写错即红），
> 与真实仓库同一约定（见 02 表注）。

夹具内的源码文件用 `.txt` 后缀（避免被 cargo 当编译目标、被 rustfmt 重写）；
门禁的 `find()` 按「先原名、后 .txt」定位。
