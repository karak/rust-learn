---
title: 9 trait と境界、単相化と dyn
sidebar_position: 9
---

# 9 trait と境界、単相化と `dyn`

ここまでの章には、後回しにした話がいくつもある。
7 章の `f64` は `Ord` を満たさないので並べられなかった。
5 章の `?` は、`From` があれば失敗の型を変換できると言っていた。
6 章の `#[derive(Debug)]` はコードを書かせる属性で、8 章のクロージャは `Fn`・`FnMut`・`FnOnce` に分かれた。
どれも [trait](../glossary.md#trait) の話である。

trait は、型が持つ操作をまとめて宣言したもので、C# や Java のインタフェースに近い。
違うのは次の 3 点である。
実装を型の定義の外に書けること、フィールドを持てないこと、そして呼び出し先が既定ではコンパイル時に決まることである。
この章では、trait を宣言して実装し、関数の型引数に trait を求め、コンパイラがそれをどんなコードにするかを見る。
最後に、呼び出し先を実行時に決める `dyn` を見る。

## この章のコードは、状態コードとログの重さを同じ関数で数える

この章のコード（`book/code/ch09`）は、HTTP の状態コード（`u16`）とログの重さ（自分で定義した `Level`）という別々の型を、同じ関数で欄ごとに数える。

```rust
fn tally<T: Bucket>(items: &[T]) -> BTreeMap<String, u64>
fn count_if<F: FnMut(&str) -> bool>(lines: &[&str], mut keep: F) -> usize
fn longer_than(min: usize) -> impl Fn(&str) -> bool
fn buckets(items: &[&dyn Bucket]) -> Vec<String>
fn parse_codes(fields: &[&str]) -> Result<Vec<u16>, CountError>
```

`cargo run -p book-ch09` の出力はこうなる。

```text
{"2xx": 2, "3xx": 1, "4xx": 1, "5xx": 1}
{"debug": 1, "error": 1, "info": 2, "warn": 1}
失敗: 2 と 1
[Debug, Info, Info, Warn, Error]
残した 3 行、空行 2 行
4 文字より長い行: 1
["4xx", "warn"]
&u16: 8 バイト、&dyn Bucket: 16 バイト
Ok([200, 404])
Err(BadNumber(ParseIntError { kind: InvalidDigit }))
```

1 行目は状態コードを百の位で、2 行目はログを重さの名前で数えた結果である。
どちらも同じ `tally` が出している。

## trait は、型の定義から離れた場所で実装する

「どの欄に数えるか」を、trait として宣言する。

```rust
trait Bucket {
    fn bucket(&self) -> String;

    fn is_error(&self) -> bool {
        false
    }
}
```

`bucket` は本体の無い宣言で、実装する型が必ず書く。
`is_error` には本体があり、実装する型が書かなければこの本体が使われる（既定の実装）。

実装は `impl trait 名 for 型` と書く。

```rust
impl Bucket for u16 {
    fn bucket(&self) -> String {
        format!("{}xx", self / 100)
    }

    fn is_error(&self) -> bool {
        *self >= 400
    }
}
```

`u16` は標準ライブラリの型で、この章のコードが定義したものではない。
それでも、`Bucket` はこの章のコードの trait なので、`u16` に実装できる。
Java では、インタフェースは `class Code implements Bucket` のようにクラスの定義に書くので、`Integer` に後からインタフェースを足すことはできない。
C# も同じで、拡張メソッドは静的に解決される関数であり、`where T : IBucket` という制約を満たさない。

自分で定義した `Level` には、`bucket` と `is_error` の両方を書いた。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Bucket for Level {
    fn bucket(&self) -> String {
        self.to_string()
    }

    fn is_error(&self) -> bool {
        *self == Level::Error
    }
}
```

`self.to_string()` は、`Level` に実装した `Display` という trait（`{}` で表示するための trait）から来ている。
`Display` の実装は、名前を小文字で書き出すだけなので、ここでは省いた。

trait はフィールドを持てない。
Scala の trait はフィールドを持てるが、Rust の trait が持てるのはメソッドなどの宣言と既定の実装だけで、状態は実装する型の側に置く。
継承も無いので、`Level` が `Bucket` の既定の実装を「上書き」するのは、同じ名前のメソッドを `impl` の中に書くことだけを指す。

trait のメソッドは、その trait が見えている場所でしか呼べない。
別のモジュールで定義した trait を `use` せずにメソッドを呼ぶと、コンパイラは `E0599` で断り、`use` を足すよう勧める。

```text
   = help: items from traits can only be used if the trait is in scope
help: trait `Shout` which provides `shout` is implemented but not in scope; perhaps you want to import it
```

（このメッセージは、同じ形の小さな例で出したものである。）
5 章で `read_line` を呼ぶ前に `use std::io::BufRead;` と書いたのは、`read_line` が `BufRead` という trait のメソッドだからである。

## 他人の trait を他人の型に実装することはできない

`u16` に `Bucket` を実装できたのは、`Bucket` がこの章のコードのものだからである。
標準ライブラリの trait を標準ライブラリの型に実装しようとすると、断られる。

```rust
impl fmt::Display for u16 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "x")
    }
}
```

```text
error[E0117]: only traits defined in the current crate can be implemented for primitive types
 --> src/main.rs:3:1
  |
3 | impl fmt::Display for u16 {
  | ^^^^^^^^^^^^^^^^^^^^^^---
  |                       |
  |                       `u16` is not defined in the current crate
```

（この章のエラーは、どれも章のコードとは別の小さなクレートで出したものである。）
trait か型の少なくとも一方は、実装を書く[クレート](../glossary.md#crate)のものでなければならない。
これを[孤児ルール](../glossary.md#orphan-rule)と呼ぶ。
同じ型に同じ trait を 2 回実装しても、`E0119`（conflicting implementations）で断られる。

この 2 つの規則で、ある型にある trait の実装は、プログラム全体で多くても 1 つに決まる。
依存しているクレートのどれかが `u16` に `Display` を勝手に実装して、別のクレートの実装とぶつかることが起きない。
Scala の given（以前の implicit）も「型に後から振る舞いを足す」仕組みだが、同じ型に複数のインスタンスを置き、スコープによって選ばれるものが変わりうる。
Rust では、どこから呼んでも同じ実装が選ばれる。

他人の trait を他人の型に実装したいときは、6 章の [newtype](../glossary.md#newtype) で型を包み、包んだ型に実装する。

## 境界は、型引数に「何ができるか」を約束させる

`tally` は、どの型の値の列でも受け取れるよう、要素の型を[型引数](../glossary.md#type-parameter) `T` にしてある。

```rust
fn tally<T: Bucket>(items: &[T]) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for item in items {
        *out.entry(item.bucket()).or_insert(0) += 1;
    }
    out
}
```

`<T: Bucket>` の `: Bucket` を[境界](../glossary.md#trait-bound)と呼ぶ。
「`T` に入る型は `Bucket` を実装していること」という条件である。
Java の `<T extends Bucket>`、C# の `where T : IBucket` と同じ役目をする。

境界を外して `fn tally<T>(items: &[T])` にすると、本体の `item.bucket()` が通らなくなる。

```text
error[E0599]: no method named `bucket` found for reference `&T` in the current scope
  --> src/main.rs:17:25
   |
17 |         *out.entry(item.bucket()).or_insert(0) += 1;
   |                         ^^^^^^ method not found in `&T`
   |
   = help: items from traits can only be used if the type parameter is bounded by the trait
```

境界の無い `T` について、コンパイラが知っているのは「何かの型である」ことだけである。
本体で使える操作は、境界に書いた trait が持つものに限られる。
呼ぶ側では逆に、境界を満たさない型を渡すと断られる。

```text
error[E0277]: the trait bound `f64: Bucket` is not satisfied
  --> src/main.rs:22:28
   |
22 |     println!("{:?}", tally(&[0.5_f64]));
   |                      ----- ^^^^^^^^^^ the trait `Bucket` is not implemented for `f64`
   |                      |
   |                      required by a bound introduced by this call
```

7 章の `levels.sort()` が `f64: Ord` で断られたのは、これと同じ形である。
標準ライブラリのスライスの `sort` は `where T: Ord` という境界を持っていて、`f64` はそれを満たさない。

境界は、型引数の後ろに書く代わりに `where` 節にまとめても書ける。
この章の `errors` はそう書いた。
境界が長くなったときや、型引数が増えたときに読みやすい。

```rust
fn errors<T>(items: &[T]) -> usize
where
    T: Bucket,
{
    items.iter().filter(|item| item.is_error()).count()
}
```

5 章の `Option<T>` の `<T>` も型引数である。
`Option` は中身に何も求めないので、境界が無い。

## 型引数ごとに、別の関数が作られる

`main` は、`tally` を `u16` の列と `Level` の列で 1 回ずつ呼んでいる。
最適化なしでビルドした実行ファイルのシンボルを `nm -C` で見ると、`tally` が 2 つある。

```text
t book_ch09::tally::<book_ch09::Level>
t book_ch09::tally::<u16>
t book_ch09::errors::<book_ch09::Level>
t book_ch09::errors::<u16>
t book_ch09::buckets
```

コンパイラは、`T` に入った型ごとに、`T` をその型に置き換えた関数を別々に作る。
これを[単相化](../glossary.md#monomorphization)と呼ぶ。
`tally::<u16>` の機械語は、`item.bucket()` のところで `<u16 as Bucket>::bucket` を直接呼んでいた（aarch64 の `bl` 命令）。
どの型の `bucket` を呼ぶかが、コンパイルの時点で決まっているからである。
最適化を掛けたビルドでは、どちらの `tally` も `main` の中に展開され、名前ごと消えていた。

Java の総称型は、コンパイルすると型引数が消える（[型消去](../glossary.md#type-erasure)）。
同じ形の Java のメソッド `static <T extends Bucket> Map<String, Long> tally(List<T> items)` は 1 つだけコンパイルされ、`javap -c` で見ると `item.bucket()` は `invokeinterface Bucket.bucket` になっていた。
どの実装を呼ぶかは、実行時にインタフェースの表から引く。
`new ArrayList<Code>().getClass() == new ArrayList<String>().getClass()` は `true` を返した。

C# は型引数を実行時まで残す。
`typeof(T).Name` は `int` で呼べば `Int32`、`string` で呼べば `String` を返し、`List<int>` と `List<string>` は別の型だった。

Rust では、どの型の関数を作るかがコンパイルの時点で全部決まる。
`T` が参照や `String` のような型でも、型ごとに別の関数になる。
`std::any::type_name::<T>()` で型の名前を取り出すこともできるが、その答えも、作られた関数ごとにコンパイル時に決まっている。
代わりに、使った型の数だけコードが増え、そのぶんコンパイルが遅く、実行ファイルが大きくなる。
C++ のテンプレートと同じ費用である。

8 章で、[イテレータ](../glossary.md#iterator)の型の中にクロージャがそれぞれ自分だけの型として入っていたのも、これで説明がつく。
`filter` は型引数でクロージャを受け取るので、渡したクロージャごとに `filter` の中身が作られ、呼び先はコンパイル時に決まる。

## クロージャを受け取る関数は、`Fn` の 3 つを境界に書く

8 章で、[クロージャ](../glossary.md#closure)は呼べる回数と条件で [`Fn`・`FnMut`・`FnOnce`](../glossary.md#fn-traits) に分かれると書いた。
この 3 つは trait なので、自分の関数でクロージャを受け取るときは、型引数の境界に書く。

```rust
fn count_if<F: FnMut(&str) -> bool>(lines: &[&str], mut keep: F) -> usize {
    let mut n = 0;
    for &line in lines {
        if keep(line) {
            n += 1;
        }
    }
    n
}
```

`FnMut(&str) -> bool` は「`&str` を受け取って `bool` を返し、書き換えながら何度でも呼べる」という境界である。
`keep` を呼ぶと中身が書き換わりうるので、引数を `mut keep` にしてある。

`main` は、空行を数えながら空でない行を残すクロージャを渡している。

```rust
let mut empty = 0;
let kept = count_if(&lines, |line| {
    if line.is_empty() {
        empty += 1;
    }
    !line.is_empty()
});
```

境界を `Fn` にすると、このクロージャは渡せなくなる。

```text
error[E0594]: cannot assign to `empty`, as it is a captured variable in a `Fn` closure
  --> src/main.rs:10:13
   |
 1 | fn count_if<F: Fn(&str) -> bool>(lines: &[&str], keep: F) -> usize {
   |                                                        - change this to accept `FnMut` instead of `Fn`
...
 7 |     let mut empty = 0;
   |         --------- `empty` declared here, outside the closure
 8 |     let kept = count_if(&lines, |line| {
   |                --------         ------ in this closure
   |                |
   |                expects `Fn` instead of `FnMut`
```

境界は、受け取る側への約束であり、渡す側への要求でもある。
`Fn` は渡せるクロージャがいちばん少なく、`FnOnce` はいちばん多い。
受け取る関数は、中でする呼び方に足りるうちで、いちばん緩いものを求めるとよい。
`count_if` は 1 行ずつ順に呼ぶだけなので `FnMut` で足り、1 回しか呼ばないなら `FnOnce` で足りる。

型引数に名前を付けず、`keep: impl FnMut(&str) -> bool` と書いても同じ意味になる。
引数の位置の [`impl`](../glossary.md#impl-trait) は、名前の無い型引数の略記である。

戻り値の位置にも `impl` を書ける。
クロージャを、ヒープに置かずにそのまま返すときは、こう書く。

```rust
fn longer_than(min: usize) -> impl Fn(&str) -> bool {
    move |line| line.len() > min
}
```

クロージャの型には名前が無いので、戻り値の型を書けない。
`impl Fn(&str) -> bool` は「`Fn(&str) -> bool` を満たす、ある 1 つの型」を表す。
呼ぶ側にはその型の名前が見えないが、コンパイラは知っている。
だから `count_if(&lines, longer_than(4))` も単相化され、シンボルには `count_if::<book_ch09::longer_than::{closure#0}>` と `count_if::<book_ch09::main::{closure#0}>` の 2 つが並んでいた。
`move` は、8 章で見たとおり `min` を値で閉じ込めるためで、無ければ `longer_than` が返った時点で消える `min` を借りることになる。

## `dyn` を書いたときだけ、呼び出し先は実行時に決まる

型引数の `T` には、呼び出し 1 回につき 1 つの型しか入らない。
`tally(&[404_u16, Level::Warn])` のように、型の違う値を 1 つの列に並べることはできない。

クロージャでも同じことが起きる。
閉じ込める変数のあるクロージャを 2 つ、1 つの配列に入れようとすると断られる。

```rust
let min = 3;
let keeps = [|s: &str| !s.is_empty(), |s: &str| s.len() > min];
```

```text
error[E0308]: mismatched types
 --> src/main.rs:3:43
  |
3 |     let keeps = [|s: &str| !s.is_empty(), |s: &str| s.len() > min];
  |                  ---------                ^^^^^^^^^^^^^^^^^^^^^^^ expected closure, found a different closure
  |                  |
  |                  the expected closure
  |
  = note: expected closure `{closure@src/main.rs:3:18: 3:27}`
             found closure `{closure@src/main.rs:3:43: 3:52}`
  = note: no two closures, even if identical, have the same type
  = help: consider boxing your closure and/or using it as a trait object
```

8 章で見たとおり、クロージャはそれぞれ自分だけの型を持つ。
配列の要素は 1 つの型でなければならないので、2 つ目が入らない。
なお、2 つとも何も閉じ込めていなければ、どちらも関数ポインタ `fn(&str) -> bool` に[型強制](../glossary.md#coercion)されて、この配列は通る。

型の違う値を並べるには、`dyn Bucket` と書く。

```rust
fn buckets(items: &[&dyn Bucket]) -> Vec<String> {
    items.iter().map(|item| item.bucket()).collect()
}
```

[`&dyn Bucket`](../glossary.md#dyn) は「`Bucket` を実装した、何かの型の値への参照」である。
`buckets(&[&404_u16, &Level::Warn])` と、`u16` への参照と `Level` への参照を同じ列に並べられる。
この参照は、値のアドレスに加えて、その型の `Bucket` の実装を並べた表（[vtable](../glossary.md#vtable)）のアドレスを持つ。
3 章で見た `&str` と同じ[ファットポインタ](../glossary.md#fat-pointer)で、出力にあるとおり大きさは `&u16` の 2 倍の 16 バイトだった。
最適化なしのビルドで、`buckets` の中の `map` に渡したクロージャ（`buckets::{closure#0}`）の機械語を見ると、参照の後ろ半分から表のアドレスを読み、表の 24 バイト目に置かれた `bucket` のアドレスを読んで、そこへ飛んでいた（`blr` 命令）。
手前の 3 つには、値を片付ける関数、大きさ、整列が入っていた。
表の並びは言語が約束するものではなく、この版のコンパイラでそうなっていた、というだけである。

`buckets` は型引数を持たないので、`u16` 用と `Level` 用に分かれて作られることは無い。
`buckets` もそのクロージャも 1 つずつで、どの型の値が来ても、同じ機械語が表を引いて呼び分ける。

型引数と `dyn` の違いを表にしておく。

| | 型引数（`T: Bucket`） | `dyn Bucket` |
| --- | --- | --- |
| 呼び先が決まる時点 | コンパイル時 | 実行時（表を引く） |
| コード | 使った型の数だけ作られる | 1 つ |
| 型の違う値を 1 つの列に並べる | できない | できる |

C# や Java では、インタフェース型の変数を通した呼び出しは、いつも右の列の形になる（JIT が呼び先を見抜いて直接呼びに変えることはある）。
Java のメソッドは、何も書かなければ派生クラスで上書きできる。
Rust では、`dyn` と書いた場所だけが右の列になる。

参照の代わりに `Box<dyn Bucket>` とすれば、値を[ヒープ](../glossary.md#heap)に置いて持てる。
さっきの配列も、`[Box<dyn Fn(&str) -> bool>; 2]` と型を書いて、それぞれを `Box::new` で包めば通る。

どの trait でも `dyn` にできるわけではない。
`Box<dyn Clone>` は断られる。

```text
error[E0038]: the trait `Clone` is not dyn compatible
 --> src/main.rs:2:24
  |
2 |     let v: Vec<Box<dyn Clone>> = Vec::new();
  |                        ^^^^^ `Clone` is not dyn compatible
  |
  = note: the trait is not dyn compatible because it requires `Self: Sized`
```

`Clone` は、実装する型の大きさがコンパイル時に決まっていること（`Sized`）を求めている。
`clone` が `Self`、つまり複製した値そのものを返すので、その大きさが決まっていなければ返せないからである。
`dyn Clone` の向こうの型は実行時まで分からないので、この条件と両立しない。
自分で書いた trait に `fn dup(&self) -> Self;` があっても、同じく `dyn` にできない（エラーは「method `dup` references the `Self` type in its return type」になる）。
メソッドが型引数を持つ場合も、型引数ごとに関数が要るので表に載せられない。
ただし、そのメソッドに `where Self: Sized` を付ければ、表からはそのメソッドだけが外れ、残りのメソッドで `dyn` にできる。

## derive は、trait の実装をコンパイラに書かせる

6 章の [`#[derive(Debug, Default)]`](../glossary.md#derive) は、`Debug` と `Default` という trait の実装をコンパイラに書かせていた。
この章の `Level` には、`#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]` と、比べるための trait も derive した。

`PartialOrd` と `Ord` を derive した enum は、判別値（4 章で `Format::Csv as u8` が返した 2 のような、バリアントに対応する整数）を `Top = 2` のように明示していなければ、バリアントを書いた順に小さい。
`Level::Warn < Level::Error` が成り立ち、7 章の `sort` がそのまま使える。
出力の `[Debug, Info, Info, Warn, Error]` がそれである。

derive した実装は、フィールドの型の実装を使って書かれる。
だから、フィールドの型が trait を実装していなければ、derive もできない。
`f64` のフィールドを持つ構造体に `Eq` や `Ord` を derive すると、7 章と同じ理由で断られる。

```text
error[E0277]: the trait bound `f64: Eq` is not satisfied
   --> src/main.rs:3:5
    |
  1 | #[derive(PartialEq, PartialOrd, Eq, Ord)]
    |                                 -- in this derive macro expansion
  2 | struct Hit {
  3 |     score: f64,
    |     ^^^^^^^^^^ the trait `Eq` is not implemented for `f64`
```

`Level` の `Display` は手で書いた。
人に見せる表示はフィールドの並びからは決まらないので、標準ライブラリは `Display` の derive を用意していない。

derive は、ライブラリも用意できる。
4 章で、`tally` の `Format` には、`clap` が文字列から `Format` への変換を作る指定が付いていると書いた。
その指定がこれである。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Text,
    Json,
    Csv,
}
```

`clap::ValueEnum` を derive すると、`clap` の `ValueEnum` という trait の実装が書かれる。
中身は、全部のバリアントの一覧を返すメソッドと、バリアントごとの名前（`text`、`json`、`csv`）を返すメソッドの 2 つだった。
文字列から `Format` を探す `from_str` は、`ValueEnum` の既定の実装として trait の側にあり、この 2 つを使って探す。
`tally` の集計結果の `Report` に付いている `serde::Serialize` も同じ仕組みで、JSON に書き出すための実装を `serde` が書いている。

## `From` を実装すると、`?` が失敗を包み直す

5 章では、[`?`](../glossary.md#question-mark) が失敗の型を変換しようとし、変換のしかたが無いと `E0277` で断られた。
その変換のしかたが `From` という trait である。

```rust
#[derive(Debug, PartialEq)]
enum CountError {
    Empty,
    BadNumber(ParseIntError),
}

impl From<ParseIntError> for CountError {
    fn from(source: ParseIntError) -> Self {
        CountError::BadNumber(source)
    }
}
```

`From<ParseIntError>` の `<ParseIntError>` は、trait の型引数である。
`CountError` は「`ParseIntError` から作れる」と宣言している。
これがあれば、`CountError` を返す関数の中で、`ParseIntError` を返す `parse` に `?` をそのまま付けられる。

```rust
fn parse_codes(fields: &[&str]) -> Result<Vec<u16>, CountError> {
    if fields.is_empty() {
        return Err(CountError::Empty);
    }
    let mut codes = Vec::new();
    for field in fields {
        codes.push(field.parse::<u16>()?);
    }
    Ok(codes)
}
```

`"x"` を読むと、`?` が `ParseIntError` を `CountError::from` に渡し、`Err(BadNumber(...))` が返る。
`u16` に収まらない `"70000"` も、同じ経路で `BadNumber` になる。
エラーの型をどう分けるか、`From` でどこまで自動で包むかは、11 章で扱う。

## 本物ではどうなっているか

2 章で引数の借り方だけを見た `tally_reader` は、型引数に境界を付けている。

```rust
pub fn tally_reader<R, F>(
    counter: &mut Counter,
    mut reader: R,
    selector: &Selector,
    keep: F,
) -> Result<()>
where
    R: BufRead,
    F: Fn(&str) -> bool,
```

`R: BufRead` は「1 行ずつ読める何か」である。
ファイルでも、標準入力でも、テストで渡すバイト列でもよく、読む型ごとに単相化される。

`keep` には `FnMut` ではなく `Fn` を求めている。
`tally_reader` の中だけを見れば `FnMut` で足りるが、`main.rs` は 1 つの `keep` を、ファイルごとのジョブに `&keep` として共有の参照で渡している。
共有の参照を通して呼べるのは `Fn` だけである。
手元の例で、書き換えるクロージャを `&keep` として `FnMut` を求める関数に渡すと、`E0525`（expected a closure that implements the `Fn` trait）で断られた。

書き出す側の `write_report` も、書き出し先を型引数で受け取る。

```rust
pub fn write_report<W: Write>(out: &mut W, report: &Report, format: Format) -> io::Result<()>
```

本番では標準出力に、テストでは `Vec<u8>` に書く。
テストの補助関数は `rendered(write: impl FnOnce(&mut Vec<u8>) -> io::Result<()>)` で、書き出しを 1 回だけ呼ぶので `FnOnce` を求めている。

`tally` が自分で宣言した trait は、入力の出どころを表す `Input` である。

```rust
pub trait Input {
    type Reader: BufRead;

    fn open(&self) -> Result<Self::Reader, CliError>;

    fn name(&self) -> InputName;
}

impl Input for Path {
    type Reader = BufReader<File>;
    // ...
}
```

`type Reader: BufRead;` は[関連型](../glossary.md#associated-type)と呼ぶもので、「実装ごとに 1 つに決まる型」を宣言している。
ファイルのパス（`Path`）なら `BufReader<File>` で、標準入力にも別の型で実装してある。
`Path` は標準ライブラリの型だが、`Input` は `tally` の trait なので、孤児ルールに触れない。

これを受け取る関数はこうなっている。

```rust
pub fn tally_input<I, F>(
    input: &I,
    mut counter: Counter,
    selector: &Selector,
    keep: F,
) -> Result<Counter, CliError>
where
    I: Input + ?Sized,
    F: Fn(&str) -> bool,
```

`str` は、中身の長さが値ごとに違うので、型としての大きさが決まらない。
3 章で `&str` がアドレスと長さの 2 語を持っていたのは、そのためである。
`Path` も同じく大きさが決まらない型で、型引数は既定では大きさの決まる型しか受け取らない。
`?Sized` は、そういう型も `I` に入れてよい、という印である。
`input.open()` が返す読み手の型は `I::Reader` から決まるので、`tally_input` は出どころごとに単相化され、1 行ごとに表を引く呼び出しが残らない。

`dyn` は、型の違う値を 1 つにまとめる場所に出てくる。
並列実行の準備に失敗した原因は `Box<dyn Error + Send + Sync>` で持っている。
`Error` は標準ライブラリのエラーを表す trait で、11 章で扱う。
ジョブを並列に流すテストは、3 つのクロージャを `[Box<dyn Fn() -> Result<Counter, CliError> + Sync>; 3]` という配列に入れている。
2 つは成功するジョブを作る関数 `job` が、1 つは失敗するジョブを作る関数 `failing` が返したクロージャで、どちらの関数も戻り値の型を `impl Fn() -> ...` と書いている。
`impl` で返した型は、関数ごとに別の型として扱われる。
手元の例で、`impl Fn() -> u8` を返す 2 つの関数の戻り値を 1 つの配列に入れると、何も閉じ込めていなくても `E0308`（expected opaque type, found a different opaque type）で断られた。
だから、`Box` と `dyn` で型をそろえている。
`Send` と `Sync` は、スレッドをまたいでよいことを表す trait で、12 章で扱う。

## 演習

答えは `book/code/ch09/src/exercises.rs` にある。
`cargo test -p book-ch09` で確かめられる。

1. `tally` の結果から、いちばん多い欄の名前を返す関数 `most_common` を書く。
   同数なら名前の小さいほうを返し、空なら `None` を返す。
   `u16` の列でも `Level` の列でも呼べるようにする。
2. 応答時間をミリ秒で持つ型 `struct Latency(u32);` を作り、`Bucket` を実装する。
   100 未満を `fast`、それ以外を `slow` に数え、1000 以上を失敗とする。
   そのうえで、`Latency` と `u16` と `Level` を混ぜて `buckets` に渡す。
