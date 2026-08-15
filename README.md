# rust-learn

CLI ツール開発を題材にした Rust 学習環境。

- **学習の進め方** → [`docs/curriculum.md`](docs/curriculum.md)
- **リポジトリの方針・Claude への指示** → [`CLAUDE.md`](CLAUDE.md)
- **題材の CLI** → [`crates/tally/`](crates/tally/)

## セットアップ

`rust-toolchain.toml` があるため、`rustup` が自動でツールチェーンを揃える。
追加で必要なもの:

```bash
cargo install cargo-nextest cargo-expand cargo-deny --locked
```

## 日常のコマンド

```bash
cargo c      # 型チェック（速い）
cargo t      # テスト（nextest）
cargo lint   # clippy pedantic、警告をエラー扱い
cargo fmt --all
```

コミット前は `cargo fmt --all` → `cargo lint` → `cargo t`。CI も同じ内容を実行する。

## tally

行指向データの度数を集計する CLI。

```bash
$ printf 'a\nb\na\n' | cargo run -q -p tally
2	a
1	b

$ cat app.log | cargo run -q -p tally -- --field level -n 5 --format json
```

`lib` と `bin` を分離し、集計ロジック（`core.rs`）は I/O を持たない。
テスト 19 件のうち 11 件はプロセス起動なしで走る。
