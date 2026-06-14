---
title: "RFC-029: モジュール意味論システム"
status: "草案"
author: "晨煦"
created: "2026-06-13"
updated: "2026-06-13（孤児規則/一貫性チェックを削除、モジュールのコンパイルパイプライン統合に焦点を絞る）"
---

# RFC-029: モジュール意味論システム

## 概要

モジュールシステムをコンパイルパイプラインに統合し、複数ファイルコンパイル、モジュールレベルの可視性制御、ホットリロードを実現する。**孤児規則や一貫性チェックは導入しない**——YaoXiang の trait は構造的型（RFC-011 §2.1）であり、Rust スタイルの名目的な impl 帰属追跡を必要としない。

## 動機

### 現状の問題

モジュールシステムの**物理層**（読み込み、解析、キャッシュ、依存関係グラフ、ホットリロード）は既に完全に実装されている（`frontend/module/`）が、**コンパイルパイプラインには統合されていない**：

- `pipeline.rs` は単一のソース文字列のみを受け付け、複数ファイルプロジェクトをサポートしていない
- `use` 文は型チェック時にモジュールを実際に読み込めない
- `ModuleCache`、`HotReloader`、`VendorLoader` は実装されているが呼び出し元がない
- 標準ライブラリの native 関数は `ModuleRegistry::with_std()` でハードコード登録されており、汎用モジュール読み込みパスを通っていない

### なぜ孤児規則が不要なのか

RFC-011 は trait を構造的型として定義している：

```yaoxiang
Clone: Type = { clone: (Self) -> Self }
```

- **`impl Trait for Type` がない** — メソッドは型に直接定義される
- **孤児規則がない** — どのモジュールも自分の型にメソッドを追加できる
- **一貫性チェックがない** — メソッドは型構造の一部であり、名目的マッチングによらない

したがって `TraitImplementation` には `defined_in` や `module` フィールドは不要である。関連する issue #46 と #73 は既にクローズされている。

## 提案

### コア設計

**二層統合**：

```text
┌─────────────────────────────────────────────────┐
│  コンパイルパイプライン統合 (Pipeline Integration) │  ← 新規：複数ファイルコンパイル、モジュール読み込み
├─────────────────────────────────────────────────┤
│  可視性 (Visibility)                              │  ← 新規：pub / デフォルト（モジュール内可視）
└─────────────────────────────────────────────────┘
```

### 1. 複数ファイルコンパイル

コンパイラのエントリポイントを単一ファイルからプロジェクトディレクトリに拡張する：

```rust
/// プロジェクトをコンパイルする（単一ファイルではなく）
pub fn compile_project(&mut self, project_root: &Path) -> Result<Vec<ModuleIR>, CompileError> {
    // 1. yaoxiang.toml を読み取りエントリーファイルを取得
    // 2. エントリーファイルから依存モジュールを再帰的に読み込み
    // 3. 依存関係グラフをトポロジカルソート
    // 4. 各モジュールを順次コンパイル
    // 5. モジュール間型チェック（use 文の解決）
}
```

統合ポイント：`compiler.rs` に `compile_project` メソッドを新規追加し、内部で `ModuleLoader` を使用してモジュールを読み込む。

### 2. use 文のモジュール解決

現在の `statements.rs` には `ModuleRegistry` があるが、登録クエリのみを行う。実際に読み込むよう拡張が必要：

```yaoxiang
# 現状：use 文は型チェック時にモジュールを見つけられない
use math.geometry.Point  # ❌ ModuleRegistry に math.geometry がない

# 目標：use 文がモジュールの読み込みをトリガーする
use math.geometry.Point  # ✅ ModuleLoader が math/geometry.yx を読み込み、Point エクスポートを抽出
```

実装パス：
1. `use` 文が `ModuleLoader::load()` をトリガー
2. 読み込み結果を `ModuleRegistry` に登録
3. 型チェッカーが `ModuleRegistry` からエクスポート型をクエリ

### 3. 可視性システム

```yaoxiang
# math/geometry.yx
pub type Point = { x: Int, y: Int }       # pub = 他モジュールから利用可
type InternalState = { cache: Int }        # デフォルト = geometry モジュール内のみ可視

pub Point.distance: (self: Point, other: Point) -> Float = {
    # ...
}
```

```rust
/// 可視性レベル
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// 公開 — すべてのモジュールからアクセス可能
    Public,
    /// デフォルト — 定義モジュール内のみ可視
    Module,
}
```

型チェッカーがモジュール間参照時に可視性をチェックする。

### 4. モジュールキャッシュ

`ModuleCache` は既に LRU/TTL キャッシュ戦略を実装済み。コンパイルパイプライン統合後は：
- 初回コンパイル：読み込み + コンパイル + キャッシュ
- 以降のコンパイル：キャッシュヒットならスキップ
- ファイル変更：`HotReloader` が自動的にダーティキャッシュを無効化

### 5. ホットリロード統合

`HotReloader` は既に完全に実装されている（`frontend/module/hot_reload.rs`）が、コンパイルパイプラインへの統合が必要：

```rust
// コンパイルパイプライン起動時
let mut reloader = HotReloader::new(project_root, config, cache.clone());
let mut event_rx = reloader.start()?;

// 非同期メインループ内
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        for module in &event.affected_modules {
            pipeline.recompile_module(module).await;
        }
    }
});
```

## コンパイラの変更

| コンポーネント | 変更内容 |
|------|------|
| `compiler.rs` | `compile_project` メソッドを新規追加 |
| `pipeline.rs` | 複数モジュールコンパイル、モジュールキャッシュクエリのサポート |
| `typecheck/inference/statements.rs` | `use` 文がモジュール読み込みをトリガー |
| `typecheck/mod.rs` | 汎用モジュールパスから native 関数を登録（ハードコードの代替） |
| `frontend/module/loader.rs` | 実装済み、変更不要 |
| `frontend/module/cache.rs` | 実装済み、変更不要 |
| `frontend/module/hot_reload.rs` | 実装済み、変更不要 |
| AST 層 | 型への `pub` キーワード可視性注釈（未サポートの場合） |

## 実装戦略

### フェーズ分割

**Phase 1：複数ファイルコンパイルエントリ**
1. `compiler.rs` に `compile_project(project_root)` メソッドを新規追加
2. `ModuleLoader` を使用してエントリーファイルから依存関係を再帰的に読み込み
3. `ModuleDependencyGraph` でトポロジカルソート
4. 既存の単一ファイルコンパイルフローを順次呼び出し

**Phase 2：use 文のモジュール解決**
5. `statements.rs` の `use` 文が `ModuleLoader::load()` をトリガー
6. 読み込み結果を `ModuleRegistry` に登録
7. エクスポート型が型チェック時に利用可能

**Phase 3：可視性**
8. AST 層で型への `pub` 注釈を解析
9. 型チェッカーがモジュール間参照時に可視性をチェック

**Phase 4：キャッシュとホットリロード**
10. `pipeline.rs` に `ModuleCache` を統合
11. `pipeline.rs` に `HotReloader` を統合
12. インクリメンタル再コンパイルは影響を受けるモジュールのみ処理

### 依存関係

- RFC-014（パッケージマネージャ）— パッケージ名は `yaoxiang.toml` から取得
- RFC-011（ジェネリクスシステム）— trait は構造的型であり、モジュール帰属は関与しない

## 開放問題

- [ ] デフォルト可視性は「モジュール内」か「パッケージ内」か？（Rust はデフォルトでモジュール内、Go はデフォルトでパッケージ内）
- [ ] `pub(crate)` レベルは必要か？
- [ ] ホットリロードはモジュール間の依存関係チェーン再コンパイルをサポートする必要があるか？
- [ ] 複数ファイルコンパイルのエラーレポートをどのように集約するか？

---

## 参考文献

- [RFC-011: ジェネリック型システム](accepted/011-generic-type-system.md) — 構造的型定義
- [RFC-014: パッケージ管理システム設計](accepted/014-package-manager.md) — パッケージ名のソース