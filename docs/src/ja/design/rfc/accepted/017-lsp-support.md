```yaml
---
title: "'RFC-017: 言語サーバープロトコル（LSP）サポート設計'"
---

# RFC-017: 言語サーバープロトコル（LSP）サポート設計

> **状態**: レビュー中
>
> **著者**: 晨煦
>
> **作成日**: 2026-02-15
>
> **最終更新**: 2026-02-22

> **参照**: RFC の書き方については、[完全な例](EXAMPLE_full_feature_proposal.md) を参照してください。

## ⚠️ 実装前置条件（重要）

LSP を実装する前に、以下の2つのコア問題を解決する必要があります：

### 問題1：診断エラーの収集

**現状**: 現在の型チェッカーは最初のエラーに遭遇した時点で（`?` 演算子を使用して）即座に返すため、すべてのエラーを収集できません。

**LSP 要件**: IDE は**すべての**エラーを表示する必要があります。最初のエラーのみではありません。

**解決策**:

#### 1.1 エラー収集パターン
- `src/frontend/typecheck/inference/` モジュールを修正し、`Result<Type, Vec<Error>>` を返す
- エラーに遭遇しても即座に返さず、检查を継続する
- 檢查完了後にすべてのエラーをまとめて返す

#### 1.2 エラーレベル
異なる重大度レベルのエラーを区別します：

```rust
enum ErrorKind {
    Error,      // 严重エラー、カスケードエラーの原因となる可能性
    Warning,    // 警告、检查は続行するが阻断しない
    Note,       // 付随情報
}
```

- `Error` がある場合：`publishDiagnostics` でエラーを表示
- `Warning` のみの場合：コンパイルを続行し、警告を表示

#### 1.3 Parser エラー回復
- 解析エラー時、放弃する代わりに **プレースホルダーノード**（例：`MissingExpression`）を挿入する
- AST が不完全なことによる型チェックのパニックを避ける
- 例：`let x = ;` → `let x = MissingExpression`

#### 1.4 遅延レポート（Delayed Emission）
- 一部のエラーは「级聯」の可能性があります（前のエラーのためです）
- まず収集し、AST の解析完了後に明らかなカスケードエラーをフィルタリングできます
- またはシンプルに処理：すべてレポートし、ユーザーに逐一修正させる

### 問題2：ファイルレベル解析キャッシュ

**現状**: 各 LSP リクエストがファイル全体を再解析し、キャッシュメカニズムがありません。

**LSP 要件**: 每次の編集に迅速に応答する必要があり、変化のないファイルを再解析する必要がありません。

**解決策**:

#### 2.1 ファイルキャッシュ構造
```rust
struct DocumentCache {
    version: u32,           // LSP ドキュメントバージョン番号
    content: String,        // 現在のコンテンツ
    content_hash: u64,      // コンテンツハッシュ（高速比較）
    ast: Option<Ast>,       // キャッシュされた AST（オプション）
}
```

#### 2.2 変化の検出
- 每次 `textDocument/didChange` で新コンテンツを受信
- 新規コンテンツのハッシュを計算し、キャッシュされた `content_hash` と比較
- **変化がある場合：ファイル全体を再解析**
- **変化がない場合：キャッシュ結果を直接返す**

#### 2.3 再解析戦略
- **ファイルレベル**: 現在のファイルのみを再解析し、プロジェクト全体ではない
- これは簡略化された設計であり、関数レベルの増分解析は行わない
- 現代のコンピュータでは数千行のファイルを数ミリ秒で解析可能

#### 2.4 cargo check との違い
| | cargo check | YaoXiang LSP |
|---|---|---|
| 範囲 | プロジェクト全体 | 单一ファイル |
| 頻度 | 手動トリガー | 每次編集 |
| 目的 | 完全コンパイルチェック | 高速增量応答 |

### 既存モジュールとの統合

| 既存モジュール | LSP 統合方式 |
|----------|-------------|
| `util/span.rs` | ✅ すでに `Position`/`Span` があり、LSP `Position` に直接マッピング可能 |
| `util/diagnostic/collect.rs` | ⚠️ 「収集モード」に修正し、エラーを継続的に蓄積 |
| `frontend/core/lexer/symbols.rs` | ⚠️ 拡張が必要、`uri` + `span` 位置情報を追加 |
| `frontend/typecheck/mod.rs` | ⚠️ `TypeResult` を修正し、すべてのエラーを返す |
| `frontend/core/parser/ast.rs` | ✅ 各ノードにはすでに `Span` があり、変更不要 |

---

## 摘要

YaoXiang に Language Server Protocol（LSP）サポートを追加し、完全な言語サーバーを実装し、主流 IDE（VS Code、Neovim、Emacs など）がコード補完、定義へのジャンプ、診断、参照検索などの開発ツール機能を提供できるようにします。

## 動機

### なぜこの機能が必要ですか？

現在の YaoXiang 言語は公式の IDE 統合サポートがなく、開発者は基礎的なテキストエディタでしかコードを書くことができず、以下の機能がありません：

1. **コード補完** - コンテキストに基づいてインテリジェントに識別子、キーワード、タイプを補完できない
2. **定義へのジャンプ** - 関数、タイプ、変数の定義位置に迅速にジャンプできない
3. **リアルタイム診断** - 編集時に構文エラー、型エラーを即座に表示できない
4. **参照検索** - シンボルのすべての参照位置を検索できない
5. **ホバリングヒント** - マウスホバー時にタイプ情報、ドキュメントコメントを表示できない

LSP は現代プログラミング言語の標準装備であり、主流言語（Rust、Python、TypeScript、Go など）はすべて成熟した LSP 実装を提供ています。LSP サポートの実装は YaoXiang の開発体験を大きく向上させます。

### 現在の問題

1. **開発効率が低い** - コード補完とインテリジェントヒントがない
2. **デバッグが困難** - シンボル定義に迅速に到達できない
3. **学習曲線が急** - IDE の補助機能がない
4. **エコシステムが未完成** - 現代の IDE に慣れた開発者を惹きつけられない

## 提案

### コア設計

独立した LSP サーバー进程中を実現し、JSON-RPC 経由で IDE と通信します：

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

    subgraph Frontend [コンパイラボードエンド Compiler Frontend]
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
├── main.rs              # LSP サーバーエントリ
├── server.rs           # サーバーコラー論理
├── session.rs          # セッション管理
├── capabilities.rs     # サーバー能力宣言
├── handlers/
│   ├── mod.rs
│   ├── initialize.rs   # 初期化処理
│   ├── text_document.rs # ドキュメント操作処理
│   ├── completion.rs   # 補完処理
│   ├── definition.rs   # 定義ジャンプ処理
│   ├── references.rs   # 参照検索処理
│   ├── hover.rs        # ホバリングヒント処理
│   └── diagnostics.rs  # 診断処理
├── world.rs            # コンパイル世界（シンボルテーブル、AST キャッシュ）
├── scroller.rs         # シンボルインデックス構築
├── protocol.rs         # LSP プロトコルタイプ定義
└── cache/              # 增量キャッシュモジュール（新規追加）
    ├── mod.rs
    ├── document.rs     # ドキュメントキャッシュ（バージョン、AST、シンボルテーブル）
    └── incremental.rs  # 增量解析戦略
```

### コンパイル世界（World）設計

グローバルコンパイル状態を管理：
- ドキュメントキャッシュ（バージョン、AST、シンボルテーブル）
- グローバルシンボルインデックス
- エラー収集器
- タイプ環境キャッシュ

コラー方法：
- `on_document_change`：增量変更を処理
- `incremental_reparse`：增量再解析
- `collect_diagnostics`：すべてのエラーを収集（阻断しない）

### コラー LSP 方法サポート

| カテゴリ | 方法 | 説明 |
|------|------|------|
| **ライフサイクル** | `initialize` / `initialized` / `shutdown` / `exit` | サーバー Lifecycle |
| **ドキュメント同期** | `didOpen` / `didChange` / `didClose` | ドキュメント管理 |
| **診断** | `publishDiagnostics` | 診断の公開 |
| **補完** | `completion` | コード補完 |
| **ジャンプ** | `definition` | 定義にジャンプ |
| **参照** | `references` | 参照を検索 |
| **ホバー** | `hover` | ホバリングヒント |
| **シンボル** | `workspace/symbol` | ワークスペースシンボル検索 |

### 文書ドキュメント同期メカニズム

增量同期戦略を使用：
- ドキュメントバージョン番号を保持
- 增量変更を適用（range + text）
- 大幅な変更時は完全置換に降格

### シンボルインデックス構築

既存のシンボルテーブルシステムを利用し、リバースインデックスを構築：
- `SymbolEntry` を拡張し、`location` フィールドを追加する必要があります
- インデックス：名前 → 位置リスト、ファイル → シンボルリスト

### コード補完実装

補完ソース：キーワード、変数、関数、タイプ、構造体フィールド、モジュール

### 定義ジャンプ実装

AST ベースのシンボル解析：識別子/関数呼び出しに対応する定義位置を検索

## 詳細設計

### タイプシステムへの影響

1. **シンボル情報拡張** - シンボルテーブルに位置情報（ファイル、行番号、列番号）を追加
2. **タイプ情報公開** - LSP にタイプクエリインターフェースを提供
3. **ドキュメントコメント統合** - コメントからドキュメント文字列を生成するサポート

### 実行時動作

- LSP サーバーは独立したプロセスとして実行
- stdin/stdout を使用して JSON-RPC 通信
- マルチセッション同時処理をサポート

### コラー変更

| コンポーネント | 変更 |
|------|------|
| `frontend/events` | LSP 通知をサポートするようイベントシステムを拡張 |
| `frontend/core/lexer/symbols` | シンボルテーブルを強化し、位置情報を追加 |
| 新規追加 `src/lsp/` | LSP サーバー実装 |

### 後方互換性

- ✅ 完全な後方互換性
- LSP サーバーは独立コンポーネントであり、既存コンパイルプロセスに影響しない
- 既存の CLI ツールには影響なし

### 既存システムとの統合

1. **イベントシステム** - `frontend/events/` のイベントサブスクリプション機構を利用
2. **診断システム** - `util/diagnostic/` の診断出力を再利用
   - すべてのエラーを収集するために `ErrorCollector<E>` を再利用
   - `Diagnostic` を LSP の `Diagnostic` 形式に変換
3. **シンボルテーブル** - `symbols.rs` のシンボル位置能力を拡張
   - `SymbolEntry` を拡張し、`location: Location` フィールドを追加
   - `SymbolIndex` リバースインデックスを構築（名前 -> 位置リスト）
4. **コラー前端** - Lexer、Parser、型チェックを直接呼び出す
   - **コラー変更**：型チェッカーを「収集モード」に変更し、実行を阻断しない

#### 診断形式変換

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

## YaoXiang 固有的高级機能

YaoXiang の強力なコンパイル時評価と所有権システムを利用し、他の言語では実装できないユニークな開発体験を提供します：

### 1. ゴーストヒント（Inlay Hints）

- **定数值ヒント**：コンパイル時にすでに計算済みの定数を表示（例：`const MAX = 100 + 200` の隣に `300` を表示）
- **可変性ヒント**：変数が可変かどうかを表示（例：`mut x`、`x` には明らかな下線）
- **所有権消費ヒント**：関数パラメータが消費されるかどうかを表示（例：`consumed` / `borrowed`）
- **空所有権语义ヒント**：変数の色を薄めて、変数が move された後に再代入できることを表示
- **タイプ推論ヒント**：推論された具体的なタイプを表示（例：`x = vec![]` の隣に `Vec<i32>` を表示）

### 2. 所有権语义可视化

- 変数の move パスを表示（定義位置からすべての使用位置まで）
- 借用ライフタイムの可視化

### 3. コンパイル時評価プレビュー

- ホバーで定数式のコンパイル時計算結果を表示

### 実装優先度

| 機能 | 優先度 |
|------|--------|
| 定数值ゴーストヒント | P0 |
| 可変性ヒント | P0 |
| 所有権消費ヒント | P1 |
| 所有権可視化 | P2 |

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

DAP（Debug Adapter Protocol）に基づく実装：
- 行ブレークポイント、関数ブレークポイント、条件ブレークポイントをサポート
- YaoXiang 固有ブレークポイント：変数が move されたときにトリガー

### 起動パラメータ

```bash
# ローカルモード
yaoxiang-lsp

# TCP サーバー
yaoxiang-lsp --tcp --port 8765

# デバッグを有効にして同時に起動
yaoxiang-lsp --tcp --port 8765 --enable-debug
```

---

## 並行モデル

**設計決定：シングルスレッド + 非同期イベントループ**

理由：
- コラーはスレッドセーフではなく、改造コストが高い
- LSP リクエストは本質的にシリアルであり、並行処理は不要
- シングルスレッドはよりシンプルでデバッグしやすい
- 非同期 I/O のシングルスレッド性能は十分

バックグラウンドタスクは `spawn_blocking` を使用してマルチコアを活用。

---

## LSP 内蔵テストツール（オプション）

> この機能は MVP 必须ではなく、後続バージョンで追加できます。

JSON テストケースフォーマットを提供：

```bash
# テストを実行
yaoxiang-lsp --test
```

---

## 权衡

### 利点

1. **開発体験の向上** - 主流言語に近い IDE サポート
2. **エコシステムの完善** - より多くの開発者に YaoXiang を使用してもらえる
3. **コード品質の向上** - リアルタイム診断により実行時エラーが减少
4. **コミュニティの貢献** - 開発者が LSP ツールチェーンの開発に参加できる

### 欠点

1. **実装复杂度が高い** - 大量にある LSP エッジケースを処理する必要がある
2. **メンテナンスコスト** - LSP プロトコルバージョンの更新に追従する必要がある
3. **パフォーマンス考慮** - 大規模プロジェクトのインデックスとクエリパフォーマンス
4. **テスト难度** - IDE 動作をシミュレートしてテストする必要がある

## 代替案

| 方案 | なぜ選択しないか |
|------|--------------|
| 構文ハイライトのみ提供 | 現代の開發ニーズを満たせない |
| Tree-sitter を使用 | 追加の学習コストがかかり、機能に限界がある |

## 実装戦略

### 段階分け

1. **段階 0 (前置)**: コラー适配 ⚠️ **重要**
   - 型チェッカーを「収集モード」に修正し、`Result<Type, Vec<Error>>` を返す
   - エラーレベル（Error / Warning / Note）を実装
   - Parser エラー回復：プレースホルダーノードを挿入
   - シンボルテーブル `SymbolEntry` を拡張し、`location` フィールドを追加
   - DocumentCache キャッシュシステムを実現（バージョン + コンテンツ + ハッシュ）
   - **この段階は LSP 実装の前提であり、まず完了する必要があります**

2. **段階 1 (v0.7)**: 基础フレームワーク
   - LSP サーバー骨格
   - ライフサイクル方法（initialize/shutdown/exit）
   - 基础ログとエラー処理

3. **段階 2 (v0.7)**: 診断サポート
   - 文書ドキュメント同期
   - コラー診断統合
   - `textDocument/publishDiagnostics`

4. **段階 3 (v0.8)**: 補完サポート
   - シンボルインデックス構築
   - キーワード補完
   - 識別子補完

5. **段階 4 (v0.8)**: ジャンプサポート
   - 定義にジャンプ
   - 参照を検索
   - ホバリングヒント

6. **段階 5 (v0.9)**: 高級機能
   - ワークスペースシンボル検索
   - コードフォーマット
   - リファクタリングサポート（オプション）

### 依存関係

- 外部 LSP ライブラリへの依存なし（`lsp-types` crate を使用）
- 既存コeuxプレッソラボードエンドモジュールに依存
- JSON-RPC シリアル化に `serde_json` に依存

### リスク

1. **パフォーマンス問題** - 大容量ファイルの解析によりスタックする可能性がある
   - 解決：增量解析、バックグラウンドスレッド処理
2. **メモリ使用量** - シンボルインデックスがメモリを占有
   - 解決：遅延ロード、LRU キャッシュ
3. **プロトコル互換性** - LSP バージョンの差異
   - 解決：サポートするプロトコルバージョンを宣言

## 開放問題

- [x] エラー収集メカニズム（「実装前置条件」章を参照）
- [x] 增量キャッシュシステム（「実装前置条件」章を参照）
- [x] LSP プロトコルバージョン：3.18 を使用（Inlay Hints、Inline Values などの新機能をサポート）
- [x] リモート通信サポート（TCP 経由、LSP + デバッグを兼用）
- [x] リモートデバッグサポート（DAP プロトコルに基づく）
- [x] 並行モデル：シングルスレッド + 非同期イベントループ
- [x] LSP 内蔵テストツール（オプション）：JSON テストケースを使用

---

## 付録（オプション）

### 付録A：設計議論記録

> 設計意思決定プロセスの詳細な議論を記録するために使用します。

### 付録B：設計意思決定記録

| 意思決定 | 決定 | 日付 | 記録者 |
|------|------|------|--------|
| LSP サーバーアーキテクチャ | 独立プロセス、stdio 経由で通信 | 2026-02-15 | 晨煦 |
| プロトコルバージョン | LSP 3.18 をサポート（Inlay Hints などの新機能が必要） | 2026-02-22 | 晨煦 |
| エラー収集モード | `Result<Type, Vec<Error>>` を返し、エラーレベルとエラー回復をサポート | 2026-02-22 | 晨煦 |
| キャッシュ戦略 | ファイルレベルキャッシュ：バージョン + コンテンツ + ハッシュ、ファイル全体を再解析 | 2026-02-22 | 晨煦 |
| 通信モード | stdio + TCP + UnixSocket をサポート | 2026-02-22 | 晨煦 |
| リモートデバッグ | DAP プロトコルに基づき、LSP とトランスポート層を共有 | 2026-02-22 | 晨煦 |
| 並行モデル | シングルスレッド + 非同期イベントループ | 2026-02-22 | 晨煦 |
| テストツール（オプション）| JSON テストケース + 内蔵テストランナー | 2026-02-22 | 晨煦 |

### 付録C：用語集

| 用語 | 定義 |
|------|------|
| LSP | Language Server Protocol、言語サーバープロトコル |
| JSON-RCP | JSON-Remote Procedure Call、JSON リモートプロシージャコール |
| DAP | Debug Adapter Protocol、デバッグアダプタプロトコル |
| シンボルインデックス | コンパイル時に構築されるシンボル位置マッピングテーブル |
| コンパイル世界 | すべてのコンパイル情報を含むコンテキスト |
| ゴーストヒント | Inlay Hints、行内に表示されるヒント情報 |
| 所有権追踪 | Ownership Trace、変数所有権フローの可視化 |

---

## 参考文献

- [Language Server Protocol 仕様](https://microsoft.github.io/language-server-protocol/)
- [LSP 仕様 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Debug Adapter Protocol 仕様](https://microsoft.github.io/debug-adapter-protocol/)
- [Rust Analyzer](https://rust-analyzer.github.io/) - 參考実装
- [lsp-types crate](https://crates.io/crates/lsp-types) - LSP タイプ定義
- [JSON-RPC 2.0 仕様](https://www.jsonrpc.org/specification)

---

## ライフサイクルと行き先

RFC には以下の状態フローがあります：

```
┌─────────────┐
│   下書き    │  ← 著者が作成
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中  │  ← コミュニティ議論
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  承認済み   │    │   拒否済み   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │  rejected/  │
│ (正式設計)  │     │ (拒否)     │
└─────────────┘    └─────────────┘
```

### 状態説明

| 状態 | 場所 | 説明 |
|------|------|------|
| **下書き** | `docs/design/rfc/draft/` | 著者の下書き、レビュー提出待ち |
| **レビュー中** | `docs/design/rfc/review/` | コミュニティ議論とフィードバックを募集中 |
| **承認済み** | `docs/design/accepted/` | 正式設計ドキュメントとなり、実装段階へ |
| **拒否済み** | `docs/design/rfc/` | RFC ディレクトリに残し、状態を更新 |

### 承認後の操作

1. RFC を `docs/design/accepted/` ディレクトリに移動
2. ファイル名を描述的名称（例：`lsp-support.md`）に更新
3. 状態を「正式」に更新
4. 状態を「承認済み」に更新し、承認日を追記

### 拒否後の操作

1. `docs/design/rfc/draft/` ディレクトリに残す
2. ファイル冒頭に拒否理由と日付を追加
3. 状態を「拒否済み」に更新

### 議論が確定した後の操作

ある開放問題がコンセンサスに達した場合：

1. **付録A を更新**: 議論テーマ下に「決議」を記入
2. **本文を更新**: 決定を本文に同期
3. **意思決定を記録**: 「付録B：設計意思決定記録」に追加
4. **問題をマーク**: 「開放問題」リストで `[x]` をチェック

---

> **注**: RFC 番号は議論段階でのみ使用します。承認後は削除し、記述的文件名を使用します。