---
title: 13 Drop・Deref とスマートポインタ、内部可変性
sidebar_position: 13
---

# 13 `Drop`・`Deref` とスマートポインタ、内部可変性

12 章では、`Rc`、`Arc`、`RefCell`、[`Mutex`](../glossary.md#mutex) といった型が一度に顔を出した。
そこではスレッドをまたげるかどうかに話を絞ったが、これらの型はどれも同じ土台の上に乗っている。
後始末を自動で走らせる `Drop`、参照のまま中身を見せる `Deref`、複数の持ち主を作る参照カウント、[共有参照](../glossary.md#references)のまま書き換える内部可変性の 4 つである。

C++ の [`std::unique_ptr`](../glossary.md#unique-ptr) や [`std::shared_ptr`](../glossary.md#shared-ptr)、デストラクタ、そして `mutable` に当たる役割が、Rust では独立した [trait](../glossary.md#trait) と標準ライブラリの型に分かれている。
分かれているおかげで、必要な機能だけを[所有権](../glossary.md#ownership)と排他の規則の上に載せて組み合わせられる。

## この章のコードは、後始末・共有・書き換えを小さな部品で確かめる

この章のコード（`book/code/ch13`）には、4 つの仕組みを確かめるための小さな型が並んでいる。

```rust
pub struct TraceGuard<'a> { ... }
pub enum Filter { Code(u16), And(Box<Self>, Box<Self>), Not(Box<Self>) }
pub struct Label { text: Box<str> }
pub struct SharedCounter { counts: Rc<RefCell<Counts>> }
pub struct Node { name: &'static str, parent: RefCell<Weak<Self>>, children: RefCell<Vec<Rc<Self>>> }
```

`TraceGuard` は破棄されたときにログへ自分の名前を残す型で、`Drop` が走る順を見るために使う。
`Filter` は状態コードを選ぶ条件の木、`Label` は長さを変えない名前のラッパである。
`SharedCounter` は複数の記録係（`Recorder` 型）が 1 つの集計表を共有して書き込む仕組みを持ち、`Node` は親への弱い参照で循環を切る木の節になっている（`RefCell` は「`Cell` と `RefCell`」の節で扱う）。

`cargo run -p book-ch13` を実行すると、`Node` を除く 4 つの動きが 1 行ずつ出る。
`drop:` の行は、`first` を `drop(first)` で先に手放した結果である。何もしなければ、宣言の逆順で `second` が先に記録される。

```text
drop:    ["first", "second"]
filter:  200=false, 404=true
label:   api-east (8 bytes)
counts:  {200: 2, 404: 1} (east=1, west=2)
```

## スコープを抜けると `Drop` が走り、`drop(x)` で早めに手放せる

1 章で、所有者が[スコープ](../glossary.md#scope)を抜けると値が解放されると書いた。
このとき、型が `Drop` trait を実装していれば、解放の直前に `drop(&mut self)` が自動で呼ばれる（[drop](../glossary.md#drop)）。
ファイルハンドルやロックの解放をスコープの終わりに任せられる点は、C++ のデストラクタによる [RAII](../glossary.md#raii) と同じである。

破棄の順番は、ローカル変数と[構造体](../glossary.md#struct)のフィールドで違う。
同じスコープに並べたローカル変数は、C++ と同じく宣言の逆順に破棄される（ここで使う `RefCell` は、共有参照のままログへ書き込むための型で、後ろの節で詳しく扱う）。

```rust
let log = RefCell::new(Vec::new());
{
    let _a = TraceGuard::new("a", &log);
    let _b = TraceGuard::new("b", &log);
}
assert_eq!(*log.borrow(), vec!["b", "a"]);
```

一方、構造体自身に `Drop` を実装していなくても、フィールドが `Drop` を実装していれば、各フィールドの `Drop` が宣言の順に走る。
C++ ではクラスのメンバも宣言の逆順に破棄されるので、ここは挙動が違う。
手元の `clang++` で同じ形の構造体を走らせると `second` のあとに `first` が破棄されたが、Rust では `first` のあとに `second` が破棄される。

```rust
struct Pair<'a> {
    _first: TraceGuard<'a>,
    _second: TraceGuard<'a>,
}
// 破棄の順は ["first", "second"]
```

スコープの終わりを待たずに手放したいときは、標準ライブラリの関数 `drop(x)` を呼ぶ。
[メソッド](../glossary.md#method)として `g.drop()` と書くと、コンパイラに断られる。

```text
error[E0040]: explicit use of destructor method
 --> explicit_drop.rs:6:7
  |
6 |     g.drop();
  |       ^^^^ explicit destructor calls not allowed
  |
help: consider using `drop` function
  |
6 -     g.drop();
6 +     drop(g);
```

（このメッセージは、同じ形の小さな例で出したものである。）

`Drop::drop` を直接呼べてしまうと、変数がまだスコープに残ったまま中身が解放され、スコープの終わりにもう一度 `drop` が走って二重解放になる。
対して `std::mem::drop` の中身は `pub fn drop<T>(_x: T) {}` であり、特別な構文は使っていない。
[`Copy`](../glossary.md#copy-clone) でない型なら、値を [move](../glossary.md#move) で受け取り、空の関数本体の終わりでそのまま破棄する。
呼んだ元の変数は以後使えなくなるので、二重に破棄される余地が無い（`Copy` な型は複製が渡るだけだが、そもそも `Copy` と `Drop` は同時に実装できない）。

ロックのガードや `RefCell` の借用を途中で手放したい場面では、この `drop(guard)` がそのまま使える。
ただし、`Drop` がどんな状況でも必ず走るとは限らない。
次の場面では `Drop` が走らずに終わる。

- `std::mem::forget` で破棄を意図的に省いたとき
- `Rc` や `Arc` が循環参照を作り、参照カウントが 0 にならないとき
- `std::process::exit` で即座にプロセスを終えたとき
- [panic](../glossary.md#panic) の[巻き戻し](../glossary.md#stack-unwinding)中に別の `Drop` が重ねて panic し、プロセスが強制終了したとき

## `Box` は値をヒープに置き、再帰的な型と長さを変えないスライスを持てる

値を[ヒープ](../glossary.md#heap)に置いて単独の所有権で管理する型が [`Box<T>`](../glossary.md#box) である（`()` のようなサイズ 0 の型や空の[スライス](../glossary.md#slice)では実際にはヒープを確保しない）。
C++ の `std::unique_ptr` に近いが、空（`nullptr`）の状態を持たない。
値があるかないかを表したいときは [`Option`](../glossary.md#option)`<Box<T>>` と書く。

`Box` が欠かせない場面の一つが、自分自身を子に持つ再帰的な型である。
`Filter` の `And` バリアントに `Box` を書かずに `And(Filter, Filter)` とすると、コンパイラは型の大きさを決められず止まる。

```text
error[E0072]: recursive type `Filter` has infinite size
 --> recursive_enum.rs:1:1
  |
1 | enum Filter {
  | ^^^^^^^^^^^
2 |     Code(u16),
3 |     And(Filter, Filter),
  |         ------ recursive without indirection
```

（このメッセージは、同じ形の小さな例で出したものである。）

型定義の中でも、自分自身の型を `Self` と書ける。
`And(Box<Self>, Box<Self>)` のように `Box<Self>` で包めば、`Box<Self>` 1 つあたりヒープへのポインタ 1 つになり、型の大きさが確定する。

`Box` は、長さを変えない文字列や配列を持つときにも効く。
[`String`](../glossary.md#string-str) や [`Vec<T>`](../glossary.md#vec) は、末尾に要素を足せるように「ポインタ・長さ・容量」の 3 つを持つ。
作ったあとで長さを変えないなら、容量のぶんは無駄になる。
`Box<str>` や `Box<[T]>` にすると、「ポインタ・長さ」の 2 つ（[ファットポインタ](../glossary.md#fat-pointer)）だけで済む。
手元の 64 ビット環境で `std::mem::size_of`（型のバイト数を返す関数で、C++ の `sizeof` に当たる）を測ると、`String` と `Vec<u16>` は 24 バイト、`Box<str>` と `Box<[u16]>` は 16 バイトだった。
この章の `Label` も、名前を作ったあとに長さを変えないので `Box<str>` にしている。

## `Deref` を実装した型は、`*` による参照外しと型強制の両方で中身を見せられる

3 章で、`&String` を `&str` を取る関数にそのまま渡せることを見た。
この変換（[参照外し型強制](../glossary.md#deref-coercion)）と、`*x` で中身を辿る操作（[参照外し](../glossary.md#dereference)）の両方を支えているのが [`Deref`](../glossary.md#deref) trait である。

`Deref` は[関連型](../glossary.md#associated-type) `Target` とメソッド `deref(&self) -> &Self::Target` を持つ。
`Label` に `Deref<Target = str>` を実装すると、`*label` は裏で `*label.deref()` として扱われ、`&*label` は `&str` になる。
さらに、行き先の型が `&str` と決まっている場所では、コンパイラが自動で `&Label` を `&str` へ変換する（[型強制](../glossary.md#coercion)）。
9 章の[境界](../glossary.md#trait-bound)で縛った型引数として受ける関数では、行き先の型が決まっていないので、この変換は起きない。

```rust
impl Deref for Label {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}
```

このおかげで、`fn is_api(name: &str) -> bool` に対して `is_api(&label)` とそのまま渡せるし、`label.len()` のように `str` のメソッドも直接呼べる。
`Box<T>` や `Rc<T>`、12 章で見た `Mutex` の `lock` が返すガード（`MutexGuard`）が中身 `T` のメソッドをそのまま呼べるのも、同じ `Deref` の仕組みによる。

書き換え用の参照外しと型強制には `DerefMut` がある。
`Label` は中身を書き換えさせたくないので `DerefMut` を実装していない。
`Deref` はポインタや薄いラッパに対して使うのが基本であり、C++ の継承の代わりにメソッドを増やす目的で無関係な型に実装すると、どの型のメソッドを呼んでいるのか追いにくくなる。

## `Rc` は 1 本のスレッドの中で持ち主を増やし、循環は `Weak` で切る

1 つの値を複数の場所から所有したいとき、スレッドをまたがないなら [`Rc<T>`](../glossary.md#rc-arc) を使う。
C++ の `std::shared_ptr` に当たるが、参照カウントの増減を不可分な操作にしないため、12 章で見たとおり別スレッドには渡せない。
その代わり、1 本のスレッドの中でなら同期の費用が掛からない。

`Rc::clone(&rc)` は中身を複製せず、参照カウントを 1 増やすだけである。
`rc.clone()` と書いても動くが、`Rc::clone(&rc)` と書くと「重い複製ではなくカウントを増やしているだけだ」と読む側に伝わりやすい。
持ち主が何人いるかは `Rc::strong_count(&rc)` で見られる。
`SharedCounter` では、表本体と 2 つの `Recorder` が同じ表を指しているあいだ `strong_count` が 3 になり、2 つの `Recorder` を `drop` すると 1 に戻る。

`Rc<T>` は持ち主が複数になりうるため、`DerefMut` を実装していない。
中身を直接書き換えようとすると、コンパイラが理由まで含めて断る。

```text
error[E0594]: cannot assign to data in an `Rc`
 --> rc_mut.rs:5:5
  |
5 |     *n += 1;
  |     ^^^^^^^ cannot assign
  |
  = help: trait `DerefMut` is required to modify through a dereference, but it is not implemented for `Rc<u64>`
```

（このメッセージは、同じ形の小さな例で出したものである。）

`Rc` を使うときに気をつけるのは循環参照である。
親が子を `Rc` で持ち、子も親を `Rc` で持つと、外側の変数がすべて消えても互いの参照カウントが 1 のまま残り、解放も `Drop` も起きない。
木構造の「子から親」のように所有しない向きの参照は、所有権を増やさない [`Weak<T>`](../glossary.md#weak) にする。

```rust
pub fn attach(parent: &Rc<Self>, child: &Rc<Self>) {
    *child.parent.borrow_mut() = Rc::downgrade(parent);
    parent.children.borrow_mut().push(Rc::clone(child));
}

pub fn parent_name(&self) -> Option<&'static str> {
    self.parent.borrow().upgrade().map(|p| p.name)
}
```

`Rc::downgrade` で作った `Weak` は、指す先の[寿命](../glossary.md#lifetime)を延ばさない。
中身を読むときは `upgrade()` を呼び、相手がまだ生きていれば `Some(Rc<T>)`、もう破棄されていれば `None` を受け取る（C++ の `std::weak_ptr::lock` に当たる）。
複数スレッド向けの `Arc` にも、同じ役割の `Weak`（`std::sync::Weak`）がある。

## `Cell` と `RefCell` は、共有参照のまま中身を書き換える

`Rc<T>` は、持ち主が複数いるあいだは中身を直接書き換えられない（`Weak` も含めてほかに指すものが無ければ、`Rc::get_mut` で書き換えられる）。
それでも複数の持ち主から同じ集計表に件数を足したいときに使うのが、共有参照 `&T` のまま中身を書き換える仕組み（[内部可変性](../glossary.md#interior-mutability)）である。
C++ の `mutable` に近い役割だが、Rust では修飾子の代わりに [`Cell<T>` と `RefCell<T>`](../glossary.md#cell-refcell) のような型で書き換えの許可と安全性の保証を表す。

`Cell<T>` は、共有参照 `&Cell<T>` からは中身の参照を一度も外に貸さず、`get()` で値をコピーして取り出し、`set(v)` や `replace(v)` で丸ごと入れ替える（`get()` には `Copy` が要るが、`set` や `replace` は `Copy` でない型にも使える）。
共有参照から中身への参照が外に出ないので、2 章の「共有と可変の排他」が破れる余地が無く、実行時の検査も一切入らない。
`Recorder` の `recorded: Cell<u64>` は、このおかげで `&self` の `record` の中から自分の件数を 1 足せる。
一方で、[`BTreeMap`](../glossary.md#btreemap) のように中身を丸ごと入れ替えず一部だけ書き換えたい型には向かない。

参照を借りて中身を書き換えたいときは `RefCell<T>` を使う。
`RefCell` は、2 章の[借用](../glossary.md#borrow)の規則（共有参照は複数よいが、可変参照は同時に 1 つだけ）をコンパイル時ではなく実行時に数えて守らせる。
`borrow()` で共有の借用ガード、`borrow_mut()` で可変の借用ガードを受け取り、そのガードが `Drop` されたときにカウントが戻る。

規則を破るコードはコンパイルを通るが、実行時に panic する。

```rust
let cell = RefCell::new(0u64);
let _first = cell.borrow_mut();
let _second = cell.borrow_mut(); // panic: "RefCell already borrowed"
```

`borrow()` のガードを持ったまま `borrow_mut()` を呼んでも、同じ `"RefCell already borrowed"` で panic する。
panic させずに失敗を扱いたいときは、`try_borrow` や `try_borrow_mut` を使えば [`Result`](../glossary.md#result)（失敗時は `Err(BorrowMutError)` など）で受け取れる。
また、借用ガードは必要な行が終わったらすぐ手放すのが安全で、ブロック `{ ... }` で囲むか `drop(guard)` を呼んでスコープを短く保つ。

`SharedCounter` は、この `Rc` と `RefCell` を組み合わせて `Rc<RefCell<Counts>>` にしている。
`Rc` が「1 スレッド内で複数の持ち主を作る」役割を、`RefCell` が「共有のまま書き換える」役割を受け持つ。
12 章で見た `Arc` と `Mutex` を組にした `Arc<Mutex<T>>`（12 章の演習 2 の形）は、この 2 つの役割をそのまま複数スレッド向けに入れ替えた組である。

## 本物ではどうなっているか

`tally` のコードは、ここまで見てきた部品を必要な場所だけに絞って使っている。

`crates/tally-core/src/select.rs` の `JsonPointer` は、検証済みの JSON Pointer を持つ構造体で、文字列とセグメント列に `Box<str>` と `Box<[Box<str>]>` を使っている。

```rust
pub struct JsonPointer {
    raw: Box<str>,
    first: Box<str>,
    rest: Box<[Box<str>]>,
}
```

構築したあとに綴りやセグメント数を変えないので、`String` や `Vec` の容量フィールドを持たずに済んでいる。
同じファイルで `JsonPointer` は `Deref<Target = str>` も実装している。

```rust
impl Deref for JsonPointer {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}
```

読む操作は `&str` としてそのまま通せる一方、`DerefMut` は実装していない。
検証済みの綴りを外から書き換えられないようにするためで、この設計の詳しい話は 14 章で扱う。

`Drop` による後始末は、`crates/tally/src/input.rs` の標準入力の読み取りに出ている。
`impl Input for Stdin` の `open` メソッドは、`self.lock()` で得た `StdinLock<'static>` を `Ok` に包んで返す。
`StdinLock` は標準入力の排他ロックを握ったガードであり、集計が終わってスコープを抜けると `Drop` が走ってロックを外す。
12 章で見た `crates/tally/src/error.rs` の `ExecutionError` が持つ `Box<dyn Error + Send + Sync>` も、`Box` でヒープに置いて[型を消す](../glossary.md#dyn)使い方の 1 つである。

なお、`tally` 本体には `Rc` や `RefCell` は出てこない。
普通の所有権と借用（`&` と `&mut`）だけで書ける範囲では、実行時の検査や参照カウントを持ち込まないほうが単純で速いからである。
`Rc` や `RefCell` は、単一の所有権では表しにくい共有構造が本当に要るときに取り出す道具だと考えるとよい。

## 演習

答えは [`book/code/ch13/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch13/src/exercises.rs) にある。
`cargo test -p book-ch13` で確かめられる。

1. `log: &RefCell<Vec<String>>` と名前 `name: &'static str` を受け取るガード型 `ScopeLog` を書く。
   `ScopeLog::enter(log, name)` で `"enter: {name}"` を `log` に積み、スコープを抜けて `Drop` が走ったときに `"leave: {name}"` を積むようにする。
   途中で早期 `return` する関数の中で使い、正常終了でもエラー終了でも `"leave: ..."` が記録されることをテストで確かめる。
2. `Rc<RefCell<BTreeMap<String, u64>>>` を共有して語ごとの件数を数える `WordRecorder` を書く。
   2 つの `WordRecorder` が同じ表に `record(&self, word: &str)` で数え入れられるようにし、各 `WordRecorder` 自身が記録した回数も `Cell<usize>` で数えて `own_count(&self) -> usize` で返せるようにする。
