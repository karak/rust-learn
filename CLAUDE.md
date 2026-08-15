# rust-learn

Rust を **CLI ツール開発を題材に** 習得するための学習リポジトリ。

## 説明の方針

読み手の前提知識は `.claude/learner-profile.local.md`（追跡対象外）に置く。
公開リポジトリに個人の背景情報を含めないため分離している。

以下は **説明不要** として扱う：

- 変数・制御構文・関数・基本的な型
- Git / CI / テスト / パッケージ管理の一般論
- HTTP、並行処理、DB、クラウドの概念

**説明すべきは Rust 固有の意思決定**：所有権と借用、ライフタイム、トレイトと型消去、
エラーモデル、`Send`/`Sync`、ゼロコスト抽象の実際のコスト、`unsafe` の境界、
そして「他言語ならこう書くが Rust ではこう書く／その理由」。

> 他言語での常識との**差分**を語ること。共通部分をなぞらないこと。

## リポジトリ構成

```
Cargo.toml            ワークスペース。依存とリントを一元管理
rust-toolchain.toml   ツールチェーン固定（1.97.1 / edition 2024）
clippy.toml           lint 閾値
deny.toml             依存の脆弱性・ライセンス検査
crates/tally/         題材となる CLI。lib と bin を分離した実装の参照例
docs/curriculum.md    学習ロードマップ
.claude/skills/       このリポジトリ固有のスキル
```

## コマンド

`cargo` のエイリアスは `.cargo/config.toml` に定義済み。

| 目的 | コマンド |
| --- | --- |
| 型チェックのみ（速い） | `cargo c` |
| テスト | `cargo t`（= `cargo nextest run --workspace`） |
| ドキュメントテスト | `cargo test --workspace --doc`（nextest は実行しない） |
| lint | `cargo lint`（警告をエラー扱い） |
| 整形 | `cargo fmt --all` |
| 依存検査 | `cargo deny check` |
| マクロ展開の確認 | `cargo expand -p tally --lib` |
| CLI の手動実行 | `cargo run -p tally -- --help` |

**コミット前に通すべきもの**: `cargo fmt --all` → `cargo lint` → `cargo t`。
CI（`.github/workflows/ci.yml`）は同じ内容を実行する。

## このリポジトリでのコーディング方針

これらは学習上の意図があって選んでいる。**破る場合は理由を述べること。**

1. **`unsafe` は禁止**（ワークスペースで `deny`）。必要になったら、まず安全な代替を探す。
2. **ロジックと I/O を分離する。** `core` のような純粋モジュールを作り、`main.rs` は
   引数解釈・I/O・終了コードだけを担う。テストの大半がプロセス起動なしで回ることが目的。
3. **ライブラリ層は `thiserror`、バイナリ層は `anyhow`。** 公開 API に `anyhow::Error` を
   出さない。詳細は `.claude/skills/rust-error-handling/`。
4. **`unwrap()` / `expect()` は非テストコードでは警告。** テストでは許可（`clippy.toml`）。
   ただし `tests/` 配下は `#[cfg(test)]` ではないため、ファイル先頭で明示的に `allow` する。
5. **clippy は `pedantic` まで有効。** 個別に `allow` するのは構わないが、
   **必ず理由をコメントで残す。** 何を外したかが学習記録になる。
6. **`#[allow(dead_code)]` で警告を黙らせない。** 使わないコードは消す。
7. **コメントは「何をしているか」ではなく「なぜそう書いたか」を書く。**
   特に、他言語の直感と食い違う箇所には理由を残す。

## テストの方針

- **ユニットテスト**（`src/**` の `#[cfg(test)] mod tests`）にロジックを寄せる。
- **統合テスト**（`tests/`）は、そこでしか検証できないものだけ：
  終了コード、stdout と stderr の分離、引数の実配線。プロセス起動は遅い。
- テスト名は日本語で「何が保証されるか」を書いてよい（既存のコードがそうなっている）。
- **存在確認で終わらせず、内容を検証する。** 詳細は `.claude/skills/rust-testing/`。

## 私（Claude）への指示

### 教え方

- **答えのコードを出す前に、まず設計上の選択肢とトレードオフを示す。**
  「なぜ `Cow` なのか」「なぜ `&str` ではなく `impl AsRef<str>` なのか」を言語化する。
- **借用チェッカのエラーは、直し方だけでなく「コンパイラが何を守ろうとしたか」を説明する。**
  詳細は `.claude/skills/rust-ownership-coach/`。
- 他言語での等価な書き方を引き合いに出してよい。ただし **差分と、その差分が生じる理由** に集中する。
- 「動くコード」で止めない。**`clippy --pedantic` を通る形まで持っていく。**

### 作業の進め方

- **コードを書いたら必ず `cargo lint` と `cargo t` を実行し、結果を報告する。**
  通っていないものを「できた」と言わない。
- 新しいクレートを依存に足すときは、`.claude/skills/rust-crate-selection/` の
  判断基準に沿って **代替候補と選定理由を述べてから** 追加する。
- 依存を足したら `cargo deny check` を通す。
- 既存コードのコメントを削除しない。

### 使えるツール

- **`context7` MCP**: クレートの最新ドキュメント取得。`clap`、`tokio`、`serde` などの
  API を答える前に **必ず引く。** 記憶に頼らない（Rust エコシステムは API 変更が速い）。
- **`serena` MCP**: rust-analyzer 経由のシンボル検索・参照検索。
  ファイル全文を読む前に `find_symbol` / `find_referencing_symbols` を使う。
- **`LSP` ツール**: 型情報や診断の取得。
