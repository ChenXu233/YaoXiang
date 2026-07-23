---
title: "RFC-029: モジュール意味論システム"
status: "草案"
author: "晨煦"
created: "2026-06-13"
updated: "2026-07-14（改稿：互換性セクションを削除、開放問題をRFCの子RFCへ分離）"
---

# RFC-029: モジュール意味論システム

## 概要

モジュールシステムをコンパイルパイプラインに接続し、マルチファイルコンパイルとパッケージレベルの可視性制御を実現する。

**基本原則**：型チェッカーは事前に構築されたモジュールレジストリのクエリのみを行い、ディスクにはアクセスしない。モジュールグラフは型チェック前に完全に構築される。

**含まない**：キャッシュ、ファイル監視、Hotreload、インクリメンタル再コンパイル。これらはコンパイルライフサイクル最適化であり、後続の独立したRFCで扱う。

## 動機

### 現在の問題

1. **コンパイラは単一ファイルのみサポート**：`Compiler::compile(name, source)` はファイル間依存関係を処理できない
2. **エクスポートルールが互いに競合**：型は自動エクスポート、定数は自動エクスポート、メソッドは自動エクスポート、関数は`pub`をチェック——4つの例外系
3. **2つのモジュールリゾルバ**：`frontend/module/resolver.rs`と`package/source/module_resolver.rs`の検索順序が異なる
4. **型チェッカーがファイルロードと密結合**：草案では`use`が型チェック時に`ModuleLoader::load()`をトリガーすることを要求していた

### 設計目標

- 1つのプロジェクトで複数の`.yx`ファイルをコンパイル可能
- `use`文の семантика が明確で曖昧さがない
- 可視性ルールは1つに統一
- 単一ファイルモードは引き続き動作し、`yaoxiang.toml`を要求しない
- 型チェッカーは純粋なロジックであり、ファイルI/Oを実行しない

## 提案

### 1. モジュールのアイデンティティとパス解決

#### モジュールの定義

**モジュール**は1つの`.yx`ファイルである。モジュールパスはドット区切りの名前パスであり、ファイルシステム上の位置に対応する。

```
math.geometry → src/math/geometry.yx
             → src/math/geometry/mod.yx
             → src/math/geometry/index.yx
```

**パッケージ**は`yaoxiang.toml`を持つプロジェクトであり、複数のモジュールを含む。パッケージは唯一の캡슐화境界である。

#### パス解決ルール

検索順序（唯一のルール、既存の2つのリゾルバを置き換え）：

1. **標準ライブラリ**：`std`または`std.*`→ 組み込みモジュール、`ModuleRegistry`からクエリ
2. **vendorディレクトリ**：`.yaoxiang/vendor/<pkg>-*/src/` → 依存パッケージ
3. **現在のファイルの相対パス**：現在の`.yx`ファイルがあるディレクトリ基準
4. **プロジェクトsrcディレクトリ**：`<project_root>/src/`

ファイル配置的試行順序：

```
base/name.yx
base/name/mod.yx
base/name/index.yx
```

最初に見つかったファイルで停止。`name.yx`と`name/mod.yx`が同時に存在する場合、エラー：

```
モジュールパスの曖昧：`math.geometry`が以下と一致：
  src/math/geometry.yx
  src/math/geometry/mod.yx
どちらかを削除してください。
```

#### 統一リゾルバ

既存の2つの`ModuleResolver`を削除。`frontend/module/resolver.rs`を唯一の實現として保持し、`package/source/module_resolver.rs`を削除。`YXPATH`環境変数のサポートは唯一のリゾルバに統合する。

### 2. インポート семантика

#### 構文形式

```yaoxiang
use math.geometry                          # モジュール名前空間
use math.geometry as geo                   # モジュール名前空間エイリアス
use math.geometry.{Point}                  # 選択的インポート
use math.geometry.{Point, distance}        # 複数選択的インポート
use math.geometry.{Point as P}             # 選択的インポート＋エイリアス
use math.geometry.{Point as P, distance as dist}  # 複数選択的インポート＋エイリアス
```

#### семантика

すべてのインポート形式は**コンパイル時の名前解決ルール**であり、実行時の参照コピーではない。インポートされた名前はモジュールエクスポートテーブル内の宣言アイデンティティを指す。

| 構文 | 現在のスコープへのバインド | 使用方法 |
|------|-----------------|----------|
| `use path` | パスの最後のセグメントがモジュール名前空間 | `geometry.Point` |
| `use path as alias` | aliasがモジュール名前空間 | `alias.Point` |
| `use path.{item}` | item自体 | `item` |
| `use path.{item as alias}` | alias自体 | `alias` |

#### 削除された構文

- ~~`from path use item`~~：Pythonのfrom-import形式は採用しない
- ~~`use path.*`~~：ワイルドカードインポートは競合リスクを伴う、モジュール名前空間インポートで十分
- ~~`use path.{a, b} as c, d`~~：位置でペアリングする平行リストは脆弱なデータ構造、エイリアスは各宣言の後に付ける：`use path.{a as c, b as d}`

#### パスのсемантика

`use path`の`path`は常に**モジュールパス**であり、宣言ではない。モジュールが見つからない場合は直接エラー：

```
モジュール`math.geometry.Point`が見つかりません。
`Point`がモジュール`math.geometry`内の宣言の場合、以下を使用：
use math.geometry.{Point}
```

「まず完全なモジュールを探し、失敗したら最後のセグメントを宣言として扱う」というフォールバックはしない。

#### インポート競合

同名インポートは直接エラーとなり、上書きされない：

```
名前`Point`のインポート競合：
  math.geometry.Point
  graphics.geometry.Point
選択的インポートまたはモジュール名前空間エイリアスを使用してください。
```

### 3. 可視性

#### ルール

パッケージは唯一の캡슐화境界である。モジュールは権限境界を担わない。

| 記述 | 現在のpkg内 | 其他pkg |
|------|:--------:|:------:|
| デフォルト（`pub`なし） | ✅ | ❌ |
| `pub` | ✅ | ✅ |

**型、関数、定数、メソッドを含むすべてのトップレベル宣言に適用される1つのルール。**

既存のコードの4つの例外を削除：

- ~~型定義は常にエクスポート~~ → 同一ルール
- ~~定数は自動エクスポート~~ → 同一ルール
- ~~メソッドは自動エクスポート~~ → 同一ルール
- ~~関数のみ`pub`をチェック~~ → 同一ルール

#### データ構造

ASTの`is_pub: bool`を以下で置換：

```rust
pub enum Visibility {
    Package,  // デフォルト：現在のパkg内可见
    Public,   // pub：すべてのパkg可见
}
```

#### エクスポートテーブル

各モジュールは2つのテーブルを維持：

- **PackageSymbols**：すべての人格的宣言を含むpkg内完全シンボルテーブル
- **PublicExports**：他のpkg提供する`pub`宣言のサブセット

同pkgの`use`は`PackageSymbols`をクエリ；跨pkgの`use`は`PublicExports`のみクエリ可能。

跨pkgで非`pub`宣言を参照すると直接エラー：

```
モジュール`math.geometry`の`internalHelper`は不可視。
pub宣言ではなく、`math`pkg内でのみ使用可能。
```

### 4. プロジェクトコンパイルフロー

#### コンパイルパイプライン

```
プロジェクト入口
  → yaoxiang.tomlを読み込んで入口ファイルを取得
  → 入口から再帰的にuse文を解析し、すべての依存モジュールを発見
  → モジュールの依存グラフ（ModuleDependencyGraph）を構築
  → 循環依存を検出
  → トポロジカルソート
  → 各モジュールを順に実行：字句解析 → 構文解析 → エクスポート抽出
  → ModuleRegistryを構築（すべてのモジュールのエクスポートテーブルを含む）
  → 各モジュールをトポロジカル順に型チェック（ModuleRegistryをクエリ）
  → 複数のModuleIRを生成
  → 診断を集約
```

型チェッカーは事前に構築された`ModuleRegistry`のみをクエリし、ファイルロードやディスクアクセスは実行しない。

#### 入口ファイルの選択

優先順位：

1. `[run].main`（存在する場合）
2. `[[bin]]`の最初の項目の`path`
3. `[lib].path`
4. `src/main.yx`（約定デフォルト）

単一ファイルモードは`yaoxiang.toml`を必要とせず、指定されたファイルを直接コンパイル。

#### 循環依存

```
循環依存を検出：
  math.geometry → math.transform → math.geometry
```

循環依存はコンパイルエラーであり、特別な処理はしない。

#### エラーの集約

マルチファイルコンパイルのエラーはモジュールのトポロジカル順に集約。各エラーにはソースモジュールとファイル位置を标注：

```
エラー：モジュール`math.geometry`内：
  src/math/geometry.yx:12:5
  型`Circle`が未定義

エラー：モジュール`app.main`内：
  src/main.yx:3:1
  モジュール`math.geometry`が不可視
```

### 5. コンパイラの改动

| コンポーネント | 改动 |
|------|------|
| `compiler.rs` | 新規`compile_project(project_root)`メソッド追加 |
| `pipeline.rs` | 単一モジュールコンパイル职责を維持、神オブジェクトにならない |
| `typecheck/checker.rs` | `use`文は`ModuleRegistry`をクエリ、ファイルロードをトリガーしない |
| `typecheck/inference/statements.rs` | 同上、`process_use_stmt`はクエリのみでロードなし |
| `frontend/module/resolver.rs` | `package/source/module_resolver.rs`のYXPATHサポートを統合し、唯一のリゾルバに |
| `frontend/module/loader.rs` | 拡張：再帰的発見、完全なモジュールグラフ構築をサポート |
| `frontend/module/dep_graph.rs` | 実装済み、トポロジカルソートと循環検出を再利用 |
| `frontend/module/registry.rs` | 実装済み、エクスポートテーブルクエリを再利用 |
| `frontend/module/cache.rs` | 実装済み、本RFCではコンパイルパイプラインに接続しない |
| `frontend/module/hot_reload.rs` | 実装済み、本RFCではコンパイルパイプラインに接続しない |
| AST `is_pub: bool` | `Visibility`列挙型で置換 |
| `package/source/module_resolver.rs` | 删除、职责は`frontend/module/resolver.rs`に統合 |

## 実装戦略

### フェーズ分け

**Phase 1：モジュールの統一解決**
1. 2つの`ModuleResolver`を統合、`package/source/module_resolver.rs`を削除
2. `YXPATH`環境変数をサポート
3. モジュールパスの曖昧検出

**Phase 2：可視性データ構造**
4. AST `is_pub: bool` → `Visibility`列挙型
5. リゾルバが`pub`キーワードを`Visibility::Public`にマッピングをサポート
6. `ModuleLoader::extract_exports`が`Visibility`を使用してエクスポート判断を統一

**Phase 3：プロジェクトコンパイル入口**
7. `compiler.rs`に新規`compile_project(project_root)`メソッド追加
8. 入口から再帰的にモジュールを発見、`ModuleDependencyGraph`を構築
9. トポロジカルソート、順にモジュールをロードしてエクスポートを抽出
10. 完全な`ModuleRegistry`を構築
11. 各モジュールをトポロジカル順に型チェック
12. 複数の`ModuleIR`を生成、診断を集約

**Phase 4：インポート構文**
13. `use path.{item as alias}`構文を実装
14. パス末尾のフォールバック推測を削除

### 依存関係

- RFC-014（pkgマネージャ）— 名前は`yaoxiang.toml`から、vendorディレクトリの構造
- RFC-011（genericsシステム）— 特質は構造化型であり、モジュールの帰属无关
- RFC-009（所有権モデル）— モジュールのインポートはコンパイル時の名前解決であり、実行時の参照コピー无关

## 子RFC計画

以下の子RFCは**予定計画中**であり、まだ起草を開始していない：

| 子RFC | 能力（予定） | 前提条件（予定） |
|--------|-------------|-----------------|
| 029a | モジュールのキャッシュと增量再コンパイル | モジュールグラフとエクスポートテーブルが安定 |
| 029b | ファイル監視とHotreload | 029aのキャッシュ失效メカニズム |
| 029c | 再エクスポート（`pub use`） | エクスポートテーブルと可視性ルールが実装済み |
| 029d | CLIパラメータ`--entry`による入口選択の上書き | プロジェクトコンパイル入口が利用可能 |
| 029e | マルチファイル診断`--json`出力フォーマット | 診断集約メカニズムが利用可能 |
| — | `pub(pkg)`モジュールプライベート可視性 | 現在现实的な需求がなく、当面含めない |
| — | ワークスペースのマルチパッケージコンパイル | RFC-014cが担当 |

## 参考文献

- [RFC-009: 所有権モデル](accepted/009-ownership-model.md) — Move семантика、インポートはコンパイル時の名前解決
- [RFC-011: 泛型型システム](accepted/011-generic-type-system.md) — 構造化型定義
- [RFC-014: pkg管理システムの設計](accepted/014-package-manager.md) — pkg名の來源、vendorディレクトリ
- [RFC-015: 設定システム](accepted/015-configuration-system.md) — `yaoxiang.toml`フィールド定義