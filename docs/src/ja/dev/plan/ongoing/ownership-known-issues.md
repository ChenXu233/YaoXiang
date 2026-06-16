# 所有権チェックの既知の問題

> 最終更新：2026-06-16
> 実装場所：`src/frontend/core/typecheck/layers/ownership.rs`
> テスト場所：`src/frontend/core/typecheck/layers/tests/ownership.rs`
> 61 テスト、0 失敗

## 正確性の欠陥

- [x] ### 1. ref のエイリアスが spawn に入りエスケープを見逃す（P0）— 修正済み (2026-06-15)

**シナリオ**：
```yaoxiang
shared = ref x
alias = shared       // shared を Move → alias
spawn { use(alias) } // alias ∉ ref_vars → エスケープの見落とし → Rc を選択（非アトミック、スレッド間安全でない）
```

**根本原因**：`OwnershipChecker` は `Expr::Ref` の直接代入の変数名（`ref_vars`）のみを追跡する。ref 変数が Move により中間変数に渡された後、中間変数は `ref_vars` を変更しない。

**影響**：spawn を越えて使用される ref が誤って `RcNew` にコンパイルされる可能性があり、非アトミックな参照カウントはスレッド間で使用するとデータ競合を引き起こす可能性がある。

**修正**：`StmtKind::Var` および `BinOp::Assign` のハンドラにおいて、右辺が `Expr::Var(name)` で `name ∈ ref_vars` の場合、左辺のターゲットを `ref_vars` に追加する（commit `9029d5b`）。

- [ ] ### 2. spawn が変数を Move した後でも外層が引き続き使用可能（P1 — セマンティクス未定義）

**シナリオ**：
```yaoxiang
shared = ref data
spawn { a = shared }  // spawn 本体を walk（save/restore）→ shared は体内で Moved → 外層は復元
use(shared)            // 外層の shared は依然 Alive——正しい、しかし spawn 本体内では shared は Move 済み
```

**根本原因**：`Expr::Spawn` は save/restore を使用しており、spawn 本体内での所有権変更は外層に影響しない。これは正しい設計であるが、spawn 本体内での `a = shared` の Move は spawn の「一時的な walk」内でのみ検出される。spawn 本体内が `shared` の Move を実行した場合、save/restore により外層は復元されるが、**spawn の後に外層が `shared` を使用し続けることを阻止するものはない**。

**影響**：spawn が実行時に `shared` を Move した場合（例：`a = shared`）、spawn 後の外層コードは依然として `shared` にアクセスできる——これは YaoXiang の並行性モデルでは正しい可能性があるが、セマンティクスは明確に定義されていない。

**修正方針**：言語仕様を明確化する必要がある：spawn が捕捉した Move のセマンティクスが外層スコープに影響するかどうか。「spawn が独立したコピーを取得する」であれば現在の動作は正しい。「spawn が所有権を消費する」であれば、save/restore を削除するか、クロージャに類似した Captures を導入する必要がある。

## 精度のトレードオフ

- [ ] ### 3. 分岐の排他性を保守的に判定し衝突を報告（P1）

**シナリオ**：
```yaoxiang
if cond {
    a = &mut x   // 分岐 A
} else {
    b = &mut x   // 分岐 B
}
// 理論上：A と B は排他的であり、衝突すべきでない
// 実際：2 つの WriteToken が順次作成される → BorrowConflict を報告
```

**根本原因**：`NLL without fixpoint` アーキテクチャの制限——単一パスの AST walk はパスの条件をモデル化せず、分岐の排他性と順次実行を区別できない。

**修正方針**：CFG の SMT スロー経路の介入が必要である（現在 `smt_cut` は実装されているが、`while + path_condition` のシナリオでのみトリガーされる）。if/else 分岐への拡張には、path_condition の Borrow ハンドラへの伝播が必要である。

- [ ] ### 4. ref 型が Dup として認識されない（P1）

**シナリオ**：
```yaoxiang
shared = ref x
a = shared    // Move——しかし ref は理論上 Dup 型であり、コピー可能であるべき
b = shared    // use after move——実際には許可されるべき
```

**根本原因**：所有権チェッカは `ref T` が Dup 型（コピー可能な参照カウントハンドル）であることを認識していない。`StmtKind::Var` の Move ロジックはすべての型を同様に扱う。

**影響**：ref 値の意味は期待されるよりも厳格である——RFC-009 で設計された「自由にコピー」のように動作しない。

**修正方針**：`TypeEnvironment` から変数の型を照会し、Dup 型に対して Move ロジックをスキップする必要がある。これは `clone()` の明示的な呼び出しを要求する全体的な設計と一致する——現在の保守的な動作は正しいセマンティクスよりも寛容ではない。

## インフラ

- [ ] ### 5. エラーコード形式が統一されていない（P2）

**説明**：フロントエンドの所有権チェッカは `DisproofModel.into_diagnostic()` → エラーコード E2014-E2020 を使用する。Middle 層のレガシー `lifetime/error.rs` は独立した `ValueState` + `Checker` trait を使用する。2 つのシステムは現在並存している。

**修正方針**：Middle 層の `error.rs` の `ValueState` と `Checker` trait を削除する（残り 2 つの参照：`lifecycle.rs` と `cycle_check` テスト）、フロントエンドのエラーコードシステムに統一する。

- [ ] ### 6. ネストされた関数のパラメータ付き形式を分析しない（P2）

**説明**：`StmtKind::Binding` は `params.is_empty() && !body.is_empty()` のクロージャに対してのみ捕捉分析を行う。パラメータ付きネスト関数は `vec![]` を返す（`check_module` により body を独立してチェックするが、捕捉セマンティクスは分析しない）。

**影響**：パラメータ付きネスト関数の本体内の所有権エラーは検出されず（現在直接 skip）、捕捉情報も生成されない。パラメータ付きネスト関数が外層の変数を使用した場合、所有権セマンティクスは分析されない。

**修正方針**：パラメータ付き/パラメータなし Binding を統一的に処理し、その body に対して `check_function` + 捕捉分析を同時に行う。