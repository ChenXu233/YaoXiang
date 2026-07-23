---
title: "RFC-031：最適化レベルとPassマネージャー"
status: "草案"
author: "晨煦"
created: "2026-06-16"
updated: "2026-07-05"
---

# RFC-031：最適化レベルとPassマネージャー

> **参考**:
> - [RFC-011：泛型系统设计](../accepted/011-generic-type-system.md)
> - [RFC-028：JIT 编译器](./028-jit-compiler.md)
> - [RFC-018：LLVM AOT 编译器](../accepted/018-llvm-aot-compiler.md)

## 概要

この文書は、YaoXiangに**最適化レベルシステム**と**Passマネージャー**を導入し、コンパイル最適化を「全か無か」から設定可能な最適化パッケージにすることを提案します。最適化レベル（O0-O3）は異なる最適化戦略の組み合わせを定義し、Passマネージャーは依存関係の順序に従って最適化Passを実行します。この文書はまた、最適化Passの標準インターフェースを定義し、今後の拡張（単態化、インライン展開、定数畳み込みなど）にための建築基盤を提供します。

**中心的な目標：ユーザーにコンパイル速度、バイナリサイズ、実行時パフォーマンスの間で明確なトレードオフを選択させること。**

## 動機

### なぜ最適化レベルが必要인가？

現在のコンパイラには最適化設定がなく、すべてのコードが同じ処理フローを通過します。これにより：

1. **デバッグ体験が悪い**：デバッグ時は最適化が不要だが、無効化できない
2. **バイナリサイズの制御不可**：泛型単態化でバイナリが肥大化するが、無効化できない
3. **コンパイル速度が制御不可**：シーンに応じて「高速コンパイル」または「深い最適化」を選択できない
4. **最適化Passが順序なし**：将来の複数の最適化Pass間に依存関係があり、統一的な管理が必要

### 現在の問題

```yaoxiang
# 現在：すべてのコードが同じ処理を受ける
# - デバッグ時：最適化が不要だが、閉じられない
# - 本番時：最適化が必要だが、深さの構成不可能
# - 泛型関数：複数のコードを生成するが、制御不可

identity: (T: Type) -> (x: T) -> T = (x) => x
x = identity(42)        # identity_Intが生成される
s = identity("hello")   # identity_Stringが生成される
# ユーザーは「単態化なし」（型消去モード）を選択できない
```

### 最適化レベルの価値

| シナリオ | ニーズ | 最適化レベル |
|----------|--------|
| 開発デバッグ | 高速コンパイル、デバッグ情報を保持 | O0 |
| 日常開発 | 基本最適化、コンパイル速度とのバランス | O1 |
| テスト/CI | 標準最適化、本番動作の検証 | O2 |
| 本番リリース | 深い最適化、极致なパフォーマンス | O3 |
| スクリプト/クイックプロトタイプ | 自動選択（目標プラットフォームによる） | Auto |

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
    /// - 最も高速なコンパイル速度
    /// - 適用：開発デバッグ、高速反復
    O0,

    /// O1：基本最適化（デフォルト）
    /// - 必要时才単態化（未使用の専門化バージョンを生成しない）
    /// - 基本定数畳み込み
    /// - 基本死コード消除
    /// - 適用：日常開発
    #[default]
    O1,

    /// O2：標準最適化
    /// - 必要时才単態化
    /// - 完全定数畳み込み
    /// - 完全死コード消除
    /// - 小関数のインライン展開
    /// - 末尾呼び出し最適化
    /// - 適用：テスト、CI、本番リリース
    O2,

    /// O3：gressive最適化
    /// - 完全単態化（すべての可能な型組み合わせを予生成）
    /// - gressiveインライン展開
    /// - すべての最適化Pass
    /// - コンパイル時間とバイナリサイズが増加する可能性
    /// - 適用：极致なパフォーマンスニーズ
    O3,

    /// Auto：自動選択
    /// - 目標プラットフォームと利用可能なリソースに基づいて最適化戦略を自動選択
    /// - 適用：スクリプト、快速プロトタイプ
    Auto,
}
```

#### 2. 最適化Passインターフェース

```rust
/// 最適化Passインターフェース
pub trait OptimizationPass {
    /// Pass名（ログと依存関係宣言に使用）
    fn name(&self) -> &str;

    /// Passを実行
    fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> PassResult;

    /// このPassが依存する他のPass（先に実行する必要がある）
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }

    /// このPassが現在の設定で実行されるべきか
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
    /// 目標プラットフォーム
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
    /// 単態化された関数数
    pub functions_monomorphized: usize,
    /// 移除された死コード数
    pub dead_code_removed: usize,
    /// 畳み込まれた定数数
    pub constants_folded: usize,
}
```

#### 3. Passマネージャー

```rust
/// オプティマイザ
pub struct Optimizer {
    /// 登録されたPassリスト（依存関係順に並べ替え）
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    /// 最適化レベルに基づいてオプティマイザを作成
    pub fn for_opt_level(level: OptLevel) -> Self {
        let passes = Self::create_passes_for_level(level);
        Self { passes }
    }

    /// 指定レベルのPassリストを作成
    fn create_passes_for_level(level: OptLevel) -> Vec<Box<dyn OptimizationPass>> {
        match level {
            OptLevel::O0 => {
                vec![
                    // デバッグモード：最小限の最適化、必需な清理のみ
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
                    // gressive最適化
                    Box::new(ConstFoldPass::full()),
                    Box::new(MonomorphizePass::full()),
                    Box::new(InlinePass::aggressive()),
                    Box::new(DcePass::full()),
                    Box::new(TcoPass::new()),
                    // より多くのgressive最適化...
                ]
            }
            OptLevel::Auto => {
                // 自動選択：目標プラットフォームに基づいて決定
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

#### コマンドラインの使用

```bash
# デバッグモード：最適化なし
yaoxiang build --opt-level O0

# 日常開発：基本最適化（デフォルト）
yaoxiang build

# 本番リリース：標準最適化
yaoxiang build --opt-level O2

# 极致なパフォーマンス：gressive最適化
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

#### APIの使用

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

### 構文の変更

構文の変更はありません。最適化レベルはコンパイラ設定であり、言語構文には影響しません。

## 詳細な設計

### 最適化レベルとPassのマッピング

| Pass | O0 | O1 | O2 | O3 | 説明 |
|------|----|----|----|----|----|
| **定数畳み込み** | 最小限 | 基本 | 完全 | 完全 | コンパイル時に定数式を計算 |
| **単態化** | ❌ | 必要时才 | 必要时才 | 完全 | 泛型関数の特化 |
| **死コード消除** | ❌ | 基本 | 完全 | 完全 | 未使用のコードを移除 |
| **関数インライン展開** | ❌ | ❌ | 小関数 | gressive | 関数体を呼び出し点に挿入 |
| **末尾呼び出し最適化** | ❌ | ❌ | ✅ | ✅ | 末尾再帰をループに変換 |
| **逃逸分析** | ❌ | ❌ | ❌ | ✅ | 栈/ヒープ割り当てを決定 |
| **ループ最適化** | ❌ | ❌ | ❌ | ✅ | ループ展開、不変量を外挿 |

### 単態化戦略

```rust
/// 単態化戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoStrategy {
    /// 単態化なし — 型消去、泛型関数は1つのコードのみ
    /// 优点：バイナリ小、コンパイル速い
    /// 缺点：実行時に動的ディスパッチ开销あり
    Erased,

    /// 必要时才単態化 — 実際の使用された型組み合わせのみコードを生成
    /// 优点：ゼロオーバーヘッド抽象化、実行時开销なし
    /// 缺点：バイナリが肥大化する可能性
    #[default]
    OnDemand,

    /// 完全単態化 — すべての可能な型組み合わせを予生成
    /// 优点：コンパイル時にすべての呼び出しを解決
    /// 缺点：コンパイル遅い、バイナリ大きい
    Full,
}

/// 単態化設定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonoConfig {
    /// 単態化を有効にするか
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 単態化戦略
    #[serde(default)]
    pub strategy: MonoStrategy,

    /// DCE（死コード消除）を有効にするか
    #[serde(default = "default_true")]
    pub dce_enabled: bool,

    /// 最大特化深度（無限再帰泛型を防止）
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

        // 1. 基本的なIRを生成
        let mut ir = middle::generate_ir(ast, type_result)?;

        // 2. 最適化レベルに基づいて最適化Passを実行
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

直接的な影響はありません。最適化PassはIR層で実行され、型システムには影響しません。

### 実行時動作

| 最適化レベル | 実行時動作 |
|--------------|
| O0 | 最適化なし、すべてのデバッグ情報を保持 |
| O1 | 基本最適化、基本的なデバッグ情報を保持 |
| O2 | 標準最適化、デバッグ情報なし |
| O3 | gressive最適化、デバッグ情報なし |

**要点：実行時の変更は不要です**。最適化PassはIR層とコード生成層のみに影響し、実行時は関数名/IDで查找して実行し、最適化プロセスを認識しません。

### コンパイラの更改

| コンポーネント | 更改内容 |
|----------------|----------|
| `frontend/config.rs` | 新規 `OptLevel` 列挙型と `MonoConfig` |
| `frontend/pipeline.rs` | Passマネージャーを統合 |
| `middle/passes/optimizer/` | 新規最適化Passモジュール |
| `middle/passes/mono/` | 標準Passインターフェースにリファクタリング |
| CLI | 新規 `--opt-level` パラメータ |

### 後方互換性

- ✅ 完全な後方互換性
- デフォルト最適化レベルはO1で、現在の動作と一致
- ユーザーは明示的に最適化レベルを指定してデフォルト動作をオーバーライド可能

## トレードオフ

### メリット

- **柔軟性**：ユーザーはシーンに応じて最適化戦略を選択可能
- **拡張性**：標準的なPassインターフェースにより、新しい最適化の追加が容易
- **予測可能性**：各最適化レベルの動作が明確
- **デバッグフレンドリー**：O0モードは完全なデバッグ情報を保持

### デメリット

- **複雑性の増加**：複数の最適化レベルを維持する必要がある
- **テストマトリックスの增大**：各最適化レベルの動作をテストする必要がある
- **ドキュメント负担**：各最適化レベルの意味を説明する必要がある

## 代替案

| 方案 | 選擇しない理由 |
|------|----------------|
| オン/オフの2つの状態のみ | 最適化の深さを細かく制御できない |
| GCC/LLVMスタイルの `-O` 数字を使用 | YaoXiangの設定システムと一致しない |
| 各最適化Passを獨立して切り替え可能 | ユーザーは各Passの詳細を理解必要があり、使用が複雑 |
| v2.0まで遅延 | 単態化は実装済みだが統合されておらず、アーキテクチャの問題を先に解決する必要がある |

## 実装戦略

### フェーズ分け

1. **フェーズ1（現在）**：最適化レベルとPassインターフェースを定義
2. **フェーズ2**：単態化Passを実装（既存の `mono/` モジュールに基づく）
3. **フェーズ3**：定数畳み込みと死コード消除Passを実装
4. **フェーズ4**：関数インライン展開と末尾呼び出し最適化Passを実装
5. **フェーズ5**：gressive最適化Passを実装（逃逸分析、ループ最適化）

### 依存関係

- RFC-011（泛型システム）の単態化モジュールに依存
- RFC-028（JITコンパイラ）の最適化Passインターフェースに依存
- RFC-018（LLVM AOT） と最適化Pass設計を共有

### リスク

- **パフォーマンス回帰**：最適化Passがバグを導入し、パフォーマンスが低下する可能性
- **コンパイル時間の増加**：最適化Passがコンパイル時間を増加させる
- **バイナリ肥大化**：単態化によりバイナリサイズが著しく増加する可能性

## 開放問題

- [ ] O3レベルで逃逸分析をデフォルトで有効にするべきか？（@晨煦：パフォーマンステストデータが必要）
- [ ] `Os`（サイズ最適化）と`Oz`（极限サイズ最適化）レベルが必要か？
- [ ] 最適化レベルはデバッグ情報の詳細度に影響すべきか？
- [ ] 最適化Pass間の循環依存関係をどのように処理するか？

---

## 付録A：設計決定記録

| 決定 | 決定内容 | 日付 | 記録者 |
|------|----------|------|--------|
| 最適化レベルの命名 | O0-O3 + Autoを使用 | 2026-06-16 | 晨煦 |
| デフォルト最適化レベル | O1（基本最適化） | 2026-06-16 | 晨煦 |
| 単態化戦略 | Erased/OnDemand/Fullをサポート | 2026-06-16 | 晨煦 |
| Passインターフェース設計 | trait + 依存関係宣言 | 2026-06-16 | 晨煦 |

---

## 付録B：用語集

| 用語 | 定義 |
|------|------|
| **最適化Pass** | IRに対して1回の変換を行う独立モジュール |
| **単態化** | 泛型関数を具体型に特化したコード生成戦略 |
| **定数畳み込み** | コンパイル時に定数式を計算 |
| **死コード消除** | プログラム内の到達不能または未使用のコードを移除 |
| **関数インライン展開** | 関数体を呼び出し点に挿入し、関数呼び出し开销を回避 |
| **末尾呼び出し最適化** | 末尾再帰をループに変換し、栈オーバーフローを回避 |
| **逃逸分析** | 変数が作用域を逃げ出すかを分析し、栈/ヒープ割り当てを決定 |

---

## 参考文献

- [Rustコンパイラ最適化](https://rustc-dev-guide.rust-lang.org/optimizations.html)
- [GCC最適化レベル](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [LLVM Pass Manager](https://llvm.org/docs/WritingAnLLVMNewPMPass.html)
- [V8 TurboFan最適化パイプライン](https://v8.dev/docs/turbofan)

---

## ライフサイクルと運命

このRFCは最適化レベルのアーキテクチャ設計を定義し、今後の最適化Passに統一的なフレームワークを提供します。

**単態化との関係**：単態化は最適化Passの一つであり、このRFCが 受容された後に最初の実装として 开发されるPassです。