```markdown
---
title: "REPL 状態"
---

# REPL

> **モジュール状態**：完了（v0.7.2 で書き直し）
> **場所**：`src/backends/dev/repl/`
> **最終更新**：2026-06-01

---

## モジュール概要

REPL（Read-Eval-Print Loop）モジュールは対話型プログラミング環境を提供する。trait 抽象アーキテクチャを採用しており、異なるバックエンド実装をサポートしている。

**コード量**：1,037 行（8 ファイル）

---

## 機能一覧

### REPLBackend trait（backend_trait.rs）

- ✅ `eval()` 求値
- ✅ `complete()` 補完候補
- ✅ `get_symbols()` シンボルリスト
- ✅ `get_type()` 型クエリ
- ✅ `clear()` 状態クリア
- ✅ `stats()` 実行統計

### 求値エンジン（engine/evaluator.rs - 299 行）

- ✅ コードコンパイル実行
- ✅ 括弧/引用符整合性検出
- ✅ 式/文の自動包装
- ✅ バイトコードからの定義抽出

### 実行コンテキスト（engine/context.rs - 168 行）

- ✅ 変数定義/クエリ
- ✅ 関数定義/クエリ
- ✅ シンボル型クエリ
- ✅ 実行統計

### コマンドシステム（commands/mod.rs - 95 行）

- ✅ `:quit/:q` 終了
- ✅ `:help/:h` ヘルプ
- ✅ `:clear/:c` クリア
- ✅ `:type/:t` 型表示
- ✅ `:symbols/:info` シンボルリスト
- ✅ `:stats` 統計
- ⚠️ `:history` コマンド — **未実装**（プロンプト出力のみ）

### セッション REPL（session/mod.rs - 247 行）

- ✅ rustyline 統合
- ✅ 複数行入力サポート
- ✅ 履歴保存/読込
- ✅ VI/Emacs 編集モード
- ✅ ファイル読込実行
- ✅ カスタム設定

### 自動補完（session/completer.rs - 126 行）

- ✅ キーワード補完
- ✅ 変数/関数補完
- ✅ 組込み関数補完

---

## テストカバレッジ

**0 件のユニットテスト**

REPL モジュールにはテストコードが一切存在しない。`src/backends/dev/repl/` ディレクトリ全体に `#[test]` や `#[cfg(test)]` アノテーションが一切ない。

---

## コード品質評価

| 次元 | 評価 | 説明 |
|------|------|------|
| 機能完成度 | 90% | コア機能は完整、:history のみ未実装 |
| テストカバレッジ | 0% | テストなし |
| ドキュメント品質 | 良好 | 完全なユーザーガイド（`docs/src/guide/repl.md`、436 行）とコードコメントが存在 |
| アーキテクチャ設計 | 優秀 | trait 抽象化、階層化が明確、拡張性が高い |

---

## 統合状態

REPL は以下のコンポーネントに統合されている：

1. **DevShell**（`src/backends/dev/shell.rs`）：`:repl` コマンドで REPL モードに切り替え
2. **モジュールエクスポート**（`src/backends/dev/mod.rs`）：`SessionREPL`, `Evaluator`, `REPLBackend` をエクスポート
3. **CLI エントリ**：`yaoxiang repl` または `yaoxiang` で起動

---

## 改善項目

1. **ユニットテストの追加**（ゼロテストカバレッジが最大の問題）
2. **`:history` コマンドの実装**
3. **境界条件テストの追加**
```