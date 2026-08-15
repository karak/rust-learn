# rust-learn

CLI ツール開発を題材にした Rust 学習環境。

- **いまどこまで進んだか・再開するには** → [`docs/handoff.md`](docs/handoff.md)
- **学習の進め方（全 10 段階）** → [`docs/curriculum.md`](docs/curriculum.md)
- **得られた学び** → [`docs/learning-log.md`](docs/learning-log.md)
- **進め方の振り返り** → [`docs/journal/`](docs/journal/)
- **リポジトリの方針・Claude への指示** → [`CLAUDE.md`](CLAUDE.md)
- **題材の CLI** → [`crates/tally/`](crates/tally/)

## セットアップ

### A. devcontainer（推奨）

コンパイラ・ツール・依存をコンテナに固定し、ホスト環境から隔離する。

必要なもの: Docker 互換ランタイム（OrbStack / Docker Desktop / colima）。

- **VS Code**: リポジトリを開き「Reopen in Container」。
- **CLI**: `devcontainer up --workspace-folder .`

`.devcontainer/Dockerfile` はツールチェーンを **`rust-toolchain.toml` と同じ 1.97.1** で
焼き込んでいる。**片方を上げたら必ずもう片方も上げること。** ずれていると
コンテナ起動後に rustup が別バージョンを追加取得し、固定した意味が消える。

`target/` と cargo レジストリは名前付きボリュームに置いてある。
macOS の bind mount は I/O が遅く、`target/` を共有するとビルドが大幅に遅くなるため。
副作用として **ホスト側の `target/` とは完全に別物** になる。

### B. ホストに直接

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
**テストの大半がプロセス起動なしで走る**のが、この分割の目的。
（件数は変動するので記載しない。`cargo t` で確認すること。）
