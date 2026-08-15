# 進捗記録

最終更新: 2026-08-15

## 現在地

| 区分 | 状態 |
| --- | --- |
| カリキュラム実装 | 段階 1 まで完了。段階 2（エラーモデル）未着手 |
| 通読 | 段階 0・1 を通読済み |
| 個別精読 | `crates/tally/src/core.rs` のみ |

`docs/curriculum.md` の全 10 段階（段階 0〜9）のうち 2 段階が完了。

**未読のファイル**: `error.rs` / `cli.rs` / `main.rs` / `tests/cli.rs`。
`docs/learning-log.md` にはこれらに触れる記述が含まれるため、
読む順序を保ちたい場合は該当節を飛ばすこと（各節に対象ファイルを明記してある）。

## コミット履歴

| SHA | 内容 |
| --- | --- |
| `173d5da` | Rust 学習環境の初期構築 |
| `fad0464` | devcontainer によるビルド環境の固定と隔離を追加 |
| `62b5588` | tally に `--ignore-case` を追加（段階 1） |
| `00d5bd6` | `.serena/` を追跡対象から外す |

履歴は公開に備えて一度書き換えている（`git filter-repo`）。
author / committer は GitHub の noreply アドレスに統一し、
開発ツールのローカル状態を全履歴から除去した。
リポジトリローカルの `user.email` も同じ値に設定済みなので、以後のコミットも同様になる。

## 検証状態

すべてホストと devcontainer の双方で通過を確認済み。

| 項目 | 結果 |
| --- | --- |
| `cargo fmt --all --check` | OK |
| `cargo clippy --workspace --all-targets -- -D warnings`（pedantic） | OK |
| `cargo nextest run --workspace` | 32 件通過 |
| `cargo test --workspace --doc` | OK（ドキュメントテストは 0 件） |
| `cargo deny check` | advisories / bans / licenses / sources すべて ok |

テスト件数の推移: 段階 0 時点 19 件 → 段階 1 完了時 32 件。

## 環境

- ツールチェーン: Rust 1.97.1 / edition 2024（`rust-toolchain.toml` で固定）
- ワークスペース: メンバは `crates/tally` のみ（段階 5 で `tally-core` を分離予定）
- devcontainer: `rust:1.97.1-bookworm` ベース。イメージ 2.38GB。非 root 実行。
  `target/` と cargo レジストリは名前付きボリューム
- コンテナランタイム: OrbStack（個人の非商用利用のため Free の範囲）
- 追加ツール: `cargo-nextest` / `cargo-deny` / `cargo-expand`

## 保留事項

1. **GitHub への同期が未実施。** リモート未設定でローカルのみ。
   公開リポジトリとして同期する方針は決定済み。
   同期するまで `.github/workflows/ci.yml` は **一度も実行されていない**。
2. **段階 2 未着手。** 課題は `--strict` フラグの追加（`docs/curriculum.md` 参照）。
3. **完了条件・想定期間は未定義。** 名目上の終点は段階 9 の完了だが、
   「役目を終えた」とする基準は決めていない。

## 次の一手

段階 2（エラーモデル）。`--strict` を追加し、既定ではスキップしている
「フィールドが無い行」をエラーにする。エラーには行番号と該当行の先頭 40 文字を含める。
対象ファイルは `error.rs` と `core.rs`、および統合テスト。
