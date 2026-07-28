---
title: 增量チェック
description: YaoXiang check 增量チェックの設計
---

# 增量チェック

## 問題描述

watch モードでは、ファイル変更時に全ファイルを再チェック（全量再検査）し、デバウンスに busy-wait（50ms ごとにチェック）を採用しているため、CPU がアイドル状態になります。

## 解決策

`CheckSession` を使用して増分チェックの状態を管理し、`ModuleDependencyGraph::affected_modules`
を利用して影響を受けたファイルのみを再チェックします。

## 実装フロー

```text
初回チェック：
  全量チェック → 依存グラフ + 各モジュールのチェック結果をキャッシュ

ファイル変更時：
  1. affected_modules(changed_files) → 影響を受けたモジュールを特定
  2. 影響を受けたモジュルのみを再解析・再チェック
  3. キャッシュと依存グラフを更新
```

## CheckSession

```rust
pub struct CheckSession {
    dep_graph: ModuleDependencyGraph,
    cache: ModuleCache,
    all_files: Vec<PathBuf>,
}

impl CheckSession {
    pub fn check_all(&mut self, files: &[PathBuf]) -> Result<CheckResult>;
    pub fn check_incremental(&mut self, changed_files: &[PathBuf]) -> Result<CheckResult>;
}
```

## 既知の制限

- watch モードでは依然として busy-wait デバウンスを使用（`command.rs` の `Instant::now()` +
  `recv_timeout`）
- `check_incremental` 内部では依然として
  `check_files_with_diagnostics`（全量パス）を呼び出しており、真の增量を活用していない

## 今後の作業

- A2/P1：`HotReloader` で busy-wait デバウンスを置き換える
- P2/P3：watch モードで `CheckSession` を導入し、真の增量チェックを実現
- T9：增量チェックの正確性テスト
