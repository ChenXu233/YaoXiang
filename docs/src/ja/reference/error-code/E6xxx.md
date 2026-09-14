# E6xxx：ランタイムエラー

> `src/util/diagnostic/codes/` から自動生成

## エラーリスト

## E6001：Division by zero

**カテゴリ**: Runtime

**メッセージ**: Attempted to divide by zero

**ヘルプ**: Add a check to prevent division by zero

---

## E6003：Array index out of bounds

**カテゴリ**: Runtime

**メッセージ**: Array index is out of bounds at runtime

**ヘルプ**: Ensure the index is within the array bounds

---

## E6004：Stack overflow

**カテゴリ**: Runtime

**メッセージ**: Recursion depth exceeded stack limit

**ヘルプ**: Reduce recursion depth or use iteration

---

## E6005：Assert failed

**カテゴリ**: Runtime

**メッセージ**: Assertion failed at runtime

**ヘルプ**: Fix the assertion condition or provide valid input

---

## E6006：Function not found (runtime)

**カテゴリ**: Runtime

**メッセージ**: Function not found: '{func}'

**ヘルプ**: Ensure the function is defined and spelled correctly

---

## E6007：Runtime error

**カテゴリ**: Runtime

**メッセージ**: Runtime error: {message}

**ヘルプ**: See the error message for details

---

## E6008：Key not found

**カテゴリ**: Runtime

**メッセージ**: Key not found

**ヘルプ**: Use dict.has to check key existence before indexing

---

> #299
> §4：Dict の欠キーとインデックス範囲外（E6003）は意味論的に異なる種類です——キーが存在しない vs 序数の境界超過であり、別個のコードとして独立させ診断情報を保持します。安全なアクセスには
> `dict.has` で先に判定してから取得してください。
