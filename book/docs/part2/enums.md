---
title: 4 enum と網羅的な match
sidebar_position: 4
---

# 4 enum と網羅的な `match`

`tally` の出力形式は `--format` で選ぶ。
選べるのは `text`、`json`、`csv` の三つで、それ以外を渡すと、集計を始める前に止まる。

```
error: invalid value 'xml' for '--format <FORMAT>'
  [possible values: text, json, csv]
```

取りうる値が三つと決まっているなら、その三つだけを持つ型を作ればよい。
三つのどれでもない値が、プログラムの中に入ってこなくなる。
この章では、そういう型を作る [enum](../glossary.md#enum) と、その値を場合分けする `match` を扱う。

## 取りうる値を並べた型

出力形式は、次のように書ける。

```rust
enum Format {
    Text,
    Json,
    Csv,
}
```

`Text`、`Json`、`Csv` の一つひとつをバリアントと呼ぶ。
値は `Format::Text` のように、型の名前を付けて書く。

値によって処理を分けるには `match` を使う。

```rust
impl Format {
    fn name(&self) -> &str {
        match self {
            Format::Text => "text",
            Format::Json => "json",
            Format::Csv => "csv",
        }
    }
}
```

`impl Format { ... }` は、`Format` 型にメソッドを付ける書き方である。
`&self` は 2 章で見たとおり、値を借りて受け取る引数で、構造体のメソッドと合わせて 6 章で詳しく扱う。

`match` の中の `パターン => 式` の一つひとつを腕と呼ぶ。
`=>` の左が[パターン](../glossary.md#pattern)で、`match` は上から順に照らし合わせ、最初に合った腕の右辺を評価する。
`match` は式なので、選ばれた腕の値がそのまま `match` 全体の値になる。

ここまでなら、C++ の [`enum class`](../glossary.md#enum-class) と `switch` で同じものが書ける。
違いは二つある。
一覧の外の値を作れないことと、`match` が一覧の全部を扱ったかを確かめることである。

## 一覧の外の値は作れない

C++ の `enum class` は、列挙子に無い値を持てる。

```cpp
enum class Format { Text, Json, Csv };

Format f = static_cast<Format>(7);
```

`enum class` の基底型は、指定しなければ `int` に決まる。
基底型が決まっている列挙型では、基底型の値ならどれでも、その型の値として持てる。
`f` は、`Text` でも `Json` でも `Csv` でもない `Format` である。
これは誤りではなく、規格が認めている値である。
（基底型を決めていない従来の `enum` では事情が違い、列挙子から決まる範囲の外の値にすると[未定義動作](../glossary.md#undefined-behavior)になる。
clang の `-fsanitize=enum` は、そちらだけを報告した。）

Rust では、整数から enum への変換が通らない。
`as` は数値などの基本の型へ変換する演算子で、`2u8` は `u8` 型の 2 を表す。

```rust
let format = 2u8 as Format;
```

```
error[E0605]: non-primitive cast: `u8` as `Format`
   |
33 |     let format = 2u8 as Format;
   |                  ^^^^^^^^^^^^^ an `as` expression can only be used to convert between primitive types or to coerce to a specific trait object
```

逆向きの `Format::Csv as u8` は通り、2 になる（`as` で整数にできるのは、`Format` のように値を持たないバリアントだけの enum である）。
バリアントに番号を振るのは C++ と同じで、番号から値を作る道だけが閉じている。
[`unsafe`](../glossary.md#unsafe) を使わない限り、`Text`、`Json`、`Csv` のどれでもない `Format` を作る手段が無い。

## `match` は全部の場合を書かせる

`name` から `Csv` の腕を消すと、コンパイルが通らなくなる。

```
error[E0004]: non-exhaustive patterns: `&Format::Csv` not covered
  --> src/main.rs:7:15
   |
 7 |         match self {
   |               ^^^^ pattern `&Format::Csv` not covered
   |
note: `Format` defined here
  --> src/main.rs:2:6
   |
 2 | enum Format { Text, Json, Csv }
   |      ^^^^^^               --- not covered
```

漏れたバリアントの名前まで挙げて止まる。
（`&` が付いているのは、`self` が `&Format` だからである。）
これを[網羅性検査](../glossary.md#exhaustiveness)と呼ぶ。
バリアントを足せば、そのバリアントを扱っていない `match` がすべてエラーになる。
直すべき場所の一覧を、コンパイラが作ってくれることになる。

C++ でも、手元の clang（Apple clang 21）は列挙子の漏れた `switch` に警告を出した。
オプションを何も付けなくても出る。
ただし警告であって、コンパイルは通る。
`default:` を書けば、警告そのものが出なくなる（既定の設定と `-Wall -Wextra` で確かめた。`-Wswitch-enum` を付ければ出る）。

そして C++ では、全部の列挙子を書いた `switch` でも、全部の場合を扱ったことにならない。

```cpp
const char* name(Format f) {
    switch (f) {
    case Format::Text: return "text";
    case Format::Json: return "json";
    case Format::Csv: return "csv";
    }
}
```

clang はこの関数に、`-Wall -Wextra` を付けても何も言わなかった。
ところが前の節の `static_cast<Format>(7)` を渡すと、どの `case` にも合わずに関数の終わりへ抜ける。
値を返す関数が値を返さずに終わるのは、未定義動作である。
手元では、最適化なしで動かすと文字化けした文字列が出て、`-O2` では `"text"` が出た。
`-fsanitize=undefined` を付けると、実行時に報告された。

Rust の `match` で全部のバリアントを書けば、それで本当に全部である。
一覧の外の値が存在しないので、抜け道が無い。

C# と Java にも、場合分けの漏れを見る仕組みがある。
手元の C#（.NET 10）では、`switch` 式で列挙子が漏れると警告 CS8509 が出た。
全部の列挙子を書いても、今度は CS8524 という警告が出る。
C# の enum も名前の無い値を持てるので、それが漏れていると言われる。
実行時に名前の無い値が来ると、`SwitchExpressionException` が投げられた。
`switch` 文のほうは、列挙子が漏れていても何も言わなかった。

Java 21 では、`switch` 式で列挙子が漏れるとコンパイルエラーになった。
従来の形の `switch` 文は、`-Xlint:all` を付けても何も言わなかった。
列挙子を足した enum だけをコンパイルし直し、`switch` 式の側をそのままにして動かすと、足した列挙子が来たところで `MatchException` が投げられた。

どちらの言語でも、`switch` 式は検査をすり抜けた値に実行時の例外で備えている。
C# の `switch` 文と、Java の従来の形の `switch` 文は、扱っていない値が来ても例外を投げず、どの `case` も実行せずに先へ進んだ。
Rust の `match` は、値を使う式として書いても、値を使わない文として書いても、同じ検査を受ける。

## バリアントは値を持てる

ここまでの `Format` は、C++ の `enum class` で書けるものだった。
Rust の enum は、さらにバリアントごとに別の値を持てる。

`tally` は、行のどこを数えるかを選べる。
行全体を数えることもあれば、行の中の決まったフィールドを数えることもある。
フィールドを数えるときは、どのフィールドかという名前が要る。
（本物の `tally` は行を JSON として読むが、この章では `level=warn` のような `名前=値` の並びに単純化する。）

```rust
enum Key {
    WholeLine,
    Field(String),
}
```

`Key::WholeLine` は何も持たず、`Key::Field` は [`String`](../glossary.md#string-str) を一つ持つ。
値は `Key::Field(String::from("level"))` のように作る。
`Field` の値には必ず名前が入っていて、`WholeLine` の値には名前を入れる場所が無い。

C++ でこれに当たるのは [`std::variant`](../glossary.md#std-variant) である。

```cpp
struct WholeLine {};
struct Field { std::string name; };
using Key = std::variant<WholeLine, Field>;
```

`std::variant` も、どの型の値を持っているかを覚えていて、`std::visit` で場合分けできる。
`std::visit` に渡す関数で場合が漏れると、こちらもコンパイルエラーになる。
手元の libc++ では、`` `std::visit` requires the visitor to be exhaustive`` という静的表明が失敗した。
エラーの出力は、テンプレートの展開の跡を含めて全部で 167 行あった（`-std=c++20` のとき）。
`std::get<WholeLine>(k)` のように型を決め打ちして取り出せば、外れたときに `std::bad_variant_access` が投げられる。

網羅性を確かめる点は同じで、静的表明も漏れた型（`const Field &`）を挙げていた。
違いは、Rust ではそれが言語の構文だということにある。
バリアントの中身をパターンで取り出せ、パターンを入れ子にでき、漏れたときのエラーは十数行で済む（本文の E0004 は、空行を除いて 17 行だった）。

大きさにも違いが出た。
64 ビット環境で測ると、上の `std::variant` は libc++ で 32 バイトだった。
`std::string` の 24 バイトに、どちらの型かを覚える印の分が足されている。
Rust の `Key` は 24 バイトで、`String` と同じだった。
`String` の容量は、取りうる値の範囲が型の幅より狭い。
その使われない値の一つを、`WholeLine` の印に使っている。
いつもそうなるわけではない。
二つのバリアントがどちらも `String` を持つ enum は、32 バイトになった。
もう一方の `String` が同じ場所に重なるので、容量の空いている値を印に使えなくなるからである。

### 「無い」を型にする

値を持つバリアントは、「無い」という場合を表すのにも使える。
`level=warn user=alice` のような行から、`level` の値を取り出すことを考える。

値が見つからなかったことを空文字列で表すと、`level=` のように値が空の行と区別できない。
見つかった場合と見つからなかった場合を、別のバリアントにすればよい。

```rust
enum Picked {
    Found(String),
    Missing,
}
```

`Missing` には値を入れる場所が無いので、「見つからなかったのに値がある」状態は作れない。
逆に、`Found` の中身を取り出すにはパターンを通るしかない。
`match` で取り出すなら、`Missing` の場合を書き忘れればエラーになる。

この形はよく使うので、標準ライブラリに [`Option`](../glossary.md#option) として用意されている。
5 章で扱う。

## パターンで中身を取り出す

`Key` から `Picked` を作るメソッドは、こう書ける。

```rust
impl Key {
    fn extract(&self, line: &str) -> Picked {
        match self {
            Key::WholeLine => Picked::Found(line.trim().to_owned()),
            Key::Field(name) => {
                for token in line.split_whitespace() {
                    if token.starts_with(name.as_str()) && token[name.len()..].starts_with('=') {
                        return Picked::Found(token[name.len() + 1..].to_owned());
                    }
                }
                Picked::Missing
            }
        }
    }
}
```

`split_whitespace` は行を空白で区切った語を順に返し、`starts_with` は先頭が一致するかを調べる。

`Key::Field(name)` というパターンは、`Field` だったときに、中の `String` に `name` という名前を付ける。
バリアントの判定と中身の取り出しが、一つのパターンで済む。

ここで `self` は `&Key` である。
[借りて](../glossary.md#borrow)いる値の中身を持ち出すわけにはいかないので、`name` は `&String` として束縛される。
照らし合わせる値が参照なら、パターンで取り出す中身も参照になる、という決まりがある。

照らし合わせる値が参照でないと、この決まりは働かない。
`Key` を構造体のフィールドに持たせて、借りた `self` から `match` すると、こうなる。

```rust
struct Tally {
    key: Key,
    // ほかのフィールドは省く
}

impl Tally {
    fn key_name(&self) -> String {
        match self.key {
            Key::WholeLine => String::new(),
            Key::Field(name) => name,
        }
    }
}
```

```
error[E0507]: cannot move out of `self.key` as enum variant `Field` which is behind a shared reference
  --> src/main.rs:15:15
   |
15 |         match self.key {
   |               ^^^^^^^^
16 |             Key::WholeLine => String::new(),
17 |             Key::Field(name) => name,
   |                        ---- data moved here because `name` has type `String`, which does not implement the `Copy` trait
   |
help: consider borrowing here
   |
15 |         match &self.key {
   |               +
```

`self.key` は参照ではなく `Key` そのものなので、`name` は中の `String` を [move](../glossary.md#move) で受け取ろうとする。
だが `self` は借りものなので、中身は持ち出せない。
1 章の move と 2 章の借用の規則が、パターンの中でも同じように効いている。
コンパイラの言うとおり `&self.key` と書けば、このエラーは消えて、`name` は `&String` になる。
ただし戻り値は `String` なので、今度は型が合わない。
`name.clone()` で[複製](../glossary.md#copy-clone)を作って返せば通る。

`|` で複数のパターンを一つの腕にまとめることもできる。

```rust
match format {
    Format::Text | Format::Csv => "行ごと",
    Format::Json => "全体で一つ",
}
```

## `_` は、あとで足したバリアントも黙って受ける

`_` は何にでも合うパターンで、残りの場合をまとめて受ける。

```rust
match self {
    Format::Text => "text",
    _ => "other",
}
```

これは便利だが、網羅性検査と引き換えである。
`Format` に `Tsv` を足しても、この `match` はエラーにならず、`Tsv` は黙って `"other"` になる。
C++ の `switch` に `default:` を書くと警告が消えるのと、同じことが起きている。

[clippy](../glossary.md#clippy) の pedantic という厳しめの設定は、`_` がちょうど一つのバリアントしか受けていないときに限って警告を出した。

```
warning: wildcard matches only a single variant and will also match any future added variants
```

`_` が二つ以上のバリアントを受けている上の例には、何も言わなかった。

バリアントを足す見込みのある enum では、`_` を使わずに全部を挙げておくと、足したときに直す場所をコンパイラが教えてくれる。
整数のように、場合を一つずつ挙げられない型では `_` を使うことになる。
ただ、範囲のパターン（`2..=9` や `10..`）を使えば、整数でも `_` を使わずに全部を挙げられることがある。
これは演習 2 で扱う。

## ガードは網羅性に数えない

パターンの後ろに `if` で条件を付けられる。
これをガードと呼ぶ。

```rust
match n {
    x if x < 0 => "負",
    0 => "零",
}
```

```
error[E0004]: non-exhaustive patterns: `i32::MIN..=-1_i32` and `1_i32..=i32::MAX` not covered
  --> src/main.rs:22:11
   |
22 |     match n {
   |           ^ patterns `i32::MIN..=-1_i32` and `1_i32..=i32::MAX` not covered
   |
   = note: the matched value is of type `i32`
   = note: match arms with guards don't count towards exhaustivity
```

正の数が漏れているのは確かだが、負の数まで漏れていると言われる。
コンパイラはガードの条件を評価しないので、ガードの付いた腕は、何も受けないものとして数える。
最後の注記がそれを言っている。
ガードを使うときは、ガードの無い腕で残りを受けておく必要がある。

## 本物ではどうなっているか

`tally` は、失敗の種類から終了コードを決める部分を `match` で書いている。

```rust
match &self.kind {
    CliErrorKind::Write(source) if is_broken_pipe(source) => EXIT_OK,
    CliErrorKind::Open { .. }
    | CliErrorKind::Input { .. }
    | CliErrorKind::Threads { .. }
    | CliErrorKind::Write(_) => EXIT_FAILURE,
}
```

`CliErrorKind` は失敗の種類を並べた enum で、`Write` は出力に書けなかった場合である。
`Open { .. }` の `{ .. }` は、名前の付いたフィールドを持つバリアントの、中身を見ないで照らし合わせる書き方である。

一つ目の腕は、出力先のパイプが先に閉じた場合（`tally big.log | head` のように）を成功として扱う。
ガードが付いているので、網羅性には数えられない。
だから二つ目の腕に、もう一度 `CliErrorKind::Write(_)` がある。
これを消すと、`Write` が漏れているというエラーになる。
二つ目の腕は `_` でも通るが、失敗の種類を足したときに終了コードを決め忘れないよう、全部を挙げてある。

一方で、`_` を書かざるを得ない場所もある。
`tally` は集計の部分を、別の[クレート](../glossary.md#crate)（Rust のコンパイルの単位で、ここではライブラリ一つ分）に分けている。
そのクレートの enum の一つに、`#[non_exhaustive]` という[属性](../glossary.md#attribute)（`#[...]` の形で項目に付ける、コンパイラへの指示）が付いている。

```rust
#[non_exhaustive]
pub enum LineErrorKind { /* ... */ }
```

これは「この enum には、あとでバリアントを足すかもしれない」という宣言である。
定義したクレートの外で `match` すると、全部のバリアントを挙げても `_` が無ければエラーになる。

```
   = note: `Kind` is marked as non-exhaustive, so a wildcard `_` is necessary to match exhaustively
```

（このメッセージは、同じ形の小さな例で出したものである。）
定義したクレートの中では、`_` が無くても通る。
足す側が直す場所は手元にあるが、使う側のコードは足す側からは見えないので、使う側に `_` を書かせておく。
そうすれば、バリアントを足しても使う側が壊れない。
その代わり、使う側の網羅性検査は効かなくなり、足されたバリアントは `_` の腕に黙って入る。

冒頭のエラーメッセージを出しているのは、コマンドライン引数を解釈する `clap` というライブラリである。
`tally` の `Format` には、`clap` が文字列から `Format` への変換を自動で作る指定が付いている。
この仕組みは 9 章で扱う。
変換は引数を受け取った時点で行われ、三つのどれでもなければそこで止まる。
だから `tally` の中には、一覧の外の出力形式を扱うコードが無い。

## 演習

答えは [`book/code/ch04/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch04/src/exercises.rs) にある。
`cargo test -p book-ch04` で確かめられる。

**演習 1**

`Key` に、空白で区切った n 番目の語（0 から数える）をキーにするバリアント `Column(usize)` を足せ。
足したら、まず何も直さずにビルドして、コンパイラがどこを指すかを見てほしい。
そのうえで、`extract` に腕を足せ。
`match self` の中で、`Column` が持つ番号はどんな型で束縛されるか。

**演習 2**

件数を、`0`、`1`、`2` から `9`、`10` 以上の四つの区分に分ける関数を書け。
区分は enum にすること。

```rust
fn bucket(count: u64) -> Bucket
```

`_` を使わずに書けるか。
書けたら、`2..=9` を `2..9` に書き換えてビルドし、何が起きるかを見てほしい。
`_` で残りを受けていたら、同じ書き間違いで何が起きたかも考えてほしい。
