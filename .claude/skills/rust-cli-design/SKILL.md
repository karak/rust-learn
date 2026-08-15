---
name: rust-cli-design
description: Use when building or reviewing a Rust command-line tool — clap argument design, lib/bin split, exit codes, stdout vs stderr, streaming large input, broken pipe, buffering, shell completions, config precedence, or logging setup. Fires on clap, Parser, ExitCode, stdin/stdout, BufReader/BufWriter, tracing setup, and "make this a CLI".
---

# Rust CLI の設計

参照実装: `crates/tally/`。

## 1. lib と bin を分ける（最重要）

```
src/lib.rs     公開モジュール宣言
src/core.rs    I/O を持たない純粋ロジック  ← テストの主戦場
src/error.rs   thiserror のエラー型
src/cli.rs     clap の型定義
src/main.rs    引数解釈・I/O・終了コードのみ。ロジックを持たない
```

**なぜ**: `main.rs` にロジックを置くと、検証手段がプロセス起動しかなくなる。
`tally` ではユニットテスト 11 件がプロセス起動なしで走り、統合テスト 8 件だけが
`assert_cmd` を使う。この比率を保つ。

**`cli.rs` を `main.rs` から分ける理由**: `Cli::try_parse_from(["tally", "--field", "lvl"])`
で引数解釈だけを単体テストできる。`Cli::command().debug_assert()` はフラグの衝突を
コンパイル後に検出する（CLI が育つと効いてくる）。

## 2. clap の型設計

**引数を「文字列 + 後段でバリデーション」にしない。型で表す。**

```rust
#[derive(Debug, Parser)]
#[command(name = "tally", version, about, long_about = None)]
pub struct Cli {
    /// doc コメントがそのままヘルプになる。別途 help = "..." を書かない。
    pub input: Option<PathBuf>,

    #[arg(short, long, value_name = "NAME")]
    pub field: Option<String>,

    /// enum + ValueEnum にすると、不正値を clap が弾き、
    /// 補完候補にもなり、match の網羅性もコンパイラが保証する。
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
}
```

- **入力ファイルは `Option<PathBuf>`。`None` = 標準入力。** Unix の作法。
- **相互排他は `#[arg(conflicts_with = "...")]`**、あるいは `enum` に畳む。
  実行時の `if a && b { bail!() }` より、clap に検出させるほうがヘルプにも反映される。
- **設定の優先順位は「CLI 引数 > 環境変数 > 設定ファイル > 既定値」。**
  clap の `#[arg(env = "TALLY_FORMAT")]` で環境変数まではカバーできる。
- **`--version` は `#[command(version)]` だけで `Cargo.toml` から取る。** 手で書かない。

## 3. stdout と stderr の分離

**stdout はデータ専用。それ以外は全部 stderr。**

```rust
// ログは stderr へ。stdout に出すとパイプ先が壊れる。
fmt().with_writer(io::stderr).with_ansi(io::stderr().is_terminal())
```

- **進捗・診断・サマリは stderr**（`tally --stats` がその例）。
- **色は出力先が端末のときだけ。** `IsTerminal::is_terminal()` で判定する。
  パイプに ANSI エスケープを流すと下流が壊れる。`NO_COLOR` 環境変数も尊重する。
- 統合テストで **stdout の完全一致** を assert すると、この分離が回帰しない。

## 4. I/O の実際

```rust
// 読み: BufReader で包む。File を直接 lines() すると 1 行ごとに read(2)。
tally_reader(BufReader::new(file), &key, limit)

// 書き: BufWriter で包み、最後に明示的に flush。
let mut out = io::BufWriter::new(stdout.lock());
write_report(&mut out, &report, format)?;
out.flush()?;  // Drop 任せにすると、書き込みエラーを取りこぼす
```

- **`lock()` を取る。** 取らないと 1 回の書き込みごとにロックを取り直す。
- **`flush()` を明示する。** `BufWriter` の `Drop` はエラーを無視する。
- **入力全体を `String` に読まない。** `BufRead::lines()` で流す。
  ログファイルは GB 単位になりうる。`tally` は行単位のストリーミング処理。
- **関数は `impl BufRead` を受け取る。** `File` を受け取ると、テストで
  `"a\nb\n".as_bytes()` を渡せなくなる。

## 5. 終了コードと broken pipe

```rust
fn main() -> ExitCode {
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        // `tally big.log | head` で下流が先に閉じるのは異常ではない。
        Err(err) if is_broken_pipe(&err) => ExitCode::SUCCESS,
        Err(err) => { eprintln!("error: {err:#}"); ExitCode::FAILURE }
    }
}
```

- **`fn main() -> Result<...>` を使わない。** エラーが `Debug` 表示になり、
  `TallyError::OpenInput { path: "...", source: Os { code: 2, ... } }` のような
  ユーザに読めない出力になる。`ExitCode` を返して自分で整形する。
- **broken pipe を握る。** 握らないと `| head` のたびにエラーが出る。
- 終了コードの慣習: `0` 成功 / `1` 実行時エラー / `2` 使い方の誤り（clap が自動で返す）。

## 6. ログ

```rust
let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
```

- **既定では黙る。** `RUST_LOG=debug` で初めて出る。CLI がデフォルトで喋るのは害。
- **構造化フィールドを使う。** `tracing::debug!(path = %path.display(), "読み込み")`。
  文字列連結より、後でフィルタ・解析ができる。
- `%` は `Display`、`?` は `Debug` を使うという記法。

## 7. 仕上げ

- **`clap_complete` でシェル補完を生成する**（`--completions bash` のような隠しフラグ）。
- **`--help` の例を `#[command(after_help = "...")]` に入れる。** 使い方が伝わる。
- 長時間処理には `indicatif`。ただし **stderr が端末のときだけ** 出す。
- Ctrl-C で中断されうるなら、中間ファイルは `tempfile` で作り、成功時に `rename` する。

## レビュー時のチェック

- [ ] `main.rs` にロジックが漏れていないか（純粋関数に切り出せる部分が残っていないか）
- [ ] stdout にデータ以外が出ていないか
- [ ] 入力をメモリに全部載せていないか
- [ ] `BufWriter` を明示的に `flush` しているか
- [ ] broken pipe でエラーを出していないか
- [ ] 色付けが `is_terminal()` で条件づけられているか
- [ ] 相互排他な引数が実行時チェックではなく clap の制約になっているか
