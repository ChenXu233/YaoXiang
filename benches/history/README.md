# history/ — 基准性能历史（机器写入，勿手改）

本目录是**性能档案**，与代码同史：每次 nightly 的运行结果追加到这里，图表由它派生。
判据很直接——artifact 只留 30 天、nightly 的 Pre-release 每晚被删掉重建，
两者都不是档案；写进仓库后 `git log -p benches/history/` 能把性能变化与代码改动对应起来。

## 文件

| 文件 | 内容 |
| --- | --- |
| `shootout.jsonl` | 多语言横比：一行一个 `bench × 语言 × 输入` |
| `criterion.jsonl` | Yaoxiang 自身微基准（Criterion） |
| `releases.jsonl` | 发版标记：`version + sha + ts`，用于在趋势图上画竖线 |

JSONL 一行一条、自描述，三种行靠 `kind` 区分：

```json
{"kind":"meta","run":"41","attempt":"1","sha":"abc1234","ref":"dev","ts":"2026-10-16T00:41:00Z","toolchains":{"rust":"rustc 1.98.1"},"cpu":"...","os":"linux"}
{"kind":"row","run":"41","attempt":"1","sha":"abc1234","suite":"shootout","bench":"fibonacci","lang":"rust","input":"20","median_ms":24.3,"mean_ms":24.5,"stddev_ms":1.2,"rel_pct":4.9,"compile_ms":302.9,"n":10,"ok":true}
{"kind":"failure","run":"41","attempt":"1","bench":"matrix","lang":"yaoxiang","input":"20","error":"输出与金标不符：期望 \"0\"，实际 \"1\""}
```

- 头条数字是 **`median_ms`**：墙钟样本右偏，一次调度抖动就能把均值拉高。
- `meta` 行每次运行一条，带**环境归因**（工具链版本、CPU、分支、提交）。读数变了
  却不知道编译器和机器也变了，就无从判断是语言退步还是环境换版。
- 判失败的行记 `kind":"failure"` 而不是静默省略：只记成功的行，历史会读成
  「那晚这门语言没配」，与「挂了」是两回事。

## 谁写、写到哪个分支

`.github/workflows/nightly.yml` 的 `bench-history` job：

1. 两个基准 job 显式 **checkout `dev`**（schedule 事件走默认分支 `main`，而代码落在
   `dev`——照默认分支测，每晚测的都是同一个提交，趋势图会是一条直线）；
2. 结果随 artifact 带到 `bench-history`，由 `scripts/ci/bench-history.py` 追加进本目录；
3. 由 `--render-chart` 派生 `docs/src/.vitepress/public/perf/history.json`；
4. 一笔提交推回 `dev`（`git pull --rebase` 重试，与 auto-translate bot 抢分支时用）。
   提交由默认 `GITHUB_TOKEN` 发出，GitHub 的防递归规则下不会再拉起 workflow run。

**同一 (run, attempt) 幂等**：重跑 nightly 是「替换」而不是「追加」，不会把同一夜写两遍。

## 不要用本机数据往这里写

趋势图的纵轴是墙钟时间，跨机器不可比。本机跑出来的数据（Windows 开发机、
不同的 CPU/负载）混进 CI 序列会让整条曲线失真。本地演练请另指定目录：

```bash
# 本地演练：写 target/ 下，不进仓库
python scripts/ci/bench-history.py --kind shootout \
    --shootout-json target/shootout-results.json \
    --meta target/bench-meta-shootout.json \
    --runs 10 --history-dir target/hist-local
python scripts/ci/bench-history.py --render-chart \
    --history-dir target/hist-local --out target/hist-local/history.json
```

## 图表

- 页面：`docs/src/.vitepress/public/perf/index.html`（静态页，部署后在文档站 `/perf/`）
- 数据：同目录 `history.json`，由上面的 `--render-chart` 生成
- 页面可切换「相对首个点（%）」「对数纵轴」，发版竖线来自 `releases.jsonl`

`history.json` 是**派生视图**：按序列点数截断（默认 720 点）以控制体积，
原始 JSONL 不截断，随时可重建。
