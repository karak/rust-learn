# rust-learn

CLI ツール開発を題材にした Rust 学習環境。

- **いまどこまで進んだか・再開するには** → [`docs/handoff.md`](docs/handoff.md)
- **学習の進め方（全 10 段階・予）** → [`docs/curriculum.md`](docs/curriculum.md)
- **各段階で実際に何が起きたか（実）** → [`docs/stage-log.md`](docs/stage-log.md)
- **得られた学び** → [`docs/learning-log.md`](docs/learning-log.md)
- **進め方の振り返り** → [`docs/journal/`](docs/journal/)
- **リポジトリの方針・Claude への指示** → [`CLAUDE.md`](CLAUDE.md)
- **題材の CLI** → [`crates/tally/`](crates/tally/)
- **集計コア（ライブラリ）** → [`crates/tally-core/`](crates/tally-core/)
- **コードの配置ルール** → [`crates/tally-core/docs/layout.md`](crates/tally-core/docs/layout.md)
  と [`crates/tally/docs/layout.md`](crates/tally/docs/layout.md)
- **出力の外部仕様（利用者への契約）** → [`crates/tally/docs/output-format.md`](crates/tally/docs/output-format.md)

## セットアップ

### A. devcontainer（推奨）

コンパイラ・ツール・依存をコンテナに固定し、ホスト環境から隔離する。

必要なもの: Docker 互換ランタイム（OrbStack / Docker Desktop / colima）。

- **VS Code**: リポジトリを開き「Reopen in Container」。
- **CLI**: `devcontainer up --workspace-folder .`

**コンテナから push はできない。** 資格情報を渡していないため。
commit までをコンテナで行い、**push はホストから**行う
（`.git` は共有されているので、コミットはホストからそのまま見える）。
`fetch` とビルドはコンテナでも通る。
理由は [ADR-0008](docs/adr/0008-container-push-credentials.md)。

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

さらに **秘密情報スキャンの設定**を 1 回だけ実行する
（devcontainer では `postCreateCommand` が自動で走るので不要）。

```bash
# git-secrets 本体を入れたうえで（https://github.com/awslabs/git-secrets）
./scripts/setup-git-secrets.sh
```

**パターンとフックは git が追跡しない場所にある**ため、clone しただけでは
保護がかからない。このスクリプトが追跡される唯一の正本。

## 日常のコマンド

```bash
cargo c      # 型チェック（速い）
cargo t      # テスト（nextest）
cargo lint   # clippy pedantic、警告をエラー扱い
cargo fmt --all
cargo test --workspace --doc   # nextest はドキュメントテストを実行しない
./scripts/check-docs.sh        # 文書の書き分けの検査
```

コミット前は `cargo fmt --all` → `cargo lint` → `cargo t`。CI も同じ内容を実行する。

**`main` に直接コミットしない。** 作業は常にブランチで行い、`--ff-only` で `main` に載せる。
手順は [`CLAUDE.md`](CLAUDE.md) の「ブランチ運用」、
判断の過程は [ADR-0006](docs/adr/0006-branching-strategy.md)。
**CI は全ブランチの push で回る**（PR は任意）。

## tally

行指向データの度数を集計する CLI。

```bash
$ printf 'a\nb\na\n' | cargo run -q -p tally
2	a
1	b

$ cat app.log | cargo run -q -p tally -- --field level -n 5 --format json
```

**クレートが 2 つに分かれている。**

| クレート | 持つもの | 持たないもの |
| --- | --- | --- |
| `tally-core` | キーの抽出、正規化、度数の集計 | **`clap`、`anyhow`、ファイルを開く操作** |
| `tally` | 引数解釈、出力の整形、終了コード、I/O | 集計ロジック |

依存の向きは `tally` → `tally-core` の一方向。
`cargo tree -p tally-core` で確かめられる。

**テストの大半がプロセス起動なしで走る**のが、この分割の目的。
`tally-core` は `tests/` を持たない（プロセスを起動しないと観測できるものが無い）。
（件数は変動するので記載しない。`cargo t` で確認すること。）
