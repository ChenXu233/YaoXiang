# 反例/正例夹具（check-obligations.py 自检用）

本目录是 `scripts/ci/check-obligations.py`（WBS 9.2 义务字段静态门禁）
自带的**反例与正例**夹具：门禁必须对 `negative/` 判红、对 `positive/` 判绿
（`python scripts/ci/check-obligations.py --selftest`）。没有这两份夹具，
「门禁能抓到零消费字段」就只是实现者的自述。

两份夹具的差别只在两处（其余逐字相同）：

| 差异 | negative（必须红） | positive（必须绿） |
| --- | --- | --- |
| 消费者文件里有真读取 | 无 | `src/middle/core/ir_gen.rs` 有 `.ghost_field` |
| 交付点有交付标记 | 无 | `src/driver/arms.rs` 有 `deliver(Obligation::GhostField)` |

两处任一缺失，判据都必须红——这正是「摘掉任一交付标记必红」的静态侧证据。

夹具内的源码文件用 `.txt` 后缀（避免被 cargo 当编译目标、被 rustfmt 重写）；
门禁的 `find()` 按「先原名、后 .txt」定位。
