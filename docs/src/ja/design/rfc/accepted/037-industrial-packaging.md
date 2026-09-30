---
title: 'RFC-037: 工業的配布方案 — cargo-dist に基づくコンパイラ/ツールチェーンのパッケージング'
author: 'ChenXu233'
created: '2026-07-26'
updated: '2026-09-10'
accepted: '2026-09-09'
issue: '#230'
status: '受領'
---

# RFC-037: 工業的配布方案 — cargo-dist に基づくコンパイラ/ツールチェーンのパッケージング

> 本 RFC は [RFC-014b: ビルドシステムとバイナリ配布](../accepted/014b-build-system.md)
> を補完する。RFC-014b は
> **YaoXiang パッケージマネージャ**が第三者パッケージをどのようにビルド・配布するかを定義する一方、本 RFC は
> **YaoXiang コンパイラ/ツールチェーン自体**をどのようにパッケージング・配布するかを定義する。

## 要旨

`cargo-dist`（Rust エコシステムのバイナリ配布ツール）にクロスプラットフォームのビルドオーケストレーションを担当させ、自前のスクリプトが配布パッケージの構造を担う。中核となる約束は2つ：配布パッケージは**物理的に標準ライブラリのソースコードディレクトリを携带**し（ユーザは Python の
`Lib/`
を読むように直接読める）、全プラットフォーム動的リンクの Z3 共有ライブラリがパッケージに同梱される。コマンドモデルは**フロントドア/エンジン分離**：常用コマンドは
`yx`（小型フロントドア、rustup/Go GOTOOLCHAIN 相当のバージョン管理を内蔵）、エンジンは
`yaoxiang-rs`（現 `yaoxiang`
モノリスの改名）。Python/Node が事後的に nvm/pdm で補講する生態分裂を避ける。インストール方式は二層：標準チャネルは Go/Zig 方式——**配布パッケージ即プロダクト**、展開 +
PATH；お手軽チャネルは一行コマンドインストール（Linux `apt` / `curl | sh`、Windows `irm | iex` /
Inno exe ウィザード）。`libz3.dll`
欠如、標準ライブラリのユーザ不可視、CI スクリプトの重複保守などの問題を解決する。

## 動機

### なぜこの機能が必要か？

YaoXiang をダウンロードするユーザは、**追加手順なしで即座に使い始められる**べきである。標準ライブラリは**ユーザが直接読める**べきであり、バイナリ内のブラックボックスに隠されているべきではない。

### 現状の問題

#### 問題 1：Windows ユーザがダウンロード後に実行できない

現状の Release では `yaoxiang.exe` しかアップロードされておらず、`libz3.dll`
がパッケージに含まれていない。Windows 上でユーザがダブルクリックすると以下のエラーが出る：

```
The code execution cannot proceed because libz3.dll was not found.
```

これは**中断バグ**であり、ユーザは最初の一歩さえ踏み出せない。

#### 問題 2：Release アーティファクトが単一 exe のみで、標準ライブラリがユーザ不可視

現状は三重の断絶である：

- Release アーティファクトは生バイナリのみで、標準ライブラリが配布物に同梱されない
- LSP のインターフェースファイル検索チェーンがほぼ切断されている：`find_std_interface_file`
  呼び出し時にプロジェクトディレクトリを伝えない（グローバルな `~/.yaoxiang/std/`
  のみ検索し、それを埋めるフローが存在しない）；`package init` が書き込むのは `.yaoxiang/std`
  であり、検索チェーンにない
- 標準ライブラリのソースコード（`.yx`
  レイヤ）とインターフェースビュー（native レイヤ）がユーザにとって完全にブラックボックス

工業的なアプローチ：ユーザは Python の `Lib/`
を読むように標準ライブラリディレクトリを直接開いてソースを読める——**配布パッケージが物理的に std ディレクトリを携带することは本方案の必須要件**（既決）。

#### 問題 3：CI の手書きスクリプトが重複保守されている

現在、複数のビルドパイプラインを維持している：

| ファイル                  | 役割                         | 行数        |
| ------------------------- | ---------------------------- | ----------- |
| `_build-platforms.yml`    | クロスプラットフォームビルド | ~255 行     |
| `release.yml`             | バージョンリリース           | ~189 行     |
| `nightly.yml`             | デイリービルド               | ~173 行     |
| `scripts/build/setup.iss` | Inno Setup インストーラ      | ~250 行     |
| **合計**                  |                              | **~870 行** |

大半は重複しており（Rust のインストール → キャッシュ → ビルド → リネーム → アップロード）、各プラットフォームごとに一度ずつ書く必要がある。

#### 問題 4：Inno Setup のバージョン番号がハードコード

`setup.iss` の `MyAppVersion` に `0.7.0` がハードコードされており、ビルド時に `sed`
で置換している。遅かれ早かれ失敗する。

#### 問題 5：RFC-014b との境界が曖昧

RFC-014b は「YaoXiang パッケージのビルドと配布機構」（即ち `yaoxiang.toml` の `[build]` と
`[binaries]`
設定）を定義しているが、**「YaoXiang コンパイラ自体をどうリリースするか」はカバーしていない**。本 RFC はこの空白を埋める。

## 提案

### 中核設計

cargo-dist は**ビルドオーケストレーション層**のみを担い、パッケージ構造とインストーラはすべて自前。責任分担：

```
cargo-dist の責任（ビルドオーケストレーション層）:
  ├── クロスプラットフォームコンパイル（5 つの target）
  └── 圧縮パッケージとチェックサムの生成
  （ネイティブインストーラと npm wrapper は廃止——そのフラットバイナリ仮定は bin/+lib/ 構造と衝突する）

build.rs は引き続き担当:
  └── Z3 ダウンロード/リンク（全プラットフォーム動的 + rpath）

YaoXiang 自前スクリプト:
  ├── package-dist.sh — パッケージ構造の組み換え（bin/ + lib/）、共有ライブラリの同梱、
  │   std ディレクトリの充填（リポジトリで事前生成されたインターフェースビュー + .yx レイヤソース）、チェックサム再計算
  └── Inno Setup — Windows インストールウィザード（既存資産；完全なディレクトリ構造を展開）

コマンドモデル（フロントドア/エンジン分離）:
  ├── yx — フロントドア（新小型 crate）：バージョン解析 + ディスパッチ；toolchain/self 動詞のみ保持、他は透過転送
  └── yaoxiang-rs — エンジン（現 yaoxiang モノリスの改名）：コンパイル/実行/パッケージ管理/fmt/lsp サブコマンド

インストール方式（二層）:
  ├── 標準チャネル（Go/Zig 方式）: 配布パッケージ即プロダクト、展開 + PATH
  └── お手軽チャネル（Rust 方式）: 一行コマンドインストール + バージョン管理（yx フロントドア内蔵）
      ├── Linux: apt（自前 deb リポジトリ、システムレベルフラットインストール）/ curl … | sh（一括インストール）
      ├── Windows: irm … | iex（一括インストール）/ Inno Setup ウィザード（既存資産、システムレベルフラットインストール）
      └── macOS: curl … | sh（brew は homebrew-core コミュニティに委任）
```

### 配布ディレクトリ構造（既決：標準ライブラリソースを物理的に携带）

ユーザは Python の `Lib/`
を読むように標準ライブラリを直接読めなければならない——配布パッケージが std ディレクトリを自带することは必須要件であり、パッケージングの詳細ではない。Z3 も同様：外部システムとして、ディレクトリ式の共有ライブラリ配布は自然な形態である。`.so`
を exe に埋め込むことと外部に置いて動的リンクすることは「どちらもパッケージに同梱する必要がある」点では同等で、後者には交換可能性が保たれる。

各プラットフォームの配布パッケージは、`package-dist.sh` が cargo-dist ビルド後に再構成する：

```
yaoxiang-{version}-{target}.tar.gz / .zip     （ポータブル即利用：展開後 bin/ 内直接実行）
├── bin/
│   ├── yx                            # フロントドア（または yx.exe）
│   ├── yaoxiang-rs                   # エンジン（または yaoxiang-rs.exe）
│   └── libz3.so / libz3.dylib / libz3.dll
├── lib/
│   └── yaoxiang/
│       └── std/                      # ユーザが直接読める（Python Lib/ 方式）
│           ├── io.yx                 # native モジュール：リポジトリで事前生成されたインターフェースビュー
│           ├── math.yx
│           ├── test.yx               # .yx レイヤ：リポジトリ内の実ソースをそのままコピー
│           └── ...
├── README.md
└── LICENSE
```

インストール = 任意のディレクトリに展開 + `bin/` を PATH に追加（Go の `/usr/local/go/bin`
方式；`~/.yaoxiang/` への展開が一般的選択）。Windows では Inno
Setup ウィザードが同じことを行う（デフォルト Program Files）。ポータブル展開時、`yx` フロントドアは
`~/.yaoxiang` 状態を持たず、隣接する `yaoxiang-rs`
にフォールバックする——マネージドインストールと同じ挙動。エンジンの rpath と exe 相対 std 検索はフロントドアの存在に影響されない。

### プラットフォームサポート

| プラットフォーム | target triple               | 説明                    |
| ---------------- | --------------------------- | ----------------------- |
| Linux x86_64     | `x86_64-unknown-linux-gnu`  | メインプラットフォーム  |
| Linux ARM64      | `aarch64-unknown-linux-gnu` | CI 上でクロスコンパイル |
| macOS x86_64     | `x86_64-apple-darwin`       | Intel Mac               |
| macOS ARM64      | `aarch64-apple-darwin`      | Apple Silicon           |
| Windows x86_64   | `x86_64-pc-windows-msvc`    | メインプラットフォーム  |

計 5 つの target。Windows
ARM64 は当面サポートしない（Z3 公式にプレコンパイル ARM64 パッケージがないため）。

### Z3 配布戦略

**全プラットフォーム動的リンク**（再確認の上維持）：

| プラットフォーム | 変更                   | 成果物        |
| ---------------- | ---------------------- | ------------- |
| Linux            | **元静的→動的に変更**  | `libz3.so`    |
| macOS            | **元静的→動的に変更**  | `libz3.dylib` |
| Windows          | 変更なし               | `libz3.dll`   |
| wasm32           | 変更なし（静的リンク） | 内蔵 `.a`     |

理由：

- **一貫性** — 3 プラットフォームの挙動が統一され、特例がなくなる
- **これは外部ライブラリであり、共有ライブラリで配布すべき**。Python（`python3.dll`+`DLLs/lib*.dll`）、Node（`node`+`lib/`）もそうしている
- **ユーザの Z3 アップグレードはコンパイラのリリースを待たなくてよい** — `.so`/`.dylib`/`.dll`
  を差し替えるだけ
- **バイナリサイズが小さい** — Z3 は小さくないため、静的リンクは exe を数 MB 膨らませる

動的リンクには**必要な付帯事項**が一つある：Linux/macOS の動的リンカはデフォルトでバイナリのディレクトリを検索しないため、rpath を注入しなければ「展開即利用」が成立しない（Windows はデフォルトで exe ディレクトリを検索するため対処不要）。対応する
`build.rs` 修正：

```rust
// 統一動的リンク + rpath
fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    // Z3 配布パッケージのレイアウトは統一されていないため、lib/bin 両ディレクトリを探索
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // RFC-037：全プラットフォーム動的リンク。共有ライブラリは bin/ に同梱して配布、ユーザは Z3 を全体交換・アップグレード可能
    if target_os == "windows" {
        // MSVC import lib の名前は libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // 動的リンカはデフォルトでバイナリディレクトリを検索しないため、rpath を注入しなければ「展開即利用」が成立しない
        // （配布パッケージ内 exe と libz3 は同じ bin/ に配置；Windows はデフォルトで exe ディレクトリを検索するため対処不要）
        match target_os.as_str() {
            "linux" => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
            "macos" => println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path"),
            _ => {}
        }
        let cxx = if target_os == "macos" {
            "c++".to_string()
        } else {
            env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into())
        };
        println!("cargo:rustc-link-lib={}", cxx);
    }
}
```

**「全プラットフォーム静的リンク」は目標としない。**これは特例の除去ではなく、合理的な状況を間違った方法で除去するものである。共有ライブラリは外部ライブラリの通常の配布方式である。

### インストーラサポート

主要言語ツールチェーンの配布方式との比較（2026-09 時点の調査）：

| 言語     | 公式配布物                   | 公式インストール方式                             | インストーラ保守者                 |
| -------- | ---------------------------- | ------------------------------------------------ | ---------------------------------- |
| Go       | `go/{bin,src,pkg}` tarball   | 公式ドキュメント =「ダウンロード → 展開 → PATH」 | なし（brew/apt はコミュニティ）    |
| Zig      | `zig/{bin,lib/std}` tarball  | 同上、公式インストールスクリプトなし             | なし（homebrew-core コミュニティ） |
| Node     | `{bin,lib,include}` tarball  | tar + 公式 pkg/msi                               | チームが自前                       |
| Rust     | マルチコンポーネント tarball | rustup                                           | チームが自前                       |
| Crystal  | `{bin,src,embedded}` tarball | deb/rpm/tar                                      | チーム + brew コミュニティ         |
| Deno/Bun | 単一バイナリ zip             | 公式 curl スクリプト                             | チームが自前（スクリプト極小）     |
| Gleam    | cargo-dist 単一バイナリ      | cargo-dist 生成スクリプト                        | cargo-dist                         |

3 つの法則：

- **複数ファイルツールチェーンでサードパーティ製インストーラジェネレータを使う例はない**——cargo-dist のインストーラは単一バイナリシナリオにのみ対応する（Gleam が利用可能なのは外部依存なしの単一 Rust バイナリであるため）
- 最も簡素なモデルは **Go/Zig の「配布パッケージ即プロダクト」**：公式インストールガイダンスは展開 +
  PATH のみ、インストーラコードゼロ；配布パッケージに可読 std ソースが同梱される（Go の
  `src/`、Zig の `lib/std/`、Crystal の `src/`）のは常態
- curl 一発インストールを望む（Deno/Bun/rustup）はいずれも**自前スクリプト**で、ほぼ進化しない；brew フォーミュラはすべて homebrew-core にコミュニティ保守されており、言語チームは自前 tap を作らない（Crystal チームは formula はコミュニティと明言）

YaoXiang は二層モデルを採用する：

| チャネル                                                | レイヤ | 状態 | 説明                                                                                                                                      |
| ------------------------------------------------------- | ------ | ---- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| zip / tar.gz                                            | 標準   | ✅   | 展開即利用（rpath + 同ディレクトリ共有ライブラリ）、extract + PATH が公式ガイダンス                                                       |
| `yx`（フロントドア、バージョン管理内蔵）                | お手軽 | ✅   | rustup/Go GOTOOLCHAIN 相当：複数バージョンインストール/切替/更新 + プロジェクト pin                                                       |
| `curl ... \| sh`（install.sh）                          | お手軽 | ✅   | Linux / macOS：ダウンロード・再構成パッケージを `versions/` に展開、`bin/yx` をインストールルートに配置し、デフォルトバージョンを書き込み |
| `irm ... \| iex`（install.ps1）                         | お手軽 | ✅   | Windows：同ロジック                                                                                                                       |
| `apt install yaoxiang`                                  | お手軽 | ✅   | `.deb`（amd64/arm64）+ GitHub Pages 静的 apt リポジトリ；システムレベルフラットインストール、`apt upgrade` でバージョン追従               |
| Inno Setup exe                                          | お手軽 | ✅   | Windows ウィザード（既存資産）、システムレベルフラットインストール、完全な bin/+lib/ 構造を展開                                           |
| winget / `.rpm` / homebrew-core / npm                   | —      | ⏸    | オプションフォローアップ：winget と brew-core はいずれもコミュニティ保守、rpm は deb と同形                                               |
| MSI / cargo-dist ネイティブインストーラ / 自前 brew tap | —      | ❌   | 「代替案」参照                                                                                                                            |

**お手軽チャネルは Rust を参照、バージョン管理はフロントドアに内蔵。**Rust の分解は「ブートストラップスクリプト（sh.rustup.rs）→
rustup
→ ツールチェーンパッケージ」であり、rustup は最初から複数バージョン、切替、更新を管理する。Python/Node の公式インストーラはこの層を作らず、生態系が事後的に pyenv/nvm/pdm を別個に生み出した。YaoXiang は**フロントドア/エンジン分離**を採用する（Go の
`go` フロントドア + GOTOOLCHAIN、rustup のプロキシディスパッチ、両者の同形構造）：常用コマンドは
`yx`、エンジンは `yaoxiang-rs`——

```
~/.yaoxiang/
├── bin/yx                  # フロントドア：小型バイナリ、バージョン解析 + ディスパッチ（プロジェクト pin > デフォルト > 隣接エンジン）
├── settings.toml           # デフォルトバージョン、ミラーソース
└── versions/               # バージョンは第一級の概念；<ver>/ がそのバージョンの配布パッケージの展開ルートディレクトリ
    └── 0.7.14/
        ├── bin/
        │   ├── yaoxiang-rs      # エンジン：コンパイル/実行/パッケージ管理/fmt/lsp サブコマンド
        │   └── libz3.so
        └── lib/yaoxiang/std/
```

- インストールルート `~/.yaoxiang/` は業界慣行に揃える（pyenv `~/.pyenv`、nvm `~/.nvm`、deno
  `~/.deno`、bun `~/.bun`、volta `~/.volta` いずれも単一ルート；rustup の `~/.cargo`+`~/.rustup`
  二重ルートは cargo が rustup より先に存在した歴史的負債であり、模倣しない）、かつ `~/.yaoxiang`
  は既にコードベースの既存名前空間（std グローバルフォールバックスロット）；`YAOXIANG_HOME`
  環境変数での上書きをサポート（先例 RUSTUP_HOME /
  DENO_INSTALL、CI とコンテナシナリオに対応）；Windows では `%USERPROFILE%\.yaoxiang`
- **バージョンディレクトリ = 配布パッケージ展開ルートディレクトリ**：`versions/<ver>/`
  はポータブル展開、deb インストールツリーと完全同形で、1 バージョンのインストールは 1 配布パッケージの展開——3 チャネル間で構造的分岐なし
- コマンド面（rustup 相当）：`yx toolchain install / default / update / list / uninstall`、`yx self update`
  を含む；その他の動詞はそのままエンジンに透過転送
- **バージョンロックは構造的保証**：fmt などのツールは構文と同期して進化する（古い fmt は新構文を認識しない）、バージョン解析はフロントドアで一度完了し、グループ全体で切替——「新エンジンに旧 fmt」の組合せ空間は存在しない；将来 fmt/LSP を独立バイナリに分割する場合も同じバージョンの
  `bin/` 内に配置
- プロジェクトレベル pin：`yx-toolchain.toml`（先例 rust-toolchain.toml、コマンド名に追随；`yaoxiang.toml`
  には入れない——パッケージマニフェストはツールチェーンバージョンをライブラリの使用者に強制すべきでない）
- ブートストラップエントリ（`curl | sh` /
  `irm | iex`）は最新 stable パッケージを一括インストール：展開で `versions/` に配置、`bin/yx`
  をインストールルートに配置、`settings.toml`
  にデフォルトバージョンを書き込み（rustup 「ブートストラップでマネージャをインストール」分解の等価収斂——フロントドアとエンジンが同包で、二段階不要）
- ミラーソース設定可能（settings.toml）、中国国内ユーザの考慮を継続（Z3 ダウンロードと同様のネットワーク問題）
- **バージョン管理は自己完結不変量を破壊しない**：各バージョンは完全な配布ツリーであり、ツリー内で rpath と exe 相対 std 検索が自己完結し、フロントドアはディスパッチのみを行い構造を変更しない

`.deb` と Inno は**システムレベルフラットインストール**チャネル（root / Program
Files 単一バージョン、`apt upgrade`
/ コントロールパネルで更新）、サーバ、CI、純粋初心者向けシナリオに対応；マネージャとの共存は PATH 順序に依存（先例：apt の rustc と rustup は並存）。全チャネルで同一の成果物ツリーを共有する。

`.deb`
レイアウトは同じディレクトリツリーを再利用：`/usr/lib/yaoxiang/`（配布ツリー全体：`bin/{yx,yaoxiang-rs,libz3.so}` +
`lib/yaoxiang/std/`）+ `/usr/bin/yx` シンボリックリンクは `/usr/lib/yaoxiang/bin/yx`
を指す——`$ORIGIN` は解決後の**実際のパス**で計算され、リンク後も同ディレクトリの `libz3.so`
をヒットし、展開パッケージ構造と同形。ブランド全名はパッケージ名と製品名に残し（`apt install yaoxiang`、Inno 製品名 YaoXiang）、コマンド面は統一
`yx`——Go と同じ：パッケージ名 `golang-go`、コマンド `go`。apt リポジトリは GitHub
Pages に静的ホスティング（Packages/Release/InRelease メタデータは GPG 署名、release
CI で公開）；将来的に Debian/Ubuntu 公式収録を申請可能（周期が長く、バージョン遅延、主経路ではない）。

### 標準ライブラリディレクトリ

`lib/yaoxiang/std/`
の内容はすべてリポジトリの静的ファイルから取得され、パッケージングは**純粋コピー**であり、ランタイム生成エントリは設けない：

| レイヤ                               | 出所                                                                        | 性質                                           |
| ------------------------------------ | --------------------------------------------------------------------------- | ---------------------------------------------- |
| native モジュール（io/math/…）       | リポジトリ `src/std/interfaces/*.yx` で事前生成されたインターフェースビュー | インターフェース署名ビュー（実装はバイナリ内） |
| .yx レイヤモジュール（test/…増加中） | リポジトリ `src/std/*.yx` をそのままコピー                                  | 実ソース                                       |

事前生成ビューは `StdModule::exports()` から派生（`src/std/gen_interfaces.rs` の
`generate_all_interfaces()`）、**ランタイム生成サブコマンドは設けない**（2026-09-10 既決：パッケージング成形後、サブコマンドは余分なインターフェース面）。同期は「生成物をリポジトリに登録 + テストゲート」方式を採用（RFC-013 コード表と同じ）：`test_committed_interface_files_match_generation`
が事前生成ファイルと現在の生成をバイト単位で比較し、ズレがあれば赤；修復は bless エントリ
`cargo test update_committed_interface_files -- --ignored`。生成ロジックは crate 内部の `StdModule`
実装に依存し、build.rs に下ろせないため、ゲートはビルド時ではなくテスト時に行う。

**ランタイム検索チェーン**——`find_std_interface_file` に exe 相対検索を追加：

1. プロジェクト `.yaoxiang/vendor/std/<name>.yx`（プロジェクトオーバーライド、現状）
2. **exe のディレクトリ
   `../lib/yaoxiang/std/<name>.yx`（新規）**：ポータブル展開、マネージドインストール（`versions/<ver>/`）、deb フラットインストールで統一ヒット
3. `~/.yaoxiang/std/<name>.yx`（グローバルフォールバック、手動オーバーライド用として保持）

現状このチェーンはほぼ切断されている（LSP 呼び出しがプロジェクトディレクトリを伝えない、`package init`
が書き込む `.yaoxiang/std` がチェーンにない）、今回はついでに修正し、`package init` の出力を
`.yaoxiang/vendor/std` に統一（パッケージマネージャの vendor ディレクトリと一致）。

**コンパイル権威は変更なし**：`.yx` レイヤは引き続き `include_str!`
で内蔵（RFC-036 の「std バージョンとバイナリの厳格なバインド」不変量を保持）。配布ディレクトリの位置づけは**可読ビュー +
LSP 解析ソース**であり、コンパイル入力ではない——配布ディレクトリ内の `.yx`
を手動で変更してもコンパイラに採用されない（「`Lib/`
変更で即時反映」セマンティクスの開放については、開放問題を参照）。

### Wasm ビルド

**独立を維持、cargo-dist には移行しない。**

cargo-dist が管轄するのは「コンパイラをユーザに配布する」ことであり、wasm は「オンライン playground をドキュメントサイトに埋め込む」——完全に異なる二つの成果物。

| 側面           | やり方                                   |
| -------------- | ---------------------------------------- |
| ビルドツール   | `wasm-pack build` を維持                 |
| CI workflow    | `_build-wasm.yml` の独立 job を保持      |
| 起動タイミング | リリースと同じ tag push で並列の独立 job |
| 公開先         | `docs/public/wasm/` → GitHub Pages       |

### npm 公開

| パッケージ             | 内容                                 | 状態                                                                                                                                                                                                                         |
| ---------------------- | ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@yaoxiang/cli`        | 配布パッケージダウンロードのラッパー | 延期：cargo-dist の npm ラッパーも同様にフラットアーティファクト仮定に基づくため、インストーラと共に廃止；npm チャネルが必要な場合は自前ラッパーを書く（再構成パッケージをダウンロード・展開、展開インストールと同ロジック） |
| `@yaoxiang/playground` | wasm ライブラリ（JS + .wasm）        | オプション、現状は docs のみに公開                                                                                                                                                                                           |

両者は競合せず、名前も競合しない。

### 既存リリースフローとの統合

現状の `release.yml`：push main → check-version（`v{version}` tag が存在しない場合のみ通過）→ build
/ build-wasm / security / test の 4 経路 → release job（tag プッシュ + `generate-commit-list.ts`
で @mentions を含む body を生成 + アーティファクトをアップロード）。

cargo-dist が生成するパイプラインは tag 駆動で announce/publish を内蔵し、fmt/clippy/test/audit ゲートを含まず、リリースノート形式も merge
commit changelog を承载できない。**直接全体置換すると既存リリース式典を破壊する**（PR → CI 全部緑 →
bump → merge commit = changelog）。

統合原則：**トリガーとゲートは現状維持、ビルドは `cargo dist build` に委譲、公開は現状維持。**

1. check-version / security / test と tag 打ちを `dist-release.yml` の `gate` / `security` / `test`
   / `tag` job に統合、トリガーは push main を維持（**tag 駆動は不可**：workflow が `GITHUB_TOKEN`
   でプッシュした tag は他の workflow をトリガーしない；旧 `release.yml`
   はそのためフラットバイナリを自前で公開するしかなく、既に `_build-platforms.yml`
   と共に削除済み、本ファイルがリリース単一点）
2. tag 生成後にのみビルド：`plan` job が dist でランナー/システム依存マトリクスを計算 →
   `cargo dist build`（5 target）→ `package-dist.sh` で target ごとに再構成 → Inno Setup
   job（Windows 再構成パッケージをウィザードビルドに投入、`/DMyAppVersion=`
   でバージョン注入、二次コンパイル不要）→ `_build-wasm.yml`（並列 job）
3. publish job：`generate-commit-list.ts` で body を生成（既存スクリプト再利用）→ 再構成パッケージ +
   `.sha256` + `.deb` + wasm + Setup exe をアップロード、`action-gh-release` で自前 Release；独立
   `publish-apt` job で GitHub Pages apt リポジトリメタデータ公開（`secrets.APT_GPG_KEY`
   未設定時は自動スキップ、他チャネルに影響なし）
4. 再発行/再ビルド：`workflow_dispatch` で既存 tag を指定（`gate`
   がそれに基づき tag 打ちステップをスキップ）

### Nightly リリース

cargo-dist はネイティブ nightly サポートがない（[axodotdev#1143](https://github.com/axodotdev/cargo-dist/issues/1143)、open
feature request のまま）。

既存の cron + tag 上書き方式を維持し、ビルド部分を `_build-platforms.yml` から `cargo dist build`
に変更——これは本質的に cargo コマンドなので、nightly.yml で直接呼び出せる。workflow 再利用は行わない（想定される
`uses: ./release.yml` は不可：被利用側に `workflow_call` トリガーが必要、かつ cargo-dist
workflow は tag 駆動でビルドと公開が結合）：

```yaml
# nightly.yml（移行後）
on:
  schedule:
    - cron: '17 22 * * *'
jobs:
  build: # cargo dist build + package-dist.sh（正式版と同一セット）
  publish: # 現状維持：nightly tag を打つ/移動 → GitHub Pre-release を上書き
```

### cargo-dist 設定（dist-workspace.toml に反映済み）

```toml
[workspace]
members = ["cargo:.", "cargo:tools/yx"]

# Config for 'dist'
[dist]
# dist バージョンをロック（Cargo.toml SemVer 構文）
cargo-dist-version = "0.32.0"
ci = "github"
# インストーラはすべて自前、cargo-dist はビルド + 圧縮パッケージ + チェックサムのみ
installers = []
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
# 生成された workflow は意図的に改造されている（package-dist.sh 再構成 + 自前公開段、RFC-037）、
# generate テンプレートからのズレによりビルドが拒否されない
allow-dirty = ["ci"]
```

上記はリポジトリ vendor の実設定（`cargo dist init`
で生成後、必要に応じて修正）；`cargo-dist-version`
を 0.32.0 にロック、生成された workflow を vendor してリポジトリに格納し review を受け、ランタイムには取得しない。ビルド profile は init がルート Cargo.toml の
`[profile.dist]` に注入（inherits release、lto=thin）、2 バイナリは `target/<triple>/dist/`
に配置されて再構成スクリプトが利用する。

### package-dist.sh（反映済み）

リポジトリ `scripts/release/package-dist.sh` を基準とし、要点：

- 2 バイナリは `cargo dist build` のビルド出力ディレクトリ
  `target/<triple>/dist/`（profile=dist）から直接取得——cargo-dist 自身のフラット単一バイナリアーカイブは成果物ではなく、同名再構成パッケージを
  `target/distrib/` に直接上書き
- Z3 共有ライブラリもビルド出力ディレクトリから取得——build.rs のリンク時に該当プラットフォームの共有ライブラリが選択・コピー済み（`copy_shared_lib`）、**単一ソース**：パッケージングスクリプトは Z3 バージョンとプラットフォームディレクトリ名を知る必要がない；Z3 ライセンス文書はライブラリ配置時に同梱され、パッケージングに持ち込まれる（MIT 配布義務）、macOS 側のパッケージング時に dylib の install_name を
  `@rpath` に統一し、バイナリに ad-hoc 再署名
- std ディレクトリは純粋コピー：`src/std/interfaces/*.yx`（事前生成インターフェースビュー）+
  `src/std/*.yx`（.yx レイヤ実ソース）
- README/LICENSE を添付；再パッケージング（Windows zip / その他 tar.gz；Git
  Bash に zip がない場合は zip → System32 bsdtar → PowerShell の三段フォールバック）し `.sha256`
  を再計算
- Linux かつ `dpkg-deb` 利用可能時は `build-deb.sh` を呼び出して `.deb` を生成（`/usr/lib/yaoxiang`
  フラットインストールツリー + `/usr/bin/yx` シンボリックリンク）

### 廃止する手書き CI

移行完了後に調整されるファイル：

| ファイル                                 | 行数        | 処理                                                                |
| ---------------------------------------- | ----------- | ------------------------------------------------------------------- |
| `.github/workflows/_build-platforms.yml` | 254         | 削除（cargo-dist ビルドマトリクスで代替）                           |
| `.github/workflows/release.yml`          | 189         | 削除（ゲートと tag 打ちは dist-release.yml に統合、リリース単一点） |
| `.github/workflows/nightly.yml`          | 173         | ビルド段を `cargo dist build` に変更、公開ロジックは保持            |
| `scripts/build/setup.iss`                | ~250        | **保持して正式採用**（Windows ウィザード）                          |
| **合計削減**                             | **~600 行** |                                                                     |

保持：

- `ci.yml`（日常 fmt + clippy + test + MSRV、公開フローには属さない）
- `_build-wasm.yml`（独立ビルドフロー、dist-release.yml の並列 job に組み込む）
- `_build-z3-wasm.yml`（wasm 専用 Z3）
- `docs-deploy.yml`（ドキュメントデプロイ）

### 受け入れ基準

「即利用」はテスト可能であり、移行完了の判定は「新旧成果物一致」ではなく、以下すべて通過すること：

- クリーンなマシン（Rust / Z3 / `~/.yaoxiang`
  なし）で任意プラットフォームの圧縮パッケージを展開し、`bin/yaoxiang-rs --version`
  を直接実行成功——`LD_LIBRARY_PATH` 設定不要（rpath 生效）
- 展開ディレクトリ内 `lib/yaoxiang/std/*.yx`
  がすべて可読：native モジュールは署名インターフェースビュー、`.yx` レイヤは実ソース
- 展開ディレクトリ下のサンプルプロジェクトで LSP を起動し、std メンバの補完 / 定義ジャンプが利用可能（exe 相対検索が生效）
- 公式ガイダンス通り `/usr/local`（または `~/.yaoxiang`）に展開し PATH 追加後、任意のディレクトリで
  `yx --version` 成功
- `apt install yaoxiang`（自前リポジトリ）後実行可能、`apt upgrade`
  でバージョン追従可能；`/usr/bin/yx` シンボリックリンク下でエンジンの
  `$ORIGIN`（実際のパスで解決）が `bin/libz3.so` をヒット
- `curl ... | sh` と `irm ... | iex` をクリーン環境で実行後、`yx` 実行可能、PATH 設定済み
- `yx toolchain install <ver>` / `default` / `update`
  が生效：複数バージョン共存、`yx-toolchain.toml`
  のプロジェクト pin がデフォルトバージョンより優先、フロントドアが正しいバージョンにディスパッチ（バージョンツリー内で rpath と std 検索が自己完結、「新エンジンに旧 fmt」組合せなし）
- ポータブル展開後、`yx` が隣接する `yaoxiang-rs`
  にフォールバック、マネージドインストールと一致した挙動
- Inno Setup インストール後、ディレクトリ構造が完全、PATH 生效、アンインストール可能
- Release アセット完備：5 プラットフォームの再構成パッケージ + `.sha256` が実内容と一致
- Release body は `generate-commit-list.ts` の出力（merge commit changelog 完全）
- nightly 成果物は Pre-release、最新正式 tag に影響しない

## トレードオフ

### 利点

- **即利用**
  — ポータブル展開即利用（rpath + 同ディレクトリ共有ライブラリ）、インストーラが完全なディレクトリを展開
- **標準ライブラリ可読** — ユーザは Python の `Lib/` を読むように直接 std を読める（必須要件達成）
- **保守コスト削減** — ~600 行の手書きビルド YAML を cargo-dist + ~80 行の自前スクリプトに置換
- **クロスプラットフォーム一貫性**
  — 全プラットフォーム動的リンク + 同ディレクトリ共有ライブラリ、特例なし
- **二層インストール**
  — 標準チャネルは新規コードゼロ（Go/Zig 方式）；お手軽チャネルは Rust を参照、apt / curl / iex /
  exe の 4 入口で同一の成果物構造を共有
- **バージョン管理内蔵** — フロントドア `yx` は rustup/Go
  GOTOOLCHAIN 相当、Python/Node が事後的に pyenv/nvm/pdm で補講する生態分裂を回避；ツールのバージョンロックは規約ではなく構造保証

### 欠点とリスク

- **お手軽チャネルの保守面** — install.sh / install.ps1（一括スクリプト、ほぼ進化しない）+ `.deb`
  と apt リポジトリメタデータ公開（release CI 自動化）+ `yx` フロントドア crate
- **エンジン改名の影響範囲** — `yaoxiang` → `yaoxiang-rs`
  は CI アーティファクト名、Inno、テスト、ドキュメントを一度に一括移行する必要あり（フェーズ 5 内で完了）
- **学習コスト** — チームが cargo-dist 設定を学ぶ必要あり
- **cargo-dist 上流リスク** —
  2025 年中頃に Axo 停止に伴い停止、同年 9 月に原作者が復活し継続的にリリース（0.29 →
  0.32+）；`dist-version` ロック + 生成物 vendor でリポジトリ review で緩和
- **cargo-dist はネイティブ nightly なし** — nightly リリース部分は引き続き手書きが必要

### RFC-014b との関係

|            | RFC-014b                              | RFC-037                                      |
| ---------- | ------------------------------------- | -------------------------------------------- |
| **範囲**   | 第三者パッケージのビルドと配布        | コンパイラ自体のパッケージングと配布         |
| **ツール** | `yaoxiang build` / `yaoxiang publish` | `cargo-dist` + 自前スクリプト                |
| **成果物** | 第三者パッケージの FFI ライブラリ     | コンパイラ + 標準ライブラリ + ツールチェーン |
| **競合**   | いいえ、補完的                        | いいえ、補完的                               |

## 代替案

| 案                                       | 採用しない理由                                                                                                                                                                                     |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **手書き CI を継続**                     | 既に ~870 行を手書き、重複作業、DLL 漏れのリスク                                                                                                                                                   |
| **自前のパッケージングツールを書く**     | 車輪の再発明をしない、cargo-dist は既に成熟                                                                                                                                                        |
| **tar.gz のみでインストーラなし**        | 唯一の公式チャネルは展開 + PATH；Inno は Windows ウィザード慣行（中国国内ユーザ既決で保持）への対応のみ                                                                                            |
| **Docker 配布**                          | コンパイラと言語ツールチェーンはコンテナシナリオではなくネイティブバイナリが必要                                                                                                                   |
| **自前 Homebrew tap**                    | tap は一律コミュニティ保守（homebrew-core）、自前は時期尚早；お手軽チャネルの macOS 入口は curl スクリプト                                                                                         |
| **独立 `yaoxiangup` マネージャバイナリ** | rustup の原典先例として可行；しかし 2 番目のユーザ動詞を生み、「パッケージマネージャ/fmt も独立すべきか」の対称性問題を呼ぶ——フロントドア/エンジン分離で一挙に解消（2026-09-09 議論で否決）        |
| **自動更新のみ、複数バージョンなし**     | 単一バージョン自動更新では複数プロジェクトの pin 異バージョン需要に応えられない；Python/Node には公式バージョン管理がなく、生態系が pyenv/nvm/pdm を生むことを強いられた——既決で内蔵（2026-09-09） |
| **Z3 全静的リンク**                      | 既決で否決——外部システムのディレクトリ式共有ライブラリ配布は自然な形態；exe に埋め込むことと外部に置くことは「どちらもパッケージに同梱する」点で同等で、交換可能性も失われる                       |
| **Inno Setup 廃止**                      | 既決で否決——Windows ウィザードとして保持（追加チャネル）                                                                                                                                           |
| **cargo-dist ネイティブインストーラ**    | フラットバイナリ仮定が bin/+lib/ 構造と衝突、インストール直後にライブラリ不足                                                                                                                      |
| **WiX/MSI 自前保守**                     | Inno の既存価値を超えて cargo-dist ジェネレータを失うコストが高い                                                                                                                                  |

## 実装戦略

### フェーズ 1：言語側変更（P0）

1. `build.rs`：全プラットフォーム統一動的リンク + rpath link-arg；`copy_dll()` を
   `copy_shared_lib()`（so/dylib/dll）に拡張
2. リポジトリで native インターフェースビューを事前生成（`src/std/interfaces/`）、テストゲートで
   `StdModule::exports()` との同期を強制（gen-std サブコマンドは既決でキャンセル）
3. `find_std_interface_file` に exe 相対検索分岐を追加；`package init` の出力パスを
   `.yaoxiang/vendor/std` に統一

### フェーズ 2：cargo-dist 導入（P0）

1. `cargo dist init` を実行して初期設定生成（`installers = []`、dist-version ロック）
2. `package-dist.sh` 作成（再構成 + .yx ソースコピー + チェックサム再計算）
3. `dist-release.yml` にすべてを承载：push main トリガー → gate バージョンゲート → ゲート（audit /
   fmt / clippy / test）→ tag 打ち → dist build → 再構成 → wasm 並列 → 自前 publish +
   apt；`release.yml` と `_build-platforms.yml` を削除
4. 新旧パイプラインを並走、受け入れ基準に基づき逐次検証

### フェーズ 3：旧 CI 廃止（P1）

1. `_build-platforms.yml` と `release.yml` を一括削除（リリース単一点：`dist-release.yml`）
2. `nightly.yml` のビルド段を `cargo dist build` に変更
3. `setup.iss`
   を新成果物構造に統合（Inno を正式採用；バージョン番号を Cargo.toml から注入、sed 置換を廃止）

### フェーズ 4：お手軽チャネル（P2）

1. `install.sh` / `install.ps1`（プラットフォーム検出 → 最新再構成パッケージダウンロード →
   `versions/` に展開 → `bin/yx` をインストールルートに配置 → `settings.toml`
   にデフォルトバージョンを書き込み →
   PATH 通知/書き込み；フェーズ 5 と同ラウンドで最終形態として着地）
2. `.deb` パッケージング（`package-dist.sh` の同一ディレクトリツリー + `/usr/bin`
   シンボリックリンクを再利用）+ GitHub Pages 静的 apt リポジトリ（メタデータ GPG 署名、release
   CI で公開）

### フェーズ 5：フロントドア yx とエンジン改名（P2）

1. 現モノリスを `yaoxiang-rs`
   に改名（CI アーティファクト名、Inno、テスト、ドキュメントをすべて一度に一括移行）
2. フロントドア小型 crate
   `yx`（workspace メンバー）を新規追加：バージョン解析、release アーティファクトダウンロード、tar/zip 展開、settings.toml、ディスパッチ（toolchain/self 動詞のみ保持、他は透過転送、clap を使わず手書きディスパッチで引数逐語転送を保証）；バージョンインデックスは GitHub
   Releases latest API から取得（ミラーソースは ghproxy 風プレフィックス連結）；`yx`
   コマンド名は衝突チェック済み（主要ディストリビューション/Homebrew に同名の常用コマンドなし、僅かに小規模ツールが yx をエイリアスとして使用）
3. `yx toolchain install/default/update/list/uninstall` + `yx-toolchain.toml`
   プロジェクト pin + ミラーソース + `yx self update`
4. ブートストラップスクリプト：`curl | sh` / `irm | iex`
   → 再構成パッケージをダウンロードし、フロントドアとデフォルト stable を一括インストール（フェーズ 4 と同ラウンドで最終形態として着地）

### フェーズ 6：オプションフォローアップ（いずれも非ブロッキング）

1. winget 申請（Inno exe を指す、コミュニティ保守、homebrew-core 方式と対称）
2. `.rpm`（dnf ユーザ、`.deb` と同形）
3. Homebrew：知名度が homebrew-core 参入基準に達した後、コミュニティが提出
4. npm `@yaoxiang/cli` 自前ラッパー（名称は現在未登録）

## 開放問題

### 未決

- **.yx レイヤは「配布ディレクトリを手動変更すればコンパイル採用される」Python 方式セマンティクスを提供するか？**
  デフォルトは否——コンパイル権威は RFC-036 内嵌を維持（std バージョンとバイナリの厳格なバインド）、配布ディレクトリの位置づけは可読ビュー +
  LSP 解析ソース。将来開放する場合、バージョン绑定不変量を再検討する必要あり。

### 解決済み

以下の問題は設計議論で解決済み：

- ~~Windows で Z3 静的リンクは可行か？~~ →
  **静的リンクは行わない、全プラットフォーム動的**（2026-09-09 再確認の上維持）
- ~~gen-std-interfaces サブコマンド命名？~~ →
  **サブコマンドは設けない**（2026-09-10 既決：通常のパッケージング成形後、サブコマンド面は余分；native インターフェースビューをリポジトリ事前生成
  `src/std/interfaces/` + テストゲート同期に変更、パッケージングは純粋コピー）
- ~~Inno Setup を保持するか？~~ → **Windows ウィザードとして保持（追加チャネル）**
- ~~配布パッケージ構造は標準ライブラリソースを物理的に携带するか？~~ →
  **必須**（ユーザ可読性は Python `Lib/` に揃え、2026-09-09 既決）
- ~~cargo-dist ネイティブインストーラ（shell/powershell/homebrew/msi/npm）？~~ →
  **すべて廃止**、インストーラは自前（フラット仮定が bin/+lib/ と衝突）
- ~~インストーラ戦略？~~ →
  **二層**（2026-09-09 終裁）：標準チャネル = 配布パッケージ即プロダクト（Go/Zig 方式、展開 +
  PATH）；お手軽チャネルは Rust 参照で一行コマンドインストール——Linux `apt`（自前 deb リポジトリ）/
  `curl | sh`、Windows `irm | iex` / Inno
  exe。行わない：MSI、cargo-dist ネイティブインストーラ、自前 brew tap
- ~~バージョン管理を含めるか？~~ →
  **必須、インストーラ体系に属する**（2026-09-09 既決、同日早朝「遠期に独立 RFC」の境界を翻す）：動機は Python/Node に公式バージョン管理がなく pyenv/nvm/pdm 補講の生態分裂に至ったこと
- ~~バージョン管理形態：独立バイナリかサブコマンドか？~~ →
  **フロントドア/エンジン分離**（2026-09-09 終裁、三ラウンド収斂 A→C→命名反転）：常用コマンド `yx`
  = フロントドア（小型バイナリ、toolchain/self 動詞内蔵）、エンジン `yaoxiang-rs` = 現 `yaoxiang`
  モノリスの改名；ディレクトリ構造
  `versions/<ver>/`——バージョンは第一級概念、バージョンディレクトリは配布パッケージ展開ルートディレクトリ、ツールはバージョン全体でロック（旧 fmt は新構文非対応；かつて想定した内層
  `toolchains/` は冗長で取消）。先例：Go `go` フロントドア +
  GOTOOLCHAIN、rustup プロキシディスパッチ
- ~~cargo-dist extra-artifacts 条件付き実行？~~ → **`package-dist.sh` スクリプトで処理、shell
  case 分岐で進める**
- ~~標準ライブラリインターフェースバージョン互換性？~~ →
  **コンパイラバージョンと共にリリース、同一圧縮パッケージ内**

## 参考文献

- [cargo-dist 公式ドキュメント](https://axodotdev.github.io/cargo-dist/)
- [cargo-dist GitHub](https://github.com/axodotdev/cargo-dist)
- [RFC-014b: ビルドシステムとバイナリ配布](../accepted/014b-build-system.md)
- [cargo-dist nightly feature request](https://github.com/axodotdev/cargo-dist/issues/1143)
- [Z3 ビルド設定 — CMakeLists.txt](https://github.com/Z3Prover/z3/blob/master/src/CMakeLists.txt)
