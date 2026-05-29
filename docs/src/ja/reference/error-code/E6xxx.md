# E6xxx：実行時エラー

> `src/util/diagnostic/codes/` より自動生成

## エラー一覧

## E6001：ゼロ除算

**カテゴリ**: Runtime

**メッセージ**: Attempted to divide by zero

**ヘルプ**: ゼロ除算を防ぐチェックを追加してください

---

## E6002：Null ポインタ間接参照

**カテゴリ**: Runtime

**メッセージ**: Attempted to access a null value

**ヘルプ**: 値にアクセスする前にNULLチェックを追加してください

---

## E6003：配列インデックスが境界外

**カテゴリ**: Runtime

**メッセージ**: Array index is out of bounds at runtime

**ヘルプ**: インデックスが配列の境界内であることを確認してください

---

## E6004：スタックオーバーフロー

**カテゴリ**: Runtime

**メッセージ**: Recursion depth exceeded stack limit

**ヘルプ**: 再帰の深さを減らすか、反復を使用してください

---

## E6005：アサーション失敗

**カテゴリ**: Runtime

**メッセージ**: Assertion failed at runtime

**ヘルプ**: アサーション条件を修正するか有効な入力を提供してください