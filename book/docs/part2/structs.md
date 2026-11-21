---
title: 6 構造体・メソッド・可視性
sidebar_position: 6
---

# 6 構造体・メソッド・可視性

`tally` の集計器 `Counter` には、外からはメソッドを通してしか触れない。
数えた回数も、飛ばした行の数も、フィールドとしては見えない。
一方、集計結果の 1 行を表す `Entry` は、`key` と `count` の 2 つのフィールドがそのまま見える。

何を見せて何を隠すかを、Rust は**モジュール**の単位で決める。
この章では、構造体の作り方、メソッドが `self` を受け取る 3 つの形、可視性の単位を扱う。
II 部の最後の章である。

## この章のコードは 2 つのクレートに分かれている

この章のコード（`book/code/ch06`）には、`src/lib.rs` と `src/main.rs` がある。
Cargo は、この 2 つを別々の[クレート](../glossary.md#crate)としてビルドする。
`lib.rs` がライブラリ `book_ch06` になり、`main.rs` はそれを使う実行ファイルになる。

```rust
// src/main.rs
use book_ch06::{FieldName, Tally};
```

`main.rs` から見たライブラリは、他人が書いたクレートと同じ扱いになる。
ライブラリが外に出すと決めたものしか使えない。
本物の `tally` も、集計のライブラリ `tally-core` と、引数・入出力・終了コードを受け持つ実行ファイル `tally` に分かれている。
この章で見る可視性のエラーの多くは、本物と同じクレートの境界で起きる。

## 構造体にはコンストラクタが無い

集計器は、名前の付いたフィールドを並べた[構造体](../glossary.md#struct)として定義する。

```rust
pub struct Tally {
    field: FieldName,
    strict: bool,
    counts: HashMap<String, u64>,
    skipped: usize,
    lines: usize,
}
```

`FieldName` は数えるフィールドの名前を持つ型で、あとの節で作る。
`pub` の意味も、あとの節で扱う。

構造体の値は、型の名前に続けて、全部のフィールドに値を書いて作る。
1 つでも書き忘れるとコンパイルが通らない。
次は、あとで見る `new` の中で `lines` を書き忘れたときのエラーである。
この章のエラーは、どれも示したコードを `main.rs` や `tally.rs` に一時的に置いて出したもので、行番号はそのときの位置である。
（別の値から残りのフィールドを写す `..` という書き方もあるが、そのときも写し元を `..other` のように書く。）

```
error[E0063]: missing field `lines` in initializer of `Tally`
  --> src/tally.rs:86:9
   |
86 |         Self {
   |         ^^^^ missing `lines`
```

C++ で同じ形の集成体（aggregate）を `Tally t{"level", true};` のように初期化すると、書かなかったフィールドは黙って初期化される（既定の初期化子が無ければ 0 になる）。
手元の clang では、この位置で並べる形について、`-Wall` では何も言わず、`-Wextra` を付けたときだけ `-Wmissing-field-initializers` の警告が出た。
`.field = "level"` のように名前を指定する形では、`-Wextra` でも警告は出なかった。

Rust には、C++ のようなコンストラクタも無い。
値を作る手続きは、`impl` の中に書く普通の関数である。

```rust
impl Tally {
    #[must_use]
    pub fn new(field: FieldName) -> Self {
        Self {
            field,
            strict: false,
            counts: HashMap::new(),
            skipped: 0,
            lines: 0,
        }
    }
}
```

`self` を受け取らないこの関数は関連関数と呼ばれ、C++ の static メンバ関数に当たる。
呼ぶときは `Tally::new(field)` と書く。
`new` という名前は慣習にすぎず、言語が特別扱いすることはない。
`impl` の対象の型（ここでは `Tally`）は、`Self` と書ける。
`field,` は `field: field,` の省略で、変数とフィールドの名前が同じときに使える。
先頭の `#[must_use]` は、あとの節で扱う。

関数の多重定義（オーバーロード）も無い。
`new` を引数違いでもう 1 つ書くと、`E0592`（duplicate definitions with name `new`）で止まる。
既定の引数も無いので、作り方が何通りもあるときは、名前の違う関数にするか、次の節の形で設定を足していく。

## メソッドは `self` の受け取り方を 3 つから選ぶ

[メソッド](../glossary.md#method)は、第 1 引数に `self` を取る関数である。
`self` の受け取り方は 3 つあり、`Tally` はその 3 つを全部使っている。

```rust
impl Tally {
    pub fn count_of(&self, value: &str) -> u64 { /* ... */ }
    pub fn push_line(&mut self, line: &str) -> Result<(), MissingField> { /* ... */ }
    pub fn finish(self) -> Report { /* ... */ }
}
```

| 受け取り方 | できること | C++ で近いもの |
| --- | --- | --- |
| `&self` | 読むだけ | `const` メンバ関数 |
| `&mut self` | 書き換える | `const` の付かないメンバ関数 |
| `self` | 値ごと受け取る。呼んだあとの値は使えない（`Copy` な型を除く） | `&&` 修飾のメンバ関数 |

`&self` と `&mut self` は、2 章の[共有参照と可変参照](../glossary.md#references)そのものである。
`const` メンバ関数でも `mutable` を付けたメンバは書き換えられるが、`&self` のメソッドはフィールドを書き換えられない。
例外は、2 章で見た[内部可変性](../glossary.md#interior-mutability)を持つ型だけである。

`tally.push_line(line)` と `.` で呼ぶと、コンパイラが `&mut tally` を作って渡す。
呼ぶ側で `&mut` を書かなくてよい代わりに、値を持つ変数 `tally` が `let mut` で宣言されていなければ通らない。

```
error[E0596]: cannot borrow `tally` as mutable, as it is not declared as mutable
 --> src/main.rs:7:5
  |
7 |     tally.push_line("level=warn").unwrap();
  |     ^^^^^ cannot borrow as mutable
  |
help: consider changing this to be mutable
  |
6 |     let mut tally = Tally::new(field);
  |         +++
```

### `self` で受け取ると、中身を複製せずに持ち出せる

`finish` は `self` を値で受け取る。
`self` はもう `finish` のものなので、フィールドの `counts` を複製せずに `Report` へ [move](../glossary.md#move) できる。

```rust
#[must_use]
pub fn finish(self) -> Report {
    Report {
        counts: self.counts,
        skipped: self.skipped,
    }
}
```

呼んだあとの `tally` は、1 章の move のあとと同じく使えなくなる。

```
error[E0382]: borrow of moved value: `tally`
   --> src/main.rs:9:20
    |
  6 |     let tally = Tally::new(field);
    |         ----- move occurs because `tally` has type `Tally`, which does not implement the `Copy` trait
  7 |     let report = tally.finish();
    |                        -------- `tally` moved due to this method call
  8 |     println!("{}", report.skipped);
  9 |     println!("{}", tally.count_of("warn"));
    |                    ^^^^^ value borrowed here after move
    |
note: `Tally::finish` takes ownership of the receiver `self`, which moves `tally`
```

C++ でも、[`&&` 修飾](../glossary.md#ref-qualifier)のメンバ関数 `Report finish() &&` を書けば、中身を `std::move` で持ち出せる。
`t.finish()` と左辺値のまま呼べばコンパイルエラーになり、`std::move(t).finish()` と書かせるところまでは同じである。
違いはそのあとに出る。
手元の clang では、`std::move(t).finish()` のあとで `t.count_of("warn")` を呼んでも、`-Wall -Wextra` で何も言わずにコンパイルが通り、`0` を返した。
move されたあとの `std::map` は[「有効だが未規定」](../glossary.md#valid-but-unspecified)の状態で、`0` が返ったのは libc++ でたまたまそうだったにすぎない。
clang の静的解析（`--analyze`）を回したときだけ、move されたあとのオブジェクトを使ったという警告が出た。
ただし、`count_of` の中身を少し変えると、静的解析でも出なくなることがあった。

### 設定は `self` を受け取って返すメソッドで足す

`strict` も `self` を値で受け取るが、書き換えてからそのまま返す。

```rust
#[must_use]
pub fn strict(mut self, yes: bool) -> Self {
    self.strict = yes;
    self
}
```

`mut self` の `mut` は、`let mut` と同じく、受け取った値を書き換えてよいという印である。
こう書くと、作るところから設定までを 1 つの式で書ける。

```rust
let mut tally = Tally::new(field).strict(true);
```

多重定義も既定の引数も無いので、省略できる設定はこうしてメソッドの名前で足していく。

`#[must_use]` は、戻り値を捨てたら警告を出させる[属性](../glossary.md#attribute)である。
`tally.strict(true);` とだけ書くと、strict にした `Tally` は戻り値としてその場で捨てられる。
元の `tally` も `strict` に move されているので、もう使えない。
この属性があれば `` unused return value of `Tally::strict` that must be used `` と警告が出る。
属性を外した版で同じ 1 行を試すと、何も言わずにコンパイルが通った。
このリポジトリが有効にしている [clippy](../glossary.md#clippy) の pedantic は、`Self` を返す公開メソッドにこの属性が無いと警告する（`return_self_not_must_use`）。

## 参照を返すメソッドは `self` 全体を借りる

フィールドを読ませるだけのメソッドは、`&self` から参照を返す。

```rust
#[must_use]
pub fn field(&self) -> &FieldName {
    &self.field
}
```

返した参照は `self` から[借りた](../glossary.md#borrow)ものだと、コンパイラは引数と戻り値の形から読み取る。
これを自分で書く書き方（ライフタイム注釈）は 10 章で扱う。

ここに Rust 特有の制約がある。
メソッドで借りると、借りるのはフィールド 1 つではなく `self` 全体になる。
`impl Tally` の中で、名前を読んだまま別のフィールドを書き換えようとすると、こうなる。

```rust
fn note(&mut self) {
    let name = self.field();
    self.counts.insert(String::from("(none)"), 0);
    println!("{}", name.as_str());
}
```

```
error[E0502]: cannot borrow `self.counts` as mutable because it is also borrowed as immutable
   --> src/tally.rs:120:9
    |
119 |         let name = self.field();
    |                    ---- immutable borrow occurs here
120 |         self.counts.insert(String::from("(none)"), 0);
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
121 |         println!("{}", name.as_str());
    |                        ---- immutable borrow later used here
```

`field` と `counts` は別のフィールドだが、`self.field()` の戻り値は `self` 全体を借りているので、`counts` も書き換えられない。
1 行目を `let name = &self.field;` と書き換えると通る。
フィールドを直接借りれば、借りるのは `field` だけで、`counts` は空いている。

ただし、直接借りても、続けて `&mut self` のメソッドを呼べば同じエラーになる。
`self.push_line("x")` は `self` 全体を書き換える約束なので、`field` を借りているあいだは呼べない。

C++ のゲッターが `const` 参照を返しても、呼んだ側にこうした制約はかからない。
Rust では、型の中のコードはフィールドを直接触り、参照を返すメソッドは主に外から使う、という書き分けになりやすい。

## 可視性はモジュールが単位になる

ここまでのコードでは `pub` が付いたり付かなかったりしていた。
Rust の項目（型、関数、フィールド、メソッド）は、何も付けなければ非公開で、`pub` を付けたものだけが外から見える。
フィールドとメソッドにも 1 つずつ付ける。
例外の 1 つは [enum](../glossary.md#enum) のバリアントで、enum に `pub` を付ければバリアントも全部公開される。

非公開の項目が見える範囲は、型ではなく、それを定義した[モジュール](../glossary.md#module)で決まる。
モジュールは、名前をまとめる入れ物である。
`lib.rs` に `mod tally;` と書くと、`src/tally.rs` の中身が `tally` というモジュールになる。
`Tally` も `FieldName` も、そのモジュールの中に定義してある。

非公開の項目は、定義したモジュールと、その子孫のモジュールから見える。
次の 3 つが、`tally` モジュールの中で実際に起きていることである。

- **同じモジュールの、メソッドでない関数から見える。**
  行から値を探す `find_field` は、`impl FieldName` の外にある普通の関数だが、`FieldName` の非公開のフィールドを読んでいる。
- **子のモジュールから見える。**
  テストを入れた `mod tests` は `tally` の子なので、`tally.lines` や `tally.skipped` をそのまま読んで確かめられる。
- **親からは見えない。**
  `lib.rs` も `main.rs` も、`Tally` の非公開のフィールドには触れない。

```rust
// src/tally.rs の中
fn find_field(line: &str, name: &FieldName) -> Option<String> {
    let name = name.0.as_str();   // 非公開のフィールドを読める
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;   // 親のモジュールの名前を全部取り込む

    #[test]
    fn 子のモジュールからは非公開のフィールドが見える() {
        let mut tally = Tally::new(level());
        tally.push_line("level=warn").expect("緩いので失敗しない");
        tally.push_line("user=a").expect("緩いので失敗しない");
        assert_eq!(tally.lines, 2);
        assert_eq!(tally.skipped, 1);
        // ...
    }
}
```

`#[cfg(test)]` を付けたモジュールは `cargo test` のときだけコンパイルされ、その中の `#[test]` を付けた関数がテストとして走る。
`assert_eq!` は、2 つの値が等しくなければテストを失敗させる。
`super` は親のモジュールを指す。
`level()` は、`level` という名前の `FieldName` を作るテスト用の関数である。
`.0` は次の節で扱う。

ほかの言語では、`private` の単位は型である。

- **C++** は、クラスが単位になる。外の関数に見せたければ、クラスの中で [`friend`](../glossary.md#friend) と宣言する
- **Java** の `private` は、いちばん外側のクラスが単位になる。入れ子のクラスからは見えるが、同じファイルの別のクラスからは見えない。メンバに修飾子を書かなければ、同じパッケージ全体から見える
- **C#** の `private` は、型が単位になる。入れ子の型からは見える。メンバに修飾子を書かなければ `private` になる

Rust で C++ の `friend` に当たることをしたければ、同じモジュールに置く。
型ごとに境界を引きたければ、型ごとにモジュールを分ける。

範囲を区切った公開もある。
`pub(crate)` は同じクレートの中だけに見せ、C# の `internal` に近い。
`pub(super)` は親のモジュールまで見せる。

### `pub` を付けただけでは外に出ない

`tally` モジュールの中の `Tally` には `pub` が付いている。
それでも、`main.rs` から `book_ch06::tally::FieldName` や `book_ch06::tally::Tally` とは書けない。
`mod tally;` に `pub` が付いていないので、モジュールそのものが外から見えないからである。

```
error[E0603]: module `tally` is private
  --> src/main.rs:3:28
   |
 3 |     let field = book_ch06::tally::FieldName::new("level");
   |                            ^^^^^  --------- struct `FieldName` is not publicly re-exported
   |                            |
   |                            private module
```

外に出しているのは、`lib.rs` のこの 1 行である。

```rust
pub use tally::{FieldName, FieldNameError, MissingField, Report, Tally};
```

`pub use` を使うと、別のモジュールの項目を、ここにもあるものとして公開できる（再エクスポート）。
これで `book_ch06::Tally` と書けるようになる。
クレートの外から見た公開の範囲は、この 1 行を読めば全部分かる。

`pub` を付けたのに `pub use` し忘れた項目は、外からはたどり着けない。
クレートの中で使っていれば既定では何も言われないが、このリポジトリでは有効にしている `unreachable_pub` の lint が `` unreachable `pub` item `` と警告し、`pub(crate)` に絞るよう勧める。

## 隠したフィールドで、正しい値しか無いことを保証する

数えるフィールドの名前には、守ってほしい条件がある。
この章のコードは、4 章と同じく、空白で区切った `名前=値` の並びを行として読む。
`find_field` は、語が名前で始まり、そのすぐ後ろが `=` なら、残りを値とみなす。
空白を含む名前は、どの行とも一致しない。
`=` を含む名前を許すと、`a=b=c` という語を「`a` の値が `b=c`」とも「`a=b` の値が `c`」とも読めてしまう。
空の名前も困る。

そこで、名前を `String` のまま持ち回らずに、条件を確かめてから作る型で包む。

```rust
pub struct FieldName(String);

impl FieldName {
    pub fn new(raw: &str) -> Result<Self, FieldNameError> {
        if raw.is_empty() {
            return Err(FieldNameError::Empty);
        }
        for c in raw.chars() {
            if c == '=' || c.is_whitespace() {
                return Err(FieldNameError::Forbidden(c));
            }
        }
        Ok(Self(raw.to_owned()))
    }
}
```

`struct FieldName(String);` のように、フィールドに名前を付けず位置だけで並べる構造体を、タプル構造体と呼ぶ。
中身は `.0`、`.1` と番号で読む。
フィールドが 1 つだけのこの形で既存の型を包むやり方を、[newtype](../glossary.md#newtype) と呼ぶ。

`String` に `pub` が付いていないので、`main.rs` からは `FieldName` を直接作れない。

```
error[E0423]: cannot initialize a tuple struct which contains private fields
  --> src/main.rs:5:16
   |
 5 |     let name = FieldName(String::from("a b"));
   |                ^^^^^^^^^
   |
note: constructor is not visible here due to private fields
  --> src/tally.rs:13:22
   |
13 | pub struct FieldName(String);
   |                      ^^^^^^ private field
```

中身を `ok.0` で読むこと（`E0616`）も、`let FieldName(s) = ok;` と[パターン](../glossary.md#pattern)で取り出すこと（`E0532`）もできない。
外から作る道は `FieldName::new` しか無く、そこで条件を確かめている。
だから、モジュールの外にある `FieldName` は、どれも条件を満たしている。
`main.rs` から受け取った名前を、`Tally` は確かめ直さなくてよい。

ただし、`tally` モジュールの中のコードは `FieldName(...)` と直接書ける。
テストでは、条件を破った名前をわざと直接作って、`find_field` がどう読むかを確かめている。
条件を守る責任は、モジュールの中だけに残る。
モジュールを小さく保つほど、その責任を負うコードも少なくなる。

C++ でも、メンバを `private` にして、コンストラクタで確かめれば、同じことが書ける。
違うのは、失敗の伝え方である。
C++ のコンストラクタは値を返せないので、確かめて駄目だったことを伝えるには、[例外](../glossary.md#exception)を投げるか、コンストラクタの外に作り方を用意して [`std::optional`](../glossary.md#std-optional) や [`std::expected`](../glossary.md#std-expected) を返すことになる。
Rust の `new` はただの関数なので、5 章の [`Result`](../glossary.md#result) をそのまま返せる。

名前の付いたフィールドの構造体でも同じである。
`Tally` のフィールドは 1 つも公開していないので、`main.rs` で構造体の値を直接書こうとすると、`E0451`（fields `field`, `strict`, `counts`, `skipped` and `lines` of struct `Tally` are private）で止まる。
非公開のフィールドが 1 つでもあれば、外からは値を書いて作れない。

`Tally` の場合、隠しているのは条件というより釣り合いである。
`counts` に数えた回数の合計と `skipped` の和は、いつも `lines` に等しい。
`push_line` は、失敗したときには何も変えないので、`strict` の失敗のあとでもこの等しさは崩れない。
フィールドが公開されていれば、外のコードが `skipped` だけを書き換えて、この等しさを崩せてしまう。

## 全部公開した構造体は、あとからフィールドを足しにくい

逆に、集計を終えた結果 `Report` は、フィールドを全部公開している。
読み手が中身を直接見るための型で、守る釣り合いが無いからである。

```rust
#[non_exhaustive]
pub struct Report {
    pub counts: HashMap<String, u64>,
    pub skipped: usize,
}
```

全部公開すると、使う側は値を書いて作れるし、パターンで全部のフィールドを取り出せる。
その代わり、あとでフィールドを 1 つ足しただけで、使う側のその書き方が壊れる。
値を書いて作っていた箇所は `E0063`（missing field）で止まり、パターンで取り出していた箇所は `E0027`（pattern does not mention field）で止まる。

4 章で enum に付けた `#[non_exhaustive]` は、構造体にも付けられる。
付けると、クレートの外では、値を書いて作ること自体ができなくなる。

```
error[E0639]: cannot create non-exhaustive struct using struct expression
 --> src/main.rs:6:18
  |
6 |     let report = Report { counts: HashMap::new(), skipped: 0 };
  |                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

パターンで取り出すときも、`let Report { counts, skipped, .. } = report;` のように、残りを受ける `..` が要る（`E0638`）。
はじめから足される前提で書かせるので、フィールドを足しても使う側は壊れない。
クレートの中では、この制限はかからない。
`Report` を作っているのは、同じクレートの `finish` である。

## 本物ではどうなっているか

`tally-core` の `lib.rs` は、モジュールを 1 つも公開していない。

```rust
mod count;
mod error;
mod select;

pub use count::{Counter, Entry, Report, tally_reader};
pub use error::{JsonError, JsonPointerError, LineError, LineErrorKind, Result, TallyError};
pub use select::{JsonPointer, Key, Selector};
```

この章の `lib.rs` と同じ形である。
利用者が書けるのは `tally_core::Counter` だけで、`tally_core::count::Counter` という経路は存在しない。
公開の範囲は、この 3 行の `pub use` で全部である。

集計器の `Counter` は、フィールドを 1 つも公開せず、`new` と `strict` で作る。

```rust
#[derive(Debug, Default)]
pub struct Counter {
    counts: HashMap<String, u64, RandomState>,
    skipped: usize,
    total: usize,
    strict: bool,
}

impl Counter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn strict(mut self, yes: bool) -> Self {
        self.strict = yes;
        self
    }
    // ...
}
```

`#[derive(...)]` は、決まった形のコードをコンパイラに書かせる属性である。
`Default` を書かせると、全部のフィールドを既定の値（0、`false`、空のマップ）で埋める `default()` ができ、`new` はそれを呼んでいる。
`Debug` は、`{:?}` で中身を表示できるようにする。
`FieldName` や `Tally` にも付けてある。
仕組み（[trait](../glossary.md#trait)）は 9 章で扱う。
`HashMap` の 3 つ目の型 `RandomState` は、キーからハッシュ値を計算するやり方を選んでいる。
標準ライブラリにも同じ名前の型があるが、これは `foldhash` というクレートのもので、抜粋では `use` の行を省いた。

`tally` は、`--pointer` オプションで JSON の入れ子をたどる道筋を `/a/b` のように受け取る（JSON Pointer という記法）。
これを持つ `JsonPointer` も、この章の `FieldName` と同じく、確かめてから作る型である。

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPointer {
    raw: Box<str>,
    first: Box<str>,
    rest: Box<[Box<str>]>,
}

impl JsonPointer {
    pub fn new(raw: &str) -> Result<Self, JsonPointerError> {
        if raw.is_empty() {
            return Err(JsonPointerError::Empty);
        }
        let Some(body) = raw.strip_prefix('/') else {
            return Err(JsonPointerError::NotRooted);
        };
        // ...
    }
}
```

`Box<str>` は、ここでは「あとから書き換えない `String`」と読めばよい。
`strip_prefix('/')` は、先頭が `/` ならそれを除いた残りを `Some` で返し、そうでなければ `None` を返す。
フィールドが非公開なので、`/` で始まらないポインタを外から作る道は無い。
`let ... else` は 5 章で見た書き方である。

## 演習

答えは [`book/code/ch06/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch06/src/exercises.rs) にある。
`cargo test -p book-ch06` で確かめられる。

**演習 1**

0 から 100 までの値だけを持つ型 `Percent` を書け。
範囲の外の値では `None` を返すこと。

```rust
struct Percent(u8);

impl Percent {
    fn new(value: u8) -> Option<Self>
    fn get(&self) -> u8
}
```

書けたら、`Percent` を定義したモジュールの子孫でないモジュールから `Percent(150)` と書けない理由を説明してほしい。
同じクレートの中なら、エラーは `E0603`（tuple struct constructor `Percent` is private）になる。

**演習 2**

集計の途中経過を表す次の構造体に、もう 1 つの途中経過を足し込むメソッド `merge` を書け。

```rust
struct Counts {
    counts: HashMap<String, u64>,
    skipped: usize,
}
```

`self` を `&mut self` で受け取り、足し込む相手 `other` は値で受け取ること。
`other` を `&Counts` で受け取った場合と比べて、キーの `String` をどう扱えば済むかが変わる。
