---
title: タイプ属性システム実装設計 (Dup/Clone)
status: 草案
created: 2026-05-29
---

# タイプ属性システム実装設計

## 目標

コンパイラの型システムにおいて `Dup` trait（暗黙的浅複製マーカー）を実装し、trait システムの再帰的チェック機能を補完する。

## コア設計

### Dup trait 定義

```rust
// Clone、Debug と同等の marker trait
// メソッドなし——型レベルマーカーのみ
TraitDefinition {
    name: "Dup",
    methods: {},           // 空——marker trait
    parent_traits: vec!["Clone"],  // Dup は Clone  가능
    generic_params: vec![],
    is_marker: true,
}
```

### 哪些类型是 Dup

| 型 | Dup | 原因 |
|------|-----|------|
| Int, Float(32), Float(64) | ✅ | 原語 |
| Bool, Char | ✅ | 原語 |
| String, Bytes | ✅ | 内部は既に参照カウント |
| &T (ReadToken) | ✅ | ゼロサイズ、コンパイル時概念 |
| &mut T (WriteToken) | ❌ | 線形型、排他的唯一 |
| struct | 自動導出 | 全フィールド Dup → struct Dup |
| Fn (クロージャ) | ❌ | クロージャがキャプチャする環境が非 Dup の可能性 |
| Arc(T) | ✅ | Arc 自体は浅複製可能 |

### Dup と Clone の関係

```
Dup  →  Clone   （全 Dup 型は自動的に Clone を実装）
Clone  ↛  Dup   （Clone があっても Dup があるとは限らない）
```

## 実装チェックリスト

### 1. trait_data.rs — is_marker フィールド追加

**ファイル**: `src/frontend/core/types/base/trait_data.rs`

```rust
pub struct TraitDefinition {
    pub name: String,
    pub methods: HashMap<String, TraitMethodSignature>,
    pub parent_traits: Vec<String>,
    pub generic_params: Vec<String>,
    pub span: Option<Span>,
    pub is_marker: bool,  // NEW: メソッドなしの marker trait
}
```

`is_marker = true` の trait はメソッド実装チェックが不要。コンパイラが marker trait を処理する方法：
- 原語型 → 自動的に impl を登録
- struct → auto-derive 再帰的にフィールドをチェック
- ジェネリクス制約 `T: Dup` → 通常の trait 制約と同様に処理

### 2. std_traits.rs — Dup を登録、Send/Sync を削除

**ファイル**: `src/frontend/core/typecheck/traits/std_traits.rs`

```rust
// STD_TRAITS を修正（Send, Sync を削除、Dup を追加）
pub const STD_TRAITS: &[&str] = &[
    "Clone",
    "Dup",      // NEW
    "Equal",
    "Debug",
    "Iterator",
];

// 新規関数
fn add_dup_trait(trait_table: &mut TraitTable) {
    trait_table.add_trait(TraitDefinition {
        name: "Dup".to_string(),
        methods: HashMap::new(),
        parent_traits: vec!["Clone".to_string()],
        generic_params: vec![],
        span: None,
        is_marker: true,
    });
}

// init_primitive_impls で原語に Dup impl を登録
// Int, Float, Bool, Char, String, Bytes はすべて自動的に Dup impl を取得
```

### 3. solver.rs — 再帰的 struct チェックをサポート

**ファイル**: `src/frontend/core/typecheck/traits/solver.rs`

核心的な変更：`check_dup_trait` メソッドは struct フィールドに再帰的に入る必要がある。

```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        // 原語：自動 Dup
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool 
        | MonoType::Char | MonoType::String | MonoType::Bytes => true,
        
        // Arc：自動 Dup（参照カウント意味論）
        MonoType::Arc(_) => true,
        
        // Ref（借用トークン）：&T Dup、&mut T 非 Dup
        MonoType::Ref { mutable: false, .. } => true,
        MonoType::Ref { mutable: true, .. } => false,
        
        // struct：全フィールドを再帰チェック
        MonoType::Struct(s) => {
            s.fields.iter().all(|(_, field_ty)| self.check_dup_trait(field_ty))
        }
        
        // Tuple：全要素を再帰チェック
        MonoType::Tuple(elems) => {
            elems.iter().all(|t| self.check_dup_trait(t))
        }
        
        // Enum：全 variant の全フィールドをチェック
        MonoType::Enum(e) => {
            e.variants.iter().all(|v| 
                v.fields.iter().all(|(_, t)| self.check_dup_trait(t))
            )
        }
        
        // その他すべて：デフォルトで非 Dup
        _ => false,
    }
}
```

同じパターンを `check_clone_trait` にも適用——以前は原語のみ認識だったが、struct への再帰も追加。

### 4. auto_derive.rs — 複合型と再帰をサポート

**ファイル**: `src/frontend/core/typecheck/traits/auto_derive.rs`

現在の `can_auto_derive` の致命的な問題：`List[Int]` のような `Type::Generic` に遭遇すると即座に false を返す。

```rust
pub fn can_auto_derive(
    trait_table: &TraitTable,
    trait_name: &str,
    fields: &[StructField],
) -> bool {
    for field in fields {
        if !field_type_satisfies(trait_table, trait_name, &field.ty) {
            return false;
        }
    }
    true
}

// NEW: フィールド型が trait を満たすかを再帰チェック
fn field_type_satisfies(
    trait_table: &TraitTable,
    trait_name: &str,
    ty: &Type,
) -> bool {
    match ty {
        // 単純型名 → trait table を查询
        Type::Name { name, .. } => {
            trait_table.has_impl(trait_name, name)
        }
        
        // ジェネリック型 List(Int), Option(Point) → 内側をチェック
        Type::Generic { name, args, .. } => {
            // コンテナ自体が trait を実装し、全パラメータも実装
            if !trait_table.has_impl(trait_name, name) {
                return false;
            }
            args.iter().all(|arg| field_type_satisfies(trait_table, trait_name, arg))
        }
        
        //  タプル → 全要素をチェック
        Type::Tuple(elems) => {
            elems.iter().all(|e| field_type_satisfies(trait_table, trait_name, e))
        }
        
        // 関数型 → 関数は Dup 不可（保守的）
        Type::Fn { .. } => false,
        
        // その他は導出不可
        _ => false,
    }
}
```

### 5. resolution.rs — trait 解決を完善

**ファイル**: `src/frontend/core/typecheck/traits/resolution.rs`

```rust
fn find_trait_definition(&self, name: &str) -> Option<String> {
    match name {
        "Clone" => Some("std::Clone".to_string()),
        "Dup" => Some("std::Dup".to_string()),     // NEW
        "Debug" => Some("std::fmt::Debug".to_string()),
        "Equal" => Some("std::cmp::Equal".to_string()),
        "Iterator" => Some("std::iter::Iterator".to_string()),
        _ => None,
    }
}
```

### 6. bounds.rs — Dup 制約サポート

**ファイル**: `src/frontend/core/typecheck/inference/bounds.rs`

bounds checker の既存コードは既に `T: Clone` パターンをサポートしている。`T: Dup` を追加すると自動的に動作——`trait_solver.check_trait(ty, "Dup")` を呼び出すだけ。

唯一確認すべきこと：`check_trait` が失敗した場合、struct 型に対してはまず auto-derive を試行する。

```rust
pub fn check_trait_bounds(&mut self, ty: &MonoType, bounds: &[String]) -> Result<()> {
    for bound in bounds {
        if !self.trait_solver.check_trait(ty, bound) {
            // auto-derive を試行
            if let MonoType::Struct(s) = ty {
                if can_auto_derive_for_monotype(&self.trait_table, bound, s) {
                    continue;  // auto-derive 成功
                }
            }
            return Err(TypeError::TraitBoundFailed { ... });
        }
    }
    Ok(())
}
```

### 7. mono.rs — MonoType は変更不要（現在）

`MonoType` に `TypeFlags` を追加する必要はない。Dup の判定は完全に trait システムを通じて——`trait_table.has_impl("Dup", type_name)` を 查询すれば十分。これは型チェック時の操作であり、ホットパスではない。

将来パフォーマンスが必要になれば、`Cache<TypeId, bool>` を追加して 查询結果をキャッシュできる。今は不需要。

### 8. Send/Sync のクリーンアップ

**ファイル**: `src/frontend/core/typecheck/traits/std_traits.rs`
- `STD_TRAITS` から "Send", "Sync" を削除
- `add_send_trait()`, `add_sync_trait()` を削除

**ファイル**: `src/middle/passes/lifetime/send_sync.rs`
- checker 全体を削除、または no-op として保持（保守的）
- `OwnershipChecker` から `send_sync_checker` フィールドを削除
- `mod.rs` から `SendSyncChecker` の import と呼び出しを削除

**ファイル**: `src/middle/passes/lifetime/error.rs`
- `OwnershipError::NotSend`, `NotSync` バリアントを削除（または deprecated として保持）

## 実装順序

1. **trait_data.rs** — `is_marker` フィールドを追加（5 行変更）
2. **std_traits.rs** — Dup を登録、Send/Sync を削除、原語の dup impl を登録（~50 行変更）
3. **solver.rs** — 再帰的 struct チェック（~30 行変更）
4. **auto_derive.rs** — ジェネリクスパラメータチェックをサポート（~50 行書き直し）
5. **resolution.rs** — Dup パスを追加（1 行）
6. **bounds.rs** — auto-derive 統合（~10 行）
7. **Send/Sync のクリーンアップ** — 関連コードを削除

総変更量推定：~200 行。変更は6つのファイルの trait システムディレクトリに集中。

## 検証方法

```yaoxiang
# テスト 1：原語の自動 Dup
x: Int = 42
y = x        # ✅ Int: Dup
print(x)     # ✅

# テスト 2：struct の自動導出
Point2D: Type = { x: Float, y: Float }
p = Point2D(1.0, 2.0)
q = p         # ✅ Point2D: Dup（両フィールドが Float: Dup）
print(p)      # ✅

# テスト 3：非 Dup フィールドを含む struct
Buffer: Type = { data: Array(Int), len: Int }
b = Buffer(...)
b2 = b        # ❌ Move（Array は Dup 不可）
print(b)      # ❌ 既に移動済み

# テスト 4：ジェネリクス制約
dup_use: (x: T: Dup) -> T = x  # ✅ T: Dup 制約
```

## 参考

- 探查ギャップ分析（型システム統合ギャップ）
- 探查ギャップ分析（trait システムギャップ）
- [RFC-011 ジェネリクスシステム設計](../../design/rfc/accepted/011-generic-type-system.md)
- [RFC-009 所有権モデル v9](../../design/rfc/accepted/009-ownership-model.md)