---
title: 5 Result と ?、Option との違い
sidebar_position: 5
---

# 5 `Result` と `?`、`Option` との違い

`tally` で、行の中のフィールドを数えるとき、そのフィールドを持たない行があったとする。
普段は、その行を飛ばして数え続ける。
`--strict` を付けると、その行で集計を止めて失敗する。

同じ「フィールドが無い」が、あるときは飛ばすだけの行で、あるときは失敗になる。
Rust は、この二つを別の型で表す。
「無い」は [`Option`](../glossary.md#option)、「失敗」は [`Result`](../glossary.md#result) である。
この章では、二つの型の違いと、失敗を呼び手へ渡す `?` を扱う。

## 「無い」は `Option` で表す

4 章で、行から値を取り出した結果を、自前の [enum](../glossary.md#enum) で表した。

```rust
enum Picked {
    Found(String),
    Missing,
}
```

標準ライブラリの `Option` は、これと同じ形をしている。

```rust
enum Option<T> {
    Some(T),
    None,
}
```

`<T>` で、中身の型をあとから決められるようにしてある。
`Option<String>` なら `Some` が `String` を持ち、`Option<u64>` なら `u64` を持つ。
型をこうして受け取る書き方は 9 章で扱う。
`Option` はよく使うので、`Some` と `None` は `Option::` を付けずに書ける。

行から `name=値` の値を探す関数は、こうなる。

```rust
fn find_field(line: &str, name: &str) -> Option<String> {
    for token in line.split_whitespace() {
        if token.starts_with(name) && token[name.len()..].starts_with('=') {
            return Some(token[name.len() + 1..].to_owned());
        }
    }
    None
}
```

見つからないことは、この関数にとって失敗ではない。
その行にはそのフィールドが無い、というだけである。
だから `Result` ではなく `Option` で返す。

C++ にも、値が有るか無いかを表す [`std::optional`](../glossary.md#std-optional) がある（C++17）。
違いは、中身を取り出すときに出る。
`std::optional` は、空のまま `*` や `->` で中身を読むと[未定義動作](../glossary.md#undefined-behavior)になる。

```cpp
std::optional<std::string> o = find(false);   // 空
std::printf("size=%zu\n", o->size());
```

手元の clang（Apple clang 21、libc++）でこれを動かすと、最適化なしでは `size=0`、`-O2` では `size=4` と出た。
どちらも何も言わずに動き続けた。
libc++ の検査（`_LIBCPP_HARDENING_MODE`）を有効にしてビルドすると、ここで止まった。
検査は、ビルドするときに自分で有効にするものである。
`o.value()` と書けば、空のときに例外 `std::bad_optional_access` が投げられる。

Rust の `Option` は、中身を取り出す道を `match` のようなパターンに限っている。
`Some` の腕でしか中身に名前が付かないので、空のまま中身を読む書き方ができない。

```rust
match find_field(line, "level") {
    Some(level) => println!("{level}"),
    None => println!("level が無い"),
}
```

`match` を毎回書かずに済むメソッドもある。
3 章の終わりに出てきた `unwrap_or` は、`None` なら代わりの値を返す。
`Some` なら中身に関数を当て、`None` なら代わりの値を返すのが `map_or` である。
`map_or` に渡す関数の書き方（`|rest| ...`）は 8 章で扱う。

## 「失敗」は `Result` で表す

`Result` も enum である。

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

成功なら `Ok` が値を持ち、失敗なら `Err` が失敗の中身を持つ。
失敗の中身の型 `E` も、自分で決められる。

集計の失敗を enum で表すと、こうなる。

```rust
enum CountError {
    Read(io::Error),
    MissingField { line_no: usize },
}
```

入力を読めなかった場合と、`--strict` でフィールドの無い行に当たった場合である。
`io::Error` は標準ライブラリの型で、入出力の失敗を表す。

1 行ぶん数えるメソッドは、`Result` を返す。

```rust
fn push_line(&mut self, line: &str, line_no: usize) -> Result<(), CountError> {
    match find_field(line, &self.field) {
        Some(key) => *self.counts.entry(key).or_insert(0) += 1,
        None if self.strict => return Err(CountError::MissingField { line_no }),
        None => self.skipped += 1,
    }
    Ok(())
}
```

`()` は値を一つしか持たない型で、返すものが無いことを表す（[ユニット型](../glossary.md#unit-type)と呼ぶ）。
`Result<(), CountError>` は、「成功なら何も返さず、失敗なら `CountError` を返す」という意味になる。

C++ で失敗を伝える主な手段は[例外](../glossary.md#exception)である。
C++17 からは `noexcept`（例外を投げない）が関数の型の一部になったが、どの例外を投げるかは型に現れない。
呼ぶ側は、宣言を見ても、その関数がどんな失敗をしうるかが分からない。
C++23 の [`std::expected`](../glossary.md#std-expected) は、成功の値か失敗の値のどちらかを戻り値として返す型で、`Result` に近い。

Java の[検査例外](../glossary.md#checked-exception)は、投げうる例外を `throws` として宣言に書かせる。
手元の Java 21 では、宣言した例外を受け止めも宣言もせずに呼ぶとコンパイルエラーになった。
ラムダ式の中から呼んだときも同じエラーになった。
Rust の `Result` は、失敗を戻り値の型そのものに書く。
宣言を見れば失敗しうることが分かる点は検査例外と同じで、失敗は投げられるのではなく、値として返ってくる。

## 戻り値の `Result` を捨てると警告が出る

`Result` を返す関数を呼んで、戻り値を使わずに捨てると、コンパイラが警告を出す。

```rust
parse("1");
```

```
warning: unused `Result` that must be used
  --> src/main.rs:16:5
   |
16 |     parse("1");
   |     ^^^^^^^^^^
   |
   = note: this `Result` may be an `Err` variant, which should be handled
```

`Result` という型そのものに「捨てると警告する」という印が付いているので、`Result` を返すどの関数にもこの警告が効く。
わざと捨てるときは `let _ = parse("1");` と書いて、捨てることを明示する。

C++ では、同じことを関数ごとに `[[nodiscard]]` と書いて頼む。
手元の libc++ では、`std::expected` や `std::optional` を返す関数の戻り値を捨てても、警告は出なかった。
自分で `[[nodiscard]]` を付けた関数でだけ出た。

## `?` は失敗をそのまま呼び手へ返す

入力を最後まで読む関数は、二種類の失敗に出会う。
読み込みの失敗と、`push_line` が返す失敗である。

```rust
fn count_all(mut input: &[u8], tally: &mut Tally) -> Result<(), CountError> {
    let mut line = String::new();
    let mut line_no = 0;
    loop {
        line.clear();
        if input.read_line(&mut line).map_err(CountError::Read)? == 0 {
            break;
        }
        line_no += 1;
        tally.push_line(line.trim_end(), line_no)?;
    }
    Ok(())
}
```

[`?`](../glossary.md#question-mark) は、`Result` が `Ok` なら中身を取り出して先へ進み、`Err` ならその `Err` を返して関数を抜ける。
`tally.push_line(...)?` を `match` で書けば、次のようになる（失敗の型を変換する働きもあるが、ここでは型が同じなので何もしない。変換は次の節で見る）。

```rust
match tally.push_line(line.trim_end(), line_no) {
    Ok(()) => {}
    Err(error) => return Err(error),
}
```

C++ の例外が呼び出しの階段を自動で上っていくのに対し、Rust では階段を一段上がるたびに `?` を書く。
失敗が通る道が、コードの上で `?` として見える。

`input` は `&[u8]`（バイト列のスライス）で、`read_line` で 1 行ずつ読むと、読んだぶんだけ先頭が進む。
だから `mut` が要る。
`read_line` は `BufRead` という [trait](../glossary.md#trait) のメソッドで、`use std::io::BufRead;` と書いて取り込んでから呼ぶ。
trait は 9 章で扱う。

### 失敗の型が合わなければ通らない

`read_line` が返す失敗の型は `io::Error` で、`count_all` が返すのは `CountError` である。
`map_err` を書かずに `?` を付けると、こうなる。

```
error[E0277]: `?` couldn't convert the error to `CountError`
 --> src/main.rs:7:30
  |
5 | fn first_line(mut input: &[u8]) -> Result<String, CountError> {
  |                                    -------------------------- expected `CountError` because of this
6 |     let mut buf = String::new();
7 |     input.read_line(&mut buf)?;
  |           -------------------^ the trait `From<std::io::Error>` is not implemented for `CountError`
  |           |
  |           this can't be annotated with `?` because it has type `Result<_, std::io::Error>`
```

（このメッセージは、同じ形の小さな関数で出したものである。）
`?` には、失敗の型を、返す側の型へ変換しようとする働きがある。
変換のしかたを `From` という trait で教えておけば、`map_err` を書かずに済む。
これは 9 章と 11 章で扱う。

ここでは `map_err` で、`Err` の中身だけを変換している。
渡している `CountError::Read` は、`io::Error` を受け取って `CountError` を返す関数として使える。
`Read(io::Error)` のように、名前の無いフィールドを持つバリアントの名前は、そのまま値を作る関数になる。
`MissingField { line_no }` のように名前の付いたフィールドを持つバリアントは、関数としては使えない。

`Option` にも `?` が使える。
`None` ならその場で `None` を返す。
ただし、`?` が返すものは関数の戻り値の型と合っていなければならない。
`Result` を返す関数の中で `Option` に `?` を付けると通らない。

```
error[E0277]: the `?` operator can only be used on `Result`s, not `Option`s, in a function that returns `Result`
  --> src/main.rs:11:27
   |
10 | fn opt_in_result(line: &str) -> Result<usize, CountError> {
   | --------------------------------------------------------- this function returns a `Result`
11 |     let i = line.find('=')?;
   |                           ^ use `.ok_or(...)?` to provide an error compatible with `Result<usize, CountError>`
```

コンパイラが勧めているとおり、`ok_or` で `None` のときの失敗を与えれば、`Result` に変わって `?` が付けられる。

## `Option` を `Result` に変えるのは呼んだ側の事情

`find_field` は `Option` を返し、`push_line` は `Result` を返していた。
`Option` を `Result` に変えたのは、`push_line` の側である。

```rust
match find_field(line, &self.field) {
    Some(key) => *self.counts.entry(key).or_insert(0) += 1,
    None if self.strict => return Err(CountError::MissingField { line_no }),
    None => self.skipped += 1,
}
```

`find_field` は、見つからないことが失敗かどうかを知らない。
それを決められるのは、`--strict` が付いているかを知っている `push_line` である。
`find_field` が最初から `Result` を返していたら、`--strict` でないときに、失敗をわざわざ捨てることになる。

4 章のガード（`if self.strict`）がここで役に立っている。
ガードの付いた腕は網羅性に数えないので、最後の `None =>` の腕が無いとエラーになる。

`ok_or` を使えば、`match` を書かずに変換できる。
`find_field(line, &self.field).ok_or(CountError::MissingField { line_no })?` は、見つからなければ `MissingField` を返して抜ける。
ただし、これでは `--strict` でないときに飛ばす道が書けないので、`push_line` では `match` を使っている。

## panic は「起きないはず」の場合に取っておく

`Option` と `Result` には、中身を取り出し、取り出せなければ [panic](../glossary.md#panic) で止まるメソッドがある。
`unwrap` と `expect` である。

```rust
let o: Option<u8> = None;
o.unwrap();
```

```
called `Option::unwrap()` on a `None` value
```

止まるときのメッセージを自分で書きたいなら、`expect` を使う。
`o.expect("level が無い")` なら、止まったときに `level が無い` と出る。

panic は、3 章で文字列を文字の途中で切ったときと同じ止まり方である。
そのスレッドを終え、main スレッドならプログラム全体が終わる。
panic は戻り値に現れないので、呼び手がふつうのコードで受け取って対処する道は無い。
（`std::panic::catch_unwind` という特別な手段はあるが、標準ライブラリの説明は、これを一般的な例外処理のように使うことを勧めていない。）

使い分けは、その失敗が起きうるかどうかで決める。
入力が壊れている、ファイルが読めない、フィールドが無い、といったことは、プログラムが正しくても起きる。
これは `Result` で返して、呼び手に対処を任せる。
起きたらプログラムの誤りだと言えること、たとえば直前に確かめたばかりの条件が破れていることには、panic が向いている。
テストの中で、失敗したらそこで止まってほしいときにも `unwrap` や `expect` を使う。

[clippy](../glossary.md#clippy) の pedantic という厳しめの設定は、`Result` の値に `unwrap` を呼んでも何も言わなかった。
`clippy::unwrap_used` という lint を自分で有効にすると、`unwrap` を呼んだ箇所をすべて指摘させられる。

## 本物ではどうなっているか

`tally` が 1 行を数える部分は、こうなっている。

```rust
let Some(extracted) = selector.select(line, line_no)? else {
    return self.on_missing(selector.key(), line, line_no);
};

*self.counts.entry(extracted.into_owned()).or_insert(0) += 1;
Ok(())
```

`selector.select` が返すのは、`Result` の中に `Option` が入った値である。
`Err` は行を JSON として読めなかったなどの失敗、`Ok(None)` はフィールドが無かった場合、`Ok(Some(..))` は取り出せた場合を表す。
`?` が外側の `Result` をはがし、失敗ならそこで返る。
残った `Option` を、`let Some(extracted) = ... else { ... };` で受けている。

この `let ... else` は、パターンに合えば中身に名前を付けて先へ進み、合わなければ `else` の中を実行する書き方である。
`else` の中は、`return` などで必ず抜けなければならない。
`match` で書くと腕が二つになるところを、合わなかったときの処理を先に片づけて、本筋を字下げせずに書ける。

`on_missing` は、`--strict` が付いていれば失敗を返し、そうでなければ飛ばした行として数える。
この章のコードの `push_line` で、`match` の三つの腕が分担していたことと同じである。

3 章で見た、入力を読むループの `reader.read_line(&mut line)?` も、ここまでの話で読める。
読み込みに失敗すれば、`?` がその失敗を呼び手へ返す。

## 演習

答えは [`book/code/ch05/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch05/src/exercises.rs) にある。
`cargo test -p book-ch05` で確かめられる。

**演習 1**

行から `level` と `user` の値を両方取り出し、組にして返す関数を書け。
どちらか一方でも無ければ `None` を返すこと。

```rust
fn level_and_user(line: &str) -> Option<(String, String)>
```

`find_field` と `?` を使えば、`match` を書かずに済む。

**演習 2**

`count_all` を、`?` を使わずに書き直せ。
`?` 1 つが、何行の `match` に当たるかを数えてほしい。
