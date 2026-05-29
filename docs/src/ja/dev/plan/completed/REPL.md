# REPL 実装ドキュメント

## 概要

REPL（Read-Eval-Print Loop）は、YaoXiang 言語の対話型インタプリタであり、開発者に即時フィードバックを提供するプログラミング環境です。ユーザーは直接コード断片を入力して即座に実行結果を確認でき、プロトタイプ開発や言語学習の効率を大幅に向上させます。

REPL モジュールは `src/backends/dev/repl.rs` にあり、開発 Shell（`src/backends/dev/shell.rs`）と緊密に統合されており、YaoXiang の対話型開発環境を構成しています。REPL はコード評価に專門化し、Shell はファイル管理やデバッグなどのより豊富な開発コマンドセットを提供します。

現在の REPL 実装は、複数行入力、式完全性検出、履歴管理、特殊コマンド処理などのコア機能をサポートしています。コード評価はフロントエンドコンパイラとの統合を通じて行われ、リアルタイムの構文チェックとエラー報告に対応しています。

## アーキテクチャ設計

### モジュールの位置と依存関係

```
src/backends/dev/
├── repl.rs         # REPL メインモジュール
├── shell.rs        # 開発 Shell（REPL をラップ）
└── debugger.rs     # デバッガー統合

src/backends/common/
├── value.rs        # RuntimeValue 実行時値型
├── heap.rs         # ヒープメモリ管理
└── mod.rs          # 共通モジュールエクスポート

src/backends/interpreter/
├── executor.rs     # 命令エグゼキュータ
├── frames.rs       # コールフレーム管理
└── registers.rs    # レジスタ管理
```

REPL モジュールは以下のコア型に依存しています：`RuntimeValue` は統一された実行時値表現を提供し、`Interpreter` はコード実行を担当し、`Compiler` はソースコードのコンパイルを処理します。モジュール間は明確に定義されたインターフェースを通じて通信し、各コンポーネントが独立してテストおよび進化できることを保証します。

### コアデータ構造

#### REPLConfig 設定構造体

```rust
#[derive(Debug, Clone)]
pub struct REPLConfig {
    /// 標準プロンプト。各行の入力の先頭に表示される
    pub prompt: String,
    /// 複数行入力プロンプト。ブロック構造の継続行に使用
    pub multi_line_prompt: String,
    /// 構文ハイライトスイッチ（将来ターミナルハイライトライブラリと統合するためのインターフェース保持）
    pub syntax_highlight: bool,
    /// 自動インデントスイッチ（入力ライブラリと組み合わせる必要あり）
    pub auto_indent: bool,
    /// 履歴の最大エントリ数。メモリ無限増大防止
    pub history_size: usize,
}
```

設定構造体は拡張可能な設計を採用しており、`syntax_highlight` と `auto_indent` フィールドは将来のエンハンスメントのために予約されています。現在の標準プロンプトは `">> "`、複数行プロンプトは `".. "`、履歴はデフォルトで 1000 エントリを保存します。

#### REPLResult 評価結果列挙型

```rust
#[derive(Debug)]
pub enum REPLResult {
    /// 評価が実際の値を生成。表示のために出力が必要
    Value(RuntimeValue),
    /// 評価に返り値がない（unit 型）
    Ok,
    /// 評価プロセス中にエラーが発生
    Error(String),
    /// ユーザーが能動的に終了（:quit または Ctrl-D）
    Exit,
}
```

結果列挙型は4種類の評価結果を明確に区別し、メインツループがそれぞれ個別に処理できるようにしています。`Value` は実際の実行時値をラップし、`Error` は人間可読なエラー情報を携带し、`Exit` はメインツループの終了を通知します。

#### REPL メイン構造体

```rust
#[derive(Debug)]
pub struct REPL {
    /// REPL 設定
    config: REPLConfig,
    /// コードインタプリタインデンス
    interpreter: Interpreter,
    /// 入力済み履歴（上下矢印で辿ることをサポート）
    history: Vec<String>,
    /// 現在の入力バッファ（複数行継続用）
    buffer: String,
    /// 現在の行カウント（0 は新しい式の開始を意味する）
    line_count: usize,
}
```

REPL 構造体は設定、内部状態、実行エンジンを分離しています。`buffer` はユーザーが入力中の複数行式を 저장し、`line_count` は継続行の状態を追跡し、両者が協力してブロック構造（関数定義、if 式など）の複数行入力サポートを実現します。

## ワークフロー

### メインループフロー

```
┌─────────────────────────────────────────────────────┐
│                    REPL.run()                        │
├─────────────────────────────────────────────────────┤
│  1. ようこそメッセージ出力                            │
│  2. メインループに入る                                │
│     ┌─────────────────────────────────────────────┐ │
│     │  read_line()                                │ │
│     │  ├─ プロンプト表示                          │ │
│     │  ├─ 1行入力読み取り                         │ │
│     │  ├─ 特殊コマンド検出（:quit, :help 等）     │ │
│     │  └─ 履歴に追加                              │ │
│     ├─ 式完全性判断                                │ │
│     │  └─ 不完全 → read_line() 続行               │ │
│     └─ 完全 → evaluate()                          │ │
│         ├─ コードを完全関数としてラップ            │ │
│         ├─ Compiler でコンパイル                  │ │
│         └─ 結果 반환                              │ │
│  3. 結果を処理してループまたは終了                    │
└─────────────────────────────────────────────────────┘
```

メインループは古典的な REPL パターンを遵循しています：入力読み取り、コード評価、結果出力。重要な設計判断は、式完全性判断を入力読み取りから分離していることで、ユーザーが複数行入力的过程中で複雑な式を段階的に構築できます。

### 式完全性検出

`is_complete()` メソッドは、かっこのペア状態のカウントによって式が完全かどうか判断します。アルゴリズムは文字列エスケープを考慮し、文字列 내부の括弧を式区切り子として誤判定することを避けます。

```rust
fn is_complete(&self, code: &str) -> bool {
    let mut braces = 0;   // { }
    let mut brackets = 0; // [ ]
    let mut parens = 0;   // ( )
    let mut in_string = false;
    let mut escaped = false;

    for c in code.chars() {
        // エスケープ文字の処理
        if escaped { escaped = false; continue; }
        if c == '\\' { escaped = true; continue; }

        // 文字列の処理
        if c == '"' { in_string = !in_string; continue; }

        // 非文字列エリアのカウント
        if !in_string {
            match c {
                '{' => braces += 1,
                '}' => { if braces == 0 { return true; } braces -= 1; }
                '[' => brackets += 1,
                ']' => { if brackets == 0 { return true; } brackets -= 1; }
                '(' => parens += 1,
                ')' => { if parens == 0 { return true; } parens -= 1; }
                _ => {}
            }
        }
    }

    braces == 0 && brackets == 0 && parens == 0 && !in_string && !escaped
}
```

完全性検出の境界ケース処理に注意してください：不一致な閉じ括弧に遭遇했을 때（如き `}` しかし braces がすでに 0 の場合）、メソッドは즉시 `true` を返し、前序の式が完全であることを示します。これにより、ユーザーは不完全な継続行を入力して入力を続けることができます。

### コード評価フロー

```rust
fn evaluate(&mut self, code: &str) -> Result<REPLResult, io::Error> {
    // 1. コードを完全関数としてラップ
    let wrapped = format!(
        "main() -> () = () => {{\n{}\n}}",
        code
    );

    // 2. フロントエンドコンパイラを呼び出し
    let mut compiler = crate::frontend::Compiler::new();
    match compiler.compile_with_source("<repl>", &wrapped) {
        Ok(_module) => Ok(REPLResult::Ok),
        Err(e) => {
            // 3. エラー処理（出力を簡素化）
            let lines: Vec<&str> = error_msg.lines().collect();
            if lines.len() > 2 {
                Ok(REPLResult::Error(
                    lines[lines.len() - 2..].join("\n")
                ))
            } else {
                Ok(REPLResult::Error(error_msg))
            }
        }
    }
}
```

コードラップ戦略はユーザー入力を `main() -> () = () => { ... }` 関数に埋め込みます。この設計により、ユーザーが式を入力してもステートメントを入力しても、コンパイラが正しく処理できます。ラップされたコードは標準コンパイルフローを通し、最終的にインタプリタで実行可能になります。

エラー出力は簡素化処理され、ファイルのコンテキスト行が移除され、コアエラー情報のみが保持され、対話体験がよりユーザーフレンドリーになります。

## 設定とカスタマイズ

### デフォルト設定

```rust
impl Default for REPLConfig {
    fn default() -> Self {
        Self {
            prompt: ">> ".to_string(),
            multi_line_prompt: ".. ".to_string(),
            syntax_highlight: true,
            auto_indent: true,
            history_size: 1000,
        }
    }
}
```

### カスタム設定の例

```rust
let config = REPLConfig {
    prompt: "yx> ".to_string(),
    multi_line_prompt: "... ".to_string(),
    syntax_highlight: false,  // 構文ハイライト無効化
    auto_indent: false,       // 自動インデント無効化
    history_size: 500,        // 履歴减少
};

let mut repl = REPL::with_config(config);
```

### 作成方法

```rust
// デフォルト設定を使用
let repl = REPL::new();

// カスタム設定を使用
let repl = REPL::with_config(custom_config);
```

## 特殊コマンドリファレンス

### 利用可能なコマンド一覧

| コマンド | エイリアス | 機能説明 |
|------|------|----------|
| `:quit` | `:q` | REPL を終了。上位 Shell に戻る |
| `:help` | `:h` | 利用可能なコマンド一覧を表示 |
| `:clear` | `:c` | 現在のバッファをクリア（未完了の入力を放棄） |
| `:history` | `:hist` | 履歴を表示（行番号付き） |

### コマンド処理フロー

```rust
fn handle_command(&mut self, command: &str) -> Result<REPLResult, io::Error> {
    match command {
        ":quit" | ":q" => Ok(REPLResult::Exit),
        ":help" | ":h" => {
            // ヘルプ情報を表示
            Ok(REPLResult::Ok)
        }
        ":clear" | ":c" => {
            // バッファをクリア
            self.buffer.clear();
            self.line_count = 0;
            Ok(REPLResult::Ok)
        }
        ":history" | ":hist" => {
            // 履歴を遍历して表示
            for (i, line) in self.history.iter().enumerate() {
                tlog!(info, /* ... */);
            }
            Ok(REPLResult::Ok)
        }
        _ => {
            // 不明なコマンドのヒント
            Ok(REPLResult::Ok)
        }
    }
}
```

## DevShell との統合

### Shell が REPL を呼び出す

```rust
":repl" | "repl" => {
    if let Err(e) = self.repl.run() {
        ShellResult::Error(format!("REPL error: {}", e))
    } else {
        ShellResult::Success
    }
}
```

### 状態遷移

```
Shell ──:repl──> REPL.run()
                    │
                    ├─ :quit ──> Shell に戻る
                    └─ Ctrl-D ──> Shell に戻る
```

### Shell 補助機能

Shell は追加で以下の REPL が直接アクセスできない機能を提供します：

| Shell コマンド | 機能説明 |
|-----------|----------|
| `:cd <path>` | 作業ディレクトリを切换 |
| `:pwd` | 現在のディレクトリを表示 |
| `:ls [path]` | ディレクトリ内容を列表 |
| `:run <file>` | ファイルを実行して時間を計測 |
| `:load <file>` | ファイルを環境にロード |
| `:debug <file>` | デバッグモードを開始 |
| `:break <fn> <offset>` | ブレークポイントを設定 |

## 技術実装の詳細

### 入力行読み取り

```rust
fn read_line(&mut self) -> Result<REPLResult, io::Error> {
    // 1. プロンプトを決定（単一行または複数行）
    let prompt = if self.line_count == 0 {
        &self.config.prompt
    } else {
        &self.config.multi_line_prompt
    };

    // 2. プロンプトを出力して出力バッファをフラッシュ
    tlog!(debug, MSG::ReplPrompt, &prompt.to_string());
    io::stdout().flush()?;

    // 3. 標準入力を読み取り
    let mut line = String::new();
    let stdin = io::stdin();

    if stdin.read_line(&mut line)? == 0 {
        return Ok(REPLResult::Exit);  // Ctrl-D 検出
    }

    // 4. コマンドを処理またはバッファに追加
    // ...
}
```

### 履歴管理

```rust
// 空行以外のみ履歴に追加
if !line.is_empty() {
    self.history.push(line.clone());
}

// 履歴は上下矢印遍历の提供に使用
//（注：現在の実装では Vec に保存。ターミナル対話には readline ライブラリと組み合わせる必要あり）
```

履歴は空行以外的行のみ 저장し、大量の空白行がスペースを占有することを避けます。将来は `rustyline` や `liner` などの成熟したライブラリを統合し、本当の対話型履歴浏览を提供できます。

### ファイルロード

```rust
pub fn load_file(&mut self, path: &Path) -> Result<REPLResult, io::Error> {
    let source = std::fs::read_to_string(path)?;

    let mut compiler = crate::frontend::Compiler::new();
    match compiler.compile_with_source(&path.display().to_string(), &source) {
        Ok(_module) => Ok(REPLResult::Ok),
        Err(e) => Ok(REPLResult::Error(format!("{}", e))),
    }
}
```

`load_file()` はコンパイラインターフェースを再利用し、`.yx` ソースファイルのロードと実行をサポートしています。ファイルパスは `Path` 型を使用することで、クロスプラットフォーム互換性を保证します。

### i18n メッセージマッピング

REPL は統一されたメッセージシステム（`src/util/i18n/mod.rs`）を使用し、すべてのユーザー可见テキストは `MSG` 列挙型を通じてアクセスされます：

```rust
MSG::ReplWelcome     // ようこそ情報
MSG::ReplHelp        // ヘルプ情報
MSG::ReplError       // エラー接頭辞
MSG::ReplValue       // 値出力フォーマット
MSG::ReplPrompt      // プロンプトフォーマット
// ...
```

この設計は多言語拡張をサポートし、異なる言語の `MSG` 実装を提供するだけで済みます。

## テスト覆盖

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_repl_new() {
        let repl = REPL::new();
        assert!(repl.history.is_empty());
    }

    #[test]
    fn test_repl_is_complete() {
        let repl = REPL::new();

        // 完全な式
        assert!(repl.is_complete("1 + 2"));
        assert!(repl.is_complete("let x = 42"));
        assert!(repl.is_complete("fn foo() { 1 }"));

        // 不完全な式
        assert!(!repl.is_complete("fn foo() {"));
        assert!(!repl.is_complete("if true {"));
        assert!(!repl.is_complete("{"));
    }
}
```

## 将来の進化方向

### 短期增强

1. **readline ライブラリの統合**：本当の対話型編集を提供（Emacs/Vi モード、インクリメンタルサーチ）
2. **構文ハイライト**：ANSI エスケープシーケンスによるハイライト対応。キーワード、数字、文字列をサポート
3. **Tab 自動補完**：シンボル補完、関数引数ヒント
4. **複数行編集**：`(` または `{` でトリガーされるスマート継続行

### 中期目標

1. **増分型チェック**：变更部分のみを再チェック。全量コンパイル遅延を回避
2. **インライン結果出力**：複雑な値に摘要を提供。`:expand` で詳細を表示
3. **永続化履歴**：セッション間で履歴を保持
4. **モジュールインポート**：REPL 内から直接コンパイル済みモジュールをインポート

### 長期ビジョン

1. **Jupyter Kernel**：IPython/Jupyter 統合を提供
2. **グラフィカル REPL**：データ構造、コールスタック、タイムラインを可視化
3. **リモート REPL**：ネットワーク REPL でリモートプロセスをデバッグ
4. **パフォーマンス剖析**：`:profile` コマンドで実行時間、メモリ割り当て統計を出力

## 関連ファイル索引

| ファイル | 責務 |
|------|------|
| `src/backends/dev/repl.rs` | REPL メインモジュール |
| `src/backends/dev/shell.rs` | 開発 Shell |
| `src/backends/dev/debugger.rs` | デバッガー |
| `src/backends/common/value.rs` | 実行時値型 |
| `src/backends/common/heap.rs` | ヒープメモリ管理 |
| `src/backends/interpreter/mod.rs` | インタプリタモジュール |
| `src/util/i18n/mod.rs` | 国際化メッセージ |
| `src/frontend/Compiler.rs` | コンパイラフロントエンド |