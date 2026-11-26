---
title: 10 ライフタイム注釈
sidebar_position: 10
---

# 10 ライフタイム注釈

9 章までのコードには、`'a` のような印が 1 つも出てこなかった。
それでも参照を返す関数は何度も書いている。
2 章の `sorted` は `&self` から[借りた](../glossary.md#borrow) `&str` を返し、6 章の `field` も `&self` から参照を返した。
どちらも、戻り値がどの引数から借りたものかを、コンパイラが規則で決めていた。

規則で決まらない形もある。
8 章の演習 2 は、`&[&str]` を受け取って [`Vec<&str>`](../glossary.md#vec) を返す形を避け、`&str` を受け取る形にした。
3 章では、[構造体](../glossary.md#struct)に `&str` を持たせるのは 10 章まで待つと書いた。
この章では、そうした形を書くための[ライフタイム注釈](../glossary.md#lifetime-annotation)を扱う。

C++ でいえば、返した [`std::string_view`](../glossary.md#string-view) がどの引数を指しているかは、コメントに書くか、書き手が覚えておくものだった。
Rust では、それを関数の型に書く。
書いた関係は、関数の中と、呼ぶ側の両方でコンパイラが確かめる。
他の言語なら書き手が気をつけていた部分を、型に移したものである。

## この章のコードは、空でない行を選び、欄の値を借りたまま取り出す

この章のコード（`book/code/ch10`）は、行の並びから空行を除き、`name=value` の形の欄から値を取り出す。
どの関数も、新しい文字列を作らずに、受け取った文字列の一部を借りて返す。

```rust
fn first_field(line: &str) -> &str
fn longer<'a>(a: &'a str, b: &'a str) -> &'a str
fn non_empty<'a>(lines: &[&'a str]) -> Vec<&'a str>
fn describe(code: u16) -> &'static str

struct Field<'n> {
    name: &'n str,
}

impl Field<'_> {
    fn pick<'l>(&self, line: &'l str) -> Option<&'l str>
}
```

`cargo run -p book-ch10` の出力はこうなる。

```text
200 成功
404 要求の誤り
503 サーバの失敗
warning
Field { name: "lvl" }
Some("warn")
```

最後の行は、`Field` も、`Field` が借りていた名前の文字列も消えたあとに表示している。
それができる理由が、この章の後半の話になる。

この章に載せるコンパイラの出力は、末尾の `For more information` の行と、話に関係しない help を省いている。

## 引数の参照が 1 つなら、注釈を書かずに済む

`first_field` には注釈が無い。

```rust
fn first_field(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}
```

注釈を省かずに書くと、こうなる。

```rust
fn first_field<'a>(line: &'a str) -> &'a str
```

`'a` は[寿命](../glossary.md#lifetime)の名前で、`<'a>` は[型引数](../glossary.md#type-parameter)と同じ場所で宣言する。
`&'a str` は「`'a` のあいだ有効な `&str`」と読む。
引数と戻り値に同じ `'a` を書くと、「戻り値は、引数を借りているあいだだけ使える」という関係になる。
寿命は呼ぶたびに呼ぶ側の事情で決まるので、関数の側では名前を付けて関係だけを書く。

この関係を書かずに済むのは、[省略規則](../glossary.md#lifetime-elision)があるからである。
規則は 3 つある。

1. 引数の中の参照と、型の名前に隠れた寿命には、それぞれ別の寿命が付く
2. 引数の寿命がちょうど 1 つなら、戻り値の参照はすべてその寿命になる
3. 引数に `&self` か `&mut self` があれば、戻り値の参照は `self` の寿命になる

`first_field` は規則 2 で決まる。
2 章の `sorted(&self)` と 6 章の `field(&self)` は、引数が `self` だけなので、規則 2 でも 3 でも同じ結果になる。

## 借用元が規則で決まらないと、`E0106` で止まる

`longer` から注釈を外すと、通らない。

```rust
fn longer(a: &str, b: &str) -> &str {
    if a.len() >= b.len() { a } else { b }
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:32
  |
1 | fn longer(a: &str, b: &str) -> &str {
  |              ----     ----     ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but the signature does not say whether it is borrowed from `a` or `b`
help: consider introducing a named lifetime parameter
  |
1 | fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
  |          ++++     ++          ++          ++
```

引数に参照が 2 つあり、`self` も無いので、規則 2 も 3 も当てはまらない。
コンパイラは本体を読めば `a` か `b` を返すと分かるが、それで決めることはしない。
戻り値がどこから来るかは、関数の型だけで決まる。
本体を書き換えたときに、呼ぶ側の検査の結果が黙って変わらないようにするためである。

8 章で避けた形も、同じ理由で止まる。

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:37
  |
1 | fn non_empty(lines: &[&str]) -> Vec<&str> {
  |                     -------         ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but the signature does not say which one of `lines`'s 2 lifetimes it is borrowed from
help: consider introducing a named lifetime parameter
  |
1 | fn non_empty<'a>(lines: &'a [&'a str]) -> Vec<&'a str> {
  |             ++++         ++   ++               ++
```

引数は 1 つだが、`&[&str]` の中には参照が 2 つある。
[スライス](../glossary.md#slice)を指す外側の `&` と、要素の `&str` である。
規則 1 で別々の寿命が付くので、規則 2 の「ちょうど 1 つ」に当たらない。

引数に参照が 1 つも無いときも止まる。

```rust
fn describe(code: u16) -> &str {
    if code >= 400 { "失敗" } else { "成功" }
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:1:27
  |
1 | fn describe(code: u16) -> &str {
  |                           ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but there is no value for it to be borrowed from
help: consider using the `'static` lifetime, but this is uncommon unless you're returning a borrowed value from a `const` or a `static`
```

借りてくる先の引数が無いので、返せるのはプログラムの終わりまで有効な参照だけである。
[文字列リテラル](../glossary.md#string-literal)はそれに当たる。
これを `'static` と書く。
この章の `describe` は、戻り値を `&'static str` にしている。

## 注釈は寿命を延ばさず、引数と戻り値の関係だけを書く

`longer<'a>` は、`a` と `b` と戻り値に同じ `'a` を書いた。
呼ぶたびに、`'a` は 2 つの借用がどちらも有効な範囲に決まる。
戻り値はその範囲でしか使えない。
`b` のほうが先に消えるなら、戻り値も `b` と一緒に使えなくなる。

```rust
let a = String::from("warning");
let result;
{
    let b = String::from("error");
    result = longer(&a, &b);
}
println!("{result}");
```

```text
error[E0597]: `b` does not live long enough
  --> src/main.rs:10:29
   |
 9 |         let b = String::from("error");
   |             - binding `b` declared here
10 |         result = longer(&a, &b);
   |                             ^^ borrowed value does not live long enough
11 |     }
   |     - `b` dropped here while still borrowed
12 |     println!("{result}");
   |                ------ borrow later used here
```

実行すれば `longer` は長いほうの `a` を返すので、このコードが消えた `b` を読むことは無い。
それでも断られる。
呼ぶ側の検査も、本体ではなく関数の型だけを見る。
型が「`a` か `b` のどちらか」と言っている以上、`b` が消えたあとでは戻り値も使えない。
`result` を使うのを内側の `{ }` の中に移せば通る。

注釈は、値をそれだけ長く生かす指示ではない。
関数の中で作った値への参照は、注釈を付けても返せない。

```rust
fn shout<'a>(line: &'a str) -> &'a str {
    let upper = line.to_uppercase();
    &upper
}
```

```text
error[E0515]: cannot return reference to local variable `upper`
 --> src/main.rs:3:5
  |
3 |     &upper
  |     ^^^^^^ returns a reference to data owned by the current function
```

`upper` は関数を抜けるときに消える。
注釈が言っているのは「戻り値は `line` から借りる」ということで、`upper` はそれに当たらない。
新しく作った文字列を返したいなら、戻り値を `String` にして持ち主ごと渡す。

C++ の規格には、関数の型に借用の関係を書く仕組みが無い。
clang は独自の[属性](../glossary.md#attribute)をいくつか持っていて、その 1 つの `[[clang::lifetimebound]]` は、「戻り値はこの引数を指しうる」と引数に印を付ける。
手元の Apple clang 21 で `longer` を `std::string_view` で書き、両方の引数に印を付けて試した（警告の指定は `-Wall -Wextra`）。
`longer(a, std::string("error"))` のように一時オブジェクトを渡した形には、`-Wdangling-gsl` の警告が出た。
内側の `{ }` で作った `std::string b` を渡し、`{ }` を出てから戻り値を使った形には、何も出なかった。
印を付けなかった版では、どちらの形でも何も出なかった。
clang の印は書かなくてもよく、見つけたものを警告するだけである。
Rust の注釈は、規則で決まらない形では書かないと通らず、書いた関係はすべての呼び出しで確かめられる。

## 結ぶ相手を間違えると、関数の中で止まる

関係を書いたら、本体はその関係を守らなければならない。
戻り値を `a` の側にだけ結んでおいて `b` を返すと、本体のほうで止まる。

```rust
fn pick<'a, 'b>(a: &'a str, b: &'b str) -> &'a str {
    if a.is_empty() { b } else { a }
}
```

```text
error: lifetime may not live long enough
 --> src/main.rs:2:23
  |
1 | fn pick<'a, 'b>(a: &'a str, b: &'b str) -> &'a str {
  |         --  -- lifetime `'b` defined here
  |         |
  |         lifetime `'a` defined here
2 |     if a.is_empty() { b } else { a }
  |                       ^ function was supposed to return data with lifetime `'a` but it is returning data with lifetime `'b`
  |
  = help: consider adding the following bound: `'b: 'a`
```

関数の型は、本体と呼ぶ側の両方に対する約束である。
本体は約束より長く借りたものを返せず、呼ぶ側は約束より長く戻り値を使えない。

約束が通るからといって、それで十分とは限らない。
`non_empty` の `E0106` を出したとき、コンパイラは `&'a [&'a str]` という直し方を示した。
これはスライスと要素の両方を `'a` に結ぶ。
通りはするが、呼ぶ側に要らない制限がかかる。

```rust
fn non_empty<'a>(lines: &'a [&'a str]) -> Vec<&'a str>

let text = String::from("a\n\nb\n");
let kept;
{
    let lines: Vec<&str> = text.lines().collect();
    kept = non_empty(&lines);
}
println!("{kept:?}");
```

```text
error[E0597]: `lines` does not live long enough
  --> src/main.rs:10:26
   |
 7 |     let kept;
   |         ---- variable `kept` declared here
 8 |     {
 9 |         let lines: Vec<&str> = text.lines().collect();
   |             ----- binding `lines` declared here
10 |         kept = non_empty(&lines);
   |                          ^^^^^^ borrowed value does not live long enough
11 |     }
   |     - `lines` dropped here while still borrowed
12 |     println!("{kept:?}");
   |                ---- borrow later used here
   |
   = note: `kept` is a collection that stores borrowed references, but `lines` does not live long enough to be stored in it
   = help: buffer reuse with borrowed references requires unsafe code or restructuring
```

help は [`unsafe`](../glossary.md#unsafe) か書き直しを勧めているが、ここで直すのは呼ぶ側ではなく関数の型のほうである。

`kept` に入っている `&str` は、`text` の一部を指している。
途中で作った `lines` という `Vec` を指しているわけではない。
それなのに、型が「戻り値は `lines` も借りている」と言っているので、`lines` を先に消せない。

この章の `non_empty` は、要素の寿命にだけ名前を付けている。

```rust
fn non_empty<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .copied()
        .filter(|line| !line.is_empty())
        .collect()
}
```

外側の `&` には名前が無いので、規則 1 で別の寿命が付き、戻り値とは結ばれない。
本体の `copied()` は、`&&str` を 1 つずつ返す[イテレータ](../glossary.md#iterator)を、`&str` を返すイテレータに変える。
参照そのものを写すだけで、文字列は写さない。
`book/code/ch10` のテストでは、`lines` を消したあとに `kept` を使っている。
コンパイラの示す直し方は、通るための 1 つの案である。
何と何を結ぶかは、戻り値が実際にどこを指しているかを見て、書き手が決める。

## メソッドの戻り値は、既定で `self` に結ばれる

`Field` の `pick` は、行の中から値を探して返す。
戻り値が指しているのは `line` の一部で、`Field` の中身ではない。
ところが注釈を書かないと、規則 3 によって戻り値は `self` に結ばれる。

```rust
impl Field<'_> {
    fn pick(&self, line: &str) -> Option<&str> {
        line.split_whitespace()
            .filter_map(|pair| pair.split_once('='))
            .find(|&(name, _)| name == self.name)
            .map(|(_, value)| value)
    }
}
```

```text
error: lifetime may not live long enough
  --> src/main.rs:7:9
   |
 6 |       fn pick(&self, line: &str) -> Option<&str> {
   |               -            - let's call the lifetime of this reference `'1`
   |               |
   |               let's call the lifetime of this reference `'2`
 7 | /         line.split_whitespace()
 8 | |             .filter_map(|pair| pair.split_once('='))
 9 | |             .find(|&(name, _)| name == self.name)
10 | |             .map(|(_, value)| value)
   | |____________________________________^ method was supposed to return data with lifetime `'2` but it is returning data with lifetime `'1`
```

`split_once('=')` は最初の `=` で 2 つに割り、`=` が無ければ `None` を返す。
`filter_map` は、[クロージャ](../glossary.md#closure)が `Some` を返した要素だけを中身にして残す。

規則 3 は、[メソッド](../glossary.md#method)の多くが `self` の中身を返すことに合わせた既定である。
`self` 以外から借りたものを返すなら、注釈で結び直す。
4 章の `Key::extract(&self, line: &str)` は、この注釈を使わずに済ませるため、取り出した値を `String` にして返していた。

```rust
fn pick<'l>(&self, line: &'l str) -> Option<&'l str>
```

こう書くと、戻り値は `line` を借りているあいだ使え、`Field` が消えても困らない。
冒頭の出力の最後の行は、それを試したものである。

```rust
let line = String::from("lvl=warn msg=disk");
let value;
{
    let name = String::from("lvl");
    let field = Field { name: &name };
    value = field.pick(&line);
    println!("{field:?}");
}
// `name` と `field` はもう無い。`value` が借りているのは `line` だけなので使える。
println!("{value:?}");
```

規則 3 のままの `pick` なら、仮に本体が通ったとしても、この `value` は `field` が消えた時点で使えなくなる。

## 参照をしまう構造体は、寿命の引数を持つ

`Field` は、探す名前を `String` で持たず、呼び手の文字列を借りている。
構造体のフィールドに `&str` を書くときは、寿命に名前が要る。

```rust
struct Field {
    name: &str,
}
```

```text
error[E0106]: missing lifetime specifier
 --> src/main.rs:2:11
  |
2 |     name: &str,
  |           ^ expected named lifetime parameter
  |
help: consider introducing a named lifetime parameter
  |
1 ~ struct Field<'a> {
2 ~     name: &'a str,
  |
```

関数と違い、構造体のフィールドには省略規則が無い。
`struct Field<'n>` と書くと、`Field` は寿命 `'n` を引数に取る型になる。
`Field<'n>` の値は `'n` より長く生きられない。
借りている名前の文字列が消える前に、`Field` のほうが先に消えなければならない。
3 章で、C++ では構造体にしまった `string_view` の寿命を自分で管理すると書いた。
Rust では、その関係が `Field<'n>` という型に出る。

`impl Field<'_>` の `'_` は、名前を付けない寿命である。
`pick` の中で `'n` を使わないので、名前を付ける必要が無い。
名前を使う場面は、演習 2 で出てくる。

## 隠れた寿命も、`'_` で見えるようにできる

9 章では `Display` を実装するときに `f: &mut fmt::Formatter` と書いた。
`Formatter` は書き出し先を借りている型で、本当は寿命の引数を 1 つ持っている。
引数の中の型に隠れた寿命は、規則 1 で別の寿命が付くので、省いても通る。
9 章ではそこを書かなかった。
構造体のフィールドに書けば、`&str` と同じく `E0106` になる。

`tally` は、ここを `fmt::Formatter<'_>` と書いている。
`'_` があると、この型が何かを借りていることが読み手に見える。
コンパイラには、省いた箇所を警告する lint `elided_lifetimes_in_paths` がある。
既定では無効で、`rust_2018_idioms` という lint の組に含まれている。
有効にして 9 章の書き方を検査すると、`hidden lifetime parameters in types are deprecated` と警告し、`'_` を足すよう示す。

`fn quote_field(value: &str) -> Cow<str>`（`Cow` はこの章の後半で扱う）のように、引数では `&` と書いた寿命を戻り値では型の名前に隠すと、`mismatched_lifetime_syntaxes` が `hiding a lifetime that's elided elsewhere is confusing` と警告する。
どちらも同じ寿命なのに、書き方がそろっていないからである。

## `'static` は、借用を含まないことも表す

`describe` の `&'static str` は、プログラムの終わりまで有効な参照である。
文字列リテラルは実行ファイルに埋め込まれているので、いつまでも指していられる。
`tally` のエラーに添える助言も [`Option`](../glossary.md#option)`<&'static str>` で、文字列リテラルを返している。

`'static` は、[境界](../glossary.md#trait-bound)の位置にも書ける。
`T: 'static` は「`T` は `'static` より短い借用を含まない」という条件で、値がずっと生きるという意味ではない。

```rust
fn keep<T: 'static>(value: T) -> T {
    value
}

let name = String::from("warn");
let owned = keep(name.clone());
let borrowed = keep(&name);
```

`String` は何も借りていないので、`keep(name.clone())` は通る。
`String` の値そのものは、使い終われば普通に消える。
`&name` は `name` という変数を借りているので、`E0597`（argument requires that `name` is borrowed for `'static`）で断られる。

9 章の `Box<dyn Error + Send + Sync>` にも、この条件が隠れている。
[`dyn`](../glossary.md#dyn) の型は、中の値が何を借りているかを型の外に出さないので、寿命を別に持つ。
関数の引数や戻り値、構造体のフィールドに `Box<dyn Trait>` と書くと、その寿命は `'static` とみなされる。
関数の中の `let` に書いた型では、寿命は推論される。
`&str` を `Box<dyn Display>` に入れて返そうとすると、断られる。

```rust
fn boxed(value: &str) -> Box<dyn Display> {
    Box::new(value)
}
```

```text
error: lifetime may not live long enough
 --> src/main.rs:4:5
  |
3 | fn boxed(value: &str) -> Box<dyn Display> {
  |                 - let's call the lifetime of this reference `'1`
4 |     Box::new(value)
  |     ^^^^^^^^^^^^^^^ returning this value requires that `'1` must outlive `'static`
  |
help: to declare that the trait object captures data from argument `value`, you can add an explicit `'_` lifetime bound
```

借りたものを入れてよいなら、示されたとおり `Box<dyn Display + '_>` と書く。

## 寿命の長短は自動で合わせるが、`&mut` の先では合わせない

長く生きる参照は、短い寿命の参照が求められる場所にそのまま渡せる。
この章のテストでは、`&'static str` の並びと、関数の中で作った `String` を借りた参照を、同じ `'a` を求める関数に渡している。

```rust
fn first_or<'a>(items: &[&'a str], fallback: &'a str) -> &'a str {
    items.first().copied().unwrap_or(fallback)
}

let fixed: Vec<&'static str> = vec!["warn"];
let local = String::from("error");
first_or(&fixed, &local);
```

`items.first()` は `Option<&&str>` を返し、`Option` の `copied()` がそれを `Option<&str>` にする。
`'a` は `local` を借りている範囲に決まり、`&'static str` はその範囲でも有効なので、そのまま `&'a str` として扱われる。

ところが、可変参照の先では同じことが通らない。

```rust
fn push_line<'a>(lines: &mut Vec<&'a str>, line: &'a str) {
    lines.push(line);
}

let mut fixed: Vec<&'static str> = vec!["warn"];
let local = String::from("error");
push_line(&mut fixed, &local);
println!("{fixed:?}");
```

```text
error[E0597]: `local` does not live long enough
  --> src/main.rs:8:27
   |
 6 |     let mut fixed: Vec<&'static str> = vec!["warn"];
   |                    ----------------- type annotation requires that `local` is borrowed for `'static`
 7 |     let local = String::from("error");
   |         ----- binding `local` declared here
 8 |     push_line(&mut fixed, &local);
   |                           ^^^^^^ borrowed value does not live long enough
 9 |     println!("{fixed:?}");
10 | }
   | - `local` dropped here while still borrowed
```

もし `&mut Vec<&'static str>` を `&mut Vec<&'a str>` として渡せたら、`push_line` は短い寿命の `&local` を `fixed` に入れられる。
呼び終えたあとの `fixed` は、型の上では `'static` の参照だけを持つはずなのに、`local` が消えたあとも `local` を指す要素を抱えることになる。
だから、書き換えられる場所の中身の寿命は、ぴったり一致しなければならない。
`fixed` の型注釈を外せば、要素の寿命は `local` に合わせて短く推論され、通る。

型の中身の寿命や型を、長いものから短いものへ読み替えてよいかどうかを[変性](../glossary.md#variance)と呼ぶ。
Scala では型引数ごとに `+T` や `-T` と書き、C# では `out T` や `in T` と書いて宣言する。
Rust には、それを書く場所が無い。
`&T` は読み替えてよく、`&mut T` の `T` は読み替えてはいけない、といった性質を、コンパイラが型の構造から決める。

## 本物ではどうなっているか

`tally-core` で 1 行からキーを取り出す `Selector::select` は、`pick` と同じ形をしている。

```rust
pub fn select<'a>(
    &self,
    line: &'a str,
    line_no: usize,
) -> Result<Option<Cow<'a, str>>, LineError>
```

`Cow<'a, str>` は、`&'a str` を借りたまま持つか、自分の `String` を持つかのどちらかになる [enum](../glossary.md#enum) である。
行全体をキーにするときや、JSON の値に引用符の中のエスケープが無いときは、行の一部を借りたまま返し、文字列を作らない。
大文字と小文字を区別しない指定のときは、小文字にした文字列を作ることがある。
借りたものを返しうるので、戻り値を `line` の寿命 `'a` に結んでいる。
`self` に結ばれていたら、呼ぶ側は取り出したキーを使い終えるまで `Selector` を手放せない。

そのテストには、1 行だけ通して取り出す補助関数がある。

```rust
/// 参照が 2 つあるため省略規則では出力の寿命が決まらない（E0106）。
/// 戻り値が借用しうるのは `line` の側なので、明示的に紐づける。
fn select_one<'a>(selector: &Selector, line: &'a str) -> Cow<'a, str> {
```

`&Selector` を普通の引数として受け取り、`self` を取らないので、規則 3 は働かず、注釈が無ければ `E0106` になる。
コメントは、どちらに結ぶかを選んだ理由を書いている。

引数の参照が 1 つなら、`Cow` を返すときも注釈は要らない。
CSV に書き出す欄に引用符を付ける関数は、こうなっている。

```rust
fn quote_field(value: &str) -> Cow<'_, str> {
    // 配列を渡すと「いずれかの文字を含むか」になる（`Pattern` の実装）。
    // 4 回 contains を書くより速く、意図も直接読める。
    if value.contains([',', '"', '\r', '\n']) {
        Cow::Owned(format!("\"{}\"", value.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(value)
    }
}
```

規則 2 で戻り値は `value` に結ばれる。
`'_` を省いて `Cow<str>` と書くと、「隠れた寿命も、`'_` で見えるようにできる」の節で見た `mismatched_lifetime_syntaxes` が警告する。
警告をエラーとして扱う設定（`-D warnings`）では、ここで `'_` を省けない。

JSON を 1 行ずつ読むところでは、構造体が寿命の引数を持つ。
探すフィールド名を借りる `PickField<'f>` と、JSON Pointer の残りの経路を借りる `PickPath<'p>` である。

```rust
struct PickPath<'p> {
    head: &'p str,
    rest: &'p [Box<str>],
}

impl PickPath<'_> {
    fn descend(&self) -> Option<Self> {
        self.rest
            .split_first()
            .map(|(head, rest)| Self { head, rest })
    }
}
```

`descend` は 1 段先へ進んだ `PickPath` を返す。
戻り値の型 `Self` は `PickPath<'_>` で、寿命は `impl` に書いた `'_` と同じものになる。
新しい `PickPath` は、`self` を借りているのではなく、`self` と同じ経路の文字列を借りている。
そのため、元の `PickPath` を `&self` として借り終えても、返った値はそのまま使える。

9 章で載せなかった標準入力の `Input` の実装も、ここで読める。

```rust
impl Input for Stdin {
    type Reader = StdinLock<'static>;
    // ...
}
```

`StdinLock` は[関連型](../glossary.md#associated-type) `Reader` に入る型である。
標準入力をロックしているあいだ読み出しを独り占めする型で、寿命の引数を持つ。
標準入力の本体はプログラムの中に 1 つだけあり、プログラムの終わりまで消えない。
だから、ロックの寿命は `'static` になり、`Reader` に寿命の引数を足さずに済んでいる。

エラーの型には、`fn source(&self) -> Option<&(dyn Error + 'static)>` という形が出てくる。
外側の `&` は省略規則で `self` に結ばれ、`dyn Error + 'static` は中のエラーが借用を含まないことを言っている。
エラーの型をどう組むかは、11 章で扱う。

## 演習

答えは `book/code/ch10/src/exercises.rs` にある。
`cargo test -p book-ch10` で確かめられる。

1. `key: value` のような行を、区切りの文字列でキーと値に分ける関数 `split_key` を書く。
   引数は行 `line: &str` と区切り `sep: &str` の 2 つで、区切りが無ければ `None` を返す。
   区切りを `String` で作って渡し、その `String` が消えたあとでも、返したキーと値を使えるようにする。
   割るには、`pick` でも使った `str::split_once` が使える。
2. 文字列を 1 行ずつ切り出す構造体 `Lines` を作り、`next_line(&mut self)` で次の行を返す。
   切り出した 1 行目を持ったまま `next_line` をもう一度呼び、2 行目も取れるようにする。
   行の割り方は標準の `str::lines` に合わせ、末尾の改行の後ろに空の行を作らない（行末の `\r` は扱わなくてよい）。
   省略規則に任せたときにどのエラーになるかも見る。
