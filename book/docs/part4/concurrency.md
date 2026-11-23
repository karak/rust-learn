---
title: 12 並列化とその前提（Send / Sync）
sidebar_position: 12
---

# 12 並列化とその前提（Send / Sync）

2 章で、`&mut` が効かせているのは「可変」より「排他」だと書いた。
書き手がいるあいだ、ほかの書き手も読み手も同時に触れなければ、[データ競合](../glossary.md#data-race)は起きない。
8 章ではスレッドに渡す[クロージャ](../glossary.md#closure)を、9 章ではジョブの配列に付いた `+ Sync` と `Box<dyn Error + Send + Sync>` を、この章に送った。
10 章の `'static` も、ここでもう一度出てくる。

C++ の `std::thread` は、渡す関数オブジェクトが何を参照していても受け取る。
`std::atomic` でない同じ変数を 2 つのスレッドが同期なしに書き換えれば、[未定義動作](../glossary.md#undefined-behavior)になる。
それを知らせるのは、実行時に検査を埋め込む [ThreadSanitizer](../glossary.md#tsan) のような道具である。
Java や C# でも、どのオブジェクトも別のスレッドに渡せて、同期を書くかどうかは書く側に任されている。
Rust では、スレッドをまたいでよいかどうかを型が持ち、コンパイラがそれを確かめる。
この章では、その仕組みと、それを前提にした並列化の書き方を見る。

## この章のコードは、状態コードの列をスレッドに分けて数える

この章のコード（`book/code/ch12`）は、`200` や `404` のような状態コードの列を受け取り、コードごとの件数を数える。
1 本で数える `count` と同じ答えを、スレッドへの分け方を変えた 3 つの関数で出す。

```rust
pub type Counts = BTreeMap<u16, u64>;

pub fn count(codes: &[u16]) -> Counts
pub fn count_split(codes: &[u16], threads: NonZeroUsize) -> Counts
pub fn count_locked(codes: &[u16], threads: NonZeroUsize) -> Counts
pub fn count_spawned(codes: &Arc<[u16]>, threads: NonZeroUsize) -> Counts
```

`type Counts = …` は型の別名で、C++ の `using` に当たる。
`count` は 1 本のスレッドで数える。ほかの 3 つは、これと同じ答えを返す。
`NonZeroUsize` は 0 にならない `usize` で、スレッドの本数に 0 を渡せないようにしている。
`count_split` は、スレッドごとに別の表で数えて最後に足す。
`count_locked` は、1 つの表を全スレッドで書き換える。
`count_spawned` は、`count_split` と同じ分け方を、スレッドに入力を[借用](../glossary.md#borrow)させずに書いたものである。

`cargo run -p book-ch12` を実行すると、4 つが同じ表を出す。

```text
1 本:    {200: 3, 301: 1, 404: 2, 500: 1}
split:   {200: 3, 301: 1, 404: 2, 500: 1}
locked:  {200: 3, 301: 1, 404: 2, 500: 1}
spawned: {200: 3, 301: 1, 404: 2, 500: 1}
2 行目: 状態コードとして読めません
```

最後の行は、文字列の行を状態コードとして読む `parse_all` の失敗である。
これは章の後半で扱う。

## C++ では走るデータ競合が、Rust では借用の規則で断られる

C++ で、2 つのスレッドが同じ変数を 100 万回ずつ増やすと、こうなる。

```cpp
long count = 0;
auto work = [&count] { for (int i = 0; i < 1000000; ++i) ++count; };
std::thread a(work), b(work);
a.join(); b.join();
std::cout << count << '\n';
```

手元の clang で `-Wall -Wextra` を付けても、警告は出なかった。
最適化なしでビルドして 3 回走らせると、`1004541`、`1002233`、`975951` と毎回違う数が出た。
`-O2` でビルドすると `2000000` が出た。
最適化がループを 1 回の加算にまとめて競合の起きる窓が極端に狭くなり、たまたま正しく見えただけで、競合が無くなったわけではない。
`-fsanitize=thread` を付けて走らせると、ThreadSanitizer が `WARNING: ThreadSanitizer: data race` を報告した。
どれも、走らせて初めてわかることである。

Rust で同じ形を書くと、コンパイルが通らない。
ここで使う `thread::scope` は、渡したクロージャの中で立てたスレッドが、すべてスコープの終わりまでに終わることを約束する関数で、次の節で詳しく見る。

```rust
let mut count = 0;
thread::scope(|s| {
    s.spawn(|| count += 1);
    s.spawn(|| count += 1);
});
```

```text
error[E0499]: cannot borrow `count` as mutable more than once at a time
   --> src/bin/race.rs:7:17
    |
  5 |     thread::scope(|s| {
    |                    - has type `&'1 Scope<'1, '_>`
  6 |         s.spawn(|| count += 1);
    |         ----------------------
    |         |       |  |
    |         |       |  first borrow occurs due to use of `count` in closure
    |         |       first mutable borrow occurs here
    |         argument requires that `count` is borrowed for `'1`
  7 |         s.spawn(|| count += 1);
    |                 ^^ ----- second borrow occurs due to use of `count` in closure
    |                 |
    |                 second mutable borrow occurs here
```

（後ろに続く note は省いた。）

`count += 1` を書いたクロージャは、`count` を `&mut` で借りる。
2 つのクロージャが同時に生きているので、2 章の「`&mut` は同時に 1 つだけ」に当たる。
スレッドのための規則は、ここではまだ何も働いていない。
2 章の規則がそのまま、データ競合を断っている。

## `thread::spawn` に渡すクロージャは、借りずに奪う

標準ライブラリでスレッドを立てる基本の関数は `thread::spawn` で、型はこうなっている。

```rust
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
```

`F` が渡すクロージャ、`T` がその戻り値で、`JoinHandle<T>` の `join` を呼ぶとスレッドの終わりを待って `T` を受け取れる。
`Send` は次の節で扱う。
ここで効くのは `'static` である。
10 章で見たとおり、`T: 'static` は「`T` は短い[寿命](../glossary.md#lifetime)のものを借りていない」という条件だった。
`spawn` で立てたスレッドは、立てた関数が返ったあとも走り続けられる。
だからクロージャには、関数の中の変数を借りさせない。

```rust
let codes = vec![200, 404];
let handle = thread::spawn(|| codes.len());
```

```text
error[E0373]: closure may outlive the current function, but it borrows `codes`, which is owned by the current function
 --> src/bin/spawn.rs:5:32
  |
5 |     let handle = thread::spawn(|| codes.len());
  |                                ^^ ----- `codes` is borrowed here
  |                                |
  |                                may outlive borrowed value `codes`
```

（後ろに続く note と help は省いた。help は `move` を付けるよう勧める。）

8 章の [`move`](../glossary.md#move) を付ければ、クロージャは `codes` を奪い、スレッドに持っていく。
代わりに、呼んだ側は `codes` をもう使えない。
`let r = &codes;` で参照を作り、`move` で参照のほうを渡しても、参照が借りている先は関数の中にあるので、`E0597` で断られた。

1 つの入力を複数のスレッドで読みたいときは、持ち主を複数にする。
[`Arc`](../glossary.md#rc-arc) は、指す先を参照カウントで共有するポインタで、C++ の [`std::shared_ptr`](../glossary.md#shared-ptr) に当たる。
`Arc::clone` はカウントを 1 つ増やして、同じ先を指すもう 1 つの `Arc` を返す。
中身は複製しない。
`count_spawned` は、スレッドごとに `Arc` を 1 つずつ複製して渡している。

```rust
pub fn count_spawned(codes: &Arc<[u16]>, threads: NonZeroUsize) -> Counts {
    let size = chunk_len(codes.len(), threads);
    let handles: Vec<_> = (0..codes.len())
        .step_by(size)
        .map(|start| {
            let codes = Arc::clone(codes);
            thread::spawn(move || {
                let end = (start + size).min(codes.len());
                count(&codes[start..end])
            })
        })
        .collect();
    merge(
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap_or_else(|payload| panic::resume_unwind(payload))),
    )
}
```

`Arc<[u16]>` は、指す先が[スライス](../glossary.md#slice)の `Arc` で、`Arc::from` で配列や [`Vec`](../glossary.md#vec) から作れる。
`chunk_len` は 1 本あたりの要素数を決める小さな関数、`merge` は表を足し合わせる関数で、どちらも `src/lib.rs` にある。
`(0..codes.len()).step_by(size)` は、0 から `size` おきの数を順に返す。各スレッドの持ち分の先頭である。
`Arc::clone(codes)` を外し、引数の `codes`（`&Arc` の参照）をそのままクロージャに渡すと、`E0521`（borrowed data escapes outside of function）で断られた。
スレッドに持たせたのが参照だったからである。

`join` は [`Result`](../glossary.md#result) を返す。
スレッドが [panic](../glossary.md#panic) で終わると `Err` になり、中に panic の値が入っている。
5 章で「別のスレッドで起きた panic は、そのスレッドの終わりを待つ側に失敗として返る」と書いたのは、この `Err` のことである。
`unwrap_or_else` は、`Err` のときだけ、中身を引数にして渡したクロージャを呼び、その戻り値を使う。5 章の `unwrap_or` の、代わりの値をクロージャで作る版である。
`panic::resume_unwind` は、受け取った panic の値で、呼んだ側でもう一度 panic を起こす。
ここでは、スレッドの中の panic を握りつぶさず、呼んだ側にそのまま伝えている。

## `thread::scope` の中のスレッドは、スコープの外の値を借りてよい

いちいち `Arc` にせずに済ませる方法がある。
`thread::scope` は、自分が返る前に、その中で立てたスレッドをすべて待つ。
スレッドがスコープより長く生きないことが決まっているので、スコープの外の値を借りてよい。

```rust
pub fn count_split(codes: &[u16], threads: NonZeroUsize) -> Counts {
    let parts: Vec<Counts> = thread::scope(|s| {
        let handles: Vec<_> = codes
            .chunks(chunk_len(codes.len(), threads))
            .map(|chunk| s.spawn(move || count(chunk)))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap_or_else(|payload| panic::resume_unwind(payload)))
            .collect()
    });
    merge(parts)
}
```

`chunks(n)` は、スライスを先頭から `n` 個ずつ区切ったスライスを順に返す[イテレータ](../glossary.md#iterator)である。
各スレッドは、自分の塊 `chunk`（`codes` の一部を指す `&[u16]`）を借りて数え、自分の表を返す。
`s.spawn` の型は、`thread::spawn` の `'static` を `'scope` に置き換えたものになっている。

```rust
pub fn spawn<F, T>(&'scope self, f: F) -> ScopedJoinHandle<'scope, T>
where
    F: FnOnce() -> T + Send + 'scope,
    T: Send + 'scope,
```

`'scope` はスコープの長さを表すライフタイムで、スコープの外の値はそれより長く生きる。
だから借りてよい。

スコープの終わりで、まだ `join` されていないスレッドも待たれる。
そのどれかが panic していると、`thread::scope` 自身が panic する。
手元では `a scoped thread panicked` という文で panic した。
`join` し忘れたスレッドの panic が、黙って消えることはない。

C++20 の `std::jthread` も、オブジェクトが消えるときに、停止を求めてからスレッドを待つ。
ただし、待つことと、参照先がまだ生きていることを結びつけるのは書く側である。
Rust の `thread::scope` では、その結びつきが `'scope` という型の上の条件になっている。

## `Send` は渡してよい、`Sync` は共有してよい、を型が持つ

`spawn` の条件に戻る。
`F: Send` と `T: Send` は、クロージャと戻り値を別のスレッドに移してよい、という条件である。
[`Send` と `Sync`](../glossary.md#send-sync) は、次の意味を持つ [trait](../glossary.md#trait) である。

- `Send`: その型の値を、別のスレッドに移してよい
- `Sync`: その型の値への[共有参照](../glossary.md#references) `&T` を、複数のスレッドが同時に持ってよい

2 つはつながっている。
`T` が `Sync` であることは、`&T` が `Send` であることと同じ意味である。
`&T` を別のスレッドに渡せば、2 つのスレッドが同じ値を同時に読めるからである。

この 2 つは、ふつうは誰も実装を書かない。
[構造体](../glossary.md#struct)や [enum](../glossary.md#enum) は、フィールドがすべて `Send` なら自動で `Send` になり、すべて `Sync` なら自動で `Sync` になる。
メソッドを持たず型の性質だけを表す印であり、こうして構造から自動で実装が決まる trait を、Rust では auto trait（自動トレイト）と呼ぶ。
`u16` も `Vec<u16>` も [`BTreeMap`](../glossary.md#btreemap)`<u16, u64>` も両方を満たすので、この章の `Counts` も両方を満たす。
逆に、自動で決まる型は、満たさない型を 1 つでも中に持てば、外側も満たさなくなる。
自分で `Send` や `Sync` を名乗らせるには `unsafe impl` と書く必要があり、そこから先はコンパイラではなく書いた人が正しさを受け持つ（[`unsafe`](../glossary.md#unsafe)）。
標準ライブラリの型には、そうして条件を自分で決めているものがある。後で見る `Mutex` がその例である。

`Send` を満たさない型の代表が、[`Rc`](../glossary.md#rc-arc) である。
`Rc` は `Arc` と同じく参照カウントで持ち主を複数にするポインタだが、カウントの増減を不可分な（atomic な）操作にしていない。
そのぶん速いが、2 つのスレッドが同時にカウントを動かすと数が狂う。
だから `Rc` は `Send` を満たさず、スレッドに渡そうとすると断られる。

```rust
let codes = Rc::new(vec![200, 404]);
let handle = thread::spawn(move || codes.len());
```

```text
error[E0277]: `Rc<Vec<i32>>` cannot be sent between threads safely
   --> src/bin/rc.rs:6:32
    |
  6 |     let handle = thread::spawn(move || codes.len());
    |                  ------------- -------^^^^^^^^^^^^
    |                  |             |
    |                  |             `Rc<Vec<i32>>` cannot be sent between threads safely
    |                  |             within this `{closure@src/bin/rc.rs:6:32: 6:39}`
    |                  required by a bound introduced by this call
    |
    = help: within `{closure@src/bin/rc.rs:6:32: 6:39}`, the trait `Send` is not implemented for `Rc<Vec<i32>>`
```

（後ろに続く note は省いた。）

`within this {closure@…}` とあるとおり、`Send` でないのはクロージャで、その理由が中に持った `Rc` である。
自動で決まる性質は、こうして外側へ伝わる。

C++ の `std::shared_ptr` は、同じ先を指す別々の `shared_ptr` を複数のスレッドが同時に複製したり消したりしても、参照カウントの増減で競合しないことを、規格が求めている。
規格は、スレッドで使わないときのための、カウントを不可分にしない型を別に用意していない。
Rust は、速い `Rc` と、スレッドをまたげる `Arc` を別の型にした。
`Rc` を誤ってスレッドに渡す道は、型が塞いでいる。

## 共有して書き換えるなら、排他は `Mutex` が受け持つ

共有参照 `&T` からは、ふつう書き換えられない。
2 章で、`&` で受け取りながら中身を書き換えられる型がある、と書いた。
これが[内部可変性](../glossary.md#interior-mutability)で、1 本のスレッドの中でそれをする代表が `RefCell` である。
`RefCell` は、`borrow_mut` で書き換え用の借用を取り出し、いま借りられているかどうかを実行時に数えて、2 章の規則を実行時に守らせる。
ただし、その数え方は 1 本のスレッドを前提にしている。
だから `RefCell` は `Sync` を満たさず、複数のスレッドから `&RefCell` を持とうとすると断られる。

```rust
let count = RefCell::new(0);
thread::scope(|s| {
    s.spawn(|| *count.borrow_mut() += 1);
});
```

```text
error[E0277]: `RefCell<i32>` cannot be shared between threads safely
   --> src/bin/refcell.rs:7:17
    |
  7 |         s.spawn(|| *count.borrow_mut() += 1);
    |           ----- ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `RefCell<i32>` cannot be shared between threads safely
    |           |
    |           required by a bound introduced by this call
    |
    = help: the trait `Sync` is not implemented for `RefCell<i32>`
    = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` instead
    = note: required for `&RefCell<i32>` to implement `Send`
```

（後ろに続く note は省いた。）

最後の note が、前の節の「`T` が `Sync` であることは、`&T` が `Send` であることと同じ」を言っている。
`RefCell<i32>` を `move` で 1 本のスレッドに丸ごと渡すのは通った。
中身が `Send` なら `RefCell` は `Send` を満たすので、持ち主が 1 人のまま移るぶんには問題が無い。
`Arc<RefCell<i32>>` にして `thread::spawn` に渡しても、`required for Arc<RefCell<i32>> to implement Send` という note つきで同じく断られた。
`Arc` は複数のスレッドから同じ中身を指させ、どのスレッドで中身を片付けるかも決まらないので、中身に `Send` に加えて `Sync` も求める。

スレッドをまたいで書き換えるには、[`Mutex`](../glossary.md#mutex) を使う。
`count_locked` は、1 つの表を `Mutex` で包み、全スレッドがそこへ書く。

```rust
pub fn count_locked(codes: &[u16], threads: NonZeroUsize) -> Counts {
    let counts = Mutex::new(Counts::new());
    thread::scope(|s| {
        for chunk in codes.chunks(chunk_len(codes.len(), threads)) {
            let counts = &counts;
            s.spawn(move || {
                for &code in chunk {
                    let mut counts = counts.lock().unwrap_or_else(PoisonError::into_inner);
                    *counts.entry(code).or_insert(0) += 1;
                }
            });
        }
    });
    counts.into_inner().unwrap_or_else(PoisonError::into_inner)
}
```

`let counts = &counts;` は 8 章で見た借り直しで、`move` がクロージャへ移す対象を、`Mutex` 本体から参照に変えている。
この行を消すと、最初のスレッドが `Mutex` そのものを奪うので、2 本目から `E0382`（use of moved value）で断られた。
逆に `move` を外すと、ループの変数 `chunk` を借りることになり、`E0373` で断られた。

C++ の `std::mutex` は、守るデータとは別の変数で、どのデータをどの mutex で守るかは書く側が覚えておく。
Rust の `Mutex<T>` は、守るデータ `T` を中に持つ。
`&Mutex<T>` しか持たないスレッドが中身に触る道は、`lock`（や、待たずに試す `try_lock`）が返すガードだけで、ガードが消えるとロックが外れる。
共有しているあいだ、ロックを取らずに中身に触るコードは書けない。
`Mutex<T>` は、`T` が `Send` でありさえすれば `Sync` を満たす。
`T` そのものが `Sync` でなくても、`Mutex` が一度に 1 つのスレッドにしか触らせないからである。
したがって、`Send` だが `Sync` でない `RefCell` も `Mutex<RefCell<i32>>` にすれば `Sync` になる。
一方、`Send` でない `Rc` は `Mutex<Rc<i32>>` にしても `Sync` にならない（ロック越しに別スレッドから中身を取り出せるため）。

`lock` は `Result` を返す。
ロックを握ったまま panic したスレッドがあると、`Mutex` は「毒された」状態になり、毒を消す（`clear_poison`）までは、以後の `lock` は `Err`（`PoisonError`）を返す。
中身が書きかけのまま残っているかもしれない、という知らせである。
`count_locked` では、`PoisonError::into_inner` で知らせを読み捨てて中身を使っている。
panic したスレッドがあれば、どのみち `thread::scope` の終わりで panic が呼んだ側に伝わるからである。
最後の `Mutex::into_inner` は、`Mutex` を消費して中身を取り出す。全スレッドが終わったあとで、ほかに持ち手がいないので、ロックは要らない。
それでも `Result`（`PoisonError`）を返すのは、それ以前に別スレッドがロックを握ったまま panic していれば毒が残っているからである。

## 分けて数えて最後に足すほうが、ロックを取り合わない

`count_locked` は、1 件ごとにロックを取る。
ロックを取れるのは一度に 1 本のスレッドだけなので、スレッドを増やしても数える作業は 1 本ずつ順に進む。
`count_split` は、スレッドどうしで何も書き換えを共有しない。
共有するのは読むだけの入力で、各スレッドは自分の表を持ち、最後に呼んだ側が 1 本のスレッドで足し合わせる。

手元の 10 コアの Mac で、1000 万件の状態コードを release ビルドで数えた（2026-11-22）。

| スレッド数 | `count_split` | `count_locked` |
| --- | --- | --- |
| 1 | 約 29 ms | 約 62 ms |
| 4 | 約 9〜10 ms | 約 200〜220 ms |
| 8 | 約 6.5 ms | 約 257 ms |

`count_split` はスレッドを増やすと速くなり、`count_locked` は遅くなった。
1 本のスレッドで数える `count` は約 29 ms だったので、`count_locked` は 1 本でもロックの費用のぶん遅く、増やすとロックの取り合いでさらに遅くなる。

スレッドごとに手元の表へ数えて最後に 1 回だけロックして合流するなら速くなるが、それは結局 `count_split` と同じ「分けて最後に足す」構造に近づく。
`Mutex` が要るのは、スレッドどうしが同じものを書き換える必要があるときである。
書き換える相手を分けられるなら、分けたほうが単純で速い。
型の上でも、`count_split` には `Mutex` も `Arc` も出てこない。

## 並列にすると、「最初の失敗」が実行ごとに変わりうる

数えるだけなら、足す順番が変わっても答えは変わらない。
失敗を返すときは、そうはいかない。

`parse_all` は、文字列の行を状態コードとして読み、読めない行があればその行番号を返す。

```rust
pub fn parse_all(lines: &[&str], threads: NonZeroUsize) -> Result<Vec<u16>, LineError> {
    let size = chunk_len(lines.len(), threads);
    thread::scope(|s| {
        let handles: Vec<_> = lines
            .chunks(size)
            .enumerate()
            .map(|(index, chunk)| s.spawn(move || parse_chunk(chunk, index * size)))
            .collect();
        let mut codes = Vec::with_capacity(lines.len());
        for handle in handles {
            let part = handle.join().unwrap_or_else(|payload| panic::resume_unwind(payload));
            codes.extend(part?);
        }
        Ok(codes)
    })
}
```

`extend` は、受け取った列の要素をまとめて `Vec` の末尾に足す。
`parse_chunk` は 1 つの塊を前から読み、最初に読めなかった行で止まって、その行の `LineError` を返す。
`["200", "x", "404", "y"]` を 2 本に分けると、2 行目の `x` と 4 行目の `y` は別のスレッドで読まれる。
どちらのスレッドが先に終わるかは、実行ごとに変わりうる。
「先に見つかった失敗」を返すと、同じ入力で返る失敗が変わることになる。

`parse_all` は、スレッドが終わった順ではなく、塊の順に `join` する。
塊の中では最初の失敗で止まるので、塊の順に見て最初に出会う `Err` が、いちばん前の行の失敗である。
テストでは、後ろの塊が先頭で失敗する入力を 100 回読ませ、毎回前の塊の失敗が返ることを確かめている。
代わりに、前の塊に失敗があっても後ろの塊のスレッドは止めず、自分の塊を読み終えるか失敗するまで走らせる。
ループの途中で `part?` が抜けても、前の節のとおり `thread::scope` が返る前に残りのスレッドをすべて待つので、走ったまま放置されることはない。
速さを少し捨てて、返る失敗を決めている。

## 本物ではどうなっているか

`tally` は、複数の入力ファイルを並列に集計する。
並列化は `crates/tally/src/aggregate.rs` にあり、`rayon` という[クレート](../glossary.md#crate)を使っている。
`rayon` の `par_iter` は、スライスの要素をスレッドの集まりに配って処理する。
スレッドを何本立て、どう配るかは、既定では `rayon` が決める。

```rust
fn run_parallel<J>(jobs: &[J]) -> Vec<Result<Counter, CliError>>
where
    J: Fn() -> Result<Counter, CliError> + Sync,
{
    jobs.par_iter().map(|job| job()).collect()
}
```

`jobs` の要素は、1 つの入力ファイルを開いて集計するクロージャである（8 章で見た `jobs` そのもの）。
`par_iter` は `&[J]` を複数のスレッドで共有して、各スレッドが自分の担当の `&J` を呼ぶ。
だから `J` に `Sync` が要る。
[`Fn`](../glossary.md#fn-traits) なのは、`&J` から呼べるのが `Fn` だけだからである（8 章）。
9 章で、テストがジョブを `[Box<dyn Fn() -> Result<Counter, CliError> + Sync>; 3]` に入れていたのは、この条件を満たすためである。

手元で `+ Sync` を外すと、`E0599`（the method `par_iter` exists for reference `&[J]`, but its trait bounds were not satisfied）で断られた。
このメッセージは `Sync` という語を出さず、`&[J]: rayon::iter::IntoParallelIterator` が満たされない、とだけ言う。
`rayon` は `IntoParallelIterator` の実装に `Sync` の条件を付けているので、エラーはその実装が見つからない、という形で出る。
並列化のエラーで `Sync` の語が見えないときは、使っている[メソッド](../glossary.md#method)が求める trait をたどるとよい。

結果を集める側では、`Result<Counter, CliError>` がスレッドから呼んだ側へ移るので、`Send` が要る。
`CliError` の中には、スレッドの集まりを作れなかった原因を持つ `ExecutionError` があり、その中身は `Box<dyn Error + Send + Sync>` である。
11 章で、`tally-core` が `serde_json` の型を外に見せない理由を見た。
`ExecutionError` も同じ判断で、`rayon` の失敗の型を [`dyn`](../glossary.md#dyn) で消している。
手元でこの `+ Send + Sync` を消して `Box<dyn Error>` にすると、`E0277` で断られた。
note は、`Box<dyn Error>`、`ExecutionError`、`CliErrorKind`、`CliError`、`Result<Counter, CliError>` と、`Send` でない理由を外側へ 1 段ずつたどって示した。
`Send` と `Sync` の節の `Rc` と同じく、自動で決まる性質が外側へ伝わった結果である。
一方、`+ Sync` だけを消した `Box<dyn Error + Send>` では、コンパイルもテストも通った。
`tally` の中には、`CliError` を複数のスレッドから同時に読む場所が無いからである。

失敗の選び方も、この章の `parse_all` と同じである。
`tally` は、全ジョブの結果を入力の順に並べてから、いちばん前の失敗を選ぶ。
並列化で揺れうるのは集計の結果ではなく返る失敗のほうで、`tally` はそれを入力の順で決めている。

ここまで見てきた並列化の仕組みは、新しい規則の積み増しではない。
所有権や排他参照から始まり、クロージャ・trait 境界・`'static` まで順に見てきた仕組みの延長線上に、`Send` / `Sync` と `thread::scope` がある。

## 演習

答えは [`book/code/ch12/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch12/src/exercises.rs) にある。
`cargo test -p book-ch12` で確かめられる。

1. `u64` のスライスからいちばん大きい値を探す関数 `max_parallel` を、`thread::scope` でスレッドに分けて書く。
   空なら `None` を返す。
   各スレッドに自分の塊の最大を返させ、最後にその中の最大を取る。
2. 行の `Vec<String>` を受け取り、空白で区切った語ごとの件数を数える関数 `count_words_spawned` を、`thread::scope` を使わず `thread::spawn` で書く。
   全スレッドが 1 つの表に数え入れるようにする。
   入力を分けずに全スレッドで共有し、表も共有する。どちらも `spawn` に借りさせられないので、何で包めばよいかを考える。
   全スレッドを待ったあとで `Arc` から中身を取り出すには、`Arc::into_inner` が使える。
