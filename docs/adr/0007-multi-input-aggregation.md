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
| 1 | `Counter` のマージの形（`strict` が食い違うときを含む） | **決定済み** | — |
| 2 | 入力 1 つぶんの集計が何を返すか（`tally_reader` の形） | 未着手 | 1 |
| 3 | 失敗にどのファイルかを持たせる場所（core か CLI か。`OpenInput` との二重） | 未着手 | — |
| 4 | 複数のファイルが失敗したとき、どれを報告するか | 未着手 | 3 |
| 5 | 並列化をどちらのクレートに置くか | 未着手 | 2 |

**1 と 3 は独立。** 4 は 3 の形に、5 は 2 の形に乗る。

## 論点 1: `Counter` のマージの形（決定済み）

**ファイル単位で集計した結果を 1 つに合流させる手段を、公開 API に 1 つ足す。**

### 決定

**専用の `merge` を足す。`Extend` も `Add` も実装しない。**

```rust
impl Counter {
    /// `other` の集計を取り込む。**方針（`strict`）は `self` のものを保つ。**
    pub fn merge(&mut self, other: Counter) { /* counts / skipped / total を足す */ }
}
```

### この論点に効く事実（2026-09-23 に確認）

| 事実 | 効き方 |
| --- | --- |
| `Report` は順位づけ済みで `limit` に切り詰められうる | **合流は `Counter` でしか行えない。** 上位 N 件同士を足しても全体の上位 N 件にならない |
| `Counter` が持つのは `counts` / `skipped` / `total` / `strict` の 4 つ | 前 3 つは足せる。**足せないのは `strict` だけ** |
| `strict` が効くのは `Counter::push_line` の最中だけで、`Report` には現れない | **マージ後の `strict` は、それ以降に行を積むかどうかにしか影響しない** |
| `HashMap<K, V>` の `Extend<(K, V)>` は **値を上書きする**（`insert` と同じ） | 同じ名前で加算すると意味が反転する |
| Rust の標準に `Monoid` に相当する trait は無い | 「足せる」を型で言う手段は `Add` / `AddAssign` / `Sum` しかない |
| rayon の `reduce` が要求するのは `Fn() -> T` と `Fn(T, T) -> T` | **trait 実装は要らない。** 関数があれば足りる |

### 検討した案

#### 1a. `Extend<(String, u64)>` + `IntoIterator`（却下）

標準ライブラリの慣習に乗せる形。`counter.extend(other)` と書ける。
`FromIterator` も一緒に実装すれば `collect()` も通る。

- 利点: 既存の trait なので、**呼び出し側が総称に書ける。**
  `fn combine<C: Extend<(String, u64)>>(...)` のような関数が書ける
- 利点: `IntoIterator` を足すと `Counter` の中身を外に出す手段が手に入る
- 欠点: **`HashMap` の `Extend` は値を上書きする。** 同じ形の引数を取って
  加算する実装は、`HashMap` を知っている読み手の予想を裏切る。
  **裏切りは型では現れず、数が静かに合わなくなる**
- 欠点: **`skipped` と `total` を運べない。** 項目の列は `(キー, 件数)` の並びで、
  「取り出せなかった行が何行あったか」はそこに入らない。
  結果として `extend` の後に `skipped` を別経路で足す必要が生まれ、
  **1 回のマージが 2 つの手順に割れる**
- 欠点: `IntoIterator` を足すと、`counts` の表現（`HashMap<String, u64>`）が
  公開の約束に近づく。ADR-0005 論点 2 が非公開フィールドを採った理由に反する

#### 1b. `AddAssign` / `Add` + `Sum`（却下）

`a += b` と書ける形。`Sum` まで実装すれば `iter.sum()` が通り、
rayon にも `ParallelIterator::sum()` がある。Scala の `Monoid`（`|+|`）や
C# の `IAggregateOperators` に相当する位置づけ。

- 利点: **記法が短い。** 合流が 1 演算子で書ける
- 利点: `Sum` があると、逐次と並列の両方で同じ書き方になる
- 欠点: **演算子は単位元と結合則と可換性を約束する。**
  `Counter` の単位元は `Counter::new()` だが、
  **`strict` を持つ `Counter` にとっての単位元は `Counter::new().strict(true)`** で、
  型としては同じものが 2 つの意味を持つ。`Sum` の実装は `Default` から始めるので、
  **空の列を `sum()` すると方針が黙って `strict(false)` に落ちる**
- 欠点: 可換性が `strict` については成り立たない（`a + b` と `b + a` で
  マージ後の方針が変わる）。**観測できる `Report` は変わらない**ので実害は無いが、
  **演算子はそこまで含めて約束する記号**である
- 欠点: `Add`（消費して新しい値を返す形）は `counts` を 2 つ持つ瞬間を作る。
  `AddAssign` だけを実装して `Add` を実装しないのは、**標準の慣習に反する**
  （`Add` があって `AddAssign` が無い型はあるが、逆は珍しい）

#### 1c. 専用の `merge`（採用）

`fn merge(&mut self, other: Counter)` を足す。rayon では次のように書く。

```rust
.try_reduce(|| Counter::new().strict(strict), |mut acc, other| {
    acc.merge(other);
    Ok(acc)
})
```

- 利点: **名前が意味を言う。** 「上書きか加算か」を読み手が推測せずに済む
- 利点: `skipped` と `total` を同じ 1 回の呼び出しで運べる
- 利点: **`strict` の扱いを doc に書ける。** 演算子と違い、
  「`self` の方針を保つ」という非対称を明示できる
- 利点: `counts` の表現を公開しない
- 欠点: **糖衣が無い。** `reduce` の閉包が 3 行になる
- 欠点: 総称の関数から使えない（`Extend` のような共通の入口が無い）。
  ただし **いま総称に書きたい呼び出し側は無い**

### 対称性の検査

**却下した案の欠点を、採用案と同じ精度で疑う**（ADR-0005 の書き方に従う）。

- **1a の「予想を裏切る」は、`Extend` の意味を誤解していないか。**
  `Extend` の契約は「項目を追加する」であって「上書きする」ではない。
  `HashMap` が上書きするのは、**キーが重複したときの `HashMap` の性質**による。
  `Counter` が加算するのは、`Counter` の性質としてむしろ自然ともいえる。
  **それでも却下したのは、`skipped` と `total` を運べないほうが重い**ため。
  こちらは慣習の解釈ではなく、**1 回の合流が 2 手順に割れるという構造の話**である
- **1b の欠点は `strict` だけか。** そうである。`counts` / `skipped` / `total` だけを
  見れば、`Counter` は真にモノイドであり、演算子が最も正確な表現になる。
  **裏を返せば、`strict` を `Counter` から外せば 1b が最善になる。**
  外す案（方針と集計状態を別の型に割る）は ADR-0005 論点 1 で
  「厳格さは消費者の方針」と決めた結果としてここにある。
  **本 ADR で覆さない** — 覆すなら ADR-0005 を supersede する新しい ADR が要る
- **1c の「総称に書けない」は将来効くか。** 効くとすれば、
  `Counter` 以外にも合流できる集計器が現れたときである。
  そのときは `merge` を持つ trait を自分で定義すればよく、
  **`Extend` を今から実装しておく必要は無い**（むしろ `Extend` は
  項目の型 `(String, u64)` を約束してしまい、後から変えられない）

### 決め手

**`strict` が `Counter` にある限り、`Counter` はモノイドではない。**
演算子（1b）はモノイドであることを記号で約束するので、
**約束と実体がずれる。** ずれは `Report` には出ないが、
`Sum` の単位元が方針を落とす形で現れる。

**`Extend`（1a）は、合流を 2 手順に割る。** `skipped` を別経路で足す実装は、
片方を忘れても型検査を通る。**忘れたことが `--stats` の出力にしか出ない。**

`merge`（1c）は糖衣が無いだけで、**間違いうる余地を作らない。**

### 決定の帰結

- **公開 API が 1 つ増える**（`Counter::merge`）。`Counter` は `Clone` を持たないので、
  `other` は消費される形になる
- **`strict` の非対称を doc に書く義務が生じる。**
  「`self` の方針を保つ」「`Report` は方針に依存しないので、
  合流の順序を変えても結果は同じ」の 2 つを書く
- **結合則と可換性はテストで担保する。** 演算子を実装しないので
  コンパイラは何も確かめない。`proptest` で
  「どう分割して合流しても `Report` が一致する」を示す
  （段階 6 の完了条件に入れた）
- 論点 2（入力 1 つぶんの集計が何を返すか）は、**`Counter` を返す形が
  成立することが確定した。** マージの手段が生えたため

## 確認方法（Confirmation）

論点を決めるたびに書き足す。

## 改訂履歴

- 2026-09-23: 起票（`proposed`）。論点の分割と現状の事実のみ
- 2026-09-23: 論点 1（`Counter` のマージの形）を決定
