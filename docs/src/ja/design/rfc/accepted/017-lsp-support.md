```markdown
---
title: 'RFC-017: 言語サーバプロトコル（LSP）サポート設計'
---

# RFC-017: 言語サーバプロトコル（LSP）サポート設計

> **ステータス**: レビュー中
>
> **著者**: 晨煦
>
> **作成日**: 2026-02-15
>
> **最終更新**: 2026-02-22

> **参考**: RFC の書き方については、[完全示例](EXAMPLE_full_feature_proposal.md) をご覧ください。

## ⚠️ 実装前置条件（重要）

LSP を実装する前に、以下の2つのコア問題を解決する必要があります：

### 問題 1: 診断エラーの収集

**現状**: 現在の型チェッカーは最初のエラーに遭遇した時点で `?` 演算子を使用して即座に返回するため、すべてのエラーを収集できません。

**LSP 要件**: IDE は最初のエラーだけでなく、**すべての**エラーを表示する必要があります。

**解決策**:

#### 1.1 エラー収集パターン
- `src/frontend/typecheck/inference/` モジュールを変更し、`Result<Type, Vec<Error>>` を返す
- エラーに遭遇しても即座に返回せず、检查を継続する
- 检查完了後、すべてのエラーを统一的に返す

#### 1.2 エラーレベル
異なる重要度のエラーを区別します：

```rust
enum ErrorKind {
    Error,      // 严重エラー、カスケードエラーを引き起こす可能性
    Warning,    // 警告、检查を継続するが阻断しない
    Note,       // 追加情報
}
```

- `Error` がある場合：`publishDiagnostics` でエラーを表示
- `Warning` のみの場合：コンパイルを継続し、警告を表示

#### 1.3 Parser エラー回復
- 解析エラー時、処理を放弃するのではなく、**プレースホルダーノード**（例：`MissingExpression`）を挿入する
- AST が不完全而导致类型检查で panic が発生するのを避ける
- 例：`let x = ;` → `let x = MissingExpression`

#### 1.4 遅延レポート (Delayed Emission)
- 一部のエラーは「级聯」的である場合があります（前のエラー导致的）
- 先に収集し、AST の解析完了後に明らかな级聯エラーをフィルターかけることができる
- またはシンプルに処理：すべて報告し、ユーザーに逐一修正させる

### 問題 2: ファイルレベルの解析キャッシュ

**現状**: 每次 LSP リクエストでファイル全体を再解析し、キャッシュメカニズムがありません。

**LSP 要件**: 每次編集時に即座に反応し、変化していないファイルの再解析は不要。

**解決策**:

#### 2.1 ファイルキャッシュ構造
```rust
struct DocumentCache {
    version: u32,           // LSP ドキュメントバージョン番号
    content: String,        // 現在のコンテンツ
    content_hash: u64,      // コンテンツハッシュ（高速比較用）
    ast: Option<Ast>,       // キャッシュされた AST（オプション）
}
```

#### 2.2 変化の検出
- 每次 `textDocument/didChange` で新しいコンテンツを受け取る
- 新しいコンテンツのハッシュを計算し、キャッシュされた `content_hash` と比較する
- **変化がある場合：ファイル全体を再解析する**
- **変化がない場合：キャッシュ結果を直接返す**

#### 2.3 再解析戦略
- **ファイルレベル**: 現在のファイルのみを再解析し、项目全体ではない
- これは簡略化された設計であり、関数レベルの增量解析は行わない
- 現代のコンピュータでは、数千行のファイルを数ミリ秒で解析できる

#### 2.4 cargo check との違い
| | cargo check | YaoXiang LSP |
|---|---|---|
| 範囲 | 项目全体 | 单一ファイル |
| 頻度 | 手動トリガー | 每次編集 |
| 目標 | 完全なコンパイルチェック | 高速增量対応 |

### 既存モジュールとの統合

| 既存モジュール | LSP 統合方式 |
|----------|-------------|
| `util/span.rs` | ✅ すでに `Position`/`Span` があり、LSP `Position` に直接マッピング可能 |
| `util/diagnostic/collect.rs` | ⚠️ 「収集モード」に変更が必要、エラーを継続的に蓄積 |
| `frontend/core/lexer/symbols.rs` | ⚠️ 拡張が必要、`uri` + `span` 位置情報を追加 |
| `frontend/typecheck/mod.rs` | ⚠️ `TypeResult` を変更し、すべてのエラーを返す |
| `frontend/core/parser/ast.rs` | ✅ 各ノードにはすでに `Span` があり、変更不要 |

---

## 摘要

YaoXiang に Language Server Protocol（LSP）サポートを追加し、完全な言語サーバーを実装することで、主流な IDE（VS Code、Neovim、Emacs など）がコード補完、定義へのジャンプ、診断、参照検索などの開発ツール機能を提供できるようにします。

## 動機

### なぜこの機能が必要ですか？

現在 YaoXiang 言語には公式の IDE 統合サポートがなく、開発者は基本的なテキストエディタでしかコードを書くことができず、以下の機能がありません：

1. **コード補完** - コンテキストに基づいてインテリジェントに識別子、キーワード、型を補完できない
2. **定義へのジャンプ** - 関数、型、変数の定義位置に快速にジャンプできない
3. **リアルタイム診断** - 編集時に構文エラー、型エラーを即座に表示できない
4. **参照検索** - シンボルのすべての参照位置を検索できない
5. **ホバリングヒント** - マウスオーバーで型情報、ドキュメントコメントを表示できない

LSP は現代プログラミング言語の標準装備であり、主流な言語（Rust、Python、TypeScript、Go など）はすべて成熟した LSP 実装を提供しています。LSP サポートを実装することで、YaoXiang の開発体験が大幅に向上します。

### 現在の問題

1. **開発効率が低い** - コード補完とインテリジェントヒントがない
2. **デバッグが困難** - シンボル定義を快速に定位できない
3. **学習曲線が急** - IDE の支援機能がない
4. **エコシステムが未完成** - 現代的な IDE に慣れた開発者を引き付けられない

## 提案

### コア設計

独立した LSP サーバープロセスを実装し、JSON-RPC を介して IDE と通信します：

```mermaid
flowchart TD
    subgraph IDE_Environment [IDE 環境]
        IDE["IDE (VS Code)"]
    end

    subgraph LSP_Server [LSP サーバー]
        LSP["YaoXiang LSP Server"]
    end

    subgraph World_Compile [コンパイル世界 World]
        direction TB
        W_Symbol["Symbol Index"]
        W_Type["Type Env"]
        W_Diag["Diagnostics"]
    end

    subgraph Cache [ドキュメントキャッシュ Document Cache]
        direction TB
        C_Version["バージョン管理"]
        C_Content["コンテンツキャッシュ"]
        C_AST["AST キャッシュ"]
        C_Delta["增量変更領域"]
    end

    subgraph Frontend [コンパイラフロントエンド Compiler Frontend]
        direction TB
        F_Lexer["Lexer (util/span.rs Position)"]
        F_Parser["Parser (ast.rs 已有 Span)"]
        F_TypeCheck["Type Check (収集モードに変更)"]
        F_ErrorCollector["ErrorCollector (util/diagnostic/)"]
    end

    IDE <-->|JSON-RPC| LSP

    LSP --- World_Compile
    LSP --- Cache

    Cache -- "增量更新" --> World_Compile

    World_Compile --- Frontend
    Cache --- Frontend
```

### LSP サーバーアーキテクチャ

```
src/lsp/
├── main.rs              # LSP サーバーエントリー
├── server.rs           # サーバーコアロジック
├── session.rs          # セッション管理
├── capabilities.rs     # サーバー機能宣言
├── handlers/
│   ├── mod.rs
│   ├── initialize.rs   # 初期化処理
│   ├── text_document.rs # ドキュメント操作処理
│   ├── completion.rs   # 補完処理
│   ├── definition.rs   # 定義ジャンプ処理
│   ├── references.rs   # 参照検索処理
│   ├── hover.rs        # ホバー提示処理
│   └── diagnostics.rs  # 診断処理
├── world.rs            # コンパイル世界（シンボルテーブル、AST キャッシュ）
├── scroller.rs         # シンボルインデックス構築
├── protocol.rs         # LSP プロトコル型定義
└── cache/              # 增量キャッシュモジュール（新規）
    ├── mod.rs
    ├── document.rs     # ドキュメントキャッシュ（バージョン、AST、シンボルテーブル）
    └── incremental.rs  # 增量解析戦略
```

### コンパイル世界（World）設計

グローバルなコンパイル状態を管理します：
- ドキュメントキャッシュ（バージョン、AST、シンボルテーブル）
- グローバルシンボルインデックス
- エラーコレクター
- 型環境キャッシュ

コアメソッド：
- `on_document_change`：增量変更を処理
- `incremental_reparse`：增量再解析
- `collect_diagnostics`：すべてのエラーを収集（阻断しない）

### コア LSP メソッドサポート

| カテゴリ | メソッド | 説明 |
|------|------|------|
| **ライフサイクル** | `initialize` / `initialized` / `shutdown` / `exit` | サーバーサイクル |
| **ドキュメント同期** | `didOpen` / `didChange` / `didClose` | ドキュメント管理 |
| **診断** | `publishDiagnostics` | 診断の公開 |
| **補完** | `completion` | コード補完 |
| **ジャンプ** | `definition` | 定義へのジャンプ |
| **参照** | `references` | 参照の検索 |
| **ホバー** | `hover` | ホバー提示 |
| **シンボル** | `workspace/symbol` | ワークスペースシンボル検索 |

### 文本文書同期メカニズム

增量同期戦略を使用します：
- ドキュメントバージョン番号を保持
- 增量変更を適用（range + text）
- 大規模変更時はフル置き换えに降格

### シンボルインデックス構築

既存のシンボルテーブルシステムを利用して、逆引きインデックスを構築します：
- `SymbolEntry` を拡張し、`location` フィールドを追加する必要がある
- インデックス：名前 → 位置リスト、ファイル → シンボルリスト

### コード補完の実装

補完ソース：キーワード、変数、関数、型、構造体フィールド、モジュール

### 定義ジャンプの実装

AST ベースのシンボル解析：識別子/関数呼び出しに対応する定義位置を検索

## 詳細設計

### 型システムへの影響

1. **シンボル情報拡張** - シンボルテーブルに位置情報（ファイル、行番号、列番号）を追加
2. **型情報露出** - LSP に型クエリインターフェースを提供
3. **ドキュメントコメント統合** - コメントからドキュメント文字列を生成するサポート

### 実行時動作

- LSP サーバーは獨立したプロセスとして実行
- stdin/stdout を使用して JSON-RPC 通信を行う
- マルチセッション并发處理をサポート

### コンパイラの改动

| コンポーネント | 改动 |
|------|------|
| `frontend/events` | LSP 通知をサポートするイベントシステムを拡張 |
| `frontend/core/lexer/symbols` | シンボルテーブルを強化し、位置情報を追加 |
| 新規追加 `src/lsp/` | LSP サーバー実装 |

### 後方互換性

- ✅ 完全な後方互換性
- LSP サーバーは独立コンポーネントであり、既存コンパイルフローに影響しない
- 既存の CLI ツールには影響なし

### 既存システムとの統合

1. **イベントシステム** - `frontend/events/` のイベントサブスクリプション機構を利用
2. **診断システム** - `util/diagnostic/` の診断出力を再利用
   - `ErrorCollector<E>` を使用してすべてのエラーを収集
   - `Diagnostic` を LSP の `Diagnostic` フォーマットに変換
3. **シンボルテーブル** - `symbols.rs` のシンボル位置決め能力を拡張
   - `SymbolEntry` を拡張し、`location: Location` フィールドを追加
   - `SymbolIndex` 逆引きインデックスを構築（名前 -> 位置リスト）
4. **コンパイラフロントエンド** - Lexer、Parser、型チェッカーを直接呼び出し
   - ** ключевое изменение **：型チェッカーを「収集モード」に変更し、実行を阻断しない

#### 診断フォーマット変換

```rust
/// YaoXiang Diagnostic を LSP Diagnostic に変換
fn to_lsp_diagnostic(diag: &Diagnostic) -> lsp_types::Diagnostic {
    let severity = match diag.severity() {
        Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
        Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
        Severity::Info => lsp_types::DiagnosticSeverity::INFORMATION,
    };

    lsp_types::Diagnostic {
        range: to_lsp_range(diag.span()),
        severity: Some(severity),
        message: diag.message().to_string(),
        code: diag.code().map(|c| lsp_types::NumberOrString::String(c.as_string())),
        ..Default::default()
    }
}

/// YaoXiang Span を LSP Range に変換
fn to_lsp_range(span: &Span) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position {
            line: span.start.line.saturating_sub(1), // LSP は 0-indexed を使用
            character: span.start.column.saturating_sub(1),
        },
        end: lsp_types::Position {
            line: span.end.line.saturating_sub(1),
            character: span.end.column.saturating_sub(1),
        },
    }
}
```

## YaoXiang 特有の高度な機能

YaoXiang の強力なコンパイル時評価と所有权システムを利用して、他の言語では実装できないユニークな開発体験を提供します：

### 1. 幽灵ヒント（Inlay Hints）

- **定数値ヒント**: コンパイル時にすでに計算済みの定数を表示（例：`const MAX = 100 + 200` の横に `300` を表示）
- **可变性ヒント**: 変数が可変かどうかを表示（例：`mut x`、`x` に明らかなアンダーライン）
- **所有权消費ヒント**: 関数パラメータが消費されるかどうかを表示（例：`consumed` / `borrowed`）
- **空所有权_semantic ヒント**: 変数を move した後再代入できることを、変数の色を薄めて表示
- **型推論ヒント**: 推論された具体的な型を表示（例：`x = vec![]` の横に `Vec<i32>` を表示）

### 2. 所有权_semantic 可視化

- 変数の move パスを表示（定義位置からすべての使用位置まで）
- 借用ライフタイム可視化

### 3. コンパイル時評価プレビュー

- ホバーで定数式のコンパイル時計算結果を表示

### 実装優先順位

| 機能 | 優先度 |
|------|--------|
| 定数値幽灵ヒント | P0 |
| 可变性ヒント | P0 |
| 所有权消費ヒント | P1 |
| 所有权可視化 | P2 |

---

## 通信とリモートサポート

### 通信モード

3つのモードをサポート：

| モード | 用途 |
|------|------|
| stdio | ローカル開発（デフォルト）|
| TCP Socket | リモート開発/デバッグ |
| Unix Domain Socket | 高性能ローカル通信 |

### リモートデバッグ

DAP（Debug Adapter Protocol）ベースの実装：
- 行ブレークポイント、関数ブレークポイント、条件ブレークポイントをサポート
- YaoXiang 特有のブレークポイント：変数が move されたときにトリガー

### 起動パラメータ

```bash
# ローカルモード
yaoxiang-lsp

# TCP サーバー
yaoxiang-lsp --tcp --port 8765

# デバッグも同時に有効化
yaoxiang-lsp --tcp --port 8765 --enable-debug
```

---

## 並発モデル

**設計方針：シングルスレッド + 非同期イベントループ**

理由：
- コンパイラのスレッドセーフティはなく、改造コストが高い
- LSP リクエストは本質的にシリアルであり、並髪が不要
- シングルスレッドの方がシンプルでデバッグしやすい
- async I/O シングルスレッドのパフォーマンスは十分

バックグラウンドタスクは `spawn_blocking` を使用してマルチコアを利用します。

---

## LSP 内蔵テストツール（オプション）

> この機能は MVP 必須ではなく、後続バージョンで追加できます。

JSON テストケースフォーマットを提供します：

```bash
# テストを実行
yaoxiang-lsp --test
```

---

## 权衡

### メリット

1. **開発体験向上** - 主流言語に近い IDE サポート
2. **エコシステム整備** - より多くの開発者に YaoXiang を使ってもらう吸引力
3. **コード品質向上** - リアルタイム診断で 런타임 エラーを削減
4. **コミュニティ貢献** - 開発者が LSP ツールチェーン開発に参加可能

### デメリット

1. **実装复杂度が高い** - 多くの LSP エッジケースを処理する必要がある
2. **メンテナンスコスト** - LSP プロトコルバージョン更新に追随する必要がある
3. **パフォーマンス考慮** - 大規模プロジェクトのインデックスとクエリパフォーマンス
4. **テスト难度** - IDE 動作をシミュレートしてテストする必要がある

## 代替方案

| 方案 | 为什么不选择 |
|------|--------------|
| 構文ハイライトのみ提供 | 現代的な開発ニーズを満たせない |
| Tree-sitter を使用 | 追加学習コストが必要で、機能に限界がある |

## 実装戦略

### フェーズ分け

1. **フェーズ 0 (前置)**: コンパイラ適応 ⚠️ **重要**
   - 型チェッカーを「収集モード」に変更し、`Result<Type, Vec<Error>>` を返す
   - エラーレベル（Error / Warning / Note）を実装
   - Parser エラー回復：プレースホルダーノードを挿入
   - シンボルテーブル `SymbolEntry` を拡張し、`location` フィールドを追加
   - DocumentCache キャッシュシステムを実現（バージョン + コンテンツ + ハッシュ）
   - **このフェーズは LSP 実装の前提であり、先に完了する必要がある**

2. **フェーズ 1 (v0.7)**: 基礎フレームワーク
   - LSP サーバー骨格
   - ライフサイクルメソッド（initialize/shutdown/exit）
   - 基本的なログとエラー処理

3. **フェーズ 2 (v0.7)**: 診断サポート
   - 文書ドキュメント同期
   - コンパイル診断統合
   - `textDocument/publishDiagnostics`

4. **フェーズ 3 (v0.8)**: 補完サポート
   - シンボルインデックス構築
   - キーワード補完
   - 識別子補完

5. **フェーズ 4 (v0.8)**: ジャンプサポート
   - 定義へのジャンプ
   - 参照の検索
   - ホバー提示

6. **フェーズ 5 (v0.9)**: 高機能機能
   - ワークスペースシンボル検索
   - コードフォーマット
   - リファクタリングサポート（オプション）

### 依存関係

- 外部 LSP ライブラリ依存なし（`lsp-types` crate を使用）
- 既存のコンパイラフロントエンドモジュールに依存
- JSON-RPC シリアライズに `serde_json` に依存

### リスク

1. **パフォーマンス問題** - 大ファイル解析で遅延が発生する可能性がある
   - 解決：增量解析、バックグラウンドスレッド処理
2. **メモリ使用量** - シンボルインデックスがメモリを占有
   - 解決：延迟読み込み、LRU キャッシュ
3. **プロトコル互換性** - LSP バージョンの差異
   - 解決：サポートするプロトコルバージョンを宣言

## オープン問題

- [x] エラー収集メカニズム（「実装前置条件」章を参照）
- [x] 增量キャッシュシステム（「実装前置条件」章を参照）
- [x] LSP プロトコルバージョン：3.18 を使用（Inlay Hints、Inline Values などの新機能サポート）
- [x] リモート通信サポート（TCP 経由、LSP + デバッグ兼顾）
- [x] リモートデバッグサポート（DAP プロトコルベース）
- [x] 並発モデル：シングルスレッド + async イベントループ
- [x] LSP 内蔵テストツール（オプション）：JSON テストケースを使用

---

## 付録（オプション）

### 付録A：設計議論記録

> 設計意思決定プロセスの詳細な議論を記録するために使用します。

### 付録B：設計意思決定記録

| 意思決定 | 決定 | 日付 | 記録者 |
|------|------|------|--------|
| LSP サーバーアーキテクチャ | 独立プロセス、stdio 通信経由 | 2026-02-15 | 晨煦 |
| プロトコルバージョン | LSP 3.18 をサポート（Inlay Hints などの新機能が必要） | 2026-02-22 | 晨煦 |
| エラー収集モード | `Result<Type, Vec<Error>>` を返し、エラーレベルとエラー回復をサポート | 2026-02-22 | 晨煦 |
| キャッシュ戦略 | ファイルレベルキャッシュ：バージョン + コンテンツ + ハッシュ、ファイル全体を再解析 | 2026-02-22 | 晨煦 |
| 通信モード | stdio + TCP + UnixSocket をサポート | 2026-02-22 | 晨煦 |
| リモートデバッグ | DAP プロトコルベース、LSP とトランスポート層を共有 | 2026-02-22 | 晨煦 |
| 並発モデル | シングルスレッド + async イベントループ | 2026-02-22 | 晨煦 |
| テストツール（オプション）| JSON テストケース + 内蔵テストレンナー | 2026-02-22 | 晨煦 |

### 付録C：用語集

| 用語 | 定義 |
|------|------|
| LSP | Language Server Protocol、言語サーバープロトコル |
| JSON-RCP | JSON-Remote Procedure Call、JSON リモートプロシージャコール |
| DAP | Debug Adapter Protocol、デバッグアダプタープロトコル |
| シンボルインデックス | コンパイル時に構築されるシンボル位置マッピングテーブル |
| コンパイル世界 | すべてのコンパイル情報を含むコンテキスト |
| 幽灵ヒント | Inlay Hints、行内に表示されるヒント情報 |
| 所有权追踪 | Ownership Trace、変数所有权の流れの可視化 |

---

## 参考文献

- [Language Server Protocol 仕様](https://microsoft.github.io/language-server-protocol/)
- [LSP 仕様 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Debug Adapter Protocol 仕様](https://microsoft.github.io/debug-adapter-protocol/)
- [Rust Analyzer](https://rust-analyzer.github.io/) - 実装リファレンス
- [lsp-types crate](https://crates.io/crates/lsp-types) - LSP 型定義
- [JSON-RPC 2.0 仕様](https://www.jsonrpc.org/specification)

---

## ライフサイクルと归宿

RFC には以下のステータスフローがあります：

```
┌─────────────┐
│   草案      │  ← 著者作成
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中     │  ← コミュニティ議論
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  承認済み     │    │   却下済み     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │  rejected/  │
│ (正式設計)  │     │ (却下)     │
└─────────────┘    └─────────────┘
```

### ステータス説明

| ステータス | 場所 | 説明 |
|------|------|------|
| **草案** | `docs/design/rfc/draft/` | 作成者の下書き、レビュー提出待ち |
| **レビュー中** | `docs/design/rfc/review/` | コミュニティ議論とフィードバックを募集中 |
| **承認済み** | `docs/design/accepted/` | 正式設計ドキュメントとなり、実装フェーズへ |
| **却下済み** | `docs/design/rfc/` | RFC ディレクトリに保持、ステータスを更新 |

### 承認後の操作

1. RFC を `docs/design/accepted/` ディレクトリに移動
2. ファイル名を説明的名称に更新（例：`lsp-support.md`）
3. ステータスを「正式」に更新
4. ステータスを「承認済み」に更新し、承認日を追記

### 却下後の操作

1. `docs/design/rfc/draft/` ディレクトリに保持
2. ファイル上部に却下理由と日付を追加
3. ステータスを「却下済み」に更新

### 議論確定後の操作

あるオープン問題がコンセンサスに達したとき：

1. **付録A を更新**: 議論テーマ下に「決議」を記入
2. **本文を更新**: 決定をドキュメント本分と同期
3. **意思決定を記録**: 「付録B：設計意思決定記録」に追加
4. **問題をマーク**: 「オープン問題」リストで `[x]` をチェック

---

> **注**: RFC 番号は議論フェーズのみ使用。承認後は番号を削除し、説明的なファイル名を使用。
```