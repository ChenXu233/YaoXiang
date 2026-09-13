---
title: 'インクリメンタルチェック'
description: 'YaoXiang check インクリメンタルチェックの設計'
---

# インクリメンタルチェック

## 問題の説明

watch モードでは、任意ファイルの変更で全ファイルを再チェック（フル再チェック）し、デバウンスに busy-wait（50ms ごとにチェック）を使用しており、CPU が空回りする。

## 解決方案

`CheckSession`
を使用してインクリメンタルチェック状態を管理し、`ModuleDependencyGraph::affected_modules`
を活用して影響を受けるファイルのみ再チェックする。

## 実装フロー

```text
初回チェック：
  フルチェック → 依存関係グラフ + 各モジュールのチェック結果をキャッシュ

ファイル変更：
  1. affected_modules(changed_files) → 影響を受けるモジュールを特定
  2. 影響を受けるモジュールのみ再解析・再チェック
  3. キャッシュと依存関係グラフを更新
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

- watch モードは依然として busy-wait デバウンスを使用（`command.rs` 内の `Instant::now()` +
  `recv_timeout`）
- `check_incremental` 内部は依然として
  `check_files_with_diagnostics`（フルパス）を呼び出しており、実際にインクリメンタルを活用していない

## 今後の作業

- A2/P1：`HotReloader` で busy-wait デバウンスを置き換え
- P2/P3：watch モードで `CheckSession` を統合し、真のインクリメンタルチェックを実現
- T9：インクリメンタルチェックの正確性テスト
