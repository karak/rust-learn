---
title: 3 String と &str、スライス
sidebar_position: 3
---

# 3 `String` と `&str`、スライス

2 章のサンプルコードのカウンタには、キーを受け取るメソッドがあった。

```rust
fn push(&mut self, key: &str) {
    *self.counts.entry(key.to_owned()).or_insert(0) += 1;
    self.total += 1;
}
```

受け取るのは `&str` で、しまうのは `key.to_owned()` が作った `String` である。
文字列の型が二つあり、関数の入口と中身で使い分けている。
なぜ二つあるのか、どこでどちらを使うのかが、この章の話である。

## 持ち主と、それを眺める窓

[`String`](../glossary.md#string-str) は文字列の持ち主である。
[ヒープ](../glossary.md#heap)にバイト列を確保し、先頭へのポインタ、容量、長さの三つを持つ。
[スコープ](../glossary.md#scope)を抜けると、確保したバイト列を解放する。

`&str` は、どこかにあるバイト列を眺める窓である。
先頭へのポインタと長さの二つだけを持ち、何も解放しない。
`String` の全体を指すこともあれば、途中の数バイトだけを指すこともある。

64 ビット環境で測ると、`String` は 24 バイト、`&str` は 16 バイトになる。
`&str` は参照だが、ポインタ一つではなく、長さを抱えた二語の値である（[ファットポインタ](../glossary.md#fat-pointer)と呼ぶ）。

C++ でいえば、`String` は `std::string`、`&str` は C++17 の [`std::string_view`](../glossary.md#string-view) に近い。
[所有](../glossary.md#ownership)するものと所有しないものに分ける、という考え方は同じである。

違いは二つある。
窓が持ち主より長生きしないことを、コンパイラが確かめる。
そして、中身が [UTF-8](../glossary.md#utf-8) であることを型が保証する。
順に見ていく。

## 窓は持ち主より長生きできない

C++ の `string_view` は、指している先の[寿命](../glossary.md#lifetime)を知らない。
次の二つは、どちらもコンパイルが通る。

```cpp
std::string lower(std::string_view s);   // 小文字にした新しい文字列を返す

std::string_view key = lower(line);      // 返った std::string は文の終わりで消える
counts[std::string(key)] += 1;           // 消えた文字列を読んでいる
```

```cpp
std::string buf = "warn";
std::string_view key = buf;
buf += std::string(1000, '!');          // 容量が足りなければ確保し直す
counts[std::string(key)] += 1;
```

一つ目は[未定義動作](../glossary.md#undefined-behavior)である。
二つ目は、`+=` が文字列の中身への[参照を無効にしてよい](../glossary.md#iterator-invalidation)操作なので、規格はこのあと `key` を使ってよいとは言っていない。
実際に壊れるかどうかは、その時点の容量で決まる。

手元の clang（Apple clang 21）は、一つ目には警告を出した。
二つ目には何も言わなかった。

Rust で一つ目と同じことを書くと、こうなる。

```rust
let key = line.trim().to_lowercase().as_str();
tally.push(key);
```

```
error[E0716]: temporary value dropped while borrowed
   |
11 |     let key = line.trim().to_lowercase().as_str();
   |               ^^^^^^^^^^^^^^^^^^^^^^^^^^         - temporary value is freed at the end of this statement
   |               |
   |               creates a temporary value which is freed while still in use
12 |     tally.push(key);
   |                --- borrow later used here
```

`to_lowercase()` は新しい `String` を作るが、それを持つ変数が無いので、文の終わりで捨てられる。
（`let key = &line.trim().to_lowercase();` のように、`let` の右辺で直接 `&` を付けた場合は寿命が延びて通る。
C++ の const 参照による[一時オブジェクトの延命](../glossary.md#lifetime-extension)に近い規則である。
先のコードでは `as_str()` が窓を返しているので、延びない。）
`as_str()` が返した窓は、その捨てられた `String` を指している。
直すには、`String` に名前を与えて生かしておけばよい。

```rust
let lowered = line.trim().to_lowercase();
tally.push(lowered.as_str());
```

二つ目に当たるのは、持ち主を書き換える操作である。

```rust
let key = line.trim_end();
line.clear();
tally.push(key);
```

```
error[E0502]: cannot borrow `line` as mutable because it is also borrowed as immutable
   |
16 |     let key = line.trim_end();
   |               ---- immutable borrow occurs here
17 |     line.clear();
   |     ^^^^^^^^^^^^ mutable borrow occurs here
18 |     tally.push(key);
   |                --- immutable borrow later used here
```

これは 2 章の規則そのものである。
`key` は `line` を[借りて](../glossary.md#borrow)おり、`clear` は `&mut self` を要求する。
`clear` を `push_str` に変えても同じエラーになる。

C++ では、確保し直すかどうかで二つ目が壊れたり壊れなかったりした。
Rust はその区別をしない。
窓が生きているあいだ、持ち主を書き換える操作はすべて通らない。

## 中身は UTF-8 と決まっている

C++ の `std::string` は `char` の並びであり、文字コードを知らない。
Rust の `String` と `&str` は、中身が妥当な UTF-8 であることを型が保証する。
[`unsafe`](../glossary.md#unsafe) と印を付けた特別なコードを使わない限り、壊れた UTF-8 を持つ `String` を作る手段が無い。
バイト列から作るときは検査が入る。
壊れたまま通ることはなく、失敗として返すか、壊れた部分を[置換文字](../glossary.md#replacement-character)に差し替えるかを、関数を選んで決める。

この保証が、添字の扱いに出てくる。

`len()` が返すのは、文字数ではなくバイト数である。
`"info".len()` は 4 だが、`"日本語".len()` は 9 になる。
UTF-8 では、この 3 文字がそれぞれ 3 バイトを使う。

そして、整数の添字で 1 文字を取り出す書き方は通らない。

```rust
let first = key[0];
```

```
error[E0277]: the type `str` cannot be indexed by `{integer}`
   |
21 |     let first = key[0];
   |                     ^ string indices are ranges of `usize`
   |
   = help: the trait `SliceIndex<str>` is not implemented for `{integer}`
```

UTF-8 は 1 文字の長さが一定でないので、n 文字目の位置を知るには先頭から数えるしかない。
`key[0]` という見た目で、その手間を隠さないようにしてある。
文字を数えたいときは `chars()` を使う。
`"日本語".chars().count()` は 3 である。

ここで言う「文字」は [Unicode のスカラ値](../glossary.md#unicode-scalar-value)であり、人が 1 文字と見なす単位（[書記素クラスタ](../glossary.md#grapheme-cluster)）と一致するとは限らない。
結合文字や、複数の[符号点](../glossary.md#code-point)を組み合わせた絵文字は、`chars()` では複数に数えられる。

## スライスはバイトの位置で切る

範囲を使えば、文字列の一部を切り出せる。

```rust
let level = &line[..4];
```

範囲の数字はバイトの位置である。
切り出した結果は `line` の中を指す `&str` で、バイト列は複製されない。
`line` が `String` でも `&str` でも同じように書ける。

C++ の `substr` は、新しい `std::string` を作って返す。
複製しないで切り出すには `string_view` の `substr` を使う。
Rust では、切り出しは最初から窓を作る操作である。

範囲が文字列の長さを超えると、実行時に止まる。
文字の途中を指したときも同じである。

```rust
let s = String::from("日本");
let head = &s[..1];
```

```
end byte index 1 is not a char boundary; it is inside '日' (bytes 0..3 of string)
```

C++ の `std::string` は、文字の途中で切っても何も言わない。
文字コードを知らないので、途中かどうかを判断できないからである。
Rust がここで止まるのは、そのまま進めば UTF-8 の保証が破れるからである。
未定義動作にはならず、決まった場所で、何が起きたかを言って止まる。

この止まり方を [panic](../glossary.md#panic) と呼ぶ。
既定では、C++ の例外のように[スタックを巻き戻し](../glossary.md#stack-unwinding)ながら各値の後始末を走らせ、そのスレッドを終える。
main スレッドで起きれば、プログラム全体が終わる。
エラーとして扱う方法との使い分けは 5 章で扱う。

止まらずに切りたいときは、[文字の境界](../glossary.md#char-boundary)を先に求めてから切る。
これは演習 2 で扱う。

## 引数は `&str` で受ける

`push` が `&str` を受け取るので、呼び手は手元の文字列をどの形でも渡せる。

```rust
tally.push("warn");          // 文字列リテラル
tally.push(&line);           // line は String
tally.push(line.trim());     // 切り出した窓
```

[文字列リテラル](../glossary.md#string-literal)は最初から `&str` である。
プログラムに埋め込まれたバイト列を指していて、プログラムが動いているあいだ消えない。

`&line` は `&String` だが、受け取る側が `&str` と決まっているので、コンパイラが `&str` に変換する。
`String` が自分の中身を `&str` として見せる仕組みを持っているためで、これを[参照外し型強制（deref coercion）](../glossary.md#deref-coercion)と呼ぶ。
変換は、関数の引数のように行き先の型が決まっている場所で起きる。
どこでも勝手に起きるわけではない。

`String` の変数に対して `trim()` のような `str` のメソッドをそのまま呼べるのも、`String` が中身を `&str` として見せる、その同じ仕組みによる。
ただし、こちらは[型強制](../glossary.md#coercion)ではない。
メソッド呼び出しでは、コンパイラが[参照を外し](../glossary.md#dereference)ながら、そのメソッドを持つ型を探す。

引数を `&String` にすると、リテラルを渡すたびに `String` を作ることになる。
[clippy](../glossary.md#clippy) は既定の設定で、この書き方を指摘する。

```
warning: writing `&String` instead of `&str` involves a new object where a slice will do
```

C++17 以降で、読むだけの引数には `const std::string&` より `std::string_view` を取るよう勧められるのと同じ理由である。
違うのは、その先で起きることである。
C++ では、受け取った `string_view` を構造体にしまい込むと、寿命を自分で管理することになる。
Rust では、受け取った `&str` をしまい込むには、しまう先と元の文字列の寿命の関係を型に書く必要がある。
書かなければ通らず、書けばその関係に反する使い方をコンパイラが止める。

## いつ `String` を作るか

`push` の中で `to_owned()` を呼んでいるのは、マップがキーを持ち続けるからである。
マップは呼び手の `line` より長く生きる。
`line` を指す窓をしまえば、`line` が消えたあとに窓だけが残る。
だからしまう前に、自分の `String` を作る。

構造体に `&str` を持たせることもできるが、そのためには 10 章で扱う記法が要る。
それまでは、しまうものは `String` にする。

`to_owned()` は、空でない文字列なら呼ぶたびにヒープを確保する。
C++ の主要な標準ライブラリ実装は、短い文字列を `std::string` オブジェクトの中に直接しまう（[SSO](../glossary.md#sso) と呼ばれる。規格の要求ではない）。
Rust の `String` はそれをしない。
空でない `String` は、短くても必ずヒープに確保する。

つまり今の `push` は、既に数えたことのあるキーでも、1 行ごとに確保している。
これを避ける書き方が演習 1 である。

## `&str` はスライスの一種

`String` と `&str` の関係は、[`Vec<T>`](../glossary.md#vec) と `&[T]` の関係と同じである。
`&[T]` を[スライス](../glossary.md#slice)と呼び、これも先頭へのポインタと長さの二語でできている。

```rust
let lines = vec!["info", "warn", "error"];
let rest = &lines[1..];      // ["warn", "error"] を指す。複製しない
```

範囲の外を指せば、`&str` と同じく実行時に止まる。

`&str` は、UTF-8 であることが保証された `&[u8]` だと考えてよい。
`as_bytes()` を呼べば、複製せずに `&[u8]` として見られる。

引数の型についても同じことが言える。
`&Vec<T>` ではなく `&[T]` を受け取れば、配列の一部も、`Vec` の全体も渡せる。
clippy が `&String` を指摘するのと同じ規則で、`&Vec<T>` も指摘される。

## 本物ではどうなっているか

`tally` が入力を読むループを、今回の話に関わる部分だけ抜き出すとこうなる。

```rust
let mut line = String::new();
loop {
    line.clear();
    if reader.read_line(&mut line)? == 0 {
        break;
    }
    let trimmed = line.strip_suffix('\n').map_or(line.as_str(), |rest| {
        rest.strip_suffix('\r').unwrap_or(rest)
    });
    counter.push_line(selector, trimmed, line_no)?;
}
```

`String` は `line` の一つだけで、すべての行がこのバッファに読み込まれる。
`trimmed` は、そこから行末の改行を除いた部分を眺める窓である。
`push_line` は `&str` を受け取り、キーをしまうときにだけ `String` を作る。

バッファを使い回すには、次の行を読む前に、前の行を眺めていた窓が無くなっていなければならない。
`trimmed` はループの中で宣言しているので、周回の終わりで消える。
だから先頭の `clear` が通る。

`BufRead::lines()` を使えば、もっと短く書ける。
ただし `lines()` は、行ごとに新しい `String` を作って返す。
1000 万行なら 1000 万回の確保になる。
`tally` を `lines()` で書いていたころ、この形に変えて測ったことがある。
行全体をキーにして 1000 万行を数える入力で、610 ms が 403 ms に縮んだ（2026 年 9 月、macOS で測った値）。

`strip_suffix` が返す [`Option`](../glossary.md#option) と、それを受ける `map_or` と `unwrap_or`、そして `?` は 5 章で扱う。
`map_or` に渡している `|rest| ...` は 8 章で扱う。

## 演習

答えは [`book/code/ch03/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch03/src/exercises.rs) にある。
`cargo test -p book-ch03` で確かめられる。
答えは 5 章で扱う `Option` を使っているが、形を見れば読めるはずである。

**演習 1**

`push` を、既に数えたことのあるキーなら `String` を作らない形に書き直せ。
[`HashMap<String, u64>`](../glossary.md#hashmap) は、`&str` のままでもキーを引ける。
その代わり、初めて見るキーで何が増えるかを考えてほしい。

**演習 2**

行の先頭から `max` 文字だけを返す関数を書け。

```rust
fn head(line: &str, max: usize) -> &str
```

日本語の行を渡しても止まらないこと。
`char_indices()` は、各文字をその開始バイト位置と組にして順に返す。
`&line[..max]` ではなぜいけないのかを、この章の言葉で説明してほしい。
