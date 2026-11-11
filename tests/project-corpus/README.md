# 项目形态判据语料（tests/project-corpus/）

`scripts/ci/check-soundness.py` 的**项目形态判据面**。每条语料是一个含
`yaoxiang.toml` 的项目目录，入口约定 `main.yx`（与
[tests/yaoxiang-multifile](../yaoxiang-multifile/README.md) 同一套约定）。

判据见 [test-specification 规则 9.3](../../docs/src/dev/test-specification.md)：
`check` 退出 0 之后，`run` 不得报出与编译期同族的错误（E0–E5）。下面三套各钉一处
「check 与 run 必须同口径」的路径：

| 夹具 | 现象 | check | run | 码族 |
| --- | --- | --- | --- | --- |
| `e3022-entry-signature/` | bin 项目 `main` 带参（入口签名不符） | 1 | 1（E3022） | E3 |
| `e3020-missing-main-binding/` | 有 manifest 但未声明 `[run] main` 且无 `main` 绑定 | 1 | 1（E3020） | E3 |
| `e3005-mono-max-depth/` | `[mono] max_depth = 1`：两条路径都必须按上限 1 报 E3005 | 1 | 1（E3005） | E3 |

三套在 2026-11-06 全部修复（E3005：`[mono]` 贯通到 check 链；E3020 / E3022：入口
契约判定补进 check）——check 与 run 已同口径，它们转为**回归守卫**：任一处判据被
退回，check 会掉回 0 而 run 仍报同一个码，探针就把对应夹具重新记成分叉。

## 为什么单独成层

`tests/project-corpus/` **不进语料差分基线**（`examples/corpus_probe.rs` 只扫
`tests/yaoxiang`、`src/std/tests`、`tests/yaoxiang-multifile`），也**不被**
`yx_multifile_runner` 收集（它只扫 `tests/yaoxiang-multifile/`）。原因：这三套编码的是
「当前实现的分叉」，写进行为语料等于把违规录成期望行为——夹具只由健全性探针消费。

因此这三套的头部**没有** `// expect:` 指令。它们断言的不是「应该报这个错」，而是
「check 与 run 两条路径不得矛盾」——分叉存在时探针必须报出来，同口径时探针放过。

## 修复后怎么退役

三处实现修好后，探针的三处分叉归零，夹具**转换而不是删除**：把各套的期望改成
「`run` 也报同一个码」（check 与 run 一致），再移进 `tests/yaoxiang-multifile/`
作为行为语料；在此之前它们只服务 `check-soundness.py` 的判据面。
