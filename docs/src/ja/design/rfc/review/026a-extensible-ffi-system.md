---
title: 'RFC-026a: 拡張可能 FFI メカニズム体系'
status: '審査中'
issue: '#135'
author: '晨煦'
created: '2026-06-05'
updated: '2026-07-05'
group: 'rfc-026'
---

# RFC-026a: 拡張可能 FFI メカニズム体系

> **親 RFC**: [RFC-026: FFI コアメカニズム](../accepted/026-ffi-core-mechanism.md)
>
> 本 RFC は RFC-026 の拡張性部分を定義する——C ABI 以外の FFI メカニズム（Wasm、Python、カスタム ABI）をプラグインとしてどのように接続するか、そして動的ローディングモードについて。

## 摘要

RFC-026 は FFI コアメカニズムを定義し、`Native.c("lib")` は組み込み C ABI を経由する。本 RFC は ABI メカニズムをプラグイン可能な
`FfiMechanism` として抽象化し、コアが特定の ABI をハードコードしないようにする：

1. **`FfiMechanism` 抽象**：メカニズムが実装すべき4つの操作を定義（ライブラリのロード、シンボルの解決、マラリング、呼び出し）
2. **メカニズムタグ即メカニズム選択**：`Native.c` / `Native.wasm` / `Native.python` がそれぞれ登録済みメカニズムを選択
3. **コンパイル時メカニズムレジストリ**：メカニズムタグはコンパイル時に検証され、未登録のタグはコンパイルエラーを発生させる
4. **静的 vs 動的ローディング**：両モードとも RFC-026 のセキュリティ境界を維持

## 動機

RFC-026 は C ABI（`Native.c`）のみを組み込んでいる。しかし YaoXiang は将来的に以下を必要とする可能性がある：

- Wasm モジュールの呼び出し（`Native.wasm`）
- Python 拡張の埋め込み（`Native.python`）
- ユーザー定義 ABI（専用ハードウェア、RPC ブリッジ）

これらの ABI をコンパイラにハードコードするのではなく、「どのようにライブラリをロードするか、シンボルを解決するか、マラリングするか、呼び出すか」を trait として抽象化し、各メカニズムをプラグインとして実装する。コアは `FfiMechanism` のみを認識し、具体的な ABI は認識しない。

### 設計制約

1. **メカニズムタグのコンパイル時検証**：`Native.xxx(...)` の `xxx` は登録済みメカニズムでなければならず、そうでなければコンパイルエラー
2. **メカニズムのハードコード禁止**：コンパイラはメカニズムリストを組み込まない（`.c` は参照実装として例外）、メカニズムはプラグインが登録する
3. **RFC-026 セキュリティ境界の維持**：どのメカニズムも型二分、マラリング一時領域の隔離、Move + RAII を遵守しなければならない
4. **ブートストラッピング互換**：メカニズムレジストリは YaoXiang の `Dict`/`Set` に退化可能

---

## 提案

### 1. `FfiMechanism` 抽象

各 FFI メカニズムは4つの操作を実装する。これがコアが ABI をハードコードしないための关键——コンパイラはこのインターフェースのみを呼び出し、背後にあるのが C、Wasm 还是他の何かを知らない：

```rust
trait FfiMechanism {
    /// メカニズムタグ、例："c" / "wasm" / "python"
    fn tag(&self) -> &str;

    /// ライブラリのロード。C: dlopen/静的リンク；Wasm: モジュールをインスタンス化；Python: import。
    /// メカニズム内部のライブラリハンドルを返す。
    fn load_library(&self, id: &str) -> Result<LibraryHandle>;

    /// シンボルの解決。コンパイル時に呼び出してシンボルの存在を検証できる。
    /// C: dlsym/シンボルテーブル検索；Wasm: エクスポートテーブル検索。
    fn resolve(&self, lib: &LibraryHandle, symbol: &str) -> Result<SymbolHandle>;

    /// 呼び出し。YaoXiang シグネチャに従って引数を马拉リング、実行、戻り値を马拉リングする。
    /// RFC-026 §3 の马拉リングルール（一時領域隔離）を遵守しなければならない。
    fn invoke(
        &self,
        sym: &SymbolHandle,
        args: &[RuntimeValue],
        sig: &Signature,
    ) -> Result<RuntimeValue>;
}
```

**要点**：`invoke` の実装は RFC-026
§3 を遵守しなければならない——入力パラメータを一時領域にコピー、戻り値を memcpy、借用は単一呼び出しに限定。メカニズムは自分の ABI 詳細を選択できるが、**セキュリティ境界を違反してはならない**。これはプラグインの義務である。

### 2. メカニズムタグ即メカニズム選択

```yaoxiang
// .c → C ABI メカニズム（RFC-026 組み込み参照実装）
sqlite3 = Native.c("libsqlite3")
SqliteDb.open: (f: String) -> ?SqliteDb = sqlite3("sqlite3_open")

// .wasm → Wasm メカニズム（yx_wasm_ffi プラグイン登録）
wasm_mod = Native.wasm("mymodule.wasm")
process: (input: String) -> String = wasm_mod("process")

// .python → Python メカニズム（yx_python_ffi プラグイン登録）
np = Native.python("numpy")
```

`Native.c` / `Native.wasm` の `.c` / `.wasm` は**メカニズムタグ**であり、どの登録済み `FfiMechanism` を使用するかを選択する。コアは `.c` を参照実装として組み込みで提供し、他はプラグインが提供한다。

### 3. メカニズム登録とコンパイル時検証

プラグインは `.so` を介してコンパイル時にメカニズムレジストリに提供するメカニズムタグを宣言する：

```text
use yx_wasm_ffi
  → libyx_wasm_ffi.so をロード
  → yx_register_mechanism() を呼び出す
  → FfiMechanism { tag: "wasm", ... } を登録
  → メカニズムレジストリに "wasm" を追加

// その後：
Native.wasm("mod.wasm")    // ✅ コンパイル成功、"wasm" は登録済み
Native.foo("x")            // ❌ コンパイルエラー: Unknown FFI mechanism 'foo'
                           //    Try: `use yx_foo_ffi`
```

コンパイル時メカニズムレジストリは**メカニズムタグのみを存储**（文字列）+ 対応する `FfiMechanism` インスタンスポインタ。`Native.xxx(...)` をコンパイルするときのテーブル参照で、タグが存在しなければコンパイルエラー。

### 4. 静的 vs 動的ローディング

`load_library` の実装がローディングタイミングを決定し、両モードとも RFC-026 のセキュリティ境界を維持する：

| モード              | `load_library` の動作              | シンボル検証                           | 型                     |
| ------------------- | ---------------------------------- | -------------------------------------- | ---------------------- |
| **静的**（デフォルト、C ABI） | コンパイル時 `-llib`、ライブラリはシンボルテーブルに含む     | コンパイル時にシンボルテーブルを読み取る             | 完全具象             |
| **動的**            | 初回呼び出し時に dlopen/インスタンス化       | 初回ロード時に検証、欠落時は即時失敗（fail-fast）   | 宣言は信頼、ロード時に検証 |

```yaoxiang
// 静的：C ライブラリはコンパイル時にリンクされる
sqlite3 = Native.c("libsqlite3")           // コンパイル時 -lsqlite3

// 動的：実行時に検出されるプラグイン
plugin = Native.c.dynamic("./plugins/foo.so")   // 実行時 dlopen
```

静的・動的に関係なく、マラリングは RFC-026
§3 の一時領域隔離を経由する。動的モードではシンボル欠落は**クリーンな実行時エラー**（fail-fast）であり、クラッシュではない。

### 5. 完全な情報フロー

```
use yx_wasm_ffi                     ← "wasm" メカニズムを登録
       │
       ▼
wasm_mod = Native.wasm("mod.wasm")
  コンパイル時：メカニズムレジストリの "wasm" が存在 ✅
         → wasm メカニズムの load_library("mod.wasm") を呼び出す
         → Wasm モジュールをインスタンス化、ライブラリハンドルを返す
       │
       ▼
process: (input: String) -> String = wasm_mod("process")
  コンパイル時：wasm メカニズムの resolve(lib, "process") を呼び出してエクスポートの存在を検証 ✅
         → CallNative { mechanism: "wasm", lib, symbol: "process", sig } を生成
       │
       ▼ 実行時
  CallNative を実行
  → メカニズムの invoke(sym, args, sig) を呼び出す
  → sig に従って马拉リング（一時領域隔離）→ Wasm を実行 → 戻り値を马拉リング
```

### 6. ブートストラッピング後の退化

Rust managed 期の `FfiMechanism` trait + メカニズムレジストリは、ブートストラッピング後に YaoXiang の обычная 構造体に退化する：

```yaoxiang
// ブートストラッピング後、メカニズムレジストリは Dict
let mechanisms: Dict(String, FfiMechanism) = {}
mechanisms["c"] = c_mechanism
mechanisms["wasm"] = wasm_mechanism

// FfiMechanism は YaoXiang におけるインターフェース（RFC-011a 動的ディスパッチ）
// Native.c("lib") → mechanisms["c"].load_library("lib")
```

Rust 期は trait object（`Box<dyn FfiMechanism>`）を使用し、ブートストラッピング後は YaoXiang インターフェース（RFC-011a）を使用する。インターフェースは一貫している：ロード、解決、マラリング、呼び出し。

---

## トレードオフ

### メリット

1. **ABI のゼロ・ハードコード**：コアは `FfiMechanism` のみを認識し、新しい ABI = 新しいプラグイン
2. **統一されたセキュリティ境界**：すべてのメカニズムは RFC-026 §3 马拉リングルールを強制的に遵守
3. **コンパイル時メカニズム検証**：メカニズムタグが存在しなければコンパイル時にエラーが発生し、実行時に才发现することがない
4. **静的・動的の統一抽象**：`load_library` の実装詳細はメカニズム内に隠蔽

### デメリット

1. **プラグイン作成の敷居**：実装 `FfiMechanism` は対象 ABI + 马拉リング契約を理解する必要がある
2. **メカニズム義務は約束に依存**：马拉リング一時領域隔離はプラグインの遵守に依存し、コアはプラグイン実装を強制検証できない

---

## 実装戦略

### フェーズ 1a: メカニズム抽象 (v0.8)

- [ ] `FfiMechanism` trait を定義（load_library / resolve / invoke）
- [ ] RFC-026 の C ABI 実装を `CMechanism: FfiMechanism` にリファクタリング
- [ ] コンパイル時メカニズムレジストリを実装（タグ → メカニズムインスタンス）
- [ ] `Native.xxx` がコンパイル時にメカニズムレジストリを検証

### フェーズ 1b: 動的ローディング + プラグイン (v0.9)

- [ ] `.so` プラグインロードを実装（`yx_register_mechanism`）
- [ ] 動的ライブラリロードモードを実装（`Native.c.dynamic`）
- [ ] 参照プラグイン：`yx_wasm_ffi`（Wasm メカニズム）

---

## 他の RFC との関係

- **RFC-026**（親）：FFI コアメカニズム——`FfiMechanism` はその马拉リングルールとセキュリティ境界を遵守しなければならない
- **RFC-011a**：インターフェースと動的ディスパッチ——ブートストラッピング後 `FfiMechanism` は YaoXiang インターフェースに退化
- **RFC-014**：パッケージ管理システム——`.so` プラグインの発見とロードはパッケージマネージャに依存
- **RFC-021**（已废弃）：ライブラリ driver FFI 拡張——本 RFC はその `ffi.load_library` API をメカニズムプラグイン層に下沉

---

## 設計決定記録

| 決定         | 決定内容                              | 理由                                     | 日付       |
| ------------ | ------------------------------------- | ---------------------------------------- | ---------- |
| メカニズム抽象     | `FfiMechanism` trait、4操作      | コアは ABI をハードコードせず、インターフェースのみを認識   | 2026-07-03 |
| メカニズム義務     | プラグインは RFC-026 马拉リングルールを遵守     | セキュリティ境界はメカニズム的不同で破壊されない   | 2026-07-03 |
| メカニズムタグ検証 | コンパイル時にレジストリを查询            | 未登録メカニズムはコンパイル時にエラー     | 2026-07-03 |
| 静的/動的    | `load_library` の実装が決定           | タイミングはメカニズムの詳細であり、セキュリティ境界は変わらない | 2026-07-03 |
| ブートストラッピング退化     | trait → YaoXiang インターフェース（RFC-011a） | ホスト言語の過度な抽象を避ける         | 2026-07-03 |

---

## ライフサイクルと行き先

| 状態       | 場所                           | 説明         |
| ---------- | ------------------------------ | ------------ |
| **審査中** | `docs/design/rfc/review/`      | コミュニティDiscussion を再開 |
| **已接受** | `docs/design/rfc/accepted/`   | 正式設計ドキュメント     |
