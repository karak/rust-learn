---
title: 11 エラー型の設計と公開 API
sidebar_position: 11
---

# 11 エラー型の設計と公開 API

5 章の `CountError` は、同じファイルの中でしか使わない型だった。
9 章では `From` を実装して、[`?`](../glossary.md#question-mark) に失敗を包ませた。
10 章では、エラーの型に `fn source(&self) -> Option<&(dyn Error + 'static)>` という形が出てくることだけを見た。
この章では、失敗の型を、別の[クレート](../glossary.md#crate)から使われるものとして組む。
使う側は、その型で分岐し、表示し、原因をたどる。
何を見せて何を隠すかを決めるのが、この章の主題である。

C++ の[例外](../glossary.md#exception)は、慣習として `std::exception` から派生させ、`what()` で文を返す。
別の例外を原因として包むなら `std::throw_with_nested` で投げ、`std::rethrow_if_nested` で原因を投げ直して受け止める。
Java の `getCause()` や C# の `InnerException` も、外側の失敗から原因を指す仕組みである。
Rust にも原因を指す仕組みがあり、それが冒頭の `source()` である。
違うのは、失敗が例外ではなく [`Result`](../glossary.md#result) の値として返ること（5 章）と、失敗の型が基底クラスを持たないことである。
失敗の型は、`Error` という [trait](../glossary.md#trait) を実装したただの [enum](../glossary.md#enum) や[構造体](../glossary.md#struct)で、関数の型に書いたものだけが呼び手に見える。

## この章のコードは、状態コードを数え、失敗を行番号つきで返す

この章のコード（`book/code/ch11`）は、`code=404` の形の欄を持つ行から状態コードを読み、コードごとの件数を数える。
6 章と同じく、ライブラリ（`src/lib.rs`）と、それを使う実行ファイル（`src/main.rs`）を別のクレートに分けている。

```rust
pub enum CountError {
    Read(io::Error),
    Line(LineError),
}

pub struct LineError {
    pub line_no: usize,
    pub kind: LineErrorKind,
}

pub enum LineErrorKind {
    MissingCode,
    BadCode(ParseIntError),
}

pub fn count_codes(input: impl BufRead) -> Result<BTreeMap<u16, u64>, CountError>
pub fn one_line(err: &(dyn Error + 'static)) -> String
```

型に付いている[属性](../glossary.md#attribute)は、あとの節で順に見る。
`cargo run -p book-ch11` の出力はこうなる。

```text
{200: 2, 404: 1}
2 行目: 欄 `code` がありません
  Line(LineError { line_no: 2, kind: MissingCode })
  分類: 欄が無い
2 行目: 状態コードとして読めません: invalid digit found in string
  Line(LineError { line_no: 2, kind: BadCode(ParseIntError { kind: InvalidDigit }) })
  分類: 数でない
入力の読み取りに失敗しました: stream did not contain valid UTF-8
  Read(Error { kind: InvalidData, message: "stream did not contain valid UTF-8" })
  分類: 読み取り
```

1 行目は、読めた入力の件数である。
残りは、`code=` の欄が無い行、値が数でない行、[UTF-8](../glossary.md#utf-8) でないバイトを含む入力の 3 つを読ませた結果である。
失敗ごとに、利用者に見せる 1 行、開発者に見せる `{:?}` の表示、`main.rs` が `match` で付けた分類を出している。

## `Error` trait が約束するのは、表示と原因だけである

標準ライブラリの [`std::error::Error`](../glossary.md#error-trait) は、失敗を表す型が実装する trait である。
宣言はこうなっている（説明に要らない部分を省いた）。

```rust
pub trait Error: Debug + Display {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
```

`Error: Debug + Display` は、`Error` を実装する型は `Debug` と `Display` も実装していなければならない、という意味である。
`Display` は `{}` で出す利用者向けの文で、`Debug` は `{:?}` で出す開発者向けの表示である。
`Debug` を実装せずに `Error` だけを実装すると、通らない。

```
error[E0277]: `E` doesn't implement `Debug`
  --> src/bin/d.rs:4:28
   |
 4 | impl std::error::Error for E {}
   |                            ^ the trait `Debug` is not implemented for `E`
```

（このメッセージは、同じ形の小さな例で出したもので、後ろに続く note と help は省いた。）

`source` は原因の失敗を返す[メソッド](../glossary.md#method)で、既定の実装は「原因は無い」を表す `None` を返す。
つまり、`Debug` と `Display` があれば、`impl Error for 型 {}` の 1 行で `Error` を実装できる。
C++ の `std::exception` が `what()` を仮想関数として持つのと違い、`Error` は基底クラスではない。
フィールドも持たず、表示と原因の 2 つの約束だけを足す。

この章のコードでは、行に紐づく失敗 `LineError` の `Display` と `Error` を手で書いている。

```rust
impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} 行目: {}", self.line_no, self.kind)
    }
}

impl Error for LineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.kind.source()
    }
}
```

`source` が `kind` そのものではなく `kind.source()` を返しているのには理由がある。
それは「表示に埋めた原因を `source()` でも指すと、つないだときに 2 度出る」の節で見る。

## `thiserror` は、`Display` と `Error` と `From` を属性から書く

`Display` と `Error` を型ごとに手で書くのは、ほとんどが決まった形の繰り返しになる。
[`thiserror`](../glossary.md#thiserror) は、それを属性から書かせるクレートである。
標準ライブラリの外にあるので、`Cargo.toml` の `[dependencies]` に足す。
手元で試すなら `cargo add thiserror` で足せる。
この章で使うのはバージョン 2 である。
6 章と 9 章の [derive](../glossary.md#derive) と同じく、`#[derive(thiserror::Error)]` と書くと実装が作られる。

```rust
#[derive(Debug, thiserror::Error)]
pub enum CountError {
    #[error("入力の読み取りに失敗しました")]
    Read(#[from] io::Error),
    #[error(transparent)]
    Line(#[from] LineError),
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LineErrorKind {
    #[error("欄 `code` がありません")]
    MissingCode,
    #[error("状態コードとして読めません")]
    BadCode(#[from] ParseIntError),
}
```

バリアントに付けた属性が、それぞれ次の実装になる。

- `#[error("...")]` は、そのバリアントの `Display` の文になる。`{0}` や `{line_no}` と書けば、フィールドの値を埋め込める
- `#[source]` をフィールドに付けると、`source()` がそのフィールドを返す
- `#[from]` は `#[source]` の働きに加えて、そのフィールドの型からの `From` を実装する
- `#[error(transparent)]` は、表示も原因も内側の値にそのまま任せる。この層は何も足さず、`source()` は内側の値そのものではなく、内側の値の `source()` を返す

`#[from]` があるので、`?` が失敗を包む。
1 行から状態コードを読む `parse_line` は、`LineErrorKind` を返す。

```rust
fn parse_line(line: &str) -> Result<u16, LineErrorKind> {
    let value = line
        .split_whitespace()
        .find_map(|pair| pair.strip_prefix("code="))
        .ok_or(LineErrorKind::MissingCode)?;
    Ok(value.parse()?)
}
```

`find_map` は、[クロージャ](../glossary.md#closure)が `Some` を返した最初の要素について、その中身を返す。
`strip_prefix("code=")` は、先頭が `code=` ならそれを除いた残りを `Some` で返す。
欄が無ければ `None` になり、5 章で見た `ok_or` が `MissingCode` に変える。
最後の `value.parse()?` は `ParseIntError` を返しうるが、`#[from]` が `From<ParseIntError>` を実装したので、`?` が `BadCode` に包んで返す。

数える側の `count_codes` では、2 種類の包み方が並ぶ。

```rust
pub fn count_codes(input: impl BufRead) -> Result<BTreeMap<u16, u64>, CountError> {
    let mut counts = BTreeMap::new();
    for (index, line) in input.lines().enumerate() {
        let line = line?;
        let code = parse_line(&line).map_err(|kind| LineError {
            line_no: index + 1,
            kind,
        })?;
        *counts.entry(code).or_insert(0) += 1;
    }
    Ok(counts)
}
```

`lines()` は 1 行ずつ `Result<String, io::Error>` を返す[イテレータ](../glossary.md#iterator)である。
`line?` の `io::Error` は、`#[from]` によって `CountError::Read` に包まれる。
`parse_line` の失敗のほうは、`map_err` で先に `LineError` にしてから `?` に渡している。
`?` は、その `LineError` を `#[from]` によって `CountError::Line` に包む。

## `#[from]` で包めるのは、元の型だけで行き先が決まるときである

9 章は、`From` でどこまで自動で包むかを、この章に送っていた。
`LineErrorKind` を `LineError` にするところで `map_err` を書いたのは、`From` では書けないからである。
`From::from` が受け取るのは元の失敗だけで、行番号は渡ってこない。
行番号を知っているのは、行を数えている `count_codes` である。
`thiserror` に、行番号を持つバリアントへの `#[from]` を書かせようとすると、断られる。

```
error: deriving From requires no fields other than source and backtrace
 --> src/bin/c.rs:5:31
  |
5 |     BadCode { line_no: usize, #[from] source: ParseIntError },
  |                               ^^^^^^^
```

もう 1 つの限界は、同じ型から 2 つのバリアントへ包めないことである。
たとえば、ファイルを開く失敗と読む失敗は、どちらも `io::Error` で返ってくる。
両方のバリアントに `#[from]` を付けると、`From<io::Error>` の実装が 2 つになってぶつかる。

```
error[E0119]: conflicting implementations of trait `From<std::io::Error>` for type `CountError`
 --> src/bin/b.rs:7:12
  |
5 |     Open(#[from] io::Error),
  |            ---- first implementation here
6 |     #[error("読めません")]
7 |     Read(#[from] io::Error),
  |            ^^^^ conflicting implementation for `CountError`
```

開いたのか読んだのかは、失敗の型ではなく、失敗した場所で決まる。
だから、その場所で `map_err` を書いて、行き先のバリアントを選ぶ。

`#[from]` で `?` に任せてよいのは、元の失敗の型だけで行き先のバリアントが 1 つに決まり、足す情報が無いときである。
行番号やファイル名のような文脈を足すときと、同じ型の失敗を場所で分けたいときは、`map_err` を書く。
`map_err` は、失敗がどこで包まれたかをコードの上に残す書き方でもある。

## 表示に埋めた原因を `source()` でも指すと、つないだときに 2 度出る

`source()` をたどると、失敗の連鎖を外側から順に取り出せる。
この章のコードは、それを 1 行につなぐ関数を持っている。

```rust
pub fn one_line(err: &(dyn Error + 'static)) -> String {
    let mut shown = Vec::new();
    let mut current = Some(err);
    while let Some(err) = current {
        shown.push(err.to_string());
        current = err.source();
    }
    shown.join(": ")
}
```

`while let Some(err) = current` は、`current` が `Some` のあいだ中身を `err` に取り出して繰り返し、`None` になったら抜ける。
7 章の `if let` の繰り返し版である。

`LineError` を `thiserror` で素直に書くと、こうなる。

```rust
#[derive(Debug, thiserror::Error)]
#[error("{line_no} 行目: {kind}")]
struct LineError {
    line_no: usize,
    #[source]
    kind: LineErrorKind,
}
```

これで数でない行を読ませて、`one_line` でつなぐと、同じ文が 2 度出る。

```text
2 行目: 状態コードとして読めません: 状態コードとして読めません: invalid digit found in string
```

`LineError` の文に `kind` の文を埋め込んだうえで、`source()` でも `kind` を指しているからである。
連鎖の各段は、内側の文を自分の文に埋め込むか、`source()` で指すかの、どちらか一方だけにする。

この章の `LineError` は、`kind` の文を埋め込み、`source()` では `kind` を飛ばしてその原因を指している。
埋め込む側を選んだのは、`LineError` を 1 つだけ `to_string()` しても、行番号と何が起きたかがそろうようにするためである。
連鎖をたどらずに 1 件だけ記録する使い方でも、文が欠けない。
外側の `CountError::Line` は `#[error(transparent)]` で、表示にも連鎖にも何も足さない。
その結果、つないだ 1 行は `2 行目: 状態コードとして読めません: invalid digit found in string` になる。

連鎖をつなぐ関数は自分で書いた。
安定版の標準ライブラリには、連鎖をたどって表示する型（`std::error::Report`）がまだ無く、使おうとすると `E0658`（use of unstable library feature）で断られる。
`main` から `Result` を返すと失敗を表示してくれるが、出るのは `Display` ではなく `Debug` の表示で、`source()` はたどらない。

```rust
fn main() -> Result<(), Box<dyn Error>> {
    let n: u16 = "200".parse()?;
    println!("{n}");
    count_codes("code=x\n".as_bytes())?;
    Ok(())
}
```

`count_codes` が失敗すると、標準エラー出力にこう出て、終了コードは 1 になる。

```text
Error: Line(LineError { line_no: 1, kind: BadCode(ParseIntError { kind: InvalidDigit }) })
```

内側の `ParseIntError` が見えているのは、derive した `Debug` がフィールドを全部出すからで、連鎖をたどったからではない。
利用者に見せる文は、`main` の中で自分で出すことになる。

## `Box<dyn Error>` に入れると、使う側は型で分岐できなくなる

上の `main` は、`ParseIntError` と `CountError` という別々の型の失敗に、同じ `?` を付けている。
標準ライブラリが、`Error` を実装したどの型からも `Box<dyn Error>` への `From` を用意しているからである。
9 章で見た [`dyn`](../glossary.md#dyn) で型を 1 つにまとめるので、どんな失敗でも包める。

その代わり、使う側には `dyn Error` としか見えない。
表示と `source()` はできるが、どの型の失敗かを `match` で分けられない。
コンパイラは、中にどんな型が入りうるかを知らないので、[網羅性検査](../glossary.md#exhaustiveness)もかからない。
型を調べる手段は、`downcast_ref` のようなメソッドだけである。
`err.downcast_ref::<ParseIntError>()` は、中身が `ParseIntError` なら `Some` でその参照を返し、違えば `None` を返す。
これを[ダウンキャスト](../glossary.md#downcast)と呼ぶ。
C++ の `dynamic_cast` を、型を 1 つずつ当てて試すのに近い。
ただし、`downcast_ref` が当たるのは中身の型そのものだけで、中身が実装している別の trait の `dyn` としては取り出せない。

`downcast_ref` は、`'static` の付いた `dyn Error` にしか呼べない。
`&dyn Error` と書いた引数の中身の[寿命](../glossary.md#lifetime)は、外側の参照と同じになり、`'static` ではない。
そのまま `downcast_ref` を呼ぶと、こう断られる。

```
error[E0521]: borrowed data escapes outside of function
 --> src/bin/g.rs:3:40
  |
3 | fn is_parse(err: &dyn Error) -> bool { err.downcast_ref::<ParseIntError>().is_some() }
  |             ---  -                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |             |    |                     |
  |             |    |                     `err` escapes the function body here
  |             |    |                     argument requires that `'1` must outlive `'static`
  |             |    let's call the lifetime of this reference `'1`
  |             `err` is a reference that is only valid in the function body
```

ダウンキャストは型ごとの識別子（`TypeId`）で型を見分けるが、この識別子は `'static` の型にしか作れないためである。
`source()` の戻り値が `&(dyn Error + 'static)` になっているのは、受け取った側がダウンキャストできるようにするためである。
`one_line` は中身を調べないので、引数を `&dyn Error` と書いても通る。
`source()` が返すものと同じ型にそろえて、`&(dyn Error + 'static)` と書いている。

`Box<dyn Error>` は、失敗を表示して終わる場所、たとえば小さなツールの `main` では手軽である。
アプリケーションの層では、同じ考え方に文脈の文を足せるようにした `anyhow` というクレートもよく使われる。
使う側が失敗の種類で分岐するなら、具体的な enum を返す。
ライブラリは、使う側が何をするかを知らないので、分岐できる形で返す。

## 公開した enum は、使う側の `match` を縛る

`main.rs` は、ライブラリとは別のクレートとして、公開されたエラー型を `match` している。

```rust
fn classify(err: &CountError) -> &'static str {
    match err {
        CountError::Read(_) => "読み取り",
        CountError::Line(line) => match line.kind {
            LineErrorKind::MissingCode => "欄が無い",
            LineErrorKind::BadCode(_) => "数でない",
            _ => "その他の行の失敗",
        },
    }
}
```

`CountError` には `_` の腕が無く、`LineErrorKind` にはある。
4 章で見たとおり、`#[non_exhaustive]` の付いた enum を別のクレートで `match` するには、全部を挙げても `_` が要る。
`LineErrorKind` には付けてあり、`CountError` には付けていない。

この 2 つは、バリアントが増えたときに使う側に何を起こしたいかで決めている。
`CountError` は「読めなかった」か「行がおかしかった」かという大きな分け方で、増えにくい。
増えたなら、使う側の `match` がコンパイルエラーになって、新しい失敗の扱いを決めさせたい。
実際、`CountError` に似せた enum にバリアントを 1 つ足すと、別のクレートの `match` は `E0004`（non-exhaustive patterns）で止まった。
`LineErrorKind` は、行の読み方の規則が増えるたびに伸びる。
そのたびに使う側を壊さないよう、`_` を書かせておく。
代わりに、新しいバリアントは使う側の `_` の腕に黙って入る。

Scala の `sealed trait` や Java の `sealed` インタフェースも、`match` や `switch` の網羅性を検査させる仕組みである。
Rust では、検査が要るかどうかを型ごとに選び、外のクレートに対してだけ `_` を求める。
定義したクレートの中では、`LineErrorKind` も全部を挙げれば `_` なしで通る。

## 本物ではどうなっているか

`tally` の集計の部分（`tally-core`）は、この章のコードと同じ 3 段の形をしている。

```rust
#[derive(Debug, thiserror::Error)]
pub enum TallyError {
    #[error("入力の読み取りに失敗しました")]
    Read(#[from] std::io::Error),
    #[error(transparent)]
    Line(#[from] LineError),
}
```

`LineError` は行番号に加えて行の先頭 40 文字の抜粋を持ち、`Display` と `Error` を手で書いて、`source()` で `kind` を飛ばしている。
`TallyError` には `#[non_exhaustive]` が無く、内訳の `LineErrorKind` には付いている。

行を JSON として読めなかったときの原因は、JSON を読むクレート `serde_json` の `serde_json::Error` である。
これをバリアントのフィールドにそのまま置くと、`serde_json` が `tally-core` の[公開依存](../glossary.md#public-dependency)になる。
enum のバリアントのフィールドには可視性を付けられず、常に公開されるからである。
`tally-core` が `serde_json` の互換性を壊すバージョンに移ると、`tally-core` も互換性を壊すバージョンを出すことになる。
`serde_json` の型が `tally-core` の API の一部だからである。
使う側は、自分のコードを変えていなくても、移行の作業を負う。

そこで、6 章の [newtype](../glossary.md#newtype) で包んで、フィールドを隠している。

```rust
#[derive(Debug)]
pub struct JsonError(serde_json::Error);

impl JsonError {
    pub(crate) fn new(source: serde_json::Error) -> Self {
        Self(source)
    }
}

impl Error for JsonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
```

隠し方は 3 か所に表れている。
フィールドの `serde_json::Error` は非公開である。
包む関数を `From` の実装ではなく `pub(crate)` の関連関数にしているのは、`impl From<serde_json::Error> for JsonError` を公開すると、trait の実装の型に `serde_json` の型が出てしまうからである。
`source()` が `None` を返すのは、中身を返すと、使う側が `downcast_ref::<serde_json::Error>()` で取り出せてしまうからである。
型に名前を出さなくても、取り出せるなら依存は成り立ってしまう。
代わりに、`JsonError` の `Display` は `serde_json` の文から「at line 1 column 2」のような位置を外し、「（2 バイト目）」と言い直して出す。
1 行ずつ渡しているので、`serde_json` が言う行番号は常に 1 で、`tally` の「何行目」と食い違うからである。

コマンドラインの側（`tally`）は、別のエラー型 `CliError` を持つ。
入力の名前の文脈を足すのは `map_err` で、`From` には任せていない。

```rust
tally_reader(&mut counter, reader, selector, keep)
    .map_err(|source| input_error(input.name(), source))?;
```

`input_error` は、入力の名前と `TallyError` を `CliErrorKind::Input { name, source }` に入れる。
行番号は入力ごとに 1 から数えるので、どの入力かが言えないと行番号も役に立たない。
そのうえで、利用者が次に何をすればよいかの一言（hint）を `CliError` に添える。
2 行目にフィールドの無い入力を `--strict` で読ませると、こう出る。

```text
error: in.jsonl の集計に失敗しました: 2 行目: フィールド `lvl` を取り出せません: "{\"x\":2}"
hint: --strict を外すと、この行はスキップされます
```

`error:` の行は、この章の `one_line` と同じ関数がつないだものである。
4 章で見た終了コードの `match` も、この `CliError` に対して書かれている。
書き出し先のパイプが閉じた失敗を成功として扱う判定では、連鎖を 1 段ずつたどり、`downcast_ref::<io::Error>()` で `BrokenPipe` を探している。
`io::Error` が別の失敗に包まれて届いても見つけられるようにするためである。

## 演習

答えは `book/code/ch11/src/exercises.rs` にある。
`cargo test -p book-ch11` で確かめられる。

1. 欄の値を状態コードとして読む関数 `parse_code(value: &str) -> Result<u16, CodeError>` を書く。
   `CodeError` は `thiserror` で作り、数として読めない場合と、100 から 599 の範囲の外の場合を分ける。
   数として読めない場合は `ParseIntError` を原因に持ち、`?` で包めるようにする。
   範囲の外の場合は原因を持たず、`700 は 100 から 599 の範囲にありません` のように値を文に埋め込む。
2. 失敗の連鎖をいちばん奥までたどり、最後の失敗を返す関数 `root_cause` を書く。
   引数と戻り値の型は `&(dyn Error + 'static)` にし、原因が無ければ引数そのものを返す。
   `count_codes` に数でない行を読ませた失敗を渡し、返った値が `ParseIntError` であることを `downcast_ref` で確かめる。
   10 章の[省略規則](../glossary.md#lifetime-elision)が、この関数の形に効くかどうかも確かめる。
