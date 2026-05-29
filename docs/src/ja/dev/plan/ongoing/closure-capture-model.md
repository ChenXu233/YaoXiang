```md
---
title: クロージャ捕獲モデル実装設計
status: draft
created: 2026-05-29
---

# クロージャ捕獲モデル実装設計

## 目標

クロージャが外部変数を捕獲する分析及び、捕獲モード選択と IR 生成を実装する。

## コアルール

```
変数の型    クロージャがエスケープするか    捕獲方式
───────────────────────────────────────────────────────
Dup         任意                          コピー（ゼロコスト、副作用なし）
非 Dup      エスケープしない              自動借用（&T または &mut T トークン）
非 Dup      エスケープする                Move（所有権移動）
```

このルール体系和は関数呼び出し時の自動借用選択と**同一のロジック**である。新しい概念を持ち込まない。

## 実装チェックリスト

### Step 1：エスケープ解析

**ファイル**: `src/frontend/core/typecheck/inference/expressions.rs`（または新規作成 `capture.rs`）

クロージャが「エスケープする」の定義：

```rust
enum ClosureUsage {
    Inline,    // その場で呼び出す、または同期関数に渡す（エスケープしない）
    Escaping,  // spawn、return、ヒープに保管、全域変数に保管
}
```

エスケープ判定ルール：

```
lambda を spawn { ... } の引数として渡す        → Escaping
lambda を return 値として返す                    → Escaping
lambda を外部変数/フィールドに代入する           → Escaping
lambda を（非 spawn の）関数パラメータに渡す     → Inline（保守的）
lambda をその場で呼び出す                        → Inline
```

**保守的原則**：判断できない場合は Escaping として扱う。

### Step 2：捕獲変数解析

**クロージャ本体 AST を巡回**し、クロージャの外部スコープの変数への参照を見つける。

```rust
struct CaptureInfo {
    captures: Vec<CapturedVar>,
}

struct CapturedVar {
    name: String,           // 変数名
    usage: CaptureUsage,    // 使用方式
}

enum CaptureUsage {
    Read,           // 読み取りのみ（&T だけで十分）
    Write,          // 読み書き（&mut T が必要）
    Move,           // 所有権移動（非 Dup + エスケープ）
    DupCopy,        // Dup 型を直接コピー
}
```

**解析手順**：

1. lambda body の AST を巡回する
2. すべての `Expr::Var(name)` 参照を記録する
3. フィルタリング：クロージャの外部スコープの変数のみ残す
4. 使用方式で分類：
   - 代入/mut メソッド呼び出し → Write
   - 読み取りのみ → Read
   - 他の場所に Move される → Move

### Step 3：捕獲モード選択

```rust
fn determine_capture_mode(
    var: &CapturedVar,
    ty: &MonoType,
    usage: ClosureUsage,
    is_dup: bool,
) -> CaptureMode {
    match (is_dup, usage) {
        // Dup 型：直接コピー——最速パス
        (true, _) => CaptureMode::Copy,
        
        // 非 Dup + エスケープ → Move
        (false, ClosureUsage::Escaping) => CaptureMode::Move,
        
        // 非 Dup + エスケープしない → 自動借用
        (false, ClosureUsage::Inline) => match var.usage {
            CaptureUsage::Read => CaptureMode::Borrow,     // &T
            CaptureUsage::Write => CaptureMode::BorrowMut, // &mut T
            CaptureUsage::Move => CaptureMode::Move,
            CaptureUsage::DupCopy => unreachable!(),
        },
    }
}

enum CaptureMode {
    Copy,       // 値を直接コピー
    Borrow,     // &T トークン
    BorrowMut,  // &mut T トークン
    Move,       // 所有権移動
}
```

**重要なシナリオ**：

```yaoxiang
# 1. &T トークンをクロージャに渡す——Dup → Copy、ゼロコスト
threshold: &Float = &some_float
items.filter(|p| p.x > threshold)
# threshold: &Float → Dup → CaptureMode::Copy
# コンパイラ：トークンをコピー（サイズゼロ、ランタイムオーバーヘッドなし）

# 2. 非 Dup 値、クロージャがエスケープしない——自動借用
buffer: Buffer = ...
process(|b| b.read())
# buffer は Dup ではない、クロージャはエスケープしない → CaptureMode::Borrow
# コンパイラ：自動的に &Buffer トークンを作成してクロージャに渡す

# 3. クロージャがエスケープ——Move
big_data: Data = ...
spawn { use(big_data) }
# big_data は Dup ではない、spawn → Escaping → CaptureMode::Move
```

### Step 4：IR 生成

**ファイル**: `src/middle/core/ir_gen.rs`

```rust
// 現在（空実装）
Instruction::MakeClosure {
    dst: Operand::Local(result_reg),
    func: closure_name,
    env: Vec::new(),  // ← 常に空
}

// 改める
Instruction::MakeClosure {
    dst: Operand::Local(result_reg),
    func: closure_name,
    env: captured_vars,  // Vec<(Operand, CaptureMode)>
}
```

各捕獲変数の IR 生成：

```rust
for captured in &captures {
    let src = self.lookup_local(&captured.name);
    match captured.mode {
        CaptureMode::Copy => {
            // Dup 型：Mov 命令でコピー（Step 5 のゼロコスト最適化参照）
            instructions.push(Instruction::Move {
                dst: new_temp(),
                src,
            });
        }
        CaptureMode::Borrow => {
            // 自動借用：ReadToken を作成
            instructions.push(Instruction::Borrow {
                dst: new_temp(),
                src,
                mutable: false,
            });
        }
        CaptureMode::BorrowMut => {
            instructions.push(Instruction::Borrow {
                dst: new_temp(),
                src,
                mutable: true,
            });
        }
        CaptureMode::Move => {
            // Move：所有権移動
            instructions.push(Instruction::Move {
                dst: new_temp(),
                src,
            });
        }
    }
}
```

### Step 5：ZST 最適化——トークン除去

`CaptureMode::Copy` を `&T` に使用时、`&T` はゼロサイズ型である。`Instruction::Move` がゼロバイトデータをコピーする → **IR 最適化パスで除去する必要がある**。

2 つの実装方式：

**方式 A：IR 生成時にスキップ**
```rust
CaptureMode::Copy if is_zero_sized_type(ty) => {
    // IR 命令を生成しない
    // クロージャ本体は外側の変数を直接参照（コンパイル時）
}
```

**方式 B：IR 最適化パス**
```rust
// 新規 ZstElimination パス：
// すべての Move dst, src をスキャンし、src 型が ZST ならその命令を削除
// dst を src で置換（エイリアス）
```

**方式 A を推奨**——生成時に ZST であることが分かるため、後続の最適化が不要である。

### Step 6：借用トークン競合検出

クロージャが `&mut T` トークンを捕獲した後、元のスコープでは同時にそのトークンを使用できない：

```yaoxiang
tok = &mut point        # WriteToken 作成
closure = |x| {
    tok.shift(x, 0.0)   # tok はクロージャに借用される
}
tok.shift(1.0, 0.0)     # ❌ コンパイルエラー：tok の WriteToken はクロージャに保持されている
```

これは既存のトークン競合検出（RFC-009 v9 2.6 節）がカバーする——borrow checker はフロー感受性の生存性分析で処理する。

## ファイル変更リスト

| # | ファイル | 変更内容 |
|---|----------|----------|
| 1 | `typecheck/inference/capture.rs`（新規作成） | 捕獲解析 + エスケープ解析 + モード選択 |
| 2 | `typecheck/inference/expressions.rs` | lambda 型推論で捕獲解析を呼び出す |
| 3 | `middle/core/ir_gen.rs` | MakeClosure env 填充、ZST スキップ |
| 4 | `middle/core/ir.rs` | Borrow 命令が必要なら追加（IR に必要であれば） |
| 5 | `middle/passes/lifetime/mod.rs` | クロージャ関連の借用検査を登録（新しい検査があれば） |

総変更量目安：~300 行。

## 実装順序

1. **捕獲解析**（capture.rs）——純粋な AST 巡回、捕獲変数リストを返す
2. **エスケープ解析**——クロージャがエスケープするかを判定
3. **モード選択**——Dup/非Dup + エスケープ/非エスケープ に基づいて CaptureMode を決定
4. **IR 生成**——MakeClosure env を填充
5. **ZST 最適化**——Dup + ZST の IR 命令をスキップ

1-3 は純粋な型検査層（フロントエンド）。4-5 は IR 生成層（ミッドエンド）。分开実装可能である。

## 検証シナリオ

```yaoxiang
# ✅ シナリオ 1：Dup トークンコピー（最もコアなシナリオ）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# ✅ シナリオ 2：非 Dup 自動借用
process_buffer: (buf: Buffer) -> Void = {
    transform(|b| b.read())  # buf はエスケープしない → &T 借用
}

# ✅ シナリオ 3：跨タスク強制 Move
spawn_worker: (data: Data) -> Void = {
    spawn { use(data) }  # エスケープ → Move
}

# ❌ シナリオ 4：借用 + 続けて使用で競合
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf は既にクロージャに借用されている
}
```

## 参考

- [RFC-009 v9 所有権モデル](../../design/rfc/accepted/009-ownership-model.md) — 借用トークンシステム
- [RFC-007 関数構文統一](../../design/rfc/accepted/007-function-syntax-unification.md) — lambda 定義
- 探查レポート：IR 生成ギャップ（MakeClosure env 空実装）
```