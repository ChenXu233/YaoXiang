# YaoXiang ドキュメントディレクトリ再構築計画

## 概要

**目標**：`docs/` ディレクトリを再構築し、独立した**設計議論エリア**と**実装計画追跡**ディレクトリを作成する

---

## 目標ディレクトリ構造

```
docs/
├── design/                    # ⭐ 設計議論エリア（新規コアディレクトリ）
│   ├── README.md              # 設計ドキュメントインデックス
│   ├── manifesto.md           # 設計宣言
│   ├── language-spec.md       # 言語仕様
│   ├── async-whitepaper.md    # 非同期白書
│   ├── 00-wtf.md              # 設計のトレードオフ（FAQ）
│   └── 01-philosophy.md       # 設計哲学（「2006年生まれ者の言語設計観.md」から改名）
│
├── design/rfc/                # RFC スタイル設計提案（オプション）
│   └── (将来の提案)
│
├── design/discussion/         # オープン議論エリア（草案）
│   └── (議論中の設計ドキュメント)
│
├── plans/                     # ⭐ 実施計画（works/plans から昇格）
│   ├── README.md
│   ├── book-improvement.md
│   ├── stdlib-implementation.md
│   ├── test-organization.md
│   └── async/
│       ├── implementation-plan.md
│       └── threading-safety.md
│
├── implementation/            # ⭐ 実装追跡（新規）
│   ├── README.md
│   ├── phase1/
│   │   └── type-check-inference.md
│   └── phase5/
│       ├── bytecode-generation.md
│       └── gap-analysis.md
│
├── architecture/              # アーキテクチャ設計（維持）
├── guides/                    # ユーザーガイド（維持）
├── examples/                  # サンプルコード（維持）
└── reference/                 # リファレンス（維持）
```

---

## ディレクトリ責務説明

| ディレクトリ | 責務 | コンテンツタイプ |
|------|------|----------|
| `design/` | 完了した設計意思決定の議論 | 宣言、仕様、白書、設計トレードオフ |
| `design/rfc/` | 提案中の設計（オプション） | RFC ドキュメント、草案 |
| `design/discussion/` | 討議中の設計 | オープンな問題、議論中の草案 |
| `plans/` | 実施予定の計画 | 実施ロードマップ、課題分解 |
| `implementation/` | 完了/進行中の実装詳細 | 技術詳細、段階レポート |

---

## 移行チェックリスト

### 1. `design/` への移動

| 元の位置 | 新しい位置 |
|--------|--------|
| `docs/YaoXiang-design-manifesto.md` | `docs/design/manifesto.md` |
| `docs/YaoXiang-language-specification.md` | `docs/design/language-spec.md` |
| `docs/YaoXiang-async-whitepaper.md` | `docs/design/async-whitepaper.md` |
| `docs/YaoXiang-WTF.md` | `docs/design/00-wtf.md` |
| `docs/一个2006年出生者的语言设计观.md` | `docs/design/01-philosophy.md` |

### 2. `works/plans/` をルートレベルへ昇格

| 元の位置 | 新しい位置 |
|--------|--------|
| `docs/plans/` | `docs/plans/` |

### 3. `implementation/` への移動

| 元の位置 | 新しい位置 |
|--------|--------|
| `docs/works/phase/phase1/type-check-inference-rules.md` | `docs/implementation/phase1/type-check-inference.md` |
| `docs/works/phase/phase5/phase5-bytecode-generation.md` | `docs/implementation/phase5/bytecode-generation.md` |
| `docs/works/phase/phase5/phase5-implementation-gap-analysis.md` | `docs/implementation/phase5/gap-analysis.md` |

### 4. 現状維持

| ディレクトリ | 説明 |
|------|------|
| `docs/architecture/` | アーキテクチャ設計は独立しているので現状維持 |
| `docs/guides/` | ユーザーガイドは独立しているので現状維持 |
| `docs/examples/` | サンプルコード、現状維持 |
| `docs/works/old/` | 歴史的アーカイブ、現状維持または削除 |
| `docs/plans/async/` | `plans/async/` へ昇格済み |

### 5. オプション：`docs/README.md` の更新

新しいディレクトリ構造を反映するためにドキュメントインデックスを更新する必要がある。

---

## 実行手順

### 手順 1：ディレクトリ構造の作成

```bash
mkdir -p docs/design/discussion
mkdir -p docs/design/rfc
mkdir -p docs/plans/async
mkdir -p docs/implementation/phase1
mkdir -p docs/implementation/phase5
```

### 手順 2：設計ドキュメントの移動

```bash
# design/ への移動
mv docs/YaoXiang-design-manifesto.md docs/design/manifesto.md
mv docs/YaoXiang-language-specification.md docs/design/language-spec.md
mv docs/YaoXiang-async-whitepaper.md docs/design/async-whitepaper.md
mv docs/YaoXiang-WTF.md docs/design/00-wtf.md
mv "docs/一个2006年出生者的语言设计观.md" docs/design/01-philosophy.md

# design/discussion/ への移動（オプション：議論中の草案を格納）
```

### 手順 3：plans ディレクトリの昇格

```bash
# works/plans をルートレベルへ移動
mv docs/plans/* docs/plans/
rmdir docs/works/plans
```

### 手順 4：実装ドキュメントの移動

```bash
# implementation/ への移動
mv docs/works/phase/phase1/type-check-inference-rules.md docs/implementation/phase1/type-check-inference.md
mv docs/works/phase/phase5/phase5-bytecode-generation.md docs/implementation/phase5/bytecode-generation.md
mv docs/works/phase/phase5/phase5-implementation-gap-analysis.md docs/implementation/phase5/gap-analysis.md
```

### 手順 5：`docs/README.md` の更新

ドキュメントインデックスを更新し、新しいディレクトリ説明を追加する。

### 手順 6：空ディレクトリのクリーンアップ

```bash
rmdir docs/works/phase/phase5
rmdir docs/works/phase/phase1
rmdir docs/works/phase
rmdir docs/works/old/archived
rmdir docs/works/old
```

---

## 後方互換性

⚠️ **重要**：この再構築は既存のリファレンスを破壊する可能性がある。以下の点を推奨する：

1. **元のファイルを削除しない**：まずシンボリックリンクを作成するか、移動後に検証する
2. **すべての内部リンクを更新する**：`docs/**/*.md` 内の相対パス参照を確認する
3. **IDE 設定を更新する**：`.vscode` やその他の設定が存在する場合は更新する

---

## 期待されるメリット

1. **責務の明確化**：設計 vs 計画 vs 実装、境界が明確
2. **アクセスの利便性**：`design/` と `plans/` がルートレベルにあるため、`works/` 深处まで移動する必要がない
3. **拡張性**：`design/rfc/` と `design/discussion/` の追加により RFC プロセスがサポートされる
4. **ドキュメントタイプの明確化**：完了した設計、議論中の設計、実施計画、実装追跡がそれぞれ適切な場所に配置される

---

## 注意事項

- `works/` ディレクトリのアーカイブコンテンツを保持するかどうかを確認する
- 他のドキュメントがこのファイルパスを参照していないか確認する
- `design/rfc/` 向けに RFC テンプレートの必要性を検討する