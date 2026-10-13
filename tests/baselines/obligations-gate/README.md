# 反例/正例夹具（check-obligations.py 自检用）

本目录是 `scripts/ci/check-obligations.py`（WBS 9.2 义务字段静态门禁）
自带的**五类夹具**：门禁必须对四个反例判红、对正例判绿
（`python scripts/ci/check-obligations.py --selftest`）。没有这些夹具，
「门禁能抓到零消费字段 / 清单不同步」就只是实现者的自述。

| 夹具 | 形态 | 期望 | 命中的判据 |
| --- | --- | --- | --- |
| `negative/` | 幽灵义务：定义 + 写入但零读取、未登记红名单 | 红 | 判据 3（缺交付标记）+ 判据 2（消费者文件无真读取） |
| `positive/` | 正例：有交付标记 + 真实读取 | 绿 | — |
| `stale-manifest/` | **代码删了清单没同步**：结构体已无该字段，INVENTORY 仍登记 | 红 | 判据 1（INVENTORY ⊆ TypeCheckResult） |
| `stale-struct/` | **清单删了代码还在**：INVENTORY 少登记，结构体与文档表仍在 | 红 | 判据 1（文档表 ↔ INVENTORY 对账 + 表头条数） |
| `stale-red-list/` | **红名单陈旧**：登记为「零消费者」的字段长出了真读取 | 红 | 判据 4（红名单陈旧检测） |

`negative` 与 `positive` 的差别只在两处（其余逐字相同）：消费者文件里有真读取、
交付点有交付标记——任一缺失都必须红（这正是「摘掉任一交付标记必红」的静态侧证据）。

`stale-manifest` 与 `stale-struct` 钉的是**清单同步**这一施工流程缺口
（t4 R3 / t3 F2 的部分面）：删字段或加字段时忘了同步 INVENTORY / 文档表，门禁必须当场红。
（尚未覆盖的剩余盲区：结构体加了字段但 INVENTORY 与文档表同时漏登记——那种「三处一起漏」
登记为 4.3.2 的门禁加固项。）

夹具内的源码文件用 `.txt` 后缀（避免被 cargo 当编译目标、被 rustfmt 重写）；
门禁的 `find()` 按「先原名、后 .txt」定位。
