---
status: "proposed"
date: 2026-09-23
decision-makers: 学習者, Claude
consulted: —
informed: —
---

# 複数入力の集計と、その失敗の文脈

**論点ごとに、扱うときに案と評価を書き足していく文書である。**
書き方は [ADR-0005](0005-selector-public-api.md) の「この文書の書き方」に従う
（**案を先に書き溜めない。** 1 論点を扱うとき、その論点の案だけを書く）。

## コンテキストと問題提起

段階 6 で `tally` は複数のファイルを引数に取り、ファイル単位で並列に集計する
（課題は `docs/curriculum.md` 段階 6）。

**段階 5 の公開 API は「入力は最大 1 つ」を前提にしている。**
その前提は 3 箇所で明示的に先送りされていた。

| 先送りした場所 | 書かれていたこと |
| --- | --- |
| [ADR-0004](0004-error-type-shape.md) の Confirmation | `CliErrorKind::Tally` の `#[error(transparent)]` は「複数入力を扱うようになったら見直す」 |
| [ADR-0005](0005-selector-public-api.md)「実装時に決めたこと」 | `tally_reader` の引数の形は「複数入力を 1 つの集計に合流させたくなったら見直す」 |
| `docs/handoff.md` 未決事項 5 | 複数入力になったとき、どのファイルの失敗かを言えない |

**前提は [ADR-0004](0004-error-type-shape.md) の「前提」節と共有する。**
要点だけ: 判断の基準は「中規模の実務ツールで公開 API だったときの定石」、
**1.0 以前は破壊的変更を受け入れる。**

### 現状の事実（2026-09-23 に確認）

| 事実 | 場所 |
| --- | --- |
| `tally_reader` は `Counter` を消費して **`Report` を返す** | `crates/tally-core/src/count.rs`（`tally_reader`） |
| `Report` は順位づけ済みで、`limit` で **切り詰められうる** | 同（`Counter::report`） |
| `Counter` のフィールドは非公開。derive は `Debug, Default` のみ（`Clone` なし） | 同（`Counter`） |
| `strict` は `Counter` が持つ（ADR-0005 論点 1） | 同 |
| `TallyError::OpenInput` は `path` を持つ。`Read` と `Line` は持たない | `crates/tally-core/src/error.rs`（`TallyError`） |
| `LineError::line_no` は **入力ごとに 1 から数える** | `count.rs`（`tally_reader` の `enumerate`） |
| `CliErrorKind::Tally` は `#[error(transparent)]` で文脈を足さない | `crates/tally/src/error.rs` |
| 入力は `Cli::input: Option<PathBuf>` の 1 つ | `crates/tally/src/cli.rs` |
| `rayon` はワークスペースの依存表にあるが、**どのクレートも使っていない** | `Cargo.toml` |

**`Report` はマージできない。** 上位 N 件に切り詰めた 2 つの結果を足しても、
全体の上位 N 件にはならない（片方で N+1 位だったキーが、合算すると 1 位になりうる）。
**したがって、合流は `Report` になる前の `Counter` で行うしかない。**

## 論点の分割

| # | 論点 | 状態 | 依存 |
| --- | --- | --- | --- |
| 1 | `Counter` のマージの形（`strict` が食い違うときを含む） | 未着手 | — |
| 2 | 入力 1 つぶんの集計が何を返すか（`tally_reader` の形） | 未着手 | 1 |
| 3 | 失敗にどのファイルかを持たせる場所（core か CLI か。`OpenInput` との二重） | 未着手 | — |
| 4 | 複数のファイルが失敗したとき、どれを報告するか | 未着手 | 3 |
| 5 | 並列化をどちらのクレートに置くか | 未着手 | 2 |

**1 と 3 は独立。** 4 は 3 の形に、5 は 2 の形に乗る。

## 確認方法（Confirmation）

論点を決めるたびに書き足す。

## 改訂履歴

- 2026-09-23: 起票（`proposed`）。論点の分割と現状の事実のみ
