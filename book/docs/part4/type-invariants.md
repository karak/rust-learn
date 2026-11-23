---
title: 14 型で不変条件を守る（newtype・RAII ガード・typestate）
sidebar_position: 14
---

# 14 型で不変条件を守る（newtype・RAII ガード・typestate）

「この文字列は `/` で始まる」「この配列は空ではない」「読み込みが終わるまで集計を呼ばない」といった約束は、コメントや毎回の `if` で守ることもできる。
しかし、呼び出しが増えるほど検査の書き忘れが紛れ込み、壊れた値が奥まで流れてから別の場所で失敗する。

1 章の[所有権](../glossary.md#ownership)、6 章の[モジュール](../glossary.md#module)による可視性、13 章の [drop](../glossary.md#drop) と [`Deref`](../glossary.md#deref) を組み合わせると、こうした約束の多くを型へ移せる。
誤った呼び出しが実行時のバグではなくコンパイルエラーで止まる設計を、4 つの形に分けて確かめる。

## この章のコードは、経路・空でない列・一時変更ガード・手順つき集計器で制約を確かめる

この章のコード（`book/code/ch14`）は、型に約束を持たせる 4 つの部品を並べている。

```rust
pub struct Host(String);
pub struct Endpoint(String);
pub struct FieldPath { raw: Box<str>, first: Box<str>, rest: Box<[Box<str>]> }
pub struct NonEmpty<T> { first: T, rest: Vec<T> }
pub struct OverrideGuard<'a, T> { ... }
pub struct Draft { ... } // -> Loaded -> Aggregated
```

`Host` と `Endpoint` はどちらも中身が [`String`](../glossary.md#string-str) の [newtype](../glossary.md#newtype) で、引数の取り違えをコンパイル時に止める。
`FieldPath` は先頭の `/` と空でないセグメントを入口で一度だけ検証する型、`NonEmpty<T>` は構造そのもので空を禁じる型である。
`OverrideGuard` は[スコープ](../glossary.md#scope)のあいだだけ値を差し替えて元へ戻すガード、`Draft` から `Aggregated` への連なりは手順の順序を型で縛る例になっている。

`cargo run -p book-ch14` を実行すると、それぞれの動きが順に出る。

```text
url:       https://example.com/health
path:      /req/status -> ["req", "status"]
non_empty: first=200, len=2
override:  inside=debug
override:  after=info
pipeline:  {200: 2, 404: 1}
```

## newtype で、同じ型の引数の取り違えをコンパイルエラーにする

`fn format_url(host: &str, endpoint: &str) -> String` のように同じ型の引数が並ぶ関数は、順番を逆にして渡してもコンパイルが通る。
C++ の `using Host = std::string;` のような型別名（Rust の `type Host = String;` も同じ）は元の型と区別されないため、取り違えを防げない。

フィールドが 1 つのタプル[構造体](../glossary.md#struct)（newtype）で包むと、コンパイラは別の型として扱う。

```rust
pub struct Host(String);
pub struct Endpoint(String);

pub fn format_url(host: &Host, endpoint: &Endpoint) -> String {
    format!("https://{}{}", host.as_str(), endpoint.as_str())
}
```

引数を逆にして渡すと、コンパイラが入れ替えまで提案して止める。

```text
error[E0308]: arguments to this function are incorrect
 --> newtype_swap.rs:8:13
  |
8 |     let _ = format_url(&endpoint, &host);
  |             ^^^^^^^^^^ ---------  ----- expected `&Endpoint`, found `&Host`
  |                        |
  |                        expected `&Host`, found `&Endpoint`
  |
help: swap these arguments
  |
8 -     let _ = format_url(&endpoint, &host);
8 +     let _ = format_url(&host, &endpoint);
```

（このメッセージは、同じ形の小さな例で出したものである。）

余分なフィールドを持たない単一フィールドのタプル構造体では、手元の 64 ビット環境で `std::mem::size_of` を測ると、`String` も `Host` も同じ 24 バイトだった。

## 入口で一度だけ検証し、不正な値を作れない型にする

非公開フィールドと検証つきのコンストラクタを組み合わせると、「この型の値が存在する時点で条件を満たしている」と保証できる。
`FieldPath::new` は、先頭が `/` で始まり、空のセグメントを含まない文字列だけを受け入れ、合わなければ `Err(FieldPathError)` を返す（[`Result`](../glossary.md#result)）。

フィールドを非公開にしてあるため、定義元のモジュールの外から構造体を直接組み立てる道は塞がれている。

```text
error[E0451]: fields `raw`, `first` and `rest` of struct `FieldPath` are private
  --> private_struct.rs:11:9
   |
10 |     let _p = path::FieldPath {
   |              --------------- in this type
11 |         raw: Box::from("bad"),
   |         ^^^ private field
12 |         first: Box::from("bad"),
   |         ^^^^^ private field
13 |         rest: Box::from([]),
   |         ^^^^ private field
```

（このメッセージは、同じ形の小さな例で出したものである。）

一度 `FieldPath` にしてしまえば、受け取る側の関数は先頭が `/` かどうかを毎回問い直す必要が無い。
ただし、フィールドを非公開にするだけでは抜け道が残ることがある。
たとえば中身への `&mut String` を返す[メソッド](../glossary.md#method)や `DerefMut<Target = String>` を生やすと、構築のあとで外から中身を書き換えて検証を迂回できてしまう。
`FieldPath` の場合も、`DerefMut<Target = str>` を生やせば `make_ascii_uppercase` などで `raw` だけ書き換わり、`first` や `rest` との整合が崩れる。
ほかにも [`#[derive(Default)]`](../glossary.md#derive) をうっかり付けると、コンストラクタを通らずに空の値が作れてしまう。
`FieldPath` が `Deref<Target = str>` だけを実装して `DerefMut` や `Default` を付けないのは、こうした迂回路を塞ぐためである。

なお、非公開の壁が効くのは定義したモジュールの外に対してだけである。
同じモジュールの中のコードはフィールドへ直接触れられるので、不変条件を堅く保ちたい型は専用のモジュールに分けておくと、検証を迂回できる範囲を小さくできる。

## 構造そのもので「空」を禁じると、毎回の検査が消える

検証を関数に書く代わりに、型の形そのものから不正な状態を消せる場面もある。
「要素が 1 つ以上ある列」を [`Vec<T>`](../glossary.md#vec) で持つと、`first()` の戻り値は [`Option`](../glossary.md#option)`<&T>` になり、使う側は毎回 `None` の場合を扱うことになる。

先頭の 1 要素と残りの要素を別のフィールドに分けると、空の列はそもそも作れなくなる。

```rust
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}
```

この形なら、`first(&self)` は `Option` を介さずに `&T` を直接返せる。
要素数を返す `len(&self)` も必ず 1 以上になるので、12 章で出た `NonZeroUsize` で返せる。
`FieldPath` も同じ工夫をしており、最初のセグメントを `first: Box<str>`、2 つ目以降を `rest: Box<[Box<str>]>` に分けることで（[`Box`](../glossary.md#box)）、「セグメントは 1 つ以上ある」を型に刻んでいる。
おかげで `first_segment(&self)` も `&str` を直接返せ、`segments(&self)` は 1 つ以上の要素を順に流す[イテレータ](../glossary.md#iterator)になる。

## RAII ガードと `#[must_use]` で、後始末の忘れを防ぐ

13 章で見た `Drop` による [RAII](../glossary.md#raii) は、資源の解放だけでなく「一時的な変更をスコープの終わりに元へ戻す」約束にも使える。
`OverrideGuard` は、[`RefCell`](../glossary.md#cell-refcell) の中身を一時的に差し替え、スコープを抜けたときに元の値へ戻すガードである。

```rust
let level = RefCell::new("info");
{
    let _guard = OverrideGuard::new(&level, "debug");
    assert_eq!(*level.borrow(), "debug");
}
assert_eq!(*level.borrow(), "info");
```

途中で早期 `return` しても、通常の終了経路ではスコープを抜けるときに `Drop` が走るので戻し忘れを防げる（ただし 13 章で見た `std::mem::forget` や `std::process::exit` など、`Drop` 自体が走らない例外は残る）。
また `OverrideGuard` は `Drop` の中で `borrow_mut()` を呼ぶため、同じ `RefCell` の[借用](../glossary.md#borrow)をガードより外側の[寿命](../glossary.md#lifetime)へ持ち出すと実行時に [panic](../glossary.md#panic) する。

ガードを使うときに踏みやすい落とし穴が 2 つある。
1 つ目は、戻り値を変数に受け取らず `OverrideGuard::new(&level, "debug");` とだけ書いてしまう誤りである。
無名の一時値はその文の終わりですぐに破棄されるため、次の行へ進む前に元の値へ戻ってしまう。

6 章でも触れた [`#[must_use]`](../glossary.md#must-use) [属性](../glossary.md#attribute)を型や関数に付けておくと、値を使わずに捨てた箇所をコンパイラが警告する。

```text
warning: unused `OverrideGuard` that must be used
  --> must_use_warn.rs:10:5
   |
10 |     OverrideGuard::new(&level, "debug");
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: ガードを変数に束縛しないと、その場ですぐ元に戻ります
   = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
help: use `let _ = ...` to ignore the resulting value
   |
10 |     let _ = OverrideGuard::new(&level, "debug");
   |     +++++++
```

（このメッセージは、同じ形の小さな例で出したものである。）

2 つ目の落とし穴は、この `help` にそのまま従って `let _ = OverrideGuard::new(...);` と書いてしまうことである。
5 章で見た `let _ = ...` は、右辺が一時値のとき変数へ束縛せずその場で即座に破棄する。
スコープの終わりまで生かしておきたいガードは、`let guard = ...;` のように変数へ束縛する（以後その変数名を使わないときの未使用警告を避けたいなら、`let _guard = ...;` と先頭に `_` を付ける）。

## 所有権を消費して型を変えると、手順の前後をコンパイル時に縛れる

`Draft` から `Loaded` を経て `Aggregated` へ進むように、手順の段階ごとに別の型を用意する書き方を [typestate](../glossary.md#typestate)（型状態）と呼ぶ。

この章のコードでは 3 つの型を用意し、各段階のメソッドが `self` を値で受け取って次の段階の型を返す。
また各型のフィールドを非公開にしているため、途中の `Loaded` や `Aggregated` を外から直接作れない。

```rust
impl Draft {
    pub fn load(self, codes: Vec<u16>) -> Loaded { ... }
}
impl Loaded {
    pub fn aggregate(self) -> Aggregated { ... }
}
impl Aggregated {
    pub fn counts(&self) -> &Counts { ... }
}
```

読み込む前に `aggregate` を呼ぼうとしても、そもそも `Draft` にそのメソッドが存在しないため、コンパイル時に止まる。

```text
error[E0599]: no method named `aggregate` found for struct `Draft` in the current scope
  --> typestate_wrong_order.rs:13:19
   |
 2 | struct Draft { _private: () }
   | ------------ method `aggregate` not found for this struct
...
13 |     let _ = draft.aggregate();
   |                   ^^^^^^^^^ method not found in `Draft`
```

（このメッセージは、同じ形の小さな例で出したものである。）

さらに、状態の型が [`Copy`](../glossary.md#copy-clone) も `Clone` も実装しておらず、遷移のメソッドが `self` を [move](../glossary.md#move) で受け取るため、前の段階の変数を二度使う誤りもコンパイルエラーになる。

```text
error[E0382]: use of moved value: `draft`
  --> typestate_reuse.rs:11:18
   |
 9 |     let draft = Draft::new();
   |         ----- move occurs because `draft` has type `Draft`, which does not implement the `Copy` trait
10 |     let _loaded = draft.load(vec![200]);
   |                   ----- value moved here
11 |     let _again = draft.load(vec![404]);
   |                  ^^^^^ value used here after move
```

（このメッセージは、同じ形の小さな例で出したものである。）

逆に、空の構造体に見える `Draft` へうっかり `#[derive(Copy, Clone)]` を付けてしまうと、move されずに複製が渡るため、この歯止めが効かなくなる。

1 章で見たとおり、C++ では move したあとも元の変数がスコープに残り、標準ライブラリの型（別に定めが無いもの）なら[有効だが未規定の状態](../glossary.md#valid-but-unspecified)として触れられてしまう。
Rust では `Copy` でない型を move するとコンパイラが元の変数を使わせないため、「古い状態へ戻って二重に操作する」という誤りを型と所有権だけで閉じられる。

## 何でも型で包めばよいわけではない

型を増やすと、そのぶん変換や型定義の記述も増える。
値への条件も取り違える恐れも無い単なる文字列を newtype で包んでも、呼び出し側の手間が増えるだけで守れるものが無い。

typestate にも向き不向きがある。
状態が一直線に進む手順や、6 章で見た `Tally::new(field).strict(true)` のように設定を順に足して組み立てるビルダーとは相性がよい。
一方、入力しだいで実行時に状態が分岐したり、同じ配列に違う状態の値を混ぜて持ちたくなったりすると、型が別のままでは扱いにくくなる。
実行時の分岐が中心なら、4 章で見た [`enum`](../glossary.md#enum) と `match` に戻すほうが素直に書ける。

## 本物ではどうなっているか

`tally` の中にも、ここまで見てきた使い分けがそのまま現れている。

`crates/tally-core/src/select.rs` の `JsonPointer` は、入口で一度だけ検証する型であり、同時に構造で空を禁じる例でもある。
単一フィールドのタプル構造体からさらに踏み込み、解釈済みのセグメント列を 3 フィールドに分けて持っている。

```rust
pub struct JsonPointer {
    raw: Box<str>,
    first: Box<str>,
    rest: Box<[Box<str>]>,
}
```

先頭が `/` でない綴りを `JsonPointer::new` の時点で弾くため、綴り間違いのポインタが「どの行にも一致しない」結果へ静かに化ける事故を防いでいる。
さらに `first` と `rest` を分けることで、走査のたびにセグメント列が空かどうかを確かめずに済ませている。
13 章で見たとおり、`Deref<Target = str>` は実装しているが `DerefMut` は実装していない。

同じファイルの `Key` では、逆に包まない判断も並んでいる。
`Key::JsonPointer(JsonPointer)` は検証が必要なため専用型で包んでいる一方、`Key::JsonField(String)` は任意のフィールド名を許すため生の `String` のままにしている。
また `Selector` はフィールドを非公開にし、`Selector::new(key).ignore_case(true)` のように `self` を受け取って `Self` を返すビルダーで組み立てる形を採っている。
`crates/tally/src/input.rs` の `StdinLock<'static>` は、スコープ終了時の `Drop` で標準入力のロックを外す RAII ガードの実例である。

## 演習

答えは [`book/code/ch14/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch14/src/exercises.rs) にある。
`cargo test -p book-ch14` で確かめられる。

1. 1 以上 65535 以下のポート番号だけを表す検証つき newtype `Port(u16)` を書く。
   フィールドを非公開にし、`Port::new(raw: u16) -> Result<Self, ZeroPortError>` が `0` を `Err(ZeroPortError)` で弾くようにする。
   構築後に値を取り出す `get(self) -> u16` を付け、0 が弾かれて 8080 が通ることをテストで確かめる。
2. typestate を使って、閉じている `ClosedBuffer` と開いている `OpenBuffer` の 2 つの型を書く。
   `ClosedBuffer::new()` で空のバッファを作り、`ClosedBuffer::open(self) -> OpenBuffer` で開き、`OpenBuffer::push(&mut self, line: &str)` で行を追加し、`OpenBuffer::close(self) -> ClosedBuffer` で閉じてから `ClosedBuffer::lines(&self) -> &[String]` で読めるようにする。
   開く前に `push` できないこと、閉じたあとに元の `OpenBuffer` を使えないことが型の上で成り立つことを確かめる。
