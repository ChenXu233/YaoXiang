```yaml
---
title: "RFC-026a: 拡張可能な FFI 機構体系"
status: "レビュー中"
author: "晨煦"
created: "2026-06-05"
updated: "2026-07-03"
group: "rfc-026"
---
```

# RFC-026a: 拡張可能な FFI 機構体系

> **親 RFC**: [RFC-026: FFI 中核機構](../accepted/026-ffi-core-mechanism.md)
>
> 本 RFC は RFC-026 の拡張可能性を定義する——C ABI 以外の FFI 機構（Wasm、Python、カスタム ABI）をプラグインとして組み込む方法、および動的ロードモードについて述べる。

## 概要

RFC-026 は FFI の中核機構を定義し、`Native.c("lib")` は組み込みの C ABI を通る。本 RFC は ABI 機構をプラガブルな `FfiMechanism` として抽象化し、コアが具体的な ABI を一切ハードコードしないようにする：

1. **`FfiMechanism` 抽象**：機構が実装すべき四つの操作（ライブラリのロード、シンボルの解決、マーシャリング、呼び出し）を定義
2. **機構タグがそのまま機構選択**：`Native.c` / `Native.wasm` / `Native.python` はそれぞれ登録済みの機構を選択する
3. **コンパイル時の機構登録表**：機構タグはコンパイル時に検証され、未登録のタグはコンパイルエラーとなる
4. **静的ロードと動的ロード**：両モードとも RFC-026 の安全性境界を維持する

## 動機

RFC-026 は C ABI（`Native.c`）のみを組み込んでいる。しかし YaoXiang は将来的に以下を必要とする可能性がある：
- Wasm モジュールの呼び出し（`Native.wasm`）
- Python 拡張の埋め込み（`Native.python`）
- ユーザー定義の ABI（専用ハードウェア、RPC ブリッジ）

コンパイラにこれらの ABI をハードコードするよりも、「ライブラリのロード方法、シンボルの解決方法、マーシャリング方法、呼び出し方法」を trait として抽象化し、各機構をプラグインとして実装するほうがよい。コアは `FfiMechanism` のみを認識し、具体的な ABI は認識しない。

### 設計上の制約

1. **機構タグのコンパイル時検証**：`Native.xxx(...)` の `xxx` は登録済みの機構でなければならない。さもないとコンパイルエラーとなる
2. **機構をハードコードしない**：コンパイラは機構リストを内蔵しない（参考実装としての `.c` を除く）。機構はプラグインによって登録される
3. **RFC-026 の安全性境界を維持**：すべての機構は型の二分割、マーシャリングの一時領域隔離、Move + RAII を遵守しなければならない
4. **セルフホスティング互換**：機構登録表は YaoXiang の `Dict`/`Set` に縮退する

---

## 提案

### 1. `FfiMechanism` 抽象

各 FFI 機構は四つの操作を実装する。これはコアが ABI をハードコードしないための鍵である——コンパイラはこのインターフェースを呼び出すだけで、背後が C、Wasm、その他のいずれかを知らない：

```rust
trait FfiMechanism {
    /// 機構タグ。例："c" / "wasm" / "python"
    fn tag(&self) -> &str;

    /// ライブラリのロード。C: dlopen/静的リンク；Wasm: モジュールのインスタンス化；Python: import。
    /// 機構内部のライブラリハンドルを返す。
    fn load_library(&self, id: &str) -> Result<LibraryHandle>;

    /// シンボルの解決。シンボルの存在を検証するためにコンパイル時にも呼び出し可能。
    /// C: dlsym/シンボル表の検索；Wasm: エクスポート表の検索。
    fn resolve(&self, lib: &LibraryHandle, symbol: &str) -> Result<SymbolHandle>;

    /// 呼び出し。YaoXiang シグネチャに従って引数をマーシャリングし、実行し、戻り値をマーシャリングする。
    /// RFC-026 §3 のマーシャリング規則（コンパイル時隔離）を遵守しなければならない。
    fn invoke(
        &self,
        sym: &SymbolHandle,
        args: &[RuntimeValue],
        sig: &Signature,
    ) -> Result<RuntimeValue>;
}
```

**重要**：`invoke` の実装は RFC-026 §3 を遵守しなければならない——引数はコンパイル時領域にコピーし、戻り値は memcpy し、借用限定は単一呼び出しに閉じる。機構は ABI の詳細を自由に選択できるが、**安全性境界を違反することはできない**。これはプラグインの義務である。

### 2. 機構タグがそのまま機構選択

```yaoxiang
// .c → C ABI 機構（RFC-026 内蔵の参考実装）
sqlite3 = Native.c("libsqlite3")
SqliteDb.open: (f: String) -> ?SqliteDb = sqlite3("sqlite3_open")

// .wasm → Wasm 機構（yx_wasm_ffi プラグインが登録）
wasm_mod = Native.wasm("mymodule.wasm")
process: (input: String) -> String = wasm_mod("process")

// .python → Python 機構（yx_python_ffi プラグインが登録）
np = Native.python("numpy")
```

`Native.c` / `Native.wasm` における `.c` / `.wasm` は**機構タグ**であり、登録済みの `FfiMechanism` のいずれかを選択する。コアは `.c` を参考実装として内蔵する；それ以外はプラグインが提供す