# VS Code 拡張パック実装計画

> **タスク**：YaoXiang VS Code 拡張パック（Extension Pack）を実装する
> **目標**：VS Code で YaoXiang 言語サポートを自動有効化する
> **日付**：2026-02-23
> **ステータス**：未開始
> **前提依存**：LSP サーバーの実装完了

---

## 概要

本計画では、VS Code 拡張パックの実装を 4 つのステップに分解し、各ステップに実装目標、受け入れ基準、テスト項目を含める。

### LSP との関係

```
┌─────────────────────────────────────────────────────┐
│              VS Code (組み込み LSP クライアント)    │
└──────────────────────┬──────────────────────────────┘
                       │ stdio 経由で LSP サーバーと通信
                       ▼
┌─────────────────────────────────────────────────────┐
│            YaoXiang LSP Server (実装済み)           │
│  - コード補完 ✓                                      │
│  - 定義へのジャンプ ✓                                │
│  - リアルタイム診断 ✓                                │
└─────────────────────────────────────────────────────┘
                       ▲
                       │ 依存関係
┌──────────────────────┴──────────────────────────────┐
│           VS Code 拡張パック（本計画で実装）        │
│  - 構文ハイライト                                    │
│  - 言語設定                                          │
│  - LSP 自動検出設定                                  │
└─────────────────────────────────────────────────────┘
```

---

## ステップ 1：拡張パックプロジェクト構造の作成

**目標**：

- プロジェクトルートに `vscode-extension/` ディレクトリを作成
- 標準的な VS Code 拡張パック構造を確立

**ディレクトリ構造**：

```
vscode-extension/
├── package.json                    # 拡張機能設定
├── language-configuration.json     # 言語設定
├── syntaxes/
│   └── yaoxiang.tmLanguage.json    # 構文ハイライト（オプション）
└── README.md                       # インストール手順
```

**受け入れ基準**：

- [ ] `vscode-extension/` ディレクトリが作成されている
- [ ] ディレクトリ構造が VS Code 拡張パック仕様に準拠している

**テスト項目**：

- [ ] ディレクトリ作成の検証

---

## ステップ 2：package.json の作成

**目標**：

- YaoXiang 言語 ID を定義
- ファイル拡張子 `.yx` を関連付け
- 組み込み LSP クライアントサポートを設定

**コア設定**：

```json
{
  "name": "yaoxiang",
  "displayName": "YaoXiang Language",
  "description": "YaoXiang programming language support",
  "languages": [{
    "id": "yaoxiang",
    "aliases": ["YaoXiang", "yx"],
    "extensions": [".yx"]
  }],
  "grammars": {
    "language": "yaoxiang",
    "scopeName": "source.yaoxiang"
  }
}
```

**受け入れ基準**：

- [ ] package.json に正しい言語 ID 設定が含まれている
- [ ] ファイル拡張子 `.yx` が関連付けられている
- [ ] 言語表示名が "YaoXiang" である

**テスト項目**：

- [ ] package.json の構文検証
- [ ] 設定の完全性チェック

---

## ステップ 3：language-configuration.json の作成

**目標**：

- 行コメント形式 `//` を設定
- ブロックコメント形式 `/* */` を設定
- 括弧マッチングルールを設定
- 自動インデントルールを設定

**コア設定**：

```json
{
  "comments": {
    "lineComment": "//",
    "blockComment": ["/*", "*/"]
  },
  "brackets": [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"]
  ],
  "indentationRules": {
    "increaseIndentPattern": "^.*\\{[^}]*$",
    "decreaseIndentPattern": "^\\s*\\}"
  }
}
```

**受け入れ基準**：

- [ ] 行コメントで `//` が使用されている
- [ ] ブロックコメントで `/* */` が使用されている
- [ ] 括弧マッチングが正常に動作する

**テスト項目**：

- [ ] VS Code で .yx ファイルを開き、コメントショートカット（Ctrl+/）が有効であることを検証
- [ ] 括弧マッチングハイライトの検証

---

## ステップ 4：（オプション）構文ハイライトの作成

**目標**：

- YaoXiang のキーワードに基づいて TextMate 構文定義を作成
- キーワード、文字列、数字、コメントの色分けをサポート

**YaoXiang キーワード一覧**：

- 制御フロー：`if`, `elif`, `else`, `match`, `while`, `for`, `in`, `return`, `break`, `continue`
- 宣言：`pub`, `use`, `spawn`, `ref`, `mut`
- 型：`as`, `unsafe`

**TextMate 構文構造**：

```json
{
  "name": "YaoXiang",
  "patterns": [
    {
      "include": "#keywords"
    },
    {
      "include": "#strings"
    },
    {
      "include": "#numbers"
    },
    {
      "include": "#comments"
    }
  ]
}
```

**受け入れ基準**：

- [ ] キーワードが正しく色分けされている
- [ ] 文字列が正しく色分けされている
- [ ] 数字が正しく色分けされている
- [ ] コメントが正しく色分けされている

**テスト項目**：

- [ ] .yx ファイルを開き、構文ハイライト効果を検証
- [ ] 各種トークンタイプの色分けが正しいか確認

---

## ステップ 5：README.md の作成

**目標**：

- 拡張パックのインストール手順を提供
- LSP サーバー設定方法を説明

**受け入れ基準**：

- [ ] README にインストール手順が含まれている
- [ ] README に LSP 設定の説明が含まれている

---

## 受け入れ基準のまとめ

| ステップ | 受け入れ項目 | ステータス |
|---------|-------------|-----------|
| 1 | ディレクトリ構造の作成 | ⬜ |
| 2 | package.json 設定 | ⬜ |
| 3 | language-configuration.json | ⬜ |
| 4 | 構文ハイライト（オプション） | ⬜ |
| 5 | README.md | ⬜ |

---

## テスト項目のまとめ

### 手動テスト

1. **ステップ 2**：package.json 構文の検証
2. **ステップ 3**：
   - .yx ファイルを開き、Ctrl+/ でコメントが有効か検証
   - 括弧を入力してマッチングハイライトを検証
3. **ステップ 4**：キーワード、文字列、数字、コメントの色分けを検証
4. **ステップ 5**：ドキュメントの可読性を検証

---

## 今後の拡張

LSP サーバーの実装完了後、以下の機能を拡張できる：

1. **自動 LSP 検出**：拡張パックが `yaoxiang-lsp` が PATH にあるかを自動検出
2. **ステータスバー統合**：LSP 接続状態を表示
3. **デバッグ統合**：DAP ベースのデバッグエントリ
4. **プロジェクトテンプレート**：ワンクリックで YaoXiang プロジェクトを作成

---

## 参考資料

- [VS Code Extension Guidelines](https://code.visualstudio.com/api)
- [Language Extension Overview](https://code.visualstudio.com/api/language-extensions)
- [Syntax Highlight Guide](https://code.visualstudio.com/api/language-extensions/syntax-highlight-guide)