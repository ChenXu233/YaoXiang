# エラーコードリファレンス

YaoXiang コンパイラは診断情報を識別するためにエラーコードを使用します。エラーコードは番号範囲でグループ化されており、各エラーコードは特定のエラーシナリオに対応しています。

---

## E0xxx -- 字句解析と構文解析

字句解析器（Lexer）と構文解析器（Parser）段階で発生するエラーです。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| E0001 | `Invalid character: '{char}'` | 無効な文字 |
| E0002 | `Invalid number literal: '{literal}'` | 無効な数値リテラル |
| E0003 | `Unterminated string starting at line {line}` | 終了していない文字列 |
| E0004 | `Invalid character literal: '{literal}'` | 無効な文字リテラル |
| E0010 | `Expected {expected}, found {found}` | 期待されるトークン |
| E0011 | `Unexpected token: '{token}'` | 予期しないトークン |
| E0012 | `Invalid syntax: {reason}` | 無効な構文 |
| E0013 | `Mismatched {bracket_type}: opened at line {open_line}, column {open_col}, not closed` | 対応しない括弧 |
| E0014 | `Missing semicolon after {statement}` | セミコロンが不足 |

## E1xxx -- 型チェック

型チェック段階で発生するエラー。変数の型、関数呼び出し、パターンマッチング、ジェネリクスのインスタンス化、並行セマンティクス、エラー伝播などを涵盖します。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| E1001 | `Unknown variable: '{name}'` | 未知の変数 |
| E1002 | `Expected type '{expected}', found type '{found}'` | 型の不一致 |
| E1003 | `Unknown type: '{type}'` | 未知の型 |
| E1010 | `Function '{func}' expects {expected} arguments, found {found}` | 引数の数が一致しない |
| E1011 | `Parameter type mismatch: expected '{expected}', found '{found}'` | 引数の型が一致しない |
| E1012 | `Return type mismatch: expected '{expected}', found '{found}'` | 戻り値の型が一致しない |
| E1013 | `Function not found: '{func}'` | 関数が見つからない |
| E1020 | `Cannot infer type for '{expr}'` | 型を推論できない |
| E1021 | `Type inference conflict: {reason}` | 型推論の競合 |
| E1030 | `Pattern non-exhaustive: missing patterns {patterns}` | パターン網羅性の不足 |
| E1031 | `Unreachable pattern: '{pattern}'` | 到達不能なパターン |
| E1040 | `Operation '{op}' is not supported for type '{type}'` | 操作がサポートされていない |
| E1041 | `Index out of bounds: valid range is 0..{max}, found {index}` | インデックスが範囲外 |
| E1042 | `Field '{field}' not found in struct '{struct}'` | フィールドが見つからない |
| E1050 | `Logical operation requires boolean operands, found '{left}' and '{right}'` | ブール型オペランドが必要 |
| E1051 | `Logical NOT requires boolean operand, found '{type}'` | 論理 NOT にはブール型オペランドが必要 |
| E1052 | `Cannot dereference type '{type}', expected pointer type` | 無効な逆参照 |
| E1053 | `Cannot access field on non-struct type '{type}'` | 非構造体フィールドへのアクセス |
| E1054 | `Condition must be boolean, found '{type}'` | 条件の型が一致しない |
| E1055 | `Constraint type '{type}' can only be used in generic context` | 制約が非ジェネリックコンテキストで使用されている |
| E1060 | `Expected {expected} type argument(s), found {found}` | 型引数の数が一致しない |
| E1061 | `Cannot instantiate generic type with given arguments` | ジェネリック型をインスタンス化できない |
| E1070 | `Unknown label: '{label}'` | 未知のラベル |
| E1080 | `` `spawn` is only allowed inside @block scope (current: @{mode}) `` | spawn は @block スコープ内でのみ使用可能 |
| E1081 | `` `?` is only allowed inside functions returning Result `` | `?` は Result を返す関数内でのみ使用可能 |
| E1082 | `` `?` requires a Result expression, found '{type}' `` | `?` は Result 式に対してのみ使用可能 |
| E1083 | `` Result error type mismatch for `?`: expected '{expected}', found '{found}' `` | `?` のエラー型が一致しない |
| E1090 | `Type: Type = Type` | 言い表せない（イースターエッグ） |
| E1091 | `Generic meta-type self-reference is not allowed: '{decl}'` | 無効なジェネリック メタ型自己参照 |

## E2xxx -- 意味解析

意味解析段階で発生するエラー。スコープ、変数のライフタイム、所有権、関数シグネチャの解決などを涵盖します。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| E2001 | `Variable '{name}' is not in scope` | スコープエラー |
| E2002 | `Duplicate definition: '{name}' is already defined in this scope` | 重複定義 |
| E2003 | `Ownership constraint violated: {reason}` | 所有権エラー |
| E2010 | `Cannot assign to immutable variable '{name}'` | 不変変数への代入不可 |
| E2011 | `Use of uninitialized variable '{name}'` | 未初期化変数の使用 |
| E2012 | `Mutability conflict: cannot use mutable reference in immutable context` | 可変性衝突 |
| E2013 | `Cannot shadow existing variable '{name}'` | 変数の遮蔽 |
| E2014 | `'{name}' has been moved and cannot be used` | 移動済み変数の使用 |
| E2090 | `Invalid signature: {reason}` | 無効なシグネチャ |
| E2091 | `Invalid signature: unknown type '{type_name}'` | シグネチャの未知の型 |
| E2092 | `Invalid signature: missing '->'` | シグネチャに矢印がない |
| E2093 | `Invalid signature: duplicate parameter '{name}'` | パラメータ名の重複 |
| E2094 | `Invalid signature: generic '{name}' shadows outer generic` | ジェネリックパラメータの遮蔽 |
| E2095 | `Invalid signature: parameter '{name}' shadows generic` | パラメータ名がジェネリックを遮蔽 |

## E4xxx -- ジェネリクスとトレイト

ジェネリック制約とトレイトシステム関連のエラーです。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| E4001 | `Type '{type}' does not satisfy the trait bound '{trait}'` | ジェネリック制約違反 |
| E4002 | `Trait '{trait}' not found` | トレイトが見つからない |
| E4003 | `Missing implementation for trait '{trait}' for type '{type}'` | トレイト実装の欠落 |
| E4004 | `Conflicting trait implementations for '{trait}'` | トレイト実装の競合 |
| E4005 | `Associated type '{assoc_type}' not found in '{container}'` | 関連型が見つからない |

## E5xxx -- モジュールとインポート

モジュールシステムとインポート関連のエラーです。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| E5001 | `Module '{module}' not found` | モジュールが見つからない |
| E5002 | `Failed to import module '{module}': {reason}` | インポートエラー |
| E5003 | `Export '{export}' not found in module '{module}'` | エクスポートが見つからない |
| E5004 | `Circular dependency detected: {path}` | 循環依存 |
| E5005 | `Invalid module path: '{path}'` | 無効なモジュールパス |
| E5006 | `Duplicate import: '{name}' is already imported` | 重複インポート |
| E5007 | `Module '{module}' exports: {available}` | モジュールエクスポートのヒント |

## E6xxx -- ランタイム

ランタイム段階で発生するエラーです。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| E6001 | `Division by zero in expression: {expr}` | ゼロ除算エラー |
| E6002 | `Null pointer dereference at {location}` | ヌルポインタ逆参照 |
| E6003 | `Array index out of bounds: valid range is 0..{max}, found {index}` | 配列インデックスが範囲外 |
| E6004 | `Stack overflow: recursion depth exceeded limit {limit}` | スタックオーバーフロー |
| E6005 | `Assertion failed: {condition}` | アサーション失敗 |
| E6006 | `Function not found: '{func}'` | 関数が見つからない（ランタイム） |
| E6007 | `Runtime error: {message}` | ランタイムエラー |

## E7xxx -- I/O とシステム

I/O 操作とシステムレベルのエラーです。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| E7001 | `File not found: '{path}'` | ファイルが見つからない |
| E7002 | `Permission denied: '{path}'` | 権限が拒否された |
| E7003 | `I/O error: {reason}` | I/O エラー |
| E7004 | `Network error: {reason}` | ネットワークエラー |

## E8xxx -- 内部コンパイラエラー

コンパイラの内部エラー。通常、コンパイラ自体 버グを示します。この種のエラーが発生した場合は [GitHub Issues](https://github.com/yaoxiang/yaoxiang/issues) で報告してください。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| E8001 | `Internal compiler error: {message}` | 内部コンパイラエラー |
| E8002 | `Unexpected compiler panic: {reason}` | 予期しない Panic |
| E8003 | `Compiler phase error: {phase} - {message}` | コンパイラフェーズエラー |

## W1xxx -- 警告

デッドコード検出関連の警告です。警告はコンパイルを停止しませんが、コードに潜在的な問題があることを示します。

| エラーコード | テンプレート | 説明 |
|--------|------|------|
| W1001 | `Unused exported function: '{name}'` | 未使用のエクスポート関数 |
| W1002 | `Unused exported type: '{name}'` | 未使用のエクスポート型 |
| W1003 | `Unused import: '{name}'` | 未使用のインポート |
| W1004 | `Unused exported variable: '{name}'` | 未使用のエクスポート変数 |
| W1005 | `Unused exported method: '{name}'` | 未使用のエクスポートメソッド |

---

全部で **83** 個の診断コード（エラーコード 78 個 + 警告コード 5 個）。