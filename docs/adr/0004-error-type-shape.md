---
status: "proposed"
date: 2026-08-16
decision-makers: 学習者, Claude
consulted: 外部文献（下記「参照した文献」）
informed: —
---

# エラー型の形: 判別とフィールド取り出しの人間工学

**これは中間文書である。** 論点ごとに決定済みと未決定を明示し、
未決定のものは選択肢と帰結だけを置く。すべて決まった時点で `accepted` にする。

## コンテキストと問題提起

段階 5 で集計コアを `tally-core` として切り出す。**切り出した時点でエラー型の形が
外部に対する契約になる**ため、その前に形を決める。

現状の `TallyError` には 2 つの問題がある。

1. **行番号を持つ 3 バリアントのうち、抜粋を持つのは `MissingField` だけ。**
   「何行目がどう見えたか」に答えられるものと答えられないものが混在する
2. **`field` の型が `String` と `Box<str>` で揃っていない**

### 前提: 想定する利用者

**この CLI の想定は「自分または小規模プロジェクト専用の補助ツール」である。**
公開ライブラリではない。この前提が論点 2 の結論を決めるので、先に明示する。

**素性の知れない下流が存在しない場合、破壊的変更はコストではない。**
呼び出し側を全部握っているなら、シグネチャを変えたときにコンパイラが
修正箇所を列挙してくれる。これは損失ではなく作業リストである。

## 論点の分割

**各論点は独立に決められる。** 依存がある箇所だけ明記する。

| # | 論点 | 状態 | 結論 |
| --- | --- | --- | --- |
| 1 | 共有文脈を外側の構造体に括り出すか | **決定済み** | 括り出す（`LineError`） |
| 2 | `#[non_exhaustive]` を付けるか | **決定済み** | 付けない |
| 3 | 失敗の単位ごとに戻り値の型を分けるか | **未決定** | — |
| 4 | `serde_json::Error` を `#[source]` として公開するか | **未決定** | — |
| 5 | ライブラリのエラーと CLI 固有のエラーを分けるか | **決定済み** | 分ける（CLI 固有の型を持つ） |
| 6 | 対処の示唆（hint）を型で持つか | **未決定** | 置き場所は論点 5 で確保済み |
| 7 | エラー型の大きさの変化 | **未測定** | 測定義務。実装後に測る |

**本 ADR の範囲外**: `Selector` の公開 API（`#[non_exhaustive]` + コンストラクタ）は
別の判断であり、別 ADR で扱う。エラー型の形とは独立している。

---

## 論点 1: 共有文脈を外側の構造体に括り出すか（決定済み）

### 決定

**括り出す。** 行に紐づく失敗を `LineError` にまとめ、
共有する文脈（行番号・抜粋）を構造体のフィールドに、
判別と個別データを内側の `kind` enum に置く。

```rust
pub enum TallyError {
    OpenInput { path: PathBuf, source: io::Error },
    Read(io::Error),
    /// 行に紐づく失敗。**必ず行番号と抜粋を持つ**
    Line(LineError),
}

pub struct LineError {
    pub line_no: usize,
    pub snippet: Box<str>,
    pub kind: LineErrorKind,
}

pub enum LineErrorKind {
    InvalidJson { source: serde_json::Error },
    UnsupportedFieldType { field: Box<str> },
    MissingField { field: Box<str> },
}
```

### 却下した案: 各バリアントに `LineContext` を埋める

```rust
pub enum TallyError {
    InvalidJson { ctx: LineContext, source: serde_json::Error },
    UnsupportedFieldType { ctx: LineContext, field: Box<str> },
    MissingField { ctx: LineContext, field: Box<str> },
}
```

**同じ情報を持つが、取り出しの人間工学が違う。**

| やりたいこと | 各バリアントに埋める | 外側に括り出す（採用） |
| --- | --- | --- |
| 行番号を取る | **全バリアントを `match`** | `err.line_no`。`match` 不要 |
| 種類で分岐する | `match err` | `match err.kind` |
| 行エラーを 1 つ足す | 文脈フィールドを**書き忘れうる** | 書きようがない（構造体が持つ） |

**決め手は 3 行目。** 今回の問題（`MissingField` にだけ抜粋がある）は
まさに「足すときに書き忘れた」形で発生している。**同じ誤りが二度できない形**を選ぶ。

### 帰結

- **「行番号を持つが抜粋を持たない」状態が型として存在しなくなる**
- `match` が 2 段になる（`TallyError::Line(e)` → `e.kind`）
- `field` の型を `Box<str>` に統一する（`UnsupportedFieldType` が `String` だった）

---

## 論点 2: `#[non_exhaustive]` を付けるか（決定済み）

### 決定

**付けない。** enum にもバリアントにも構造体にも付けない。

### 根拠

`#[non_exhaustive]` は **素性の知れない下流を守るための機構**であり、
その対価として **下流から網羅性検査を取り上げる**（`_ =>` を強制する）。

想定利用者（前提を参照）では守るべき未知の下流がいない。
**対価だけを払って便益を受け取らない取引になる。**

### 帰結

- **バリアントを足すと呼び出し側がコンパイルエラーになる。** これは望ましい挙動として受け入れる
- 公開ライブラリに転用する場合、**後から `#[non_exhaustive]` を付けるのは破壊的変更**
  （Cargo Book の `attr-adding-non-exhaustive`）。転用時は最初に付け直す必要がある
- `std::io::Error` の「不透明構造体 + `kind()`」は採らない。
  **あの形が成立するのは `ErrorKind` が Copy でデータを持たないから**であり
  （matklad の分析）、`tally` の種類は `field` を伴うので前提が違う

---

## 論点 3: 失敗の単位ごとに戻り値の型を分けるか（未決定）

`Selector::select` は I/O では失敗しえないが、現在は `TallyError` を返している。
`LineError` を括り出す（論点 1）と、`Result<_, LineError>` に絞れるようになる。

**論点 1 を採っても、この変更をしない選択はできる。** 独立した判断である。

### 案 3a. 分ける（`select` は `LineError` を返す）

* Good, because **シグネチャが「この関数は I/O で失敗しない」を表明する。**
  読み手が本文を読まずに知れる
* Good, because 呼び出し側が `match` するとき、ありえないバリアントが選択肢に出ない
* Bad, because 型が 1 つ増え、`?` による変換（`From<LineError> for TallyError`）が要る
* Bad, because **失敗の単位が増えるたびに型が増える。** 小さな CLI では冗長になりうる

### 案 3b. 分けない（`select` は `TallyError` を返し続ける）

* Good, because 変更が最小。`?` がそのまま通る
* Good, because 型が 1 つで済み、`match` の書き口が 1 通り
* Bad, because **シグネチャが嘘をつく。** `select` が `OpenInput` を返すことはないのに、
  型はその可能性を表明している
* Bad, because 呼び出し側が「この関数は実際どれを返すのか」を本文で調べる羽目になる

### 未決定の帰結

**どちらでも論点 1 の利益は失われない。** 決定を先送りしても他の論点は進められる。

---

## 論点 4: `serde_json::Error` を `#[source]` として公開するか（未決定）

`LineErrorKind::InvalidJson { source: serde_json::Error }` は、
**`serde_json` を公開依存にする。**

### 案 4a. 公開したまま、安定 API でないと文書化する

* Good, because `{:#}` のエラーチェーンに **serde_json の詳細な位置情報**
  （行内の何文字目か）が乗る。利用者にとって実利がある
* Good, because 変更がゼロ
* Bad, because **`serde_json` が major を上げると、こちらも major を上げる必要が生じる。**
  自分のコードが 1 行も変わっていなくても
* Bad, because 文書化は自己申告であり、機械的な柵にならない

### 案 4b. 隠す（メッセージ文字列に変換して保持する）

* Good, because 公開依存が消え、依存の更新が自分の版に波及しない
* Bad, because **`source()` チェーンが切れる。** 原因を辿る手段が文字列だけになる
* Bad, because 位置情報を落とすか、自前で抜き出して持ち直すことになる

### 未決定の帰結

想定利用者（自分または小規模プロジェクト）では **4a の Bad 1 点目の損失が発生しない。**
公開ライブラリへ転用する場合にのみ効いてくる論点である。

---

## 論点 5: ライブラリのエラーと CLI 固有のエラーを分けるか（決定済み）

### 決定

**分ける。** CLI 固有のエラー型を持ち、**`(エラー) → 終了コード` を純粋関数にする**
（cargo の `CliError`、jj の `CommandError` と同じ形）。

**決め手はテストである。** 下の「テストへの影響」の表のとおり、
5a では終了コードの網羅がプロセス起動の数に比例し、
かつ **「テストを書き忘れる」が検出されない。**
5b なら変換の `match` が網羅性検査を受けるので、**忘れるとコンパイルが止まる。**
これは論点 1 で採ったのと同じ論法（同じ誤りが二度できない形を選ぶ）である。

**現状**: ライブラリは `TallyError`（`thiserror`）、バイナリは `anyhow::Error`。
終了コードは `main.rs` の `is_broken_pipe` のような述語関数で個別に判定している。

### 案 5a. 現状維持（`anyhow` + アドホックな判定）

* Good, because **追加の型がゼロ。** 失敗の経路が 3 つしかない現状に見合う
* Good, because `context()` でメッセージを積め、`{:#}` で連結表示できる
* Good, because 新しい失敗を足すのに型を触らなくてよい
* Bad, because **「どの失敗がどの終了コードか」の一覧が存在しない。**
  判定がコードに散り、述語関数が増える
* Bad, because **分類が `downcast_ref` に依存する。** 型が増えると連鎖が伸び、
  網羅性検査が効かない（漏れてもコンパイルは通る）
* Bad, because 対処の示唆（論点 6）を載せる場所が無い
* Bad, because **判定ロジックが `main.rs` にある。**
  これは `crates/tally/docs/layout.md` の「`main.rs` はロジックを持たない」に反する。
  論点 5 とは別に、いずれ直す対象

### 案 5b. CLI 固有のエラー型を持つ（cargo / jj の形）

* Good, because **`(エラー) → 終了コード` が純粋関数になる。**
  終了コードの決定を lib 側に置けるので、`main.rs` の方針違反も同時に解消する
* Good, because 終了コードの一覧が 1 箇所に集まり、
  `crates/tally/docs/output-format.md` の契約（`0` / `1` / `2`）と突き合わせられる
* Good, because 対処の示唆をフィールドとして持てる（論点 6）
* Good, because lib のエラーが増えたとき、**変換の `match` がコンパイルエラーで漏れを教える**
  （論点 2 で `#[non_exhaustive]` を付けないと決めたので、この検査が効く）
* Bad, because **型が 1 つ増える。** 現状の失敗の経路は 3 つしかない
* Bad, because `anyhow` の `context()` の手軽さを一部手放すか、
  両方を併用して**エラーの表現が 2 系統になる**
* Bad, because 変換関数の置き場所を決める必要がある（`cli.rs` か新規モジュールか）

### テストへの影響（この論点の実質)

**このリポジトリのテスト方針は「統合テストはそこでしか検証できないものだけ」。**
論点 5 はこの方針に直接効く。

| 検査したいこと | 案 5a | 案 5b |
| --- | --- | --- |
| 終了コードの決定規則（網羅） | **統合テストのみ。** 経路ごとにプロセス起動 | **ユニットテスト。** 純粋関数に入力を並べる |
| 終了コードが実際に返ること | 統合テスト | 統合テスト（**1〜2 件で足りる**） |
| BrokenPipe を成功として扱う | 統合テスト（パイプの構成が要る） | ユニットテストで規則を、統合で配線を |
| 新しい失敗を足したときの漏れ | **テストを書き忘れると気づけない** | **変換の `match` がコンパイルエラー** |

**要点は 2 つ。**

1. **5a では終了コードの網羅がプロセス起動の数に比例する。**
   失敗の経路が増えるほどテストが遅くなる。段階 6 以降で経路は増える見込み
2. **5a では「テストを書き忘れる」が検出されない。** 5b では
   変換の `match` が網羅性検査を受けるので、**忘れるとコンパイルが止まる。**
   これは論点 1 で採ったのと同じ論法（「同じ誤りが二度できない形を選ぶ」）

**ただし 5a でもユニットテスト自体は不可能ではない。**
`main.rs` に `#[cfg(test)] mod tests` を書けば実行される。
**問題は書けないことではなく、書くとロジックが `main.rs` に居座ることを追認する点にある。**

### 決定の帰結

- **終了コードの決定規則がユニットテストで網羅できる。**
  統合テストは「配線されていること」の確認だけに減らす
- **`main.rs` の方針違反（`is_broken_pipe` というロジック）が同時に解消する。**
  `crates/tally/docs/layout.md` の「`main.rs` はロジックを持たない」に戻る
- **終了コードの一覧が 1 箇所に集まり**、
  `crates/tally/docs/output-format.md` の契約（`0` / `1` / `2`）と突き合わせられる
- **論点 6（hint）の置き場所が確保された。** ライブラリ側に
  「`--strict` を外せ」のような CLI の語彙が漏れる心配がなくなる
- 型が 1 つ増える。変換関数の置き場所を決める必要がある（実装時に決める）
- **`anyhow` を残すか、CLI 固有の型に一本化するかは実装時の判断。**
  併用するとエラーの表現が 2 系統になるので、そこは意識して決める

---

## 論点 6: 対処の示唆（hint）を型で持つか（未決定・置き場所は確保済み）

jj は `CommandError` に **hint**（利用者が次に何をすればよいか）を持たせている。
「何が起きたか」ではなく「どうすればよいか」を型のフィールドにする発想。

例: `--strict` でフィールド欠損に当たったとき、
「`--strict` を外すとこの行はスキップされます」を添えられる。

* Good, because 診断の質が上がる。**CLI の価値の大半はエラーメッセージにある**
* Good, because 「示唆を持つ場所」が型にあると、書き忘れが目に見える
* Bad, because 文言の保守対象が増える
* Bad, because 示唆の無い失敗にも `Option` の分岐が付いて回る

### 未決定の帰結

**置き場所の問題は論点 5 の決定（5b）で解消した。** CLI 固有の型に載せればよく、
ライブラリ側に CLI の語彙は漏れない。残るのは「持つか持たないか」だけ。

---

## 論点 7: エラー型の大きさ（未測定）

段階 2 で **`Result<T, E>` の大きさは最大バリアントで決まり、
その `Result` は 1 行につき 3 段返る**ことを実測した
（`docs/learning-log.md` 節 6-1）。**失敗しない行もこの大きさを払う。**

論点 1 の変更は各バリアントの構成を変えるため、大きさが変わる。

**憶測で書かない。** `size_of::<TallyError>()` と `size_of::<LineError>()` を
実装後に測り、変更前の値と並べて記録する。**測る前にこの節へ数値を書かない。**

---

## 確認方法（Confirmation）

1. 論点 1 の実装後、**「行番号を取る」コードが `match` を含まないこと**をコードで示す
2. 論点 3 を採る場合、`Selector::select` のシグネチャが
   `Result<_, LineError>` になっていること
3. 論点 5 で 5b を採る場合、**終了コードの決定規則がユニットテストで網羅されていること**。
   統合テストは配線の確認だけに減らす
4. 論点 7 の実測値を記録する

段階 5 の完了条件は `docs/curriculum.md` が正本。ここには重複させない。

## 参照した文献

- **[Modular Errors in Rust — Sabrina Jewson](https://sabrinajewson.org/blog/errors)**
  論点 1 の形（外側の構造体に共有文脈、内側の `kind` に判別）と論点 3 の論拠。
  「巨大な catch-all enum は、その関数が実際にどのエラーを返すかを表現できない」
- **[Study of std::io::Error — matklad](https://matklad.github.io/2020/10/15/study-of-std-io-error.html)**
  不透明構造体 + `kind()` を **採らない**理由。
  あの形は `ErrorKind` が Copy でデータを持たないから成立する
- **[Error type design — Rust Error Documentation (nrc)](https://nrc.github.io/error-docs/error-design/error-type-design.html)**
  論点 4。「内部のエラーは API 定義ではない。晒すなら安定 API でないと文書化せよ」
- **[`cargo::util::errors::CliError`](https://docs.rs/cargo/latest/cargo/util/errors/struct.CliError.html)**
  論点 5b。lib のエラーを CLI 層で包み、終了コードを型に載せる
- **[jj の `CommandError`](https://deepwiki.com/jj-vcs/jj/5-developer-guide)**
  論点 5b・6。種類 → 終了コードの対応と hint
- **[Exit codes — Command Line Applications in Rust](https://rust-cli.github.io/book/in-depth/exit-code.html)**
  終了コードの慣習
- **[SemVer Compatibility — The Cargo Book](https://doc.rust-lang.org/cargo/reference/semver.html)**
  論点 2 の `attr-adding-non-exhaustive`

関連: [ADR-0001](0001-record-architecture-decisions.md)、
`crates/tally/docs/layout.md`、`crates/tally/docs/output-format.md`、
`docs/learning-log.md` 節 6-1。

## 改訂履歴

| 版 | 日付 | 種別 | 内容 |
| --- | --- | --- | --- |
| 1 | 2026-08-16 | 初版（中間文書） | 論点を 7 つに分割。1・2 を決定済み、3〜6 を未決定、7 を未測定として記録 |
| 2 | 2026-08-16 | 決定の追加 | 論点 5 を決定（分ける / 5b）。決め手はテスト。論点 6 の置き場所の依存が解消 |
