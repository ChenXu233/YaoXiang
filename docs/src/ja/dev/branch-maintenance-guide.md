# Git ブランチメンテナンスマニュアル

> 本書は YaoXiang プロジェクトの Git ブランチ管理戦略を定義したものであり、コードベースの秩序立った開発と効率的な協業を保証することを目的としています。

---

## 📋 目次

- [ブランチタイプ規範](#ブランチタイプ規範)
- [命名規則](#命名規則)
- [ブランチライフサイクル](#ブランチライフサイクル)
- [ワークフロー](#ワークフロー)
- [ブランチ保護戦略](#ブランチ保護戦略)
- [ベストプラクティス](#ベストプラクティス)
- [よくある質問](#よくある質問)

---

## 🏷️ ブランチタイプ規範

### コアブランチ（Core Branches）

| ブランチ名 | 用途 | ライフサイクル | 保護レベル |
|--------|------|----------|----------|
| `main` | 本番環境コード | 永久 | 厳格保護 |
| `dev` | メイン開発ブランチ | 永久 | 中程度保護 |
| `master` | メーンブランチ（互換性） | 永久 | 厳格保護 |

### 機能ブランチ（Feature Branches）

| プレフィックス | 用途 | 命名例 | マージ先 |
|------|------|----------|----------|
| `feature/` | 新機能開発 | `feature/type-inference`<br>`feature/ownership-model` | `dev` |
| `bugfix/` | 既知の欠陥修正 | `bugfix/memory-leak`<br>`bugfix/parser-error` | `dev` |
| `hotfix/` | 緊急の本番問題修正 | `hotfix/security-patch`<br>`hotfix/crash-bug` | `main` + `dev` |
| `release/` | リリース準備ブランチ | `release/v0.8.0`<br>`release/v1.0.0` | `main` |

### 補助ブランチ（Auxiliary Branches）

| プレフィックス | 用途 | 命名例 | マージ先 |
|------|------|----------|----------|
| `docs/` | ドキュメント更新 | `docs/api-reference`<br>`docs/tutorial-update` | `dev` |
| `ci/` | CI/CD 設定変更 | `ci/add-deploy-script`<br>`ci/optimize-build` | `dev` |
| `refactor/` | コードリファクタリング | `refactor/lexer-optimization`<br>`refactor/memory-manager` | `dev` |
| `test/` | テスト関連の変更 | `test/add-integration`<br>`test/performance-bench` | `dev` |

---

## 📝 命名規則

### 基本命名フォーマット

```bash
# 機能ブランチ
<type>/<short-description>

# 例
feature/add-type-inference
bugfix/fix-parser-crash
hotfix/security-vulnerability
```

### 命名規範

1. **小文字を使用**：すべてのブランチ名は小文字を使用
2. **ハイフンで区切る**：単語の区切りには `-` を使用し、アンダースコアは使用しない
3. **説明的な命名**：ブランチ名はその目的を明確に表現する
4. **特殊文字を避ける**：スペース、ドット、その他の特殊文字を使用しない
5. **長さの制限**：ブランチ名は 50 文字を超えないこと

### 詳細な例

```bash
# ✅ 良い命名
feature/user-authentication-system
bugfix/fix-compilation-error-on-windows
hotfix/memory-leak-in-vm
docs/update-api-documentation
refactor/optimize-lexer-performance
test/add-e2e-test-cases

# ❌ 悪い命名
Feature/NewFeature  # 大文字を使用
bug_fix            # アンダースコアを使用
hotfix/fix        # 説明が不鮮明
feature/ADD_NEW_FEATURE_WITH_LOTS_OF_DETAILS_THAT_IS_TOO_LONG  # 長すぎる
```

---

## 🔄 ブランチライフサイクル

### ブランチ作成

```bash
# 1. 最新の dev ブランチから作成
git checkout dev
git pull origin dev
git checkout -b feature/your-feature-name

# 2. リモートブランチにプッシュ
git push -u origin feature/your-feature-name
```

### ブランチ開発

```bash
# 定期的に最新コードを同期
git checkout dev
git pull origin dev
git checkout feature/your-feature-name
git rebase dev  # または git merge dev

# コードをコミット
git add .
git commit -m ":sparkles: feat(frontend): タイプ推論機能を追加"
git push origin feature/your-feature-name
```

### ブランチマージ

```bash
# 1. Pull Request を作成
# 2. コードレビュー通過後
git checkout dev
git pull origin dev
git merge --no-ff feature/your-feature-name
git push origin dev

# 3. ブランチをクリーンアップ
git branch -d feature/your-feature-name  # ローカル削除
git push origin --delete feature/your-feature-name  # リモート削除
```

### ブランチ削除

```bash
# マージ済みの機能ブランチを削除
git branch -d feature/completed-feature
git push origin --delete feature/completed-feature

# マージ済みブランチを一括クリーンアップ
git branch --merged dev | grep feature | xargs -n 1 git branch -d
```

---

## 🚀 ワークフロー

### 機能開発フロー

```mermaid
graph TD
    A[dev ブランチ] --> B[feature ブランチを作成]
    B --> C[機能を開発]
    C --> D[コードをコミット]
    D --> E[PR を dev に作成]
    E --> F[コードレビュー]
    F -->|通過| G[dev にマージ]
    F -->|拒否| C
    G --> H[feature ブランチを削除]
    G --> I[CI/CD がトリガー]
```

### 緊急修正フロー

```mermaid
graph TD
    A[main ブランチ] --> B[hotfix ブランチを作成]
    B --> C[問題を修正]
    C --> D[コードをコミット]
    D --> E[PR を main + dev に作成]
    E --> F[クイックレビュー]
    F --> G[main と dev に同時にマージ]
    G --> H[hotfix をリリース]
    I[hotfix ブランチを削除]
```

### リリースフロー

```mermaid
graph TD
    A[dev ブランチ] --> B[release ブランチを作成]
    B --> C[バージョン準備]
    C --> D[テスト検証]
    D --> E[PR を main に作成]
    E --> F[最終レビュー]
    F --> G[main にマージ]
    G --> H[バージョンタグを打つ]
    H --> I[dev にマージバック]
    J[release ブランチをクリーンアップ]
```

---

## 🛡️ ブランチ保護戦略

### メインブランチ保護

**main ブランチ**
- 直接プッシュを禁止
- PR を介したマージのみ
- フォースプッシュを禁止
- コードレビューを要求
- ステータスチェックの通過を必須

**dev ブランチ**
- 直接プッシュを禁止（開発メンバー）
- PR マージを要求
- ステータスチェックの通過を必須
- 管理者の直接プッシュを許可

### ブランチ権限設定

| ブランチタイプ | 開発者 | メンテナー | 管理者 |
|----------|--------|--------|--------|
| `main` | PR のみ | PR のみ | PR 承認 |
| `dev` | PR マージ | PR マージ | 直接プッシュ |
| `feature/*` | フル権限 | フル権限 | フル権限 |
| `hotfix/*` | フル権限 | フル権限 | フル権限 |

---

## ✅ ベストプラクティス

### 1. ブランチ管理

- **頻繁な同期**：`dev` ブランチから定期的に最新コードを取得
- **アトミックコミット**：各コミットには関連性のある変更のみを含める
- **タイムリーなクリーンアップ**：マージ後はすぐに完了した機能ブランチを削除
- **明確な説明**：ブランチ名とコミットメッセージは意図を明確に表現する

### 2. コミット規範

[コミット規範](./commit-convention.md)に従ってください：

```bash
# フォーマット
:emoji: type(scope): テーマ（日本語）

# 例
:sparkles: feat(frontend): タイプ推論機能を追加
:bug: fix(parser): パーサークラッシュ問題を修正
:recycle: refactor(vm): 仮想マシン内存管理をリファクタリング
```

### 3. Pull Request

- **明確な説明**：変更内容と理由を詳しく記載
- ** issue との関連付け**：`Closes #123` を使用して関連 Issue を紐付け
- **タイムリーな対応**：レビューの意見には速やかに返信
- **十分なテスト**：すべてのテストが通過していることを確認

### 4. コードレビュー

- **機能の正しさ**：コードの機能が正しいか確認
- **コード品質**：コードが規範是否符合かく確認
- **テストカバレッジ**：適切なテストがあるか確認
- **ドキュメント更新**：ドキュメントの更新が必要か確認

---

## ❓ よくある質問

### Q1: ブランチタイプはどのように選択すればよいですか？

**答：**
- 新機能 → `feature/`
- 既知の欠陥修正 → `bugfix/`
- 緊急の本番修正 → `hotfix/`
- ドキュメント更新 → `docs/`
- コードリファクタリング → `refactor/`
- テスト関連 → `test/`

### Q2: feature ブランチはどのブランチから作成すべきですか？

**答：**
常に `dev` ブランチから作成し、機能が最新の開発コードに基づいていることを確認してください：

```bash
git checkout dev
git pull origin dev
git checkout -b feature/new-feature
```

### Q3: release ブランチはいつ作成すべきですか？

**答：**
- 新バージョンのリリースを準備する時
- 新機能の追加をフリーズする必要がある時
- 安定バージョンの专门的なテストが必要な時

### Q4: ブランチの競合（コンフリクト）はどのように処理しますか？

**答：**
1. 対象ブランチを更新：`git checkout dev && git pull origin dev`
2. 機能ブランチに切り替え：`git checkout feature/your-branch`
3. マージして競合を解決：`git rebase dev` または `git merge dev`
4. 競合解決後、开发を続ける

### Q5: hotfix ブランチはどのように處理しますか？

**答：**
1. `main` ブランチから作成：`git checkout main && git checkout -b hotfix/urgent-fix`
2. 問題を修正してテスト
3. `main` と `dev` に同時に PR を作成
4. マージ後立即デプロイ

### Q6: ブランチ名の長さに制限はありますか？

**答：**
50 文字を超えないことを推奨し、簡潔で明了に保ってください。Git は本身はより長い名前をサポートしていますが、过长な名前は可読性に影響します。

---

## 📚 関連ドキュメント

- [コミット規範](./commit-convention.md)
- [コードレビューガイド](./code-review.md)
- [リリースプロセス](./release-guide.md)
- [CI/CD 設定](../../.github/workflows/)

---

## 🔧 ツールとスクリプト

### マージ済みブランチの一括クリーンアップ

```bash
# dev にマージ済みのローカルブランチを削除
git checkout dev
git pull origin dev
git branch --merged dev | grep -E "^(feature|bugfix|docs|refactor|test)/" | xargs -n 1 git branch -d

# リモートでマージ済みのブランチを削除
git remote prune origin
```

### ブランチ作成テンプレート

```bash
#!/bin/bash
# 機能ブランチ作成のヘルパースクリプト

BRANCH_TYPE=$1
BRANCH_NAME=$2

if [ -z "$BRANCH_TYPE" ] || [ -z "$BRANCH_NAME" ]; then
    echo "用法: $0 <タイプ> <ブランチ名>"
    echo "タイプ: feature, bugfix, hotfix, docs, refactor, test"
    exit 1
fi

git checkout dev
git pull origin dev
git checkout -b "$BRANCH_TYPE/$BRANCH_NAME"
git push -u origin "$BRANCH_TYPE/$BRANCH_NAME"

echo "ブランチを作成・プッシュしました: $BRANCH_TYPE/$BRANCH_NAME"
```

---

> 💡 **ヒント**：ブランチのアトミック性と集中性を保ち、各ブランチで1つのことだけを遂行すれば、コード管理がより清晰で効率的になります！

> 📞 **サポート**：質問がある場合は、GitHub Discussions で議論してください。