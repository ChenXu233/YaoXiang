```markdown
---
title: RFC-009 v9 問題修復方案
status: ongoing
created: 2026-05-29
---

# RFC-009 v9 問題修復方案

[監査レポート](rfc009-v9-implementation-audit.md) の問題リストに基づき、個別の修復方案を提示する。

---

## 修復順序の総覧

```
フェーズ 1（クイック修正、1 行変更）:
  P1-1: solver.rs — &T Dup マッチ

フェーズ 2（残存物の清理）:
  P2-2: Send/Sync 残存清理

フェーズ 3（IR 層コア、P0 ブロック項目）:
  P0-1: ir.rs — Borrow/Release IR コマンド追加
  P0-2: ir_gen.rs — Expr::Borrow IR 生成
  P0-3: execute.rs — インタープリタ Borrow 処理

フェーズ 4（IR 層補完）:
  P1-3: bytecode.rs — Borrow/Release バイトコードコマンド + Opcode
  P2-3: bytecode.rs — From<MonoType> 本当の実装

フェーズ 5（最適化項目）:
  P2-1: ir_gen.rs — MakeClosure ZST 最適化
  P1-2: borrow_checker.rs — ブランド機構（延期可）
```

---

## P1-1: solver.rs — `&T` Dup マッチ（1 行変更）

**問題**：`check_dup_trait` が `MonoType::Ref` を明示的にマッチしておらず、`&T` と `&mut T` のどちらも `_ => false` に落ちる。RFC のコアセマンティクスは「`&T` は自由に複製可能（Dup）、`&mut T` は不可」。

**ファイル**：`src/frontend/core/typecheck/traits/solver.rs` L201-233

**現在のコード**：
```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool
        | MonoType::Char | MonoType::String | MonoType::Bytes
        | MonoType::Void => true,
        MonoType::Arc(_) => true,
        MonoType::Tuple(elems) => elems.iter().all(|t| self.check_dup_trait(t)),
        MonoType::Struct(s) => s.fields.iter().all(|(_, ft)| self.check_dup_trait(ft)),
        MonoType::Enum(_) => true,
        _ => false,  // ← &T と &mut T はここに落ちる
    }
}
```

**修復**：`MonoType::Arc(_)` の後に1行追加：
```rust
// &T はゼロサイズトークンなので自由に複製可能（Dup）；&mut T は不可
MonoType::Ref { mutable: false, .. } => true,
```

**修正後の完全な match**：
```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool
        | MonoType::Char | MonoType::String | MonoType::Bytes
        | MonoType::Void => true,
        MonoType::Arc(_) => true,
        // &T はゼロサイズトークンなので自由に複製可能（Dup）；&mut T は不可
        MonoType::Ref { mutable: false, .. } => true,
        MonoType::Tuple(elems) => elems.iter().all(|t| self.check_dup_trait(t)),
        MonoType::Struct(s) => s.fields.iter().all(|(_, ft)| self.check_dup_trait(ft)),
        MonoType::Enum(_) => true,
        _ => false,
    }
}
```

**検証**：既存テスト + 新規テストケース：
```rust
#[test]
fn test_ref_is_dup() {
    let solver = create_test_solver();
    let ref_ty = MonoType::Ref { mutable: false, inner: Box::new(MonoType::Int(IntType::I64)) };
    assert!(solver.check_dup_trait(&ref_ty), "&T should be Dup");
}

#[test]
fn test_mut_ref_is_not_dup() {
    let solver = create_test_solver();
    let mut_ref_ty = MonoType::Ref { mutable: true, inner: Box::new(MonoType::Int(IntType::I64)) };
    assert!(!solver.check_dup_trait(&mut_ref_ty), "&mut T should NOT be Dup");
}
```

**影響範囲**：純粋な論理修正で、破壊的影響なし。クロージャ捕獲分析における `determine_capture_mode` の `&T` 変数の判定問題が解決される。

---

## P2-2: Send/Sync 残存清理

**問題**：`check_send_trait`/`check_sync_trait` メソッド、`BUILTIN_DERIVES` 内の Send/Sync、`send_sync.rs` モジュールがまだ残っている。

**ファイルリスト**：
1. `src/frontend/core/typecheck/traits/solver.rs` — `check_send_trait`/`check_sync_trait` メソッドを削除
2. `src/frontend/core/typecheck/traits/auto_derive.rs` — `BUILTIN_DERIVES` から Send/Sync エントリを削除
3. `src/frontend/core/typecheck/traits/send_sync.rs` — ファイルごと削除
4. `src/frontend/core/typecheck/traits/mod.rs` — `pub mod send_sync;` を削除

**検証**：`cargo build` コンパイル通過 + 全テスト通過。

---

## P0-1: ir.rs — Borrow/Release IR コマンド追加

**問題**：`Instruction` 列挙型に `Borrow` も `Release` もなく借用トークンを IR で表現できない。

**ファイル**：`src/middle/core/ir.rs`

**設計方案**：`ArcNew`/`ArcClone`/`ArcDrop` のパターンを参照し、2つのコマンドを追加：

```rust
// =====================
// 借用トークン コマンド
// =====================
/// 借用トークンを作成：dst = &src（不変）または dst = &mut src（可変）
/// 借用トークンはゼロサイズ型で、コンパイル後に消える。
/// このコマンドは借用チェッカーのフロー感度分析のみに使用され、
/// ランタイムでは Mov と等価。
Borrow {
    dst: Operand,
    src: Operand,
    mutable: bool,
},
/// 借用トークンを解放し、借用ライフサイクルを終了する
/// 借用チェッカーはこれに基づいてトークン状態を更新する。
Release(Operand),
```

**挿入位置**：`ArcDrop(Operand)` の後（約 L268）、`ShareRef` の前。

**影響**：すべての `match` Instruction の箇所を同期更新する必要がある：
- `ir_gen.rs` 内の IR → バイトコード変換
- `borrow_checker.rs` 内のコマンド走査

---

## P0-2: ir_gen.rs — `Expr::Borrow` IR 生成

**問題**：`generate_expr_ir` に `Expr::Borrow` 分岐が全くなく、`&expr` は IR 段階でサイレントに無視される。

**ファイル**：`src/middle/core/ir_gen.rs`

**設計方案**：`generate_expr_ir` の match に `Expr::Borrow` 分岐を追加する。

**現在の状態**：`Expr::Borrow` は `get_expr_span` でのみマッチ（L1996）され、span 取得に使用されている。

**追加する分岐**（`Expr::Lambda` 分岐の後に挿入推奨）：

```rust
ast::Expr::Borrow { mutable, expr, span } => {
    // 1. 内包する式の IR を生成
    let inner_reg = self.next_temp_reg();
    self.generate_expr_ir(expr, inner_reg, instructions, constants)?;

    // 2. 借用トークン コマンドを作成
    // 借用トークンはゼロサイズ型で、ランタイムでは Mov と等価。
    // このコマンドの存在により、借用チェッカーがフロー感度分析を行える。
    instructions.push(Instruction::Borrow {
        dst: Operand::Local(result_reg),
        src: Operand::Local(inner_reg),
        mutable: *mutable,
    });
}
```

**重要なポイント**：
- 借用トークンは ZST なので、ランタイムで実際に何かを作成する必要はない
- `Borrow` コマンドはランタイムで `Mov` に退化（dst = src）
- しかし借用チェッカーはコンパイル時に `Borrow`/`Release` コマンドを分析して衝突を検出する

---

## P0-3: execute.rs — インタープリタ Borrow 処理

**問題**：インタープリタに borrow 関連処理がなく、`RuntimeValue` に borrow 変種がない。

**ファイル**：`src/backends/interpreter/executor/execute.rs`

**設計方案**：借用トークンは ZST なので、ランタイムで特別な処理は不要。`Borrow` コマンドは `Mov` と等価、`Release` コマンドは `Nop` と等価。

**追加する match 分岐**：

```rust
BytecodeInstr::Borrow { dst, src, mutable: _ } => {
    // 借用トークンはゼロサイズ型で、ランタイムでは Mov と等価
    let val = frame
        .registers
        .get(src.0 as usize)
        .cloned()
        .unwrap_or(RuntimeValue::Unit);
    frame.set_register(dst.0 as usize, val);
    frame.advance();
}
BytecodeInstr::Release { src: _ } => {
    // 借用トークン解放、ランタイムでは何もしない
    frame.advance();
}
```

**重要な設計判断**：`RuntimeValue` に `Borrow` 変種を追加する必要はない。借用トークンの全セマンティクスはコンパイル時に借用チェッカーが保証し、ランタイムでは単に値を受け渡すだけ。

---

## P1-3: bytecode.rs — Borrow/Release バイトコード コマンド

**問題**：`BytecodeInstr` 列挙型に Borrow/Release がなく、IR → バイトコード変換でこれらのコマンドを処理できない。

**ファイル**：`src/middle/core/bytecode.rs`

**変更が必要な箇所**：

### 1. `BytecodeInstr` 列挙型（約 L322、Arc 操作区の後）

```rust
// =====================
// Borrow Token Operations
// =====================
/// 借用トークンを作成（ZST、ランタイム ≈ Mov）
Borrow {
    dst: Reg,
    src: Reg,
    mutable: bool,
},
/// 借用トークンを解放（ZST、ランタイム ≈ Nop）
Release {
    src: Reg,
},
```

### 2. `Opcode` 列挙型

```rust
Borrow,
Release,
```

### 3. `opcode()` メソッド

```rust
BytecodeInstr::Borrow { .. } => Opcode::Borrow,
BytecodeInstr::Release { .. } => Opcode::Release,
```

### 4. `size()` メソッド

```rust
BytecodeInstr::Borrow { .. } => 5,   // dst(2) + src(2) + mutable(1)
BytecodeInstr::Release { .. } => 2,  // src(2)
```

### 5. IR → バイトコード変換（`ir_to_bytecode` または同等功能の関数）

`Instruction::Borrow` を処理する箇所：
```rust
Instruction::Borrow { dst, src, mutable } => {
    BytecodeInstr::Borrow {
        dst: self.operand_to_reg(dst),
        src: self.operand_to_reg(src),
        mutable: *mutable,
    }
}
Instruction::Release(src) => {
    BytecodeInstr::Release {
        src: self.operand_to_reg(src),
    }
}
```

### 6. バイトコード逆シリアル化（`decode_instructions`）

```rust
Opcode::Borrow => {
    // Borrow: dst(2) + src(2) + mutable(1)
    if instr.operands.len() >= 5 {
        let dst = u16::from_le_bytes([instr.operands[0], instr.operands[1]]);
        let src = u16::from_le_bytes([instr.operands[2], instr.operands[3]]);
        let mutable = instr.operands[4] != 0;
        decoded_instructions.push(BytecodeInstr::Borrow {
            dst: Reg(dst),
            src: Reg(src),
            mutable,
        });
    }
}
Opcode::Release => {
    // Release: src(2)
    if instr.operands.len() >= 2 {
        let src = u16::from_le_bytes([instr.operands[0], instr.operands[1]]);
        decoded_instructions.push(BytecodeInstr::Release { src: Reg(src) });
    }
}
```

---

## P2-3: bytecode.rs — `From<MonoType>` 本当の実装

**問題**：`From<MonoType> for IrType` はスタブ実装で、すべての型が `IrType::Void` にマッピングされている。

**ファイル**：`src/middle/core/bytecode.rs` L1418-1424

**現在のコード**：
```rust
impl From<MonoType> for IrType {
    fn from(_: MonoType) -> Self {
        IrType::Void  // スタブ
    }
}
```

**修復**：
```rust
impl From<MonoType> for IrType {
    fn from(ty: MonoType) -> Self {
        match ty {
            MonoType::Int(_) => IrType::I64,
            MonoType::Float(_) => IrType::F64,
            MonoType::Bool => IrType::Bool,
            MonoType::Char => IrType::Char,
            MonoType::String => IrType::String,
            MonoType::Bytes => IrType::Bytes,
            MonoType::Void => IrType::Void,
            MonoType::List(_) => IrType::List,
            MonoType::Tuple(_) => IrType::Tuple,
            MonoType::Struct(_) => IrType::Struct,
            MonoType::Enum(_) => IrType::Enum,
            MonoType::Fn { .. } => IrType::Function,
            MonoType::Arc(_) => IrType::Arc,
            MonoType::Weak(_) => IrType::Weak,
            MonoType::Ref { .. } => IrType::Void,  // ZST、ランタイム表現なし
            _ => IrType::Void,
        }
    }
}
```

**注意**：`IrType` 列挙型にこれらの変種が既に定義されているか確認が必要。なければ、まず `IrType` を拡張する必要がある。

---

## P2-1: ir_gen.rs — MakeClosure ZST 最適化

**問題**：`&T` トークンが捕獲されるときに無意味な env コストが発生する。

**ファイル**：`src/middle/core/ir_gen.rs` L3196-3198

**現在のコード**：
```rust
for captured in &captured_vars {
    if let Some(local_idx) = self.lookup_local(&captured.name) {
        // TODO: ZST 最適化 — 捕獲された変数が ZST（&T トークンなど）の場合、
        // スキップすべき。ランタイム表現がないため。
        env_vars.push(Operand::Local(local_idx));
    }
}
```

**修復**：
```rust
for captured in &captured_vars {
    if let Some(local_idx) = self.lookup_local(&captured.name) {
        // ZST 最適化：借用トークンはゼロサイズ型なので env をスキップ
        if let Some(type_result) = &self.type_result {
            if let Some(mono_type) = type_result.local_var_types.get(&captured.name) {
                if matches!(mono_type, MonoType::Ref { .. }) {
                    continue;  // ZST、env に追加不要
                }
            }
        }
        env_vars.push(Operand::Local(local_idx));
    }
}
```

---

## P1-2: ブランド機構（延期可）

**問題**：変数名の文字列のみで出所を追跡しており、コンパイル時一意 ID がない。

**影響**：現在の変数名追跡は基本機能を支えるのに十分。ブランド機構は防御的強化であり、以下を防止：
1. 異なる出所の同名義変数が誤って同一出所と判断されること
2. トークンの「偽造」（コンパイラ内部の一貫性）

**延期理由**：
- 現在の変数は同一関数内で一意（SSA またはスコープ隔离）
- ブランド機構はコンパイラ内部の最適化であり、ユーザーが見える動作に影響しない
- 実装复杂度が高く、`BorrowToken` に `brand_id: u64` と派生チェーンを追加する必要がある

**推奨**：P0/P1 完了後、LLVM バックエンド前に実装する。

---

## テスト戦略

### 新規テスト

1. **solver.rs**：`&T` Dup / `&mut T` 非 Dup ユニットテスト
2. **ir_gen.rs**：`Expr::Borrow` IR 生成テスト（`Instruction::Borrow` 生成を検証）
3. **execute.rs**：借用トークン ランタイムテスト（`Borrow` ≈ `Mov` を検証）
4. **borrow_checker.rs**：既存11テスト、`Borrow`/`Release` コマンド統合を検証

### 回帰テスト

- `cargo test` 全量通過（現在 2125 テスト）
- `cargo build` コンパイル警告なし

---

## ファイル変更リスト

| ファイル | 変更タイプ | 優先度 |
|----------|------------|--------|
| `src/frontend/core/typecheck/traits/solver.rs` | +1 行 match 分岐 | P1-1 |
| `src/frontend/core/typecheck/traits/send_sync.rs` | ファイル削除 | P2-2 |
| `src/frontend/core/typecheck/traits/mod.rs` | -1 行 mod 宣言 | P2-2 |
| `src/frontend/core/typecheck/traits/auto_derive.rs` | Send/Sync エントリ削除 | P2-2 |
| `src/middle/core/ir.rs` | +10 行（Borrow/Release コマンド） | P0-1 |
| `src/middle/core/ir_gen.rs` | +15 行（Expr::Borrow 分岐 + ZST 最適化） | P0-2, P2-1 |
| `src/middle/core/bytecode.rs` | +30 行（コマンド + Opcode + エンコデック） | P1-3, P2-3 |
| `src/backends/interpreter/executor/execute.rs` | +12 行（Borrow/Release 処理） | P0-3 |

**総変更量**：約 70 行新規コード + 1 ファイル削除。
```