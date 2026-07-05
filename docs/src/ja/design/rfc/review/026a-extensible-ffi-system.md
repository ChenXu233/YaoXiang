```yaml
---
title: "RFC-026a: 拡張可能 FFI メカニズム体系"
status: "レビュー中"
issue: "#135"
author: "晨煦"
created: "2026-06-05"
updated: "2026-07-05"
group: "rfc-026"
---
```

# RFC-026a: 拡張可能 FFI メカニズム体系

> **親 RFC**: [RFC-026: FFI 中核メカニズム](../accepted/026-ffi-core-mechanism.md)
>
> 本 RFC は RFC-026 の拡張性部分を定義する——C ABI 以外の FFI メカニズム（Wasm、Python、カスタム ABI）をプラグインとして組み込む方法、および動的ロードモードについて。

## 概要

RFC-026 は FFI 中核メカニズムを定義し、`Native.c("lib")` は組み込み C ABI を用いる。本 RFC は ABI メカニズムをプラグイン可能な `FfiMechanism` として抽象化し、中核にいかなる具体 ABI もハードコードしない：

1. **`FfiMechanism` 抽象**：メカニズムが実装すべき四つの操作を定義（ライブラリのロード、シンボルの解決、マーシャリング、呼び出し）
2. **メカニズムタグがメカニズム選択を行う**：`Native.c` / `Native.wasm` / `Native.python` はそれぞれ登録済みのメカニズムを選択する
3. **コンパイル時メカニズム登録表**：メカニズムタグはコンパイル時に検証され、未登録タグはコンパイルエラーとなる
4. **静的ロード vs 動的ロード**：いずれのモードも RFC-026 の安全境界を維持する

## 動機

RFC-026 は C ABI（`Native.c`）のみを組み込む。しかし YaoXiang は将来以下の必要性に迫られる可能性がある：
- Wasm モジュールの呼び出し（`Native.wasm`）
- Python 拡張の埋め込み（`Native.python`）
- ユーザー定義 ABI（専用ハードウェア、RPC ブリッジ）

これらの ABI をコンパイラにハードコードするよりも、「ライブラリのロード方法、シンボル解決方法、マーシャリング方法、呼び出し方法」を trait として抽象化し、各メカニズムをプラグインとして実装する方が望ましい。中核は `FfiMechanism` のみを認識し、具体的な ABI は認識しない。

### 設計上の制約

1. **メカニズムタグのコンパイル時検証**：`Native.xxx(...)` 内の `xxx` は登録済みのメカニズムでなければならない。さもなくばコンパイルエラー
2. **メカニズムをハードコードしない**：コンパイラは（参考実装としての `.c` を除き）メカニズム一覧を組み込まず、プラグインが登録する
3. **RFC-026 の安全境界を維持**：あらゆるメカニズムが型の二分法、マーシャリング一時領域の隔離、Move + RAII を遵守しなければならない
4. **セルフホスティング互換**：メカニズム登録表は YaoXiang の `Dict`/`Set` に縮退する

---

## 提案

### 1. `FfiMechanism` 抽象

各 FFI メカニズムは四つの操作を実装する。これは中核が ABI をハードコードしないための鍵である——コンパイラはこのインターフェースのみを呼び出し、背後が C なのか Wasm なのかは関知しない：

```rust
trait FfiMechanism {
    /// メカニズムタグ。例："c" / "wasm" / "python"
    fn tag(&self) -> &str;

    /// ライブラリのロード。C: dlopen/静的リンク；Wasm: モジュールのインスタンス化；Python: import。
    /// メカニズム内部のライブラリハンドルを返す。
    fn load_library(&self, id: &str) -> Result<LibraryHandle>;

    /// シンボルの解決。コンパイル時にも呼び出され、シンボル存在の検証を行う。
    /// C: dlsym/シンボル表検索；Wasm: エクスポート表検索。
    fn resolve(&self, lib: &LibraryHandle, symbol: &str) -> Result<SymbolHandle>;

    /// 呼び出し。YaoXiang シグネチャに従い引数をマーシャリングし、実行し、戻り値をマーシャリングする。
    /// RFC-026 §3 のマーシャリング規則（仮領域隔離）を遵守しなければならない。
    fn invoke(
        &self,
        sym: &SymbolHandle,
        args: &[RuntimeValue],
        sig: &Signature,
    ) -> Result<RuntimeValue>;
}
```

**重要**：`invoke` の実装は RFC-026 §3 を遵守しなければならない——引数を仮領域へコピー、戻り値は memcpy、借用限定は単一呼び出し。メカニズムは自身の ABI 詳細を選択できるが、**安全境界を違反してはならない**。これはプラグインの義務である。

### 2. メカニズムタグがメカニズム選択を行う

```yaoxiang
// .c → C ABI メカニズム（RFC-026 内蔵の参考実装）
sqlite3 = Native.c("libsqlite3")
SqliteDb.open: (f: String) -> ?SqliteDb = sqlite3("sqlite3_open")

// .wasm → Wasm メカニズム（yx_wasm_ffi プラグインが登録）
wasm_mod = Native.wasm("mymodule.wasm")
process: (input: String) -> String = wasm_mod("process")

// .python → Python メカニズム（yx_python_ffi プラグインが登録）
np = Native.python("numpy")
```

`Native.c` / `Native.wasm` 中の `.c` / `.wasm` は**メカニズムタグ**であり、登録済みの `FfiMechanism` のいずれかを選択する。中核は参考実装として `.c` を内蔵する；その他はプラグインが提供する。

### 3. メカニズムの登録とコンパイル時検証

プラグインは `.so` を介してコンパイル時にメカニズム登録表へ、提供するメカニズムタグを宣言する：

```text
use yx_wasm_ffi
  → libyx_wasm_ffi.so をロード
  → yx_register_mechanism() を呼び出す
  → FfiMechanism { tag: "wasm", ... } を登録
  → メカニズム登録表に "wasm" を追加

// その後：
Native.wasm("mod.wasm")    // ✅ コンパイル成功、"wasm" は登録済み
Native.foo("x")            // ❌ コンパイルエラー: Unknown FFI mechanism 'foo'
                           //    Try: `use yx_foo_ffi`
```

コンパイル時メカニズム登録表は**メカニズムタグ**（文字列）+ 対応する `FfiMechanism` インスタンスのポインタのみを保持する。`Native.xxx(...)` のコンパイル時に表を引き、タグが存在しなければコンパイルエラーとなる。

### 4. 静的 vs 動的ロード

`load_library` の実装がロードタイミングを決定する。いずれのモードも RFC-026 の安全境界を維持する：

| モード | `load_library` の動作 | シンボル検証 | 種別 |
|------|-------------------|---------|------|
| **静的**（デフォルト、C ABI） | コンパイル時に `-llib`、ライブラリはシンボル表に入る | コンパイル時にシンボル表を読む | 完全実装 |
| **動的** | ランタイム初回呼び出し時に dlopen/インスタンス化 | 初回ロード時に検証、欠落時はフェイルファスト | 宣言は信用、ロード時に検証 |

```yaoxiang
// 静的：C ライブラリはコンパイル時にリンクされる
sqlite3 = Native.c("libsqlite3")           // コンパイル時に -lsqlite3

// 動的：ランタイムに発見されるプラグイン
plugin = Native.c.dynamic("./plugins/foo.so")   // ランタイムに dlopen
```

静的・動的を問わず、マーシャリングは RFC-026 §3 の仮領域隔離に従う。動的モードではシンボル欠落は**クリーンなランタイムエラー**（フェイルファスト）であり、クラッシュではない。

### 5. 完全な情報フロー

```
use yx_wasm_ffi                     ← "wasm" メカニズムを登録
       │
       ▼
wasm_mod = Native.wasm("mod.wasm")
  コンパイル時：メカニズム登録表で "wasm" の存在を確認 ✅
         → wasm メカニズムの load_library("mod.wasm") を呼び出す
         → Wasm モジュールをインスタンス化し、ライブラリハンドルを返す
       │
       ▼
process: (input: String) -> String = wasm_mod("process")
  コンパイル時：wasm メカニズムの resolve(lib, "process") を呼び出し、エクスポートの存在を検証 ✅
         → CallNative { mechanism: "wasm", lib, symbol: "process", sig } を生成
       │
       ▼  ランタイム
  CallNative を実行
  → メカニズムの invoke(sym, args, sig)
  → sig に従いマーシャリング（仮領域隔離）→ Wasm を実行 → 戻り値をマーシャリング
```

### 6. セルフホスティング後の縮退

Rust ホスト期の `FfiMechanism` trait とメカニズム登録表は、セルフホスティング後に YaoXiang の通常の構造体へ縮退する：

```yaoxiang
// セルフホスティング後、メカニズム登録表は Dict
let mechanisms: Dict(String, FfiMechanism) = {}
mechanisms["c"] = c_mechanism
mechanisms["wasm"] = wasm_mechanism

// FfiMechanism は YaoXiang におけるインターフェース（RFC-011a 動的ディスパッチ）
// Native.c("lib") → mechanisms["c"].load_library("lib")
```

Rust 期には trait object（`Box<dyn FfiMechanism>`）を用い、セルフホスティング後は YaoXiang のインターフェース（RFC-011a）を用いる。インターフェースは一貫している：ロード、解決、マーシャリング、呼び出し。

---

## トレードオフ

### 利点

1. **ABI のゼロハードコード**：中核は `FfiMechanism` のみを認識し、新規 ABI = 新規プラグイン
2. **統一された安全境界**：すべてのメカニズムに RFC-026 §3 のマーシャリング規則を強制
3. **コンパイル時メカニズム検証**：メカニズムタグが存在しなければコンパイル時にエラー、ランタイムまで発見が遅れない
4. **静的・動的の統一抽象**：`load_library` の実装詳細はメカニズム内に隠蔽される

### 欠点

1. **プラグイン作成の敷居の高さ**：`FfiMechanism` の実装には対象 ABI とマーシャリング契約の理解が必要
2. **メカニズムの義務は規約に依存**：マーシャリングの仮領域隔離はプラグインの遵守に委ねられ、中核はプラグイン実装を強制検証できない

---

## 実装戦略

### フェーズ 1a：メカニズム抽象 (v0.8)

- [ ] `FfiMechanism` trait を定義（load_library / resolve / invoke）
- [ ] RFC-026 の C ABI 実装を `CMechanism: FfiMechanism` としてリファクタリング
- [ ] コンパイル時メカニズム登録表を実装（タグ → メカニズムインスタンス）
- [ ] `Native.xxx` のコンパイル時メカニズム登録表引きによる検証

### フェーズ 1b：動的ロード + プラグイン (v0.9)

- [ ] `.so` プラグインロードを実装（`yx_register_mechanism`）
- [ ] 動的ライブラリロードモードを実装（`Native.c.dynamic`）
- [ ] 参考プラグイン：`yx_wasm_ffi`（Wasm メカニズム）

---

## 他の RFC との関係

- **RFC-026**（親）：FFI 中核メカニズム——`FfiMechanism` はそのマーシャリング規則と安全境界を遵守しなければならない
- **RFC-011a**：インターフェースと動的ディスパッチ——セルフホスティング後、`FfiMechanism` は YaoXiang のインターフェースに縮退する
- **RFC-014**：パッケージ管理システム——`.so` プラグインの発見とロードはパッケージマネージャーに依存する
- **RFC-021**（廃止済）：ライブラリ駆動 FFI 拡張——本 RFC はその `ffi.load_library` API をメカニズムプラグイン層に降格させる

---

## 設計決定の記録

| 決定 | 結論 | 理由 | 日付 |
|------|------|------|------|
| メカニズム抽象 | `FfiMechanism` trait、四操作 | 中核は ABI をハードコードせず、インターフェースのみを認識 | 2026-07-03 |
| メカニズムの義務 | プラグインは RFC-026 マーシャリング規則を遵守 | メカニズムが異なっても安全境界は破壊されない | 2026-07-03 |
| メカニズムタグの検証 | コンパイル時に登録表を参照 | 未登録メカニズムはコンパイル時にエラー | 2026-07-03 |
| 静的/動的 | `load_library` の実装が決定 | タイミングはメカニズムの詳細であり、安全境界は不変 | 2026-07-03 |
| セルフホスティング後の縮退 | trait → YaoXiang インターフェース（RFC-011a） | ホスト言語の過剰な抽象化を行わない | 2026-07-03 |

---

## ライフサイクルと帰属

| 状態 | 場所 | 説明 |
|------|------|------|
| **レビュー中** | `docs/design/rfc/review/` | コミュニティでの議論を公開 |
| **承認済** | `docs/design/rfc/accepted/` | 正式な設計文書 |