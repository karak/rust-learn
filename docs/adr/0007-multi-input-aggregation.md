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
| 3 | 失敗にどのファイルかを持たせる場所（core か CLI か。`OpenInput` との二重） | **決定済み** | — |
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

## 論点 3: 失敗にどのファイルかを持たせる場所（決定済み）

### 決定

**CLI 側で包む。同時に `TallyError::OpenInput` を `tally-core` から取り除き、
CLI へ移す。**

```rust
// crates/tally-core: 開く失敗が消える。残るのは「読めない」と「行が悪い」だけ
pub enum TallyError {
    Read(std::io::Error),
    Line(LineError),
}

// crates/tally: 入力の呼び名を持つのは、入力を開いた側
pub enum InputName { Stdin, Path(PathBuf) }

pub enum CliErrorKind {
    /// 開けなかった。**標準入力では起きない**ので `PathBuf` を直接持つ
    Open { path: PathBuf, source: io::Error },
    /// 集計に失敗した。**どの入力かを必ず持つ**
    Input { name: InputName, source: TallyError },
    Write(io::Error),
}
```

**これは [ADR-0004](0004-error-type-shape.md) 論点 1 の
`TallyError` の形と、Confirmation の判明事項 2 を置き換える。**
ADR-0004 側に `Superseded by` の指し先を書いた（[ADR-0001](0001-record-architecture-decisions.md) の手順）。

### この論点に効く事実（2026-09-23 に確認）

| 事実 | 効き方 |
| --- | --- |
| `tally-core` の不変条件は「**ファイルを開かない**」（`lib.rs` のモジュールドキュメント） | **`OpenInput` はその不変条件と既に食い違っている** |
| `OpenInput` を構築している本番コードは `crates/tally/src/main.rs` の 1 箇所（`File::open` の `map_err`） | **構築しているのは CLI だけ。** core 内に構築点が無い |
| `TallyError::Read` と `TallyError::Line` は path を持たない | 読み取り途中の失敗と行の失敗が、どの入力かを言えない |
| `LineError::line_no` は **入力ごとに 1 から数える** | path が無いと **行番号まで意味を失う**（「3 行目」がどのファイルか言えない） |
| `hint_for` は `TallyError` を受け、`OpenInput` に対して `None` を返す | `OpenInput` を移すと、この枝が CLI 側の `match` へ移る |
| 標準入力には path が無い | 「どの入力か」は `PathBuf` では表せない場合がある |

### 検討した案

#### 3a. CLI 側で包む（採用。ただし単体では二重表示が残る）

`CliErrorKind::Input { name, source: TallyError }` を足し、表示で名前を前置する。

- 利点: **core は `BufRead` しか知らないまま。** 不変条件を強めこそすれ弱めない
- 利点: 標準入力を `InputName::Stdin` として同じ型で扱える。
  **`PathBuf` を `Option` にする必要が無い**
- 欠点（**素のままの 3a**）: `OpenInput` が core に残っていると、
  「開けなかった」に名前を前置したときに **path が 2 度出る**。
  段階 5 で transparent を選んだ理由がまさにこれで、
  **`Input` で包むか否かをバリアントごとに場合分けする**羽目になる
- **→ `OpenInput` を CLI へ移すと、この欠点が消える。**
  移した先では `Open { path, .. }` が自分で path を表示し、
  `Input { name, .. }` が名前を前置する。**場合分けが型の形で消える**

#### 3b. core の `TallyError` に path を足す（却下）

`Read` と `Line` にも `path: PathBuf` を持たせ、1 つの型で自己完結させる。

- 利点: **消費者が何もしなくても、失敗が自己完結する。**
  包み忘れという失敗様式が存在しない
- 利点: `LineError` を直接受け取る文脈（ADR-0004 論点 3 の帰結）でも
  どの入力かが分かる
- 欠点: **`BufRead` しか受け取らない関数が path を要求することになる。**
  `tally_reader` の呼び出し側は、メモリ上の文字列やソケットを渡せる。
  そのとき path に何を入れるかが決まらない（`Option<PathBuf>` にすると
  「持つが空」という状態が生まれ、ADR-0004 論点 1 が排除した形に戻る）
- 欠点: **`tally-core` が `std::path` に依存する理由が無くなる。**
  `OpenInput` を移せば、core から `PathBuf` が消える
- 欠点: 行の失敗すべてに `PathBuf` が載るので、`LineError` が太る
  （ADR-0004 論点 7 で大きさを実測した経緯がある）

#### 3c. core に「入力の名前」の層を足す（却下）

`TallyError::Input { name: Box<str>, source: Box<TallyError> }` のように、
core 側で名前を預かる層を作る。

- 利点: **名前の付与が core の型で表現される。** 消費者が自前で層を作らずに済み、
  `tally` 以外の消費者（linter、連携の入り口）でも同じ形になる
- 利点: 名前が文字列なので、ファイルでもソケットでも URL でも載る
- 欠点: **`Box<TallyError>` の再帰になる。** 入れ子の深さが型で決まらず、
  「名前の層が 2 つある `TallyError`」が作れてしまう
- 欠点: **`OpenInput` の path と意味が重なる。** 同じ入力について
  path と name が別々に載りうる（3a で二重表示を嫌ったのと同じ問題が、
  型の中に移るだけ）
- 欠点: core が名前を持っても、**名前を知っているのは開いた側**である事実は変わらない。
  構築点は結局 CLI になる

### 対称性の検査

- **3b の「包み忘れが存在しない」は本当に利点か。** 利点である。
  3a は `CliErrorKind::Input` で包む責任を呼び出し側に置くので、
  **包み忘れても型検査を通る。** これは実際の弱点であり、
  採用案が 3b に負けている唯一の軸である。
  **緩和策**: 入力を開いて集計する経路を 1 つの関数に閉じ、
  そこを通らずに `tally_reader` を呼ばない形にする。
  **テストで担保する**（複数入力のどれが失敗しても path が出る）
- **3a の「core は `BufRead` しか知らない」は、実際に守られているか。**
  いまは守られていない（`OpenInput` がある）。
  **採用案はその違反を消す方向なので、この利点は「守る」ではなく「直す」である。**
  段階 5 では「同じ分類に載せるための置き場所」として意図的に残した
  （`error.rs` のドキュメントにそう書いてある）。
  **その意図は、入力が 1 つで、開く主体も 1 つだったときには成立していた**
- **`OpenInput` を移すのは破壊的変更である。** `tally-core` の公開 API から
  バリアントが 1 つ消える。**1.0 以前なので受け入れる**（ADR-0004 の但し書き）。
  ただし `TallyError` は `#[non_exhaustive]` ではない（ADR-0004 論点 2）ので、
  **消費者の `match` はコンパイルエラーになる。** 黙って壊れはしない

### 決め手

**`OpenInput` は、置き場所を間違えていた。**
構築しているのは CLI だけで、core のどこからも作られない。
**`tally-core` が「ファイルを開かない」と宣言している以上、
「開けなかった」は core の語彙ではない。**

段階 5 の transparent の判断は「path が 2 度出るのを避ける」ためだった。
**その二重は、置き場所の誤りが表示に現れたものだった。**
置き場所を直すと、表示の場合分けは書かずに済む。

### 決定の帰結

- **`tally-core` から `PathBuf` が消える**（`OpenInput` が唯一の使用箇所だった）
- **`CliErrorKind` が 2 つ増える**（`Open` と `Input`）。
  `Tally` は消える。終了コードの `match` は網羅性検査で全件洗い出される
- **`hint_for` の引数が変わる。** `OpenInput` の枝が CLI 側へ移る
- **表示が変わる。** 「`path` の集計に失敗しました: 3 行目 ...」のように、
  入力の名前が前置される。`crates/tally/docs/output-format.md` は
  **stderr の文面を契約にしていない**ので、外部契約は変わらない
- **包み忘れをテストで担保する。** 3b に負けている軸なので、
  「どの入力が失敗しても名前が出る」を統合テストで押さえる
- 論点 4（複数の失敗のどれを報告するか）は、**`CliErrorKind::Input` が
  名前を持つことを前提にできる**

## 確認方法（Confirmation）

論点を決めるたびに書き足す。

## 改訂履歴

- 2026-09-23: 起票（`proposed`）。論点の分割と現状の事実のみ
- 2026-09-23: 論点 1（`Counter` のマージの形）を決定
- 2026-09-23: 論点 3（失敗にどのファイルかを持たせる場所）を決定。**ADR-0004 論点 1 の `TallyError` の形と Confirmation の判明事項 2 を supersede する**
