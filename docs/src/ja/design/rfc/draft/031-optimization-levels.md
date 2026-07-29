---
title: 'RFC-031：最適化レベルとPassマネージャー'
status: '草案'
author: '晨煦'
created: '2026-06-16'
updated: '2026-07-05'
---

# RFC-031：最適化レベルとPassマネージャー

> **参考**:
>
> - [RFC-011：ジェネリクスシステム設計](../accepted/011-generic-type-system.md)
> - [RFC-028：JITコンパイラー](./028-jit-compiler.md)
> - [RFC-018：LLVM AOTコンパイラー](../accepted/018-llvm-aot-compiler.md)

## 概要

本ドキュメントはYaoXiangに**最適化レベルシステム**と**Passマネージャーを**導入することを提案する。コンパイル最適化を「全か無か」から設定可能な最適化パッケージに変更する。最適化レベル（O0-O3）は異なる最適化戦略の組み合わせを定義し、Passマネージャーは依存順序に従って最適化Passを実行する責任を負う。本ドキュメントは最適化Passの標準インターフェースも定義し、后续の拡張（单态化、インライン展開、定数畳み込みなど）のためのアーキテクチャ基盤を提供する。

**コア目標：ユーザーにコンパイル速度、バイナリサイズ、実行時パフォーマンスの間で明確なトレードオフを選択させること。**

## 動機

### なぜ最適化レベルが必要か？

現在のコンパイラーには最適化設定がなく、すべてのコードが同じ処理フローを通過する。これにより以下の問題が生じる：

1. **デバッグ体験の悪化**：デバッグ時には最適化が不要だが、無効化できない
2. **バイナリサイズの制御不可**：ジェネリクスの单态化はバイナリを肥大化させるが、無効化できない
3. **コンパイル速度の制御不可**：シナリオに応じて「高速コンパイル」または「深度最適化」を選択できない
4. **最適化Passの順序不在**：将来の複数の最適化Pass間に依存関係があり、統一的な管理が必要

### 現在の問題

```yaoxiang
# 現在：すべてのコードが同じ処理を受ける
# - デバッグ時：最適化は不要だが、閉じられない
# - 本番時：最適化は必要だが、深さを設定できない
# - ジェネリック関数：複数のコードが生成されるが、制御できない

identity: (T: Type) -> (x: T) -> T = (x) => x
x = identity(42)        # identity_Intが生成される
s = identity("hello")   # identity_Stringが生成される
# ユーザーは「单态化しない」（型消去モード）を選択できない
```

### 最適化レベルの価値

| シナリオ                    | 要件                                         | 最適化レベル |
| --------------------------- | -------------------------------------------- | ------------ |
| 開発デバッグ                | 高速コンパイル、デバッグ情報保持             | O0           |
| 日常開発                    | 基本最適化、コンパイル速度のバランス         | O1           |
| テスト/CI                   | 標準最適化、本番動作の検証                   | O2           |
| 本番リリース                | 深度最適化、极致なパフォーマンス             | O3           |
| スクリプト/高速プロトタイプ | 自動選択（ターゲットプラットフォームによる） | Auto         |

## 提案

### コア設計

#### 1. 最適化レベルの定義

```rust
/// 最適化レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptLevel {
    /// O0：最適化なし（デバッグモード）
    /// - すべてのデバッグ情報を保持
    /// - 最適化変換を一切行わない
    /// - 最速のコンパイル速度
    /// - 適用：開発デバッグ、高速イテレーション
    O0,

    /// O1：基本最適化（デフォルト）
    /// - 需要に応じた单态化（未使用の特殊化バージョンは生成しない）
    /// - 基本定数畳み込み
    /// - 基本デッドコード消除
    /// - 適用：日常開発
    #[default]
    O1,

    /// O2：標準最適化
    /// - 需要に応じた单态化
    /// - 完全定数畳み込み
    /// - 完全デッドコード消除
    /// - 小関数インライン展開
    /// - 末尾呼び出し最適化
    /// - 適用：テスト、CI、本番リリース
    O2,

    /// O3：積極的最適化
    /// - 完全单态化（すべての可能な型组合をプリ生成）
    /// - 積極的インライン展開
    /// - すべての最適化Pass
    /// - コンパイル時間とバイナリサイズが増加する可能性
    /// - 適用：极致なパフォーマンス要件
    O3,

    /// Auto：自動選択
    /// - ターゲットプラットフォームと利用可能なリソースに応じて最適化戦略を自動選択
    /// - 適用：スクリプト、高速プロトタイプ
    Auto,
}
```

#### 2. 最適化Passインターフェース

```rust
/// 最適化Passインターフェース
pub trait OptimizationPass {
    /// Pass名（ログと依存関係宣言用）
    fn name(&self) -> &str;

    /// Passの実行
    fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> PassResult;

    /// このPassが依存する他のPass（先に実行する必要がある）
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }

    /// このPassが現在の設定下で実行されるべきか
    fn should_run(&self, config: &PassConfig) -> bool {
        true
    }
}

/// Pass設定
#[derive(Debug, Clone)]
pub struct PassConfig {
    /// 最適化レベル
    pub opt_level: OptLevel,
    /// デバッグ情報を有効にするか
    pub debug_info: bool,
    /// ターゲットプラットフォーム
    pub target_platform: TargetPlatform,
}

/// Pass実行結果
#[derive(Debug, Default)]
pub struct PassResult {
    /// IRを変更したか
    pub changed: bool,
    /// 統計情報
    pub stats: PassStats,
}

/// Pass統計情報
#[derive(Debug, Default)]
pub struct PassStats {
    /// インライン展開された関数数
    pub functions_inlined: usize,
    /// 单态化された関数数
    pub functions_monomorphized: usize,
    /// 削除されたデッドコード数
    pub dead_code_removed: usize,
    /// 畳み込まれた定数数
    pub constants_folded: usize,
}
```

#### 3. Passマネージャー

```rust
/// 最適化エンジン
pub struct Optimizer {
    /// 登録されたPassリスト（依存順序でソート済み）
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    /// 最適化レベルに基づいて最適化エンジンを作成
    pub fn for_opt_level(level: OptLevel) -> Self {
        let passes = Self::create_passes_for_level(level);
        Self { passes }
    }

    /// 指定レベルのPassリストを作成
    fn create_passes_for_level(level: OptLevel) -> Vec<Box<dyn OptimizationPass>> {
        match level {
            OptLevel::O0 => {
                vec![
                    // デバッグモード：最小最適化、必要なクリーンアップのみ
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
                    // 積極的最適化
                    Box::new(ConstFoldPass::full()),
                    Box::new(MonomorphizePass::full()),
                    Box::new(InlinePass::aggressive()),
                    Box::new(DcePass::full()),
                    Box::new(TcoPass::new()),
                    // 追加の積極的最適化...
                ]
            }
            OptLevel::Auto => {
                // 自動選択：ターゲットプラットフォームに応じて決定
                Self::create_passes_for_level(OptLevel::O1)
            }
        }
    }

    /// すべての最適化Passを実行
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

#### コマンドライン使用

```bash
# デバッグモード：最適化なし
yaoxiang build --opt-level O0

# 日常開発：基本最適化（デフォルト）
yaoxiang build

# 本番リリース：標準最適化
yaoxiang build --opt-level O2

# 极致なパフォーマンス：積極的最適化
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

#### API使用

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

構文変更なし。最適化レベルはコンパイラー設定であり、言語構文には影響しない。

## 詳細な設計

### 最適化レベルとPassのマッピング

| Pass                   | O0   | O1     | O2     | O3     | 説明                          |
| ---------------------- | ---- | ------ | ------ | ------ | ----------------------------- |
| **定数畳み込み**       | 最小 | 基本   | 完全   | 完全   | コンパイル時の定数式の計算    |
| **单态化**             | ❌   | 需要時 | 需要時 | 完全   | ジェネリック関数の特殊化      |
| **デッドコード消除**   | ❌   | 基本   | 完全   | 完全   | 未使用のコードを削除          |
| **関数インライン展開** | ❌   | ❌     | 小関数 | 積極的 | 関数本体を呼び出し点に挿入    |
| **末尾呼び出し最適化** | ❌   | ❌     | ✅     | ✅     | 末尾再帰をループに変換        |
| **エスケープ分析**     | ❌   | ❌     | ❌     | ✅     | スタック/ヒープ割り当てを決定 |
| **ループ最適化**       | ❌   | ❌     | ❌     | ✅     | ループ展開、不変式持ち上げ    |

### 单态化戦略

```rust
/// 单态化戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoStrategy {
    /// 单态化なし — 型消去、ジェネリック関数は1つのコードのみ
    /// 利点：バイナリ小、コンパイル高速
    /// 欠点：実行時に動的ディスパッチオーバーヘッドあり
    Erased,

    /// 需要時单态化 — 実際に使用される型组合のみコードを生成
    /// 利点：ゼロコスト抽象、実行時オーバーヘッドなし
    /// 欠点：バイナ리가肥大化する可能性
    #[default]
    OnDemand,

    /// 完全单态化 — すべての可能な型组合をプリ生成
    /// 利点：コンパイル時にすべての呼び出しを解決
    /// 欠点：コンパイル遅い、バイナリ大きい
    Full,
}

/// 单态化設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonoConfig {
    /// 单态化を有効にするか
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 单态化戦略
    #[serde(default)]
    pub strategy: MonoStrategy,

    /// DCE（デッドコード消除）を有効にするか
    #[serde(default = "default_true")]
    pub dce_enabled: bool,

    /// 最大特殊化深度（無限再帰ジェネリクスを防止）
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

### コンパイルフロー統合

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

        // 1. 基本IRを生成
        let mut ir = middle::generate_ir(ast, type_result)?;

        // 2. 最適化レベルに応じて最適化Passを実行
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

### タイプシステムへの影響

直接的な影響なし。最適化PassはIR層で実行され、タイプシステムには影響しない。

### 実行時動作

| 最適化レベル | 実行時動作                             |
| ------------ | -------------------------------------- |
| O0           | 最適化なし、すべてのデバッグ情報を保持 |
| O1           | 基本最適化、基本的なデバッグ情報を保持 |
| O2           | 標準最適化、デバッグ情報なし           |
| O3           | 積極的最適化、デバッグ情報なし         |

**重要ポイント：実行時の変更は不要**。最適化PassはIR層とコード生成層にのみ影響し、実行時は関数名/IDで查找して実行するため、最適化プロセスを認識しない。

### コンパイラーの変更

| コンポーネント             | 変更                                         |
| -------------------------- | -------------------------------------------- |
| `frontend/config.rs`       | 新規 `OptLevel` enumと `MonoConfig`          |
| `frontend/pipeline.rs`     | Passマネージャーの統合                       |
| `middle/passes/optimizer/` | 新規最適化Passモジュール                     |
| `middle/passes/mono/`      | 標準Passインターフェースへのリファクタリング |
| CLI                        | 新規 `--opt-level` パラメータ                |

### 後方互換性

- ✅ 完全な後方互換性
- デフォルト最適化レベルはO1で、現在の動作と一致
- ユーザーはデフォルト動作をオーバーライドするために明示的に最適化レベルを指定可能

## トレードオフ

### 利点

- **柔軟性**：ユーザーはシナリオに応じて最適化戦略を選択可能
- **拡張性**：標準Passインターフェースにより、新しい最適化の追加が容易
- **予測可能性**：各最適化レベルの動作が明確
- **デバッグ 친善性**：O0モードは完全なデバッグ情報を保持

### 欠点

- **複雑性の増加**：複数の最適化レベルを維持する必要がある
- **テストマトリクスの拡大**：各最適化レベルの動作をテストする必要がある
- **ドキュメント負担**：各最適化レベルの意味を説明する必要がある

## 代替案

| 方案                             | 为什么不選択                                                                         |
| -------------------------------- | ------------------------------------------------------------------------------------ |
| オン/オフの2状態のみ             | 最適化深度を微細に制御できない                                                       |
| GCC/LLVMスタイルの`-O`数字を使用 | YaoXiangの設定システムと整合しない                                                   |
| 各最適化Passを個別にスイッチ     | ユーザーは各Passの詳細を理解する必要があり、使用が複雑                               |
| v2.0まで延期                     | 单态化は実装済みだが統合されていないため、アーキテクチャ問題を先に解決する必要がある |

## 実装戦略

### フェーズ分け

1. **フェーズ1（現在）**：最適化レベルとPassインターフェースの定義
2. **フェーズ2**：单态化Passの実装（既存の `mono/` モジュールベース）
3. **フェーズ3**：定数畳み込みとデッドコード消除Passの実装
4. **フェーズ4**：関数インライン展開と末尾呼び出し最適化Passの実装
5. **フェーズ5**：積極的最適化Pass（エスケープ分析、ループ最適化）の実装

### 依存関係

- RFC-011（ジェネリクスシステム）の单态化モジュールに依存
- RFC-028（JITコンパイラー）の最適化Passインターフェースに依存
- RFC-018（LLVM AOT）と最適化Pass設計を共有

### リスク

- **パフォーマンスリグレッション**：最適化Passがバグを導入し、パフォーマンスが低下する可能性
- **コンパイル時間増加**：最適化Passがコンパイル時間を増加させる
- **バイナリ肥大化**：单态化によりバイナリサイズが大幅に増加する可能性

## オープンクエスチョン

- [ ] O3レベルでエスケープ分析をデフォルト有効にするべきか？（@晨煦：パフォーマンステストデータが必要）
- [ ] `Os`（サイズ最適化）や`Oz`（极致サイズ最適化）レベルが必要か？
- [ ] 最適化レベルはデバッグ情報の詳細度に影響すべきか？
- [ ] 最適化Pass間の循環依存関係をどのように処理するか？

---

## 付録A：設計決定記録

| 決定                     | 決定                           | 日付       | 記録者 |
| ------------------------ | ------------------------------ | ---------- | ------ |
| 最適化レベル命名         | O0-O3 + Autoを使用             | 2026-06-16 | 晨煦   |
| デフォルト最適化レベル   | O1（基本最適化）               | 2026-06-16 | 晨煦   |
| 单态化戦略               | Erased/OnDemand/Fullをサポート | 2026-06-16 | 晨煦   |
| Passインターフェース設計 | trait + 依存関係宣言           | 2026-06-16 | 晨煦   |

---

## 付録B：用語集

| 用語                   | 定義                                                                    |
| ---------------------- | ----------------------------------------------------------------------- |
| **最適化Pass**         | IRを1回変換する独立モジュール                                           |
| **单态化**             | ジェネリック関数を具体型に特殊化するコード生成戦略                      |
| **定数畳み込み**       | コンパイル時に定数式を計算                                              |
| **デッドコード消除**   | プログラム内の到達不能または未使用のコードを削除                        |
| **関数インライン展開** | 関数本体を呼び出し点に挿入し、関数呼び出しオーバーヘッドを回避          |
| **末尾呼び出し最適化** | 末尾再帰をループに変換し、スタックオーバーフローを回避                  |
| **エスケープ分析**     | 変数がスコープをエスケープするかを分析し、スタック/ヒープ割り当てを決定 |

---

## 参考文献

- [Rust コンパイラーの最適化](https://rustc-dev-guide.rust-lang.org/optimizations.html)
- [GCC 最適化レベル](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [LLVM Pass Manager](https://llvm.org/docs/WritingAnLLVMNewPMPass.html)
- [V8 TurboFan 最適化パイプライン](https://v8.dev/docs/turbofan)

---

## ライフサイクルと归宿

本RFCは最適化レベルのアーキテクチャ設計を定義し、後続の最適化Passに統一的なフレームワークを提供する。

**单态化との関係**：单态化は最適化Passの一つであり、本RFCが承認された後に最初に実装されるPassとなる。
