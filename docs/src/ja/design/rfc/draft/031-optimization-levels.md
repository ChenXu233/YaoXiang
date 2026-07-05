```markdown
---
title: "RFC-031：最適化レベルと Pass マネージャー"
status: "ドラフト"
author: "晨煦"
created: "2026-06-16"
updated: "2026-07-05"
---

# RFC-031：最適化レベルと Pass マネージャー

> **参考**:
> - [RFC-011：generics システム設計](../accepted/011-generic-type-system.md)
> - [RFC-028：JIT コンパイラ](./028-jit-compiler.md)
> - [RFC-018：LLVM AOT コンパイラ](../accepted/018-llvm-aot-compiler.md)

## 概要

本文書では、YaoXiang に**最適化レベルシステム**と**Pass マネージャー**を導入し、コンパイル最適化を「全か無か」から設定可能な最適化パッケージへと変更することを提案する。最適化レベル（O0-O3）は異なる最適化戦略の組み合わせを定義し、Pass マネージャーは依存関係に従って最適化 Pass を実行する責任を持つ。本文書は同時に最適化 Pass の標準インターフェースを定義し、将来的な拡張（単相化、インライン、定数畳み込みなど）のためのアーキテクチャ基盤を提供する。

**核心的な目標：ユーザーにコンパイル速度、バイナリサイズ、runtime パフォーマンスの間で明確なトレードオフを選択させる。**

## 動機

### なぜ最適化レベルが必要なのか？

現在のコンパイラには最適化設定がなく、すべてのコードが同じ処理フローを経る。これにより以下の問題が生じる：

1. **デバッグ体験が悪い**：デバッグ時には最適化は不要だが、無効化できない
2. **バイナリサイズを制御できない**：generics 単相化はバイナリを膨張させるが、無効化できない
3. **コンパイル速度が制御不能**：シナリオに応じて「高速コンパイル」か「深い最適化」を選択できない
4. **最適化 Pass が順序付けられていない**：将来的に複数の最適化 Pass 間に依存関係があり、統一管理が必要

### 現在の問題

```yaoxiang
# 現在：すべてのコードが同じ処理を受ける
# - デバッグ時：最適化は不要だが、無効化できない
# - 本番時：最適化が必要だが、深さを設定できない
# - generics 関数：複数のコードが生成されるが、制御できない

identity: (T: Type) -> (x: T) -> T = (x) => x
x = identity(42)        # identity_Int が生成される
s = identity("hello")   # identity_String が生成される
# ユーザーは「単相化しない」（型消去モード）を選択できない
```

### 最適化レベルの価値

| シナリオ | 要件 | 最適化レベル |
|------|------|----------|
| 開発デバッグ | 高速コンパイル、デバッグ情報の保持 | O0 |
| 日々の開発 | 基本最適化、コンパイル速度と性能のバランス | O1 |
| テスト/CI | 標準最適化、本番動作の検証 | O2 |
| 本番リリース | 深い最適化、究極の性能 | O3 |
| スクリプト/迅速なプロトタイピング | 自動選択（ターゲットプラットフォームに応じて） | Auto |

## 提案

### 中核設計

#### 1. 最適化レベルの定義

```rust
/// 最適化レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptLevel {
    /// O0：最適化なし（デバッグモード）
    /// - すべてのデバッグ情報を保持
    /// - 最適化変換を行わない
    /// - 最も高速なコンパイル
    /// - 用途：開発デバッグ、高速イテレーション
    O0,

    /// O1：基本最適化（デフォルト）
    /// - オンデマンド単相化（未使用の特化バージョンは生成しない）
    /// - 基本的な定数畳み込み
    /// - 基本的なデッドコード除去
    /// - 用途：日々の開発
    #[default]
    O1,

    /// O2：標準最適化
    /// - オンデマンド単相化
    /// - 完全な定数畳み込み
    /// - 完全なデッドコード除去
    /// - 小さな関数のインライン化
    /// - 末尾呼び出し最適化
    /// - 用途：テスト、CI、本番リリース
    O2,

    /// O3：積極的な最適化
    /// - 完全な単相化（すべての可能な型組み合わせを事前に生成）
    /// - 積極的なインライン化
    /// - すべての最適化 Pass
    /// - コンパイル時間とバイナリサイズが増加する可能性がある
    /// - 用途：究極の性能要求
    O3,

    /// Auto：自動選択
    /// - ターゲットプラットフォームと利用可能なリソースに応じて最適化戦略を自動選択
    /// - 用途：スクリプト、迅速なプロトタイピング
    Auto,
}
```

#### 2. 最適化 Pass インターフェース

```rust
/// 最適化 Pass インターフェース
pub trait OptimizationPass {
    /// Pass 名（ログと依存関係の宣言用）
    fn name(&self) -> &str;

    /// Pass を実行
    fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> PassResult;

    /// この Pass が依存する他の Pass（先に実行される必要がある）
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }

    /// この Pass が現在の設定下で実行されるべきかどうか
    fn should_run(&self, config: &PassConfig) -> bool {
        true
    }
}

/// Pass 設定
#[derive(Debug, Clone)]
pub struct PassConfig {
    /// 最適化レベル
    pub opt_level: OptLevel,
    /// デバッグ情報を有効にするか
    pub debug_info: bool,
    /// ターゲットプラットフォーム
    pub target_platform: TargetPlatform,
}

/// Pass 実行結果
#[derive(Debug, Default)]
pub struct PassResult {
    /// IR を変更したかどうか
    pub changed: bool,
    /// 統計情報
    pub stats: PassStats,
}

/// Pass 統計情報
#[derive(Debug, Default)]
pub struct PassStats {
    /// インライン化された関数の数
    pub functions_inlined: usize,
    /// 単相化された関数の数
    pub functions_monomorphized: usize,
    /// 除去されたデッドコードの数
    pub dead_code_removed: usize,
    /// 畳み込まれた定数の数
    pub constants_folded: usize,
}
```

#### 3. Pass マネージャー

```rust
/// オプティマイザ
pub struct Optimizer {
    /// 登録された Pass のリスト（依存関係順にソート）
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    /// 最適化レベルに基づいてオプティマイザを作成
    pub fn for_opt_level(level: OptLevel) -> Self {
        let passes = Self::create_passes_for_level(level);
        Self { passes }
    }

    /// 指定されたレベルの Pass リストを作成
    fn create_passes_for_level(level: OptLevel) -> Vec<Box<dyn OptimizationPass>> {
        match level {
            OptLevel::O0 => {
                vec![
                    // デバッグモード：最小限の最適化、必要なクリーンアップのみ
                    Box::new(ConstFoldPass::minimal()),
                ]
            }
            OptLevel::O1 => {
                vec![
                    // 基本最適化
                    Box::new(ConstFoldPass::basic()),
                    Box::new(MonomorphizePass::on_demand()),
                    Box::new(DcePass::basic()),
                ]
            }
            OptLevel::O2 => {
                vec![
                    // 標準最適化
                    Box::new(ConstFoldPass::full()),
                    Box::new(MonomorphizePass::on_demand()),
                    Box::new(DcePass::full()),
                    Box::new(InlinePass::small_functions()),
                    Box::new(TcoPass::new()),
                ]
            }
            OptLevel::O3 => {
                vec![
                    // 積極的な最適化
                    Box::new(ConstFoldPass::full()),
                    Box::new(MonomorphizePass::full()),
                    Box::new(InlinePass::aggressive()),
                    Box::new(DcePass::full()),
                    Box::new(TcoPass::new()),
                    // より積極的な最適化...
                ]
            }
            OptLevel::Auto => {
                // 自動選択：ターゲットプラットフォームに基づいて決定
                Self::create_passes_for_level(OptLevel::O1)
            }
        }
    }

    /// すべての最適化 Pass を実行
    pub fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> OptimizerResult {
        let mut total_stats = OptimizerStats::default();

        for pass in &self.passes {
            if !pass.should_run(config) {
                continue;
            }

            let result = pass.run(module, config);
            total_stats.merge(result.stats);
        }

        OptimizerResult {
            module: module.clone(),
            stats: total_stats,
        }
    }
}
```

### 例

#### コマンドライン使用法

```bash
# デバッグモード：最適化なし
yaoxiang build --opt-level O0

# 日々の開発：基本最適化（デフォルト）
yaoxiang build

# 本番リリース：標準最適化
yaoxiang build --opt-level O2

# 究極の性能：積極的な最適化
yaoxiang build --opt-level O3

# 自動選択
yaoxiang build --opt-level Auto
```

#### 設定ファイル

```json
{
  "optimization_level": "O2",
  "mono": {
    "enabled": true,
    "strategy": "OnDemand"
  },
  "debug_info": false
}
```

#### API 使用法

```rust
use yaoxiang::frontend::{Compiler, CompileConfig, OptLevel};

// デバッグモード
let config = CompileConfig::new()
    .with_opt_level(OptLevel::O0);
let mut compiler = Compiler::with_config(config);

// 本番モード
let config = CompileConfig::new()
    .with_opt_level(OptLevel::O2);
let mut compiler = Compiler::with_config(config);
```

### 構文変更

構文変更なし。最適化レベルはコンパイラ設定であり、言語構文には影響しない。

## 詳細設計

### 最適化レベルと Pass のマッピング

| Pass | O0 | O1 | O2 | O3 | 説明 |
|------|----|----|----|----|------|
| **定数畳み込み** | 最小 | 基本 | 完全 | 完全 | compile-time での定数式の計算 |
| **単相化** | ❌ | オンデマンド | オンデマンド | 完全 | generics 関数の特化 |
| **デッドコード除去** | ❌ | 基本 | 完全 | 完全 | 未使用コードの削除 |
| **関数インライン化** | ❌ | ❌ | 小さな関数 | 積極的 | 関数本体を呼び出し点に挿入 |
| **末尾呼び出し最適化** | ❌ | ❌ | ✅ | ✅ | 末尾再帰をループに変換 |
| **エスケープ分析** | ❌ | ❌ | ❌ | ✅ | スタック/ヒープ割り当ての決定 |
| **ループ最適化** | ❌ | ❌ | ❌ | ✅ | ループ展開、不変条件の外出し |

### 単相化戦略

```rust
/// 単相化戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoStrategy {
    /// 単相化しない — 型消去、generics 関数はコードの一部の存在のみ
    /// 利点：バイナリが小さい、コンパイルが高速
    /// 欠点：runtime に動的ディスパッチのオーバーヘッド
    Erased,

    /// オンデマンド単相化 — 実際に使用される型の組み合わせに対してのみコードを生成
    /// 利点：ゼロコスト抽象、runtime オーバーヘッドなし
    /// 欠点：バイナリが膨張する可能性がある
    #[default]
    OnDemand,

    /// 完全な単相化 — すべての可能な型組み合わせを事前に生成
    /// 利点：compile-time にすべての呼び出しが確定
    /// 欠点：コンパイルが遅い、バイナリが大きい
    Full,
}

/// 単相化設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonoConfig {
    /// 単相化を有効にするか
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 単相化戦略
    #[serde(default)]
    pub strategy: MonoStrategy,

    /// DCE（デッドコード除去）を有効にするか
    #[serde(default = "default_true")]
    pub dce_enabled: bool,

    /// 最大特化深度（無限再帰 generics を防ぐ）
    #[serde(default = "default_max_mono_depth")]
    pub max_depth: usize,
}

impl Default for MonoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: MonoStrategy::OnDemand,
            dce_enabled: true,
            max_depth: 100,
        }
    }
}
```

### コンパイルフローへの統合

```rust
// src/frontend/pipeline.rs

impl Pipeline {
    fn run_ir_generation(
        &mut self,
        source_name: &str,
        source: &str,
        ast: &Module,
        type_result: &TypeCheckResult,
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> IRResult {
        let start = Instant::now();

        // 1. 基本 IR を生成
        let mut ir = middle::generate_ir(ast, type_result)?;

        // 2. 最適化レベルに基づいて最適化 Pass を実行
        let optimizer = Optimizer::for_opt_level(self.config.optimization_level);
        let pass_config = PassConfig {
            opt_level: self.config.optimization_level,
            debug_info: self.config.generate_debug_info,
            target_platform: TargetPlatform::detect(),
        };

        let result = optimizer.run(&mut ir, &pass_config);

        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::Optimization, duration));

        IRResult::success(result.module)
    }
}
```

### 型システムへの影響

直接的な影響なし。最適化 Pass は IR 層で動作し、type system には影響しない。

### runtime 動作

| 最適化レベル | runtime 動作 |
|----------|---------------|
| O0 | 最適化なし、すべてのデバッグ情報を保持 |
| O1 | 基本最適化、基本的なデバッグ情報を保持 |
| O2 | 標準最適化、デバッグ情報なし |
| O3 | 積極的な最適化、デバッグ情報なし |

**要点：runtime の修正は不要**。最適化 Pass は IR 層とコード生成層にのみ影響し、runtime は関数名/ID で検索して実行するため、最適化プロセスを認識しない。

### コンパイラの変更

| コンポーネント | 変更 |
|------|------|
| `frontend/config.rs` | `OptLevel` enum と `MonoConfig` を追加 |
| `frontend/pipeline.rs` | Pass マネージャーを統合 |
| `middle/passes/optimizer/` | 最適化 Pass モジュールを追加 |
| `middle/passes/mono/` | 標準 Pass インターフェースにリファクタリング |
| CLI | `--opt-level` パラメータを追加 |

### 後方互換性

- ✅ 完全な後方互換性
- デフォルトの最適化レベルは O1 で、動作は現状と一致
- ユーザーは最適化レベルを明示的に指定してデフォルト動作を上書きできる

## トレードオフ

### 利点

- **柔軟性**：ユーザーはシナリオに応じて最適化戦略を選択可能
- **拡張性**：標準 Pass インターフェースにより、新しい最適化を簡単に追加可能
- **予測可能性**：各最適化レベルの動作が明確
- **デバッグフレンドリー**：O0 モードで完全なデバッグ情報を保持

### 欠点

- **複雑度の増加**：複数の最適化レベルを維持する必要がある
- **テストマトリクスの拡大**：各最適化レベルの動作をテストする必要がある
- **ドキュメント負担**：各最適化レベルの意味を説明する必要がある

## 代替案

| 案 | 採用しない理由 |
|------|----------|
| オン/オフの2状態のみ | 最適化の深さを細かく制御できない |
| GCC/LLVM スタイルの `-O` 数字を使用 | YaoXiang の設定システムと整合しない |
| 各最適化 Pass を個別に切り替え | ユーザーが各 Pass の詳細を理解する必要があり、使用法が複雑 |
| v2.0 まで延期 | 単相化はすでに実装済みだが統合されておらず、まずアーキテクチャの問題を解決する必要がある |

## 実装戦略

### 段階分け

1. **フェーズ 1（現在）**：最適化レベルと Pass インターフェースの定義
2. **フェーズ 2**：単相化 Pass の実装（既存の `mono/` モジュールに基づく）
3. **フェーズ 3**：定数畳み込みとデッドコード除去 Pass の実装
4. **フェーズ 4**：関数インライン化と末尾呼び出し最適化 Pass の実装
5. **フェーズ 5**：積極的な最適化 Pass（エスケープ分析、ループ最適化）の実装

### 依存関係

- RFC-011（generics システム）の単相化モジュールに依存
- RFC-028（JIT コンパイラ）の最適化 Pass インターフェースに依存
- RFC-018（LLVM AOT）と最適化 Pass 設計を共有

### リスク

- **性能回帰**：最適化 Pass がバグを導入し、性能が低下する可能性
- **コンパイル時間の増加**：最適化 Pass がコンパイル時間を増加させる
- **バイナリ膨張**：単相化によりバイナリサイズが大幅に増加する可能性

## 未解決の問題

- [ ] O3 レベルでエスケープ分析をデフォルトで有効化すべきか？（@晨煦：性能テストデータが必要）
- [ ] `Os`（サイズ最適化）と `Oz`（極限サイズ最適化）レベルが必要か？
- [ ] 最適化レベルはデバッグ情報の詳細度に影響すべきか？
- [ ] 最適化 Pass 間の循環依存をどのように処理するか？

---

## 付録A：設計意思決定記録

| 意思決定 | 決定 | 日付 | 記録者 |
|------|------|------|--------|
| 最適化レベルの命名 | O0-O3 + Auto を使用 | 2026-06-16 | 晨煦 |
| デフォルト最適化レベル | O1（基本最適化） | 2026-06-16 | 晨煦 |
| 単相化戦略 | Erased/OnDemand/Full をサポート | 2026-06-16 | 晨煦 |
| Pass インターフェース設計 | trait + 依存関係宣言 | 2026-06-16 | 晨煦 |

---

## 付録B：用語集

| 用語 | 定義 |
|------|------|
| **最適化 Pass** | IR に対して1回の変換を行う独立したモジュール |
| **単相化** | generics 関数を具体的な型に特化するコード生成戦略 |
| **定数畳み込み** | compile-time における定数式の計算 |
| **デッドコード除去** | プログラム内の到達不能または未使用コードの削除 |
| **関数インライン化** | 関数本体を呼び出し点に挿入し、関数呼び出しのオーバーヘッドを回避 |
| **末尾呼び出し最適化** | 末尾再帰をループに変換し、スタックオーバーフローを回避 |
| **エスケープ分析** | 変数がスコープ外にエスケープするかどうかを分析し、スタック/ヒープ割り当てを決定 |

---

## 参考文献

- [Rust コンパイラ最適化](https://rustc-dev-guide.rust-lang.org/optimizations.html)
- [GCC 最適化レベル](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [LLVM Pass Manager](https://llvm.org/docs/WritingAnLLVMNewPMPass.html)
- [V8 TurboFan 最適化パイプライン](https://v8.dev/docs/turbofan)

---

## ライフサイクルと帰結

本 RFC は最適化レベルのアーキテクチャ設計を定義し、将来的な最適化 Pass のための統一フレームワークを提供する。

**単相化との関係**：単相化は最適化 Pass の1つであり、本 RFC が承認された後、最初に実装される Pass として位置付けられる。
```