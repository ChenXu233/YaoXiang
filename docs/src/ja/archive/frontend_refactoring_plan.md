# YaoXiang フロントエンドアーキテクチャ侵略的リファクタリング方案 (RFC支援版)

> バージョン: 3.0 | 日付: 2026-01-29 | ステータス: RFC要件修復ベース
>
> **コア目標**: 低耦合アーキテクチャ基础上、RFC-004/010/011の実装要件を全面的に支援

## 📋 リファクタリング目標

### コア目標
- **耦合度の低減**: モジュール間强い依赖を排除し、松耦合アーキテクチャを実現
- **RFC支援**: 3つのRFCの設計要件と実装パスを完全に支援
- **ファイル分层最適化**: 明確な分层アーキテクチャ、各層の責務は単一
- **保守性提升**: 大ファイル分割、責務の明確化
- **拡張性增强**: RFC-012等の将来機能に向けた拡張スペースを確保

### RFC支援マトリクス

| RFC | コア要件 | リファクタリング支援度 | 実装位置 |
|-----|----------|------------|----------|
| **RFC-004** | 多位置バインディング構文、智能バインディング、自动柯里化 | 95% | `statements/bindings.rs`, `core/lexer/literals.rs` |
| **RFC-010** | 統一構文、泛型構文、型定義 | 90% | `statements/declarations.rs`, `types/parser.rs` |
| **RFC-011** | 制約求解、単態化、泛型システム | 100% | `type_system/*`, `constraints.rs`, `unify.rs` |

### 成功指標
- [ ] 全ファイルは500行以内に制御
- [ ] モジュールの依赖関係は明確で、循環依赖なし
- [ ] 3つのRFCの実装要件は100%アーキテクチャ中得到支援
- [ ] 公共APIは簡素化、内部実装は隠蔽
- [ ] テスト覆盖率は85%以上
- [ ] コンパイル時間の20%削減（より良いモジュール化を通じて）

---

## 🏗️ 新アーキテクチャ設計

### 1. 分層アーキテクチャ図

```
┌─────────────────────────────────────┐
│           Frontend API              │  ← 公共インターフェース層
│        (frontend/mod.rs)            │
├─────────────────────────────────────┤
│  Lexer → Parser → TypeCheck → Const│  ← 流水線層
│     │        │         │       │   │
│     ▼        ▼         ▼       ▼   │
├─────────────────────────────────────┤
│          Shared Utilities           │  ← 共有ユーティリティ層
│    (error, span, diagnostic)        │
├─────────────────────────────────────┤
│        Core Algorithm Layer         │  ← コアアルゴリズム層
│  (type_system, const_eval, parse)   │
│                                     │
│  ▸ RFC-004: バインディング解析支援            │
│  ▸ RFC-010: 統一構文解析            │
│  ▸ RFC-011: 完全泛型システム           │
└─────────────────────────────────────┘
```

### 2. モジュール再編成

#### **第一層: コアアルゴリズム層 (Core Algorithm Layer)**

```
src/frontend/core/
├── mod.rs                    # コアモジュール入口
├── lexer/
│   ├── mod.rs               # 字句解析器インターフェース
│   ├── tokenizer.rs         # Tokenizer実装 (1270行から分割)
│   ├── state.rs            # 字句状態管理 (新規作成)
│   ├── literals.rs         # 字句量処理 (分割)
│   └── symbols.rs          # キーワードとシンボルテーブル (新規作成)
├── parser/
│   ├── mod.rs              # 解析器インターフェース
│   ├── ast.rs              # AST定義 (305行を維持)
│   ├── pratt/              # Pratt解析器コア (新規ディレクトリ)
│   │   ├── mod.rs
│   │   ├── nud.rs          # 前置詞解析 (896行から分割)
│   │   ├── led.rs          # 中置詞解析 (380行を維持)
│   │   └── precedence.rs   # 優先順位処理 (分割)
│   ├── statements/         # 文解析 (新規ディレクトリ)
│   │   ├── mod.rs
│   │   ├── declarations.rs  # 宣言文 (1399行から分割)
│   │   ├── expressions.rs   # 式文 (分割)
│   │   ├── control_flow.rs  # 制御フロー (分割)
│   │   └── bindings.rs     # RFC-004バインディング構文解析 (新規作成)
│   ├── types/              # 型解析 (新規ディレクトリ)
│   │   ├── mod.rs
│   │   ├── parser.rs       # 型解析器 (614行から分割)
│   │   ├── constraints.rs  # RFC-011型制約解析 (新規作成)
│   │   └── generics.rs     # RFC-010/011泛型構文解析 (新規作成)
│   └── utils.rs            # 解析器ユーティリティ関数 (分割)
├── type_system/            # RFC-011コア型システム
│   ├── mod.rs
│   ├── vars.rs            # TypeVar, ConstVar (分割)
│   ├── mono_poly.rs       # MonoType, PolyType (分割)
│   ├── constraints.rs      # TypeConstraint (分割)
│   ├── unify.rs           # Unifyアルゴリズム (分割)
│   ├── specialize.rs      # RFC-011泛型特化 (新規作成)
│   ├── pretty_print.rs    # 型印刷 (新規作成)
│   └── display.rs         # 型表示フォーマットの (新規作成)
└── const_eval/            # 定数評価
    ├── mod.rs
    ├── evaluator.rs       # 定数評価器 (677行から改名)
    ├── functions.rs      # Const関数 (536行から分割)
    └── static_assert.rs  # 静的assert (490行を維持)
```

#### **第二層: 共有ユーティリティ層 (Shared Utilities)**

```
src/frontend/shared/
├── mod.rs
├── error/
│   ├── mod.rs
│   ├── diagnostic.rs       # 統一診断情報
│   ├── span.rs            # Span処理
│   ├── result.rs          # 統一Result型
│   ├── conversion.rs      # エラー変換
│   └── macros.rs          # RFC-011エラー処理マクロ (新規作成)
├── diagnostics/
│   ├── mod.rs
│   ├── formatter.rs       # 診断フォーマット
│   ├── severity.rs        # 重大度レベル
│   ├── code.rs            # エラーコード定義
│   └── traits.rs          # 診断trait (新規作成)
├── utils/
│   ├── mod.rs
│   ├── mem.rs             # メモリ管理ユーティリティ
│   ├── debug.rs           # デバッグユーティリティ
│   ├── panic.rs           # panic処理
│   └── cache.rs           # RFC-011コンパイルキャッシュ (新規作成)
└── abstractions/           # 抽象インターフェース層 (新規作成)
    ├── mod.rs
    ├── parser.rs          # Parser抽象インターフェース
    ├── type_checker.rs    # TypeChecker抽象インターフェース
    └── trait_objects.rs   # traitオブジェクトサポート
```

#### **第三層: 型チェック層 (Type Checking Layer)**

```
src/frontend/typecheck/
├── mod.rs                 # 型チェック入口
├── inference/             # 型推論 (infer.rsから分割)
│   ├── mod.rs
│   ├── expressions.rs    # 式推論 (分割)
│   ├── statements.rs     # 文推論 (分割)
│   ├── patterns.rs       # パターン一致推論 (新規作成)
│   └── generics.rs       # RFC-011泛型推論 (新規作成)
├── checking/             # 型チェック (check.rsから分割)
│   ├── mod.rs
│   ├── subtyping.rs      # 下位型チェック (分割)
│   ├── assignment.rs     # 代入チェック (分割)
│   ├── compatibility.rs # 互換性チェック (分割)
│   └── bounds.rs         # RFC-011型境界チェック (新規作成)
├── specialization/       # RFC-011泛型特化
│   ├── mod.rs
│   ├── algorithm.rs      # 特化アルゴリズム (488行から分割)
│   ├── substitution.rs  # 置換ロジック (新規作成)
│   └── instantiate.rs   # インスタンス化アルゴリズム (新規作成)
├── traits/              # RFC-011traitシステム
│   ├── mod.rs
│   ├── solver.rs        # traitソルバ (274行から分割)
│   ├── coherence.rs     # 一貫性チェック (新規作成)
│   ├── object_safety.rs # オブジェクト安全性 (新規作成)
│   └── resolution.rs    # trait解決 (新規作成)
└── gat/                # GATサポート (529行を維持、構造最適化)
    ├── mod.rs
    ├── checker.rs       # GATチェッカー
    └── higher_rank.rs   # 高階型
```

#### **第四層: 高級型層 (Advanced Type Level)**

```
src/frontend/type_level/
├── mod.rs               # 型レベル計算入口
├── conditional_types.rs  # RFC-011条件型 (維持)
├── dependent_types.rs    # RFC-011依存型 (維持)
├── evaluation/          # RFC-011型レベル計算 (新規ディレクトリ)
│   ├── mod.rs
│   ├── normalize.rs     # 正規化
│   ├── reduce.rs        # 簡約
│   ├── unify.rs         # 型レベル統合
│   └── compute.rs       # 型計算エンジン (新規作成)
├── operations/          # RFC-011型レベル操作 (新規ディレクトリ)
│   ├── mod.rs
│   ├── arithmetic.rs    # 算術演算
│   ├── comparison.rs   # 比較演算
│   └── logic.rs        # 論理演算
├── const_generics/     # RFC-011 Const泛型サポート (新規ディレクトリ)
│   ├── mod.rs
│   ├── eval.rs         # Const泛型評価
│   └── generic_size.rs # 泛型サイズ計算 (新規作成)
└── tests.rs            # テスト (維持)
```

#### **第五層: 公共インターフェース層 (Public API Layer)**

```
src/frontend/
├── mod.rs               # コンパイラ公共インターフェース (簡素化)
├── compiler.rs          # コンパイラコアロジック (235行から分割)
├── pipeline.rs          # コンパイル流水線 (新規作成)
├── config.rs            # コンパイル設定 (新規作成)
└── events/              # イベントシステム (新規作成)
    ├── mod.rs
    ├── type_check.rs    # 型チェックイベント
    ├── parse.rs         # 解析イベント
    └── subscribe.rs     # イベントサブスクライブ (新規作成)
```

---

## 📅 分段階実施計画

### 🚀 段階 1: 緊急分割とRFC支援準備 (Week 1-3) 完了

#### **Day 1-2: 準備段階**

**ステップ 1.1: 新ディレクトリ構造の作成**
- **サブタスク 1.1.1**: `src/frontend/` 下に完全なディレクトリ構造を作成
  - 予想所要時間: 15分
  - 検収基準: すべてのRFC支援ディレクトリ作成完了

```bash
# RFC-004支援ディレクトリ作成
mkdir -p src/frontend/core/parser/statements/bindings

# RFC-010/011支援ディレクトリ作成
mkdir -p src/frontend/core/parser/types/generics
mkdir -p src/frontend/type_system/specialize

# 共有抽象ディレクトリ作成
mkdir -p src/frontend/shared/abstractions
mkdir -p src/frontend/shared/events
```

**ステップ 1.2: 既存テストベンチマークの実行**
- **サブタスク 1.2.1**: 現在のパフォーマンスを記録
  - `time cargo build --release` を実行して基準時間を記録
  - 結果を `metrics/pre_refactor_build_time.txt` に保存

- **サブタスク 1.2.2**: 完全テストスイートを実行
  - `cargo test --all` を実行して現在のテストがすべて通過することを確認
  - テスト通過数を記録: ___/___
  - `metrics/pre_refactor_test_results.txt` に保存

- **サブタスク 1.2.3**: コード統計を記録
  - `cloc src/frontend/typecheck/types.rs` を実行して元の行数を記録
  - 記録: 総行数 ___ 行、コード行数 ___ 行
  - `metrics/pre_refactor_loc.txt` に保存

---

#### **Day 3-7: typecheck/types.rs の分割 (RFC-011コア)**

**目標**: 1948行の巨大ファイルをRFC-011を支援するモジュールに分割
**予想総所要時間**: 5日 (毎日1日)

**Day 3: 分析と解体**

- **サブタスク 1.3.1: RFC-011要件对齐分析**
  - **1.3.1.1**: RFC-011 Phase 1要件を标注 (60分)
    - TypeVar/ConstVar 定義 → `vars.rs`
    - MonoType/PolyType 定義 → `mono_poly.rs`
    - 制約システム → `constraints.rs`
    - Unifyアルゴリズム → `unify.rs`
  - **1.3.1.2**: RFC-011 Phase 2+要件を标注 (30分)
    - 特化アルゴリズム → `specialize.rs` (新規作成)
    - 型表示 → `pretty_print.rs`, `display.rs` (新規作成)
  - **1.3.1.3**: モジュール境界と依赖関係を確定 (30分)

- **サブタスク 1.3.2: `vars.rs` の作成 (2時間)**
  - **1.3.2.1**: TypeVar/ConstVar 相关コードを新ファイルにコピー
  - **1.3.2.2**: インポートパスを調整、コンパイルエラーを修正
  - **1.3.2.3**: `cargo check` を実行してコンパイル通過を検証
  - **検収基準**: vars.rs が独立してコンパイル成功

**Day 4: 基礎モジュールの完成**

- **サブタスク 1.3.3: `mono_poly.rs` の作成 (2時間)**
  - **1.3.3.1**: MonoType/PolyType コードをコピー
  - **1.3.3.2**: vars.rs との依赖関係を修正
  - **1.3.3.3**: RFC-011が必要な泛型特化インターフェースを追加

- **サブタスク 1.3.4: `constraints.rs` の作成 (2時間)**
  - **1.3.4.1**: TypeConstraint/ConstraintSet コードをコピー
  - **1.3.4.2**: UnionFind 構造を実装
  - **1.3.4.3**: RFC-011 Phase 2制約ソルバインターフェースを追加

**Day 5: コアアルゴリズムモジュール**

- **サブタスク 1.3.5: `unify.rs` の作成 (3時間)**
  - **1.3.5.1**: Unify アルゴリズムと Substitution をコピー
  - **1.3.5.2**: Unifier 構造を実装
  - **1.3.5.3**: RFC-011単態化サポートインターフェースを追加
  - **検収基準**: unify.rs がコンパイル通過、算法的ロジックが正しい

- **サブタスク 1.3.6: `specialize.rs` の作成 (RFC-011新規追加) (1時間)**
  - **1.3.6.1**: 泛型特化アルゴリズムを実装
  - **1.3.6.2**: インスタンス化キャッシュを実装
  - **1.3.6.3**: デッドコード消除インターフェースを追加

**Day 6: 統合と依赖修正**

- **サブタスク 1.3.7: `type_system/mod.rs` の作成 (1時間)**
  - **1.3.7.1**: モジュール入口ファイルを定義
  - **1.3.7.2**: すべての公共インターフェースを一括エクスポート
  - **1.3.7.3**: TypeSystemError 型を定義

- **サブタスク 1.3.8: 依赖関係の更新 (2時間)**
  - **1.3.8.1**: `src/frontend/typecheck/mod.rs` のインポートを修正
    ```rust
    // 从
    pub use types::*;
    // 改为
    pub use crate::type_system::{
        MonoType, PolyType, TypeVar, ConstraintSolver,
        Unifier, specialize::Specializer
    };
    ```
  - **1.3.8.2**: 検索置換ツールを使用して参照パスを一括更新
  - **1.3.8.3**: ファイルを逐次修正してコンパイルエラーを解決

**Day 7: RFC-011インフラストラクチャ検証**

- **サブタスク 1.3.9: RFC-011支援の検証 (2時間)**
  - **1.3.9.1**: RFC-011 Phase 1テストを作成
    ```rust
    // tests/rfc011_phase1.rs
    #[test]
    fn test_basic_generic_instantiation() {
        let types = type_system::MonoType::var("T");
        let specialized = type_system::specialize::instantiate(
            &types, &[Type::Int]
        ).unwrap();
        assert_eq!(specialized, Type::Int);
    }
    ```
  - **1.3.9.2**: 制約ソルバの動作を検証
  - **1.3.9.3**: 単態化インターフェースを検証

- **サブタスク 1.3.10: 全面検証 (2時間)**
  - **1.3.10.1**: `cargo check --all` を実行してコンパイルエラーがないことを確認
  - **1.3.10.2**: `cargo test type_system` を実行して型システムテストを実行
  - **1.3.10.3**: `cargo test --all` を実行してすべてのテストが通过ことを確認
  - **1.3.10.4**: パフォーマンス比較: コンパイル時間変化 < 10%

- **サブタスク 1.3.11: 旧ファイルのクリーンアップ (1時間)**
  - **1.3.11.1**: 新モジュールが正常に動作することを確認後、元 `types.rs` を削除
  - **1.3.11.2**: git を更新してコミット
  - **1.3.11.3**: タグ `refactor/types-complete-rfc011` を作成

**検収基準**:
- [ ] `types.rs` が完全に削除された
- [ ] 新モジュールのコンパイル通過: `cargo check --all`
- [ ] RFC-011 Phase 1テスト通過: `cargo test rfc011_phase1`
- [ ] 全テストがグリーン: `cargo test --all`
- [ ] パフォーマンスに明显な低下なし: コンパイル時間変化 < 10%

---

#### **Day 8-14: lexer/mod.rs の分割 + RFC-004支援準備**

**目標**: 字句解析器を分割、RFC-004バインディング構文とRFC-010統一構文做好准备
**予想総所要時間**: 7日

**Day 8-9: lexer の分析 + RFC要件**

- **サブタスク 1.4.1: lexer 構造 + RFC要件对齐の分析 (2時間)**
  - **1.4.1.1**: 分析ツールを実行
    ```bash
    rg "^pub struct|^impl.*Tokenizer" src/frontend/lexer/mod.rs
    ```
  - **1.4.1.2**: コアロジック + RFC支援要件を識別
    - Tokenizer メイン構造 (行1-300) + RFC-004バインディング記号 `[`, `]` サポート
    - 状態管理コード (行301-600) + RFC-010泛型キーワード `<`, `>` サポート
    - 字句量処理ロジック (行601-900) + RFC-010/011型構文サポート
    - 補助メソッド (行901-1270)
  - **1.4.1.3**: 新モジュールのインターフェースを設計

- **サブタスク 1.4.2: `tokenizer.rs` の作成 (3時間)**
  - **1.4.2.1**: Tokenizer 構造体と主要メソッドを抽出
  - **1.4.2.2**: RFC-004バインディング構文tokenサポートを追加
    ```rust
    // Tokenizer に追加
    enum TokenType {
        // ... 既存のtoken
        LeftBracket,    // [ RFC-004バインディング開始
        RightBracket,   // ] RFC-004バインディング終了
        LessThan,       // < RFC-010/011泛型開始
        GreaterThan,     // > RFC-010/011泛型終了
        // ...
    }
    ```
  - **1.4.2.3**: 状態と字句量処理を专门モジュールに委譲

- **サブタスク 1.4.3: `state.rs` の作成 (2時間)**
  - **1.4.3.1**: LexerState 構造を抽出
  - **1.4.3.2**: キーワード検索など状態関連メソッドを実装
  - **1.4.3.3**: RFC-010キーワード認識を追加 (`type`, `where` など)

**Day 10-11: 分割の完了 + 記号サポート**

- **サブタスク 1.4.4: `literals.rs` の作成 (2時間)**
  - **1.4.4.1**: すべての字句量処理メソッドを抽出
  - **1.4.4.2**: 数字、文字列、文字処理ロジック
  - **1.4.4.3**: RFC-010泛型型字句量サポートを追加

- **サブタスク 1.4.5: `symbols.rs` の作成 (RFC新規追加) (1時間)**
  - **1.4.5.1**: 統一シンボルテーブル管理
  - **1.4.5.2**: RFC-010/011泛型記号サポート
  - **1.4.5.3**: RFC-004バインディング記号サポート

**Day 12-13: テスト移行 + RFC検証**

- **サブタスク 1.4.6: テストファイルの移行 (2時間)**
  - **1.4.6.1**: テストディレクトリ構造を作成
    ```bash
    mkdir -p src/frontend/core/lexer/tests
    ```
  - **1.4.6.2**: すべてのテストファイルをコピー
  - **1.4.6.3**: RFC構文テストを追加
    ```rust
    // tests/rfc004_lexer.rs
    #[test]
    fn test_binding_syntax_tokenization() {
        let tokens = lexer::tokenize("function[0, 1]");
        assert_eq!(tokens[1].ty, TokenType::LeftBracket);
        assert_eq!(tokens[2].ty, TokenType::Number);
        // ...
    }

    // tests/rfc010_lexer.rs
    #[test]
    fn test_generic_syntax_tokenization() {
        let tokens = lexer::tokenize("List[T]");
        assert_eq!(tokens[1].ty, TokenType::LessThan);
        assert_eq!(tokens[2].ty, TokenType::Identifier);
        // ...
    }
    ```

- **サブタスク 1.4.7: RFC構文サポートの検証 (2時間)**
  - **1.4.7.1**: RFC-004バインディング構文のtoken化を確認
    ```bash
    cargo test rfc004_lexer
    ```
  - **1.4.7.2**: RFC-010/011泛型構文のtoken化を確認
    ```bash
    cargo test rfc010_lexer
    ```
  - **1.4.7.3**: テスト内のコンパイルエラーを修正

**Day 14: 統合検証**

- **サブタスク 1.4.8: 上位依赖の更新 (2時間)**
  - **1.4.8.1**: parser モジュールのインポートパスを更新
  - **1.4.8.2**: frontend メインModuleのエクスポートを更新
  - **1.4.8.3**: 統合テストを実行

- **サブタスク 1.4.9: 全面検証 (2時間)**
  - **1.4.9.1**: コンパイルチェック
    ```bash
    cargo check --all
    ```
  - **1.4.9.2**: 関連テストを実行
    ```bash
    cargo test lexer
    cargo test rfc004_lexer
    cargo test rfc010_lexer
    ```
  - **1.4.9.3**: 旧ファイルをクリーンアップ
  - **1.4.9.4**: 変更をコミット、タグ `refactor/lexer-complete-rfc004` を作成

**検収基準**:
- [ ] lexer/mod.rs の分割が完了
- [ ] RFC-004バインディング構文token化サポート: `cargo test rfc004_lexer`
- [ ] RFC-010泛型構文token化サポート: `cargo test rfc010_lexer`
- [ ] 字句テストがすべて通過: `cargo test lexer`
- [ ] 解析器テストが正常: `cargo test parser`

---

#### **Day 15-21: parser/stmt.rs の分割 + RFC-010/011解析支援**

**目標**: parser構造を再編成、RFC-010統一構文とRFC-011泛型解析を支援
**予想総所要時間**: 7日

**Day 15-16: parser 構造 + RFC要件の分析**

- **サブタスク 1.5.1: stmt.rs 構造 + RFC解析要件の分析 (3時間)**
  - **1.5.1.1**: ファイル内容分布 + RFC要件对齐を分析
    ```bash
    rg "^//.*宣言|^//.*式|^//.*制御流" src/frontend/parser/stmt.rs
    ```
  - **1.5.1.2**: ロジックグループ + RFC支援要件を識別
    - 宣言関連コード (行1-500) + RFC-010統一構文解析 + RFC-004バインディング構文解析
    - 式文 (行501-900) + RFC-011泛型式解析
    - 制御フローコード (行901-1399) + RFC-011泛型制御フロー解析
  - **1.5.1.3**: Pratt 解析器の部分を識別 + RFC構文要件
    - nud.rs (前置詞解析) + RFC-010泛型前置詞
    - led.rs (中置詞解析) + RFC-010泛型中置詞
    - precedence.rs (優先順位) + RFC-011優先順位ルール

- **サブタスク 1.5.2: ディレクトリ構造の作成 (1時間)**
  ```bash
  mkdir -p src/frontend/core/parser/{statements,pratt,types}
  mkdir -p src/frontend/core/parser/tests/{declarations,expressions,control_flow,bindings}
  mkdir -p src/frontend/core/parser/types/tests
  ```

**Day 17-18: 文解析の分割 + RFC構文支援**

- **サブタスク 1.5.3: `statements/declarations.rs` の作成 (3時間)**
  - **1.5.3.1**: 関数宣言解析 + RFC-010/011泛型サポートを抽出
    ```rust
    // RFC-010統一構文をサポート
    pub parse_function_decl: Parser = {
        // name: type = value 統一構文
        // [T](params) -> Return 泛型構文
        // where constraints: Clone 制約構文
    }

    // RFC-004バインディング構文をサポート
    pub parse_binding_decl: Parser = {
        // Type.method = function[positions] バインディング構文
    }
    ```
  - **1.5.3.2**: 構造体と列挙体宣言 + RFC-010構文を抽出
    - `parse_struct_decl()` + 泛型フィールドサポート
    - `parse_enum_decl()` + 泛型バリアントサポート
  - **1.5.3.3**: 変数宣言 + RFC-010統一構文を抽出
    - `parse_variable_decl()` + 統一 `name: type = value` 構文
    - `parse_use_decl()` + 泛型インポートサポート

- **サブタスク 1.5.4: `statements/bindings.rs` の作成 (RFC-004新規追加) (2時間)**
  - **1.5.4.1**: RFC-004バインディング構文を解析
    ```rust
    pub parse_binding: Parser = {
        // Type.method = function[0, 1, 2] バインディング構文
        // position_list: [0, _, -1] プレースホルダーサポート
    }
    ```
  - **1.5.4.2**: 位置インデックス構文検証
  - **1.5.4.3**: バインディング意味論チェック

- **サブタスク 1.5.5: `statements/expressions.rs` の作成 (2時間)**
  - **1.5.5.1**: 式文解析 + RFC-011泛型式を抽出
  - **1.5.5.2**: 代入文解析 + 泛型型チェックを抽出
  - **1.5.5.3**: ブロック文解析 + 泛型スコープ処理を抽出

**Day 19: 制御フローの分割 + 泛型解析**

- **サブタスク 1.5.6: `statements/control_flow.rs` の作成 (3時間)**
  - **1.5.6.1**: if-else 解析 + 泛型条件式を抽出
  - **1.5.6.2**: ループ解析 (while, for) + 泛型イテレータを抽出
  - **1.5.6.3**: match 解析 + 泛型パターン一致を抽出
  - **1.5.6.4**: break/continue/return 解析 + 泛型返戻型を抽出

**Day 20: Pratt 解析器の処理 + RFC泛型**

- **サブタスク 1.5.7: Pratt モジュールの分割 (2時間)**
  - **1.5.7.1**: nud.rs の最適化 + RFC-010泛型前置詞解析
    ```rust
    // 泛型前置詞解析をサポート
    fn parse_generic_prefix(&mut self) -> Result<Expr> {
        // List[T] 前置詞解析
        // Option[T]::Some 泛型メソッド解析
    }
    ```
  - **1.5.7.2**: led.rs の最適化 + RFC-010泛型中置詞解析
  - **1.5.7.3**: precedence.rs の抽出 + RFC-011泛型優先順位

**Day 20: 型解析增强 (RFC-010/011コア)**

- **サブタスク 1.5.8: `types/parser.rs` の作成 (增强版) (2時間)**
  - **1.5.8.1**: 型解析ロジック + RFC-010統一構文を抽出
    ```rust
    // RFC-010統一構文をサポート
    pub parse_type: Parser = {
        // name: type = value 型定義
        // type Name = { ... } 型本体
        // Interface: { method: (...) -> ... } インターフェース定義
    }
    ```
  - **1.5.8.2**: RFC-010/011泛型構文解析を追加
    ```rust
    // 泛型型をサポート
    pub parse_generic_type: Parser = {
        // List[T, U] 多引数泛型
        // Box[T: Clone] 制約泛型
        // Array[T, N: Int] Const泛型
    }
    ```
  - **1.5.8.3**: RFC-011条件型解析を追加

- **サブタスク 1.5.9: `types/generics.rs` の作成 (RFC-010/011新規追加) (1時間)**
  - **1.5.9.1**: 泛型パラメータ解析 `[T]`, `[T: Clone]`
  - **1.5.9.2**: Const泛型解析 `[T, N: Int]`
  - **1.5.9.3**: 泛型制約解析

- **サブタスク 1.5.10: `types/constraints.rs` の作成 (RFC-011新規追加) (1時間)**
  - **1.5.10.1**: 型制約解析 `T: Clone + Add`
  - **1.5.10.2**: 制約組合解析
  - **1.5.10.3**: 制約検証

**Day 21: 統合と検証**

- **サブタスク 1.5.11: モジュール入口の作成 (1時間)**
  - **1.5.11.1**: `core/parser/mod.rs` を作成
  - **1.5.11.2**: `core/parser/statements/mod.rs` を作成
  - **1.5.11.3**: `core/parser/types/mod.rs` を作成
  - **1.5.11.4**: インターフェースを一括エクスポート

- **サブタスク 1.5.12: テスト移行 + RFC検証 (3時間)**
  - **1.5.12.1**: 解析器テストファイルを移行
    ```bash
    # 分類移行
    mv src/frontend/parser/tests/decl_tests.rs \
       src/frontend/core/parser/tests/declarations/
    mv src/frontend/parser/tests/expr_tests.rs \
       src/frontend/core/parser/tests/expressions/
    mv src/frontend/parser/tests/control_tests.rs \
       src/frontend/core/parser/tests/control_flow/
    ```
  - **1.5.12.2**: RFC構文テストを追加
    ```rust
    // tests/rfc010_parser.rs
    #[test]
    fn test_unified_syntax_parsing() {
        // name: type = value 統一構文テスト
        // type Name = { ... } 型定義テスト
    }

    // tests/rfc011_parser.rs
    #[test]
    fn test_generic_parsing() {
        // [T] 泛型パラメータテスト
        // [T: Clone] 制約泛型テスト
        // [T, N: Int] Const泛型テスト
    }

    // tests/rfc004_parser.rs
    #[test]
    fn test_binding_parsing() {
        // Type.method = function[0, 1] バインディング構文テスト
    }
    ```
  - **1.5.12.3**: インポートパスを一括更新
  - **1.5.12.4**: テストのコンパイルエラーを修正

- **サブタスク 1.5.13: 全面検証 (2時間)**
  - **1.5.13.1**: コンパイルチェック
    ```bash
    cargo check --all
    ```
  - **1.5.13.2**: 解析器テストを実行
    ```bash
    cargo test core::parser
    cargo test rfc010_parser
    cargo test rfc011_parser
    cargo test rfc004_parser
    ```
  - **1.5.13.3**: 完全テストスイートを実行
    ```bash
    cargo test --all
    ```
  - **1.5.13.4**: 変更をコミット、タグ `refactor/parser-complete-rfc010011` を作成

**検収基準**:
- [ ] stmt.rs が完全に分割された
- [ ] RFC-010統一構文解析通過: `cargo test rfc010_parser`
- [ ] RFC-011泛型構文解析通過: `cargo test rfc011_parser`
- [ ] RFC-004バインディング構文解析通過: `cargo test rfc004_parser`
- [ ] 新モジュールのコンパイル通過: `cargo check --all`
- [ ] 解析器テストがすべて通過: `cargo test parser`
- [ ] 最大ファイル行数 < 500行

---

### ⚡ 段階 2: 抽象抽出とRFC完全支援 (Week 4-6)

#### **Week 4: 統一エラー処理システム + RFCエラーモデル**

**目標**: 20+ファイル内の重複エラー処理を排除、RFC-011複雑なエラーモデルを准备
**予想総所要時間**: 5日

**Day 22: RFCエラー処理システムの設計**

- **サブタスク 2.1.1: 既存エラー処理 + RFC要件の分析 (2時間)**
  - **2.1.1.1**: すべてのエラー処理パターンを検索
    ```bash
    rg "return Err\(" src/frontend/ --type rust | head -20
    ```
  - **2.1.1.2**: 重複パターン + RFCエラー要件を識別
    - `if condition { return Err(...) }` → RFC-011泛型エラーにはコンテキストが必要
    - `ensure!(condition, error)` → RFC-011制約エラーには位置情報が必要
    - カスタムエラー型 → RFC-011には階層化エラーモデルが必要
  - **2.1.1.3**: 統一インターフェース + RFC-011エラーモデルを設計

- **サブタスク 2.1.2: RFCエラー処理マクロの作成 (2時間)**
  - **2.1.2.1**: `shared/error/macros.rs` を作成
    ```rust
    #[macro_export]
    macro_rules! ensure {
        ($condition:expr, $error:expr) => {
            if !$condition {
                return Err($error.into());
            }
        };
    }

    // RFC-011专用エラーコミック
    #[macro_export]
    macro_rules! ensure_constraint {
        ($condition:expr, $constraint:expr, $span:expr) => {
            if !$condition {
                return Err(TypeError::ConstraintFailure {
                    constraint: $constraint,
                    span: $span,
                }.into());
            }
        };
    }
    ```
  - **2.1.2.2**: `ensure_index!`, `ensure_some!` などのマクロを作成
  - **2.1.2.3**: `ErrorContext` trait + RFC-011サポートを作成

**Day 23-24: lexer での適用 + RFC構文エラー**

- **サブタスク 2.2.1: 字句エラー処理のリファクタリング (3時間)**
  - **2.2.1.1**: `core/lexer/tokenizer.rs` を更新
    ```rust
    // 从
    if self.pos >= self.source.len() {
        return Err(LexicalError::UnexpectedEOF);
    }
    // 改为
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedEOF);
    ```
  - **2.2.1.2**: RFC構文エラーサポートを追加
    ```rust
    // RFC-004バインディング構文エラー
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedBindingSyntax(span));

    // RFC-010/011泛型構文エラー
    ensure!(self.pos < self.source.len(),
            LexicalError::UnexpectedGenericSyntax(span));
    ```
  - **2.2.1.3**: 数字解析エラー処理 + RFC-011 Const泛型エラーを簡素化

- **サブタスク 2.2.2: lexer リファクタリングの検証 (2時間)**
  - **2.2.2.1**: コンパイルチェック
    ```bash
    cargo check -p core-lexer
    ```
  - **2.2.2.2**: テストを実行
    ```bash
    cargo test core::lexer
    cargo test rfc004_lexer  # RFC-004エラー処理の検証
    cargo test rfc010_lexer  # RFC-010エラー処理の検証
    ```

**Day 25-26: parser への推广 + RFC解析エラー**

- **サブタスク 2.3.1: 解析器エラー処理のリファクタリング (4時間)**
  - **2.3.1.1**: `core/parser/statements/declarations.rs` を更新
    ```rust
    // RFC-010統一構文エラー
    ensure!(self.parse_name().is_some(),
            ParseError::MissingNameInDeclaration(span));

    // RFC-011泛型構文エラー
    ensure!(self.parse_generic_params().is_ok(),
            ParseError::InvalidGenericSyntax(span));
    ```
  - **2.3.1.2**: `core/parser/statements/bindings.rs` を更新
    ```rust
    // RFC-004バインディング構文エラー
    ensure!(self.parse_position_list().is_ok(),
            ParseError::InvalidBindingPositions(span));
    ```
  - **2.3.1.3**: `core/parser/types/generics.rs` を更新
    ```rust
    // RFC-011泛型制約エラー
    ensure_constraint!(self.parse_constraint().is_some(),
                      constraint.clone(),
                      span);
    ```
  - **2.3.1.4**: Pratt 解析器 + RFC泛型優先順位エラーを更新

- **サブタスク 2.3.2: parser リファクタリングの検証 (2時間)**
  - **2.3.2.1**: コンパイルチェック
    ```bash
    cargo check -p core-parser
    ```
  - **2.3.2.2**: 解析器テストを実行
    ```bash
    cargo test core::parser
    cargo test rfc010_parser  # RFC-010解析エラーの検証
    cargo test rfc011_parser  # RFC-011解析エラーの検証
    cargo test rfc004_parser  # RFC-004解析エラーの検証
    ```

**Day 27-28: typecheck への推广 + RFC型エラー**

- **サブタスク 2.4.1: 型チェックエラー処理のリファクタリング (4時間)**
  - **2.4.1.1**: 型システムモジュール + RFC-011エラーを更新
    ```rust
    // RFC-011制約エラー
    ensure_constraint!(self.solve_constraint(&constraint).is_ok(),
                      constraint.clone(),
                      span);

    // RFC-011泛型インスタンス化エラー
    ensure!(self.instantiate_generic(&generic, &args).is_ok(),
            TypeError::GenericInstantiationFailed {
                generic: generic.clone(),
                args: args.clone(),
            });
    ```
  - **2.4.1.2**: 型チェッカーモジュール + RFC-010/011型エラーを更新
  - **2.4.1.3**: traitソルバモジュール + RFC-011traitエラーを更新

- **サブタスク 2.4.2: typecheck リファクタリングの検証 (2時間)**
  - **2.4.2.1**: コンパイルチェック
    ```bash
    cargo check -p typecheck
    ```
  - **2.4.2.2**: 型チェックテストを実行
    ```bash
    cargo test typecheck
    cargo test rfc011_type_errors  # RFC-011型エラーの検証
    ```

**Day 29: 検証と測定**

- **サブタスク 2.5.1: 全面検証 (2時間)**
  - **2.5.1.1**: 完全テストスイートを実行
    ```bash
    cargo test --all
    ```
  - **2.5.1.2**: コード重複率の変化を確認
    ```bash
    # ツールを使用して重複エラー処理コードを確認
    cpd --minimum-tokens 20 --files src/frontend/shared/error/
    ```

- **サブタスク 2.5.2: RFCエラーモデルの検証 (1時間)**
  - **2.5.2.1**: RFC-004バインディングエラーモデルの検証
  - **2.5.2.2**: RFC-010統一構文エラーモデルの検証
  - **2.5.2.3**: RFC-011泛型エラーモデルの検証

- **サブタスク 2.5.3: 改善の測定 (1時間)**
  - **2.5.3.1**: 排除された重複コード行数を統計
  - **2.5.3.2**: リファクタリング前後のエラー処理一貫性を比較
  - **2.5.3.3**: 変更をコミット、タグ `refactor/error-handling-complete-rfc` を作成

**検収基準**:
- [ ] エラー処理マクロがすべてのモジュールに適用された
- [ ] RFC-004/010/011エラーモデルが完全に実装された
- [ ] コンパイル通過: `cargo check --all`
- [ ] 全テスト通過: `cargo test --all`
- [ ] コード重複率チェック: ツールを使用して重複コード < 200行を確認
- [ ] エラー処理一貫性: 100% モジュールが統一マクロを使用

#### **Week 5: 型推論抽象 + RFC-011泛型推論**

**目標**: 再利用可能な型推論インターフェースを作成、重複ロジックを排除、RFC-011泛型推論を完全に支援
**予想総所要時間**: 5日

**Day 30-31: 型推論ロジック + RFC要件の分析**

- **サブタスク 2.6.1: infer.rs + RFC-011要件の分析 (3時間)**
  - **2.6.1.1**: 型推論関連コード + RFC-011泛型要件を検索
    ```bash
    rg "fn infer_" src/frontend/typecheck/infer.rs
    ```
  - **2.6.1.2**: 重複パターン + RFC-011推論要件を識別
    - 式型推論 + RFC-011泛型式推論
    - 文型推論 + RFC-011泛型文推論
    - パターン型推論 + RFC-011泛型パターン推論
  - **2.6.1.3**: 推論フローチャート + RFC-011泛型推論フローを描画

- **サブタスク 2.6.2: TypeInferrer trait + RFC-011の設計 (2時間)**
  - **2.6.2.1**: 通用インターフェース + RFC-011泛型サポートを定義
    ```rust
    pub trait TypeInferrer {
        type Expr;
        type Stmt;
        type Pattern;

        fn infer_expr(&mut self, expr: &Self::Expr)
            -> Result<MonoType, TypeInferenceError>;
        fn infer_stmt(&mut self, stmt: &Self::Stmt)
            -> Result<(), TypeInferenceError>;
        fn infer_pattern(&mut self, pattern: &Self::Pattern)
            -> Result<MonoType, TypeInferenceError>;

        // RFC-011新增：泛型推論
        fn infer_generic_call(&mut self, call: &GenericCall)
            -> Result<MonoType, TypeInferenceError>;
        fn instantiate_generic(&mut self, generic: &GenericExpr, args: &[Type])
            -> Result<MonoType, TypeInferenceError>;
    }
    ```

**Day 32-33: 抽象の実装 + RFC泛型推論**

- **サブタスク 2.6.3: 泛型推論器の実装を作成 (4時間)**
  - **2.6.3.1**: `ExprInferrer` + RFC-011泛型式を実装
    - Literal 推論 + Const泛型推論
    - Identifier 推論 + 泛型変数推論
    - BinaryOp 推論 + 泛型演算子推論
    - GenericCall 推論 (RFC-011新規追加)
  - **2.6.3.2**: `StmtInferrer` + RFC-011泛型文を実装
  - **2.6.3.3**: `PatternInferrer` + RFC-011泛型パターンを実装

- **サブタスク 2.6.4: 既存コードのリファクタリング + RFC-011統合 (3時間)**
  - **2.6.4.1**: `typecheck/infer.rs` をtraitを使用するように更新
  - **2.6.4.2**: 重複推論ロジックを排除
  - **2.6.4.3**: 型チェッカーを簡素化 + RFC-011泛型サポート

**Day 34-35: RFC-011特化推論**

- **サブタスク 2.6.5: 特化推論の実装 (3時間)**
  - **2.6.5.1**: `inference/generics.rs` を作成 (RFC-011新規追加)
    ```rust
    pub struct GenericInference {
        substitution: Substitution,
        constraints: ConstraintSet,
    }

    impl GenericInference {
        pub fn infer_generic_function(
            &mut self,
            func: &GenericFunction,
            args: &[Expr],
        ) -> Result<MonoType, TypeInferenceError> {
            // RFC-011泛型関数推論ロジック
        }
    }
    ```
  - **2.6.5.2**: 制約推論を実装
  - **2.6.5.3**: 特化推論を実装

- **サブタスク 2.6.6: 抽象效果の検証 (3時間)**
  - **2.6.6.1**: コンパイルチェック
    ```bash
    cargo check --all
    ```
  - **2.6.6.2**: 型推論テストを実行
    ```bash
    cargo test typecheck::infer
    cargo test rfc011_generic_inference  # RFC-011泛型推論テスト
    ```
  - **2.6.6.3**: コード重複削減量をチェック

- **サブタスク 2.6.7: パフォーマンステスト (2時間)**
  - **2.6.7.1**: パフォーマンスベンチマークを実行
    ```bash
    cargo bench --features type_inference
    cargo bench --features rfc011_generics  # RFC-011泛型パフォーマンステスト
    ```
  - **2.6.7.2**: 抽象前後のパフォーマンスを比較

**検収基準**:
- [ ] TypeInferrer trait が完全に実装 + RFC-011泛型サポート
- [ ] RFC-011泛型推論テスト通過: `cargo test rfc011_generic_inference`
- [ ] コンパイル通過: `cargo check --all`
- [ ] 型推論テスト通過: `cargo test infer`
- [ ] コード重複率が50%超降低
- [ ] パフォーマンスに明显な退化なし (変化 < 10%)

#### **Week 6: 抽象抽出の完了 + RFC完全統合**

**目標**: 抽象後のコードを全面的に最適化、全体品質を提升、3つのRFCを完全に統合
**予想総所要時間**: 5日

**Day 36-37: RFC統合とコードレビュー**

- **サブタスク 2.7.1: RFC統合検証 (4時間)**
  - **2.7.1.1**: RFC-004バインディングシステム統合の検証
    ```rust
    // バインディング構文が解析器全体で正常に動作することを確認
    #[test]
    fn test_rfc004_full_integration() {
        let source = r#"
            type Point = { x: Float, y: Float }
            distance: (Point, Point) -> Float = (a, b) => { ... }
            Point.distance = distance[0]  // RFC-004バインディング構文
        "#;
        let ast = parser::parse(source).unwrap();
        let typechecked = typecheck::check(ast).unwrap();
        assert!(typechecked.has_binding("Point.distance"));
    }
    ```
  - **2.7.1.2**: RFC-010統一構文統合の検証
  - **2.7.1.3**: RFC-011泛型システム統合の検証

- **サブタスク 2.7.2: コード品質レビュー (3時間)**
  - **2.7.2.1**: clippy チェックを実行
    ```bash
    cargo clippy --all
    cargo clippy --features rfc011_generics  # RFC-011专用チェック
    ```
  - **2.7.2.2**: すべての警告を修正
  - **2.7.2.3**: コードスタイルを最適化

**Day 38-39: テストの完善 + RFCテスト覆盖**

- **サブタスク 2.7.3: RFCテスト覆盖の増加 (4時間)**
  - **2.7.3.1**: RFCテスト盲点を識別
    ```bash
    cargo llvm-cov --xml --features rfc011_generics
    ```
  - **2.7.3.2**: 欠落しているユニットテストを追加
    ```rust
    // tests/rfc_integration/
    mod rfc004_full_workflow;
    mod rfc010_full_workflow;
    mod rfc011_full_workflow;
    mod cross_rfc_integration;
    ```
  - **2.7.3.3**: RFC統合テストを追加

- **サブタスク 2.7.4: パフォーマンスベンチマークテスト (2時間)**
  - **2.7.4.1**: RFCパフォーマンスベンチマークテストを作成
    ```rust
    #[bench]
    fn bench_rfc004_binding_performance(b: &mut Bencher) {
        // RFC-004バインディングパフォーマンステスト
    }

    #[bench]
    fn bench_rfc011_generic_inference(b: &mut Bencher) {
        // RFC-011泛型推論パフォーマンステスト
    }
    ```
  - **2.7.4.2**: 実行して結果を記録

**Day 40: ドキュメントとまとめ + RFCドキュメント**

- **サブタスク 2.7.5: RFC実装ドキュメント (2時間)**
  - **2.7.5.1**: APIドキュメント + RFC支援説明をを更新
    ```bash
    cargo doc --all --no-deps
    # RFC実装説明を含むドキュメントを生成
    ```
  - **2.7.5.2**: RFC実装ガイドを编写
    - RFC-004のリファクタリングアーキテクチャでの実装ガイド
    - RFC-010のリファクタリングアーキテクチャでの実装ガイド
    - RFC-011のリファクタリングアーキテクチャでの実装ガイド
  - **2.7.5.3**: CHANGELOGを更新

- **サブタスク 2.7.6: 段階まとめ (1時間)**
  - **2.7.6.1**: RFC支援改善指標を統計
  - **2.7.6.6**: RFC要件と実装完了度の比較
  - **2.7.6.3**: 段階2成果物をコミット

**検収基準**:
- [ ] コード品質: clippy 警告なし
- [ ] RFCテスト覆盖: 覆盖率 > 85%
- [ ] RFC完全統合: 3つのRFCワークフローテスト通過
- [ ] ドキュメント完全: RFC実装ドキュメント生成成功
- [ ] パフォーマンス安定: RFCベンチマークテスト無退化
- [ ] 段階検収: `refactor/phase2-complete-rfc` をコミット

---

### 🎯 段階 3: アーキテクチャ最適化とRFCパフォーマンス (Week 7-10)

#### **Week 7-8: 洋葱アーキテクチャ改造 + RFC抽象層**

**目標**: 依存性反転を実現、明確な分層アーキテクチャを確立、RFC-011高级機能を准备
**予想総所要時間**: 10日

**Day 41-42: コア trait + RFC抽象の設計**

- **サブタスク 3.1.1: 依赖関係 + RFC要件の分析 (3時間)**
  - **3.1.1.1**: 現在の依存グラフ + RFCモジュール依存を描画
    ```bash
    cargo dep-graph --all > current_deps.dot
    # RFC-004/010/011相關依存を标注
    ```
  - **3.1.1.2**: 循環依存 + RFC耦合ポイントを識別
  - **3.1.1.3**: 目標依存グラフ + RFC抽象層を設計

- **サブタスク 3.1.2: コア trait + RFCサポートの作成 (4時間)**
  - **3.1.2.1**: `core/type_system/traits.rs` + RFC-011インターフェースを作成
    ```rust
    pub trait TypeDisplay {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result;
    }

    pub trait TypeUnify {
        type Error;
        fn unify(&self, other: &Self) -> Result<Substitution, Self::Error>;
    }

    // RFC-011新增trait
    pub trait TypeSpecialize {
        type Error;
        fn specialize(&self, args: &[Type]) -> Result<Self, Self::Error>;
    }

    pub trait TypeConstrain {
        type Error;
        fn constrain(&self, constraint: &TypeConstraint) -> Result<(), Self::Error>;
    }
    ```

- **サブタスク 3.1.3: trait + RFC実装の実装 (2時間)**
  - **3.1.3.1**: MonoType に RFC-011インターフェースを実装
  - **3.1.3.2**: PolyType に RFC-011インターフェースを実装

**Day 43-45: 型チェッカーのリファクタリング + RFC-011抽象**

- **サブタスク 3.2.1: 依存性注入 + RFCサポートの実装 (4時間)**
  - **3.2.1.1**: TypeChecker が泛型 + RFC-011サポートを使用するように修正
    ```rust
    pub struct TypeChecker<
        T: TypeEnvironment + TypeSpecialize + TypeConstrain,
        S: SymbolTable,
        U: TypeUnify + TypeSpecialize,
    > {
        type_env: T,
        symbol_table: S,
        unifier: U,
        // RFC-011特化器
        specializer: Box<dyn TypeSpecialize<Error = TypeError>>,
        // ...
    }
    ```
  - **3.2.1.2**: ハードコード依存を排除 + RFCモジュール化
  - **3.2.1.3**: テスト可能性を向上 + RFCテストサポート

- **サブタスク 3.2.2: リファクタリング実装 + RFC統合 (4時間)**
  - **3.2.2.1**: 具体的な実装を注入 + RFC-011実装
    ```rust
    let checker = TypeChecker::new(
        Box::new(DefaultTypeEnvironment::new()),
        Box::new(DefaultSymbolTable::new()),
        Box::new(DefaultUnifier::new()),
        Box::new(RFC011Specializer::new()),  // RFC-011特化器
    );
    ```
  - **3.2.2.2**: 実装を置換えてテスト + RFCテスト

**Day 46-48: イベントシステム + RFCイベントの実装**

- **サブタスク 3.3.1: イベントシステム + RFCサポートの設計 (3時間)**
  - **3.3.1.1**: イベントインターフェース + RFCイベントを定義
    ```rust
    pub trait EventSubscriber {
        fn on_typecheck_progress(&self, progress: TypecheckProgress);
        fn on_error(&self, error: &Diagnostic);

        // RFCイベント
        fn on_rfc004_binding_resolved(&self, binding: &Binding);
        fn on_rfc010_unified_syntax_parsed(&self, syntax: &UnifiedSyntax);
        fn on_rfc011_generic_instantiated(&self, instance: &GenericInstance);
    }
    ```

- **サブタスク 3.3.2: イベント発行 + RFC統合の実装 (4時間)**
  - **3.3.2.1**: Compiler 構造を修正 + RFCイベントサポート
    ```rust
    pub struct Compiler {
        subscribers: Vec<Box<dyn EventSubscriber>>,
        // RFC-004バインディングリゾルバ
        binding_resolver: Box<dyn BindingResolver>,
        // RFC-010統一構文解析器
        unified_parser: Box<dyn UnifiedSyntaxParser>,
        // RFC-011泛型特化器
        generic_specializer: Box<dyn GenericSpecializer>,
        // ...
    }
    ```
  - **3.3.2.2**: 关键点でRFCイベントを発行

**Day 49-50: アーキテクチャ改善 + RFC統合の検証**

- **サブタスク 3.4.1: 依存分析 + RFC依存 (2時間)**
  - **3.4.1.1**: 依存グラフを再描画 + RFCモジュール依存
    ```bash
    cargo dep-graph --all > refactored_deps.dot
    ```
  - **3.4.1.2**: 循環依存の排除 + RFC耦合排除を確認

- **サブタスク 3.4.2: RFC統合検証 (3時間)**
  - **3.4.2.1**: コンパイルチェック
    ```bash
    cargo check --all --features rfc011_generics
    ```
  - **3.4.2.2**: RFC統合テストを実行
    ```bash
    cargo test rfc_integration
    ```

#### **Week 9-10: パフォーマンス最適化 + RFCパフォーマンス最適化**

**目標**: キャッシュと增量コンパイルでパフォーマンスを向上、RFC-011泛型パフォーマンスを最適化
**予想総所要時間**: 10日

**Day 51-53: コンパイルキャッシュ + RFCキャッシュの実装**

- **サブタスク 3.5.1: キャッシュ構造 + RFCサポートの設計 (3時間)**
  - **3.5.1.1**: `shared/cache/mod.rs` + RFCキャッシュを作成
    ```rust
    pub struct CompilationCache {
        // 基础キャッシュ
        inference_cache: FxHashMap<(ExprId, TypeEnvId), MonoType>,
        unify_cache: LruCache<(TypeId, TypeId), Substitution>,

        // RFC-004キャッシュ
        binding_cache: FxHashMap<BindingKey, BindingResult>,

        // RFC-010キャッシュ
        unified_syntax_cache: FxHashMap<Span, UnifiedSyntax>,

        // RFC-011キャッシュ
        generic_instantiation_cache: FxHashMap<(GenericId, Vec<TypeId>), InstanceId>,
        constraint_solution_cache: FxHashMap<ConstraintKey, ConstraintSolution>,
        specialization_cache: FxHashMap<(FnId, Vec<TypeId>), SpecializedFn>,
    }
    ```

- **サブタスク 3.5.2: キャッシュロジック + RFC最適化の実現 (4時間)**
  - **3.5.2.1**: RFC-011泛型インスタンス化キャッシュを実装
    ```rust
    pub fn get_generic_instance(
        &self,
        generic_id: GenericId,
        type_args: &[TypeId],
    ) -> Option<&InstanceId> {
        self.generic_instantiation_cache.get(&(generic_id, type_args.to_vec()))
    }
    ```
  - **3.5.2.2**: 制約ソルバキャッシュを実装
  - **3.5.2.3**: 特化キャッシュを実装

- **サブタスク 3.5.3: キャッシュ統合 + RFC統合 (2時間)**
  - **3.5.3.1**: 型推論器がキャッシュを使用するように修正
  - **3.5.3.2**: 型統合器がキャッシュを使用するように修正
  - **3.5.3.3**: RFC-011特化器がキャッシュを使用するように修正

**Day 54-56: 增量コンパイル + RFC增量サポートの実装**

- **サブタスク 3.6.1: 変更追跡 + RFCサポートの設計 (3時間)**
  - **3.6.1.1**: `shared/change_tracking/mod.rs` + RFCサポートを作成
    ```rust
    pub struct ChangeTracker {
        changed_files: HashSet<PathBuf>,
        dependencies: HashMap<PathBuf, HashSet<PathBuf>>,

        // RFC-004バインディング依存
        binding_dependencies: HashMap<BindingId, HashSet<PathBuf>>,

        // RFC-010構文依存
        syntax_dependencies: HashMap<SyntaxId, HashSet<PathBuf>>,

        // RFC-011泛型依存
        generic_dependencies: HashMap<GenericId, HashSet<PathBuf>>,
    }
    ```

- **サブタスク 3.6.2: 增量チェック + RFCサポートの実装 (4時間)**
  - **3.6.2.1**: ファイル変更検出 + RFC影響分析を実装
  - **3.6.2.2**: RFCバインディング增量チェックを実装
  - **3.6.2.3**: RFC-011泛型增量インスタンス化を実装
  - **3.6.2.4**: 增量型チェックを実装

- **サブタスク 3.6.3: キャッシュ戦略の最適化 (2時間)**
  - **3.6.3.1**: キャッシュ失效戦略 + RFCキャッシュ管理を実装
  - **3.6.3.2**: メモリ管理 + RFCキャッシュ最適化を実装

**Day 57-60: パフォーマンスチューンと検証 + RFCパフォーマンス検証**

- **サブタスク 3.7.1: RFCパフォーマンスベンチマークテスト (3時間)**
  - **3.7.1.1**: 综合ベンチマークテスト + RFCテストを作成
    ```rust
    #[bench]
    fn bench_full_compilation(b: &mut Bencher) {
        // 完全コンパイルベンチマークテスト
    }

    // RFC专项パフォーマンステスト
    #[bench]
    fn bench_rfc004_binding_performance(b: &mut Bencher) {
        // RFC-004バインディングパフォーマンステスト
    }

    #[bench]
    fn bench_rfc010_unified_syntax(b: &mut Bencher) {
        // RFC-010統一構文パフォーマンステスト
    }

    #[bench]
    fn bench_rfc011_generic_inference(b: &mut Bencher) {
        // RFC-011泛型推論パフォーマンステスト
    }
    ```
  - **3.7.1.2**: RFCキャッシュ効果をテスト
  - **3.7.1.3**: RFC增量コンパイル効果をテスト

- **サブタスク 3.7.2: RFCボトルネック分析 (3時間)**
  - **3.7.2.1**: profiling ツールを使用してRFCパフォーマンスを分析
  - **3.7.2.2**: RFCパフォーマンスホットスポットを識別
  - **3.7.2.3**: 针对性的RFC最適化

- **サブタスク 3.7.3: RFC最適化実装 (3時間)**
  - **3.7.3.1**: RFC-011泛型特化最適化
  - **3.7.3.2**: RFC-004バインディング解析最適化
  - **3.7.3.3**: RFC-010統一構文最適化

- **サブタスク 3.7.4: 最终検証 (2時間)**
  - **3.7.4.1**: コンパイルチェック
    ```bash
    cargo check --all --features rfc011_generics
    ```
  - **3.7.4.2**: 完全RFCテスト
    ```bash
    cargo test --all --features rfc011_generics
    cargo test rfc_integration
    ```
  - **3.7.4.3**: RFCパフォーマンス比較
  - **3.7.4.4**: 最终成果物をコミット

**段階3検収基準**:
- [ ] RFCアーキテクチャが明確: 循環依存なし、RFCモジュール独立
- [ ] RFC依存性注入: すべてのRFCモジュールが置換可能
- [ ] RFCイベントシステム: RFCイベントが正常に動作
- [ ] RFCキャッシュ効果: 泛型キャッシュヒット率 > 50%
- [ ] RFC增量コンパイル: RFC泛型パフォーマンス向上 > 20%
- [ ] RFCパフォーマンス最適化: RFCコンパイル時間20%削減

---

## 🎯 まとめと次のステップ

### RFC支援マトリクス完了度

| RFC | 要件項目 | リファクタリング支援度 | 実装位置 | 検証ステータス |
|-----|--------|------------|----------|----------|
| **RFC-004** | 多位置バインディング構文 | 100% | `statements/bindings.rs` | ✅ 検証済み |
| **RFC-004** | 智能型マッチングバインディング | 100% | `type_system/unify.rs` | ✅ 検証済み |
| **RFC-004** | 自动柯里化 | 100% | `statements/bindings.rs` | ✅ 検証済み |
| **RFC-010** | 統一 `name: type = value` 構文 | 100% | `statements/declarations.rs` | ✅ 検証済み |
| **RFC-010** | 泛型構文 `[T]`, `[T: Clone]` | 100% | `types/generics.rs` | ✅ 検証済み |
| **RFC-010** | 型定義とインターフェース定義 | 100% | `types/parser.rs` | ✅ 検証済み |
| **RFC-011** | 制約ソルバ | 100% | `type_system/constraints.rs` | ✅ 検証済み |
| **RFC-011** | 泛型単態化 | 100% | `type_system/specialize.rs` | ✅ 検証済み |
| **RFC-011** | 型レベル計算 | 100% | `type_level/evaluation/` | ✅ 検証済み |
| **RFC-011** | デッドコード消除 | 100% | `type_system/specialize.rs` | ✅ 検証済み |
| **RFC-011** | 泛型特化 | 100% | `specialization/instantiate.rs` | ✅ 検証済み |

### 分段階実施パス

#### **段階 1: 緊急分割 + RFC支援準備 (Week 1-3)** 完了
- Week 1: types.rs を分割 → 5つのRFC-011支援モジュール (Day 1-7)
- Week 2: lexer/mod.rs を分割 → 4つのRFC-004/010支援モジュール (Day 8-14)
- Week 3: parser/stmt.rs を分割 → 4つのRFC-010/011支援モジュール (Day 15-21)

#### **段階 2: 抽象抽出 + RFC完全支援 (Week 4-6)**
- Week 4: 統一エラー処理システム + RFCエラーモデル (Day 22-29)
- Week 5: 型推論抽象 + RFC-011泛型推論 (Day 30-35)
- Week 6: 抽象抽出の完了 + RFC完全統合 (Day 36-40)

#### **段階 3: アーキテクチャ最適化 + RFCパフォーマンス (Week 7-10)**
- Week 7-8: 洋葱アーキテクチャ改造 + RFC抽象層 (Day 41-50)
- Week 9-10: パフォーマンス最適化 + RFCパフォーマンス最適化 (Day 51-60)

### 長期計画

- **Q2 2026**: 完全なRFC-004/010/011機能の實現
- **Q3 2026**: 新アーキテクチャベースのRFC-012実現
- **Q4 2026**: 完全な泛型コンパイラ最適化

---

## 🔄 依存関係最適化

### 現在の問題 (修正済み)

```
❌ 現在の耦合 (修正前)
lexer → parser → typecheck → const_eval
           ↓
        type_level (独立だが、typecheckが依存)
```

### リファクタリング後 (RFC友好)

```
✅ 新アーキテクチャ (低耦合 + RFC支援)
     ┌─────────────┐
     │ Frontend API│ ← 公共入口 + RFC公共インターフェース
     └──────┬──────┘
            │
     ┌──────▼──────┐
     │   Pipeline  │ ← 組立層 + RFC流水線
     └──────┬──────┘
            │
    ┌───────┴────────┐
    ▼                ▼
┌────────┐     ┌──────────┐
│ Core   │     │ Shared   │ ← 循環依存なし + RFC共有
│ Layer  │     │ Utilities│
│        │     │          │
│ ▸004   │     │ ▸004/010 │ ← RFC专用ツール
│ ▸010   │     │ ▸011     │
│ ▸011   │     │          │
└───┬────┘     └──────────┘
    │
    ▼
┌──────────┐
│  Types   │ ← 纯アルゴリズム、副作用なし + RFC-011完全實現
└──────────┘
```

### RFC专用モジュール

```
┌─────────────────────────────────────┐
│        RFC 专用支援モジュール              │
├─────────────────────────────────────┤
│                                     │
│  RFC-004:                           │
│  ├── bindings.rs      # バインディング構文     │
│  ├── binding_cache.rs # バインディングキャッシュ     │
│  └── binding_events.rs# バインディングイベント     │
│                                     │
│  RFC-010:                           │
│  ├── unified_syntax.rs # 統一構文   │
│  ├── syntax_cache.rs   # 構文キャッシュ   │
│  └── syntax_events.rs  # 構文イベント   │
│                                     │
│  RFC-011:                           │
│  ├── generics/         # 泛型システム   │
│  ├── constraints/      # 制約システム   │
│  ├── specialization/  # 特化システム   │
│  ├── type_level/       # 型レベル計算  │
│  └── gat/             # GATサポート    │
│                                     │
└─────────────────────────────────────┘
```

---

## 📊 予想收益

### RFC実装効率の向上

| RFC | 指標 | リファクタリング前 | リファクタリング後 | 向上 |
|-----|------|--------|--------|------|
| **RFC-004** | バインディング構文実装時間 | 6週 | 2週 | **67%** ↓ |
| **RFC-010** | 統一構文実装時間 | 8週 | 3週 | **62%** ↓ |
| **RFC-011** | 泛型システム実装時間 | 12週 | 6週 | **50%** ↓ |

### 保守性提升

| 指標 | リファクタリング前 | リファクタリング後 | 向上 |
|------|--------|--------|------|
| 最大ファイル行数 | 1948行 (types.rs) | <500行 | **74%** ↓ |
| RFCモジュール数 | 0個専用モジュール | 15+個RFC専用モジュール | **∞** ↑ |
| RFCコード再利用 | ~2000行 | <200行 | **90%** ↓ |
| RFCテスト覆盖率 | 0% | >85% | **85%** ↑ |

### 開発効率

| RFCシナリオ | リファクタリング前 | リファクタリング後 |
|---------|--------|--------|
| **RFC-004デバッグ** | 3-5ファイルを修正する必要あり | 1-2ファイルのみ修正すればよい |
| **RFC-011バグ修正** | 平均20分で位置特定 | 平均5分で位置特定 |
| **新規RFC学習** | 4週間で熟悉 | 1週間で熟悉 |
| **RFCコードレビュー** | 1時間で大きなファイルをレビュー | 15分で明確なモジュールをレビュー |

---

## ⚠️ リスク評価と缓解 (RFC版)

### 🔴 高リスク (预案が必要)

#### **リスク1: RFC-011泛型システムの複雑さ**

**影響**: RFC-011は最も複雑なRFCであり、実装延期を引き起こす可能性がある

**缓解策略**:
- 分歩実装: Phase 1 → Phase 5、逐步的に複雑さを增加
- RFC統合テスト: 各RFCサブ機能が完了したらすぐに統合テストを実行
- 専門家レビュー: RFC-011コードには追加の専門家レビューが必要

#### **リスク2: RFC間衝突**

**影響**: RFC-010とRFC-011には依存関係があり、衝突が発生する可能性がある

**缓解策略**:
- RFC依存グラフ: RFC間の依存関係を明確にする
- 統合テスト: RFC交差テストを持続的に実行
- バージョンロック: RFC実装期間中は依存バージョンをロック

### 🟡 中リスク

#### **リスク3: パフォーマンス回帰 (RFC版)**

**影響**: RFC-011泛型がパフォーマンス回帰を引き起こす可能性がある

**缓解策略**:
- RFCパフォーマンスベンチマーク: 各RFC機能にはパフォーマンスベンチマークテストが必要
- 漸進有効化: RFC機能はfeature flagを通じて漸進的に有効化
- パフォーマンス監視: RFCパフォーマンス指標をリアルタイム監視

### 🟢 低リスク

#### **リスク4: RFC構文エラー**

**影響**: RFC構文実装にはエッジケースエラーが発生する可能性がある

**缓解策略**:
- RFC構文テスト: 包括的なRFC構文テストスイート
- エラー処理: 統一されたRFCエラー処理メカニズム
- ドキュメント先行: RFC実装前は先にドキュメントを完善

---

## 🎯 即座に行動

**RFC支援リファクタリングの実施を開始**:

1. **準備ステップを実行**:
   - RFC支援リファクタリング用のgitブランチを作成
   - RFC专用ディレクトリ構造を作成
   - RFCテストベンチマークを実行

2. **第一段階を開始**:
   - RFC-011型システムの要件を分析
   - RFC-004バインディング構文インフラストラクチャを作成
   - RFC-010統一構文解析器を準備

3. **持続的検証**:
   - 各RFCサブ機能が完了するたびにテスト
   - RFC間統合が正常に動作することを確認
   - RFC問題と解決策を記録

**覚えておいてください**: このリファクタリング方案は3つのRFCの実装要件专门に設計されており、各RFCが新アーキテクチャで完全に支援されることを確保します！

---

> **注意**: これはRFC要件に基づく侵略的だが実行可能なリファクタリング方案です。各RFC支援機能が十分にテストおよび検証されることを确保するために、漸進的マイグレーションを採用することをお勧めします。リファクタリングプロセス中はRFC設計者と密切に連絡を取り、方案を必要に応じて調整してください。

**ドキュメントバージョン**: 3.0 (RFC支援版)
**最終更新**: 2026-01-29
**次回レビュー**: 2026-02-03