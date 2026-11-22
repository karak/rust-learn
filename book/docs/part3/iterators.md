---
title: 8 イテレータとクロージャ
sidebar_position: 8
---

# 8 イテレータとクロージャ

7 章の最後に見た本物の `report` は、`for` と `push` の代わりに `.iter().map(...).collect()` と書き、比べる関数を `|a, b| ...` とその場で書いていた。
この章は、その 2 つを読めるようにする。

[イテレータ](../glossary.md#iterator)は、`next` を呼ばれるたびに次の要素を 1 つ返す値である。
[クロージャ](../glossary.md#closure)は、周りの変数を閉じ込めた、名前の無い関数である。
どちらも普通の値なので、2 章の[借用](../glossary.md#borrow)の規則がそのまま掛かる。
C++ なら書く側が気をつけていた「[反復子](../glossary.md#iterator-invalidation)や[ラムダ](../glossary.md#cpp-lambda)が、指している先より長生きしないこと」を、コンパイラが確かめる。

## この章のコードは、7 章の詰め替えを式で書き直す

この章のコード（`book/code/ch08`）は、7 章と同じように行を数え、結果をいくつかの形に作り直す。

```rust
fn ranked(counts: &HashMap<String, u64>) -> Vec<(&str, u64)>
fn by_key(counts: &HashMap<String, u64>) -> BTreeMap<&str, u64>
fn at_least(counts: &HashMap<String, u64>, min: u64) -> Vec<&str>
fn total(counts: &HashMap<String, u64>) -> u64
fn parse_all(fields: &[&str]) -> Result<Vec<u64>, ParseIntError>
```

`ranked` は、7 章の `ranked` と同じ順序（件数の降順、同数ならキーの昇順）で並べる。
7 章では `for` と `push` で [`Vec`](../glossary.md#vec) に詰め替え、タプルと `Reverse` で比べ方を書かずに済ませた。
この章では、詰め替えを 1 つの式にし、比べ方をその場で書く。

```rust
fn ranked(counts: &HashMap<String, u64>) -> Vec<(&str, u64)> {
    let mut out: Vec<(&str, u64)> = counts.iter().map(|(key, &n)| (key.as_str(), n)).collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    out
}
```

`counts.iter()` が [`HashMap`](../glossary.md#hashmap) の要素を 1 組ずつ出すイテレータを作る。
`map` は、その 1 組ずつに `|(key, &n)| (key.as_str(), n)` を当てる。
`|...|` の中が引数で、後ろが本体である。
引数の位置にも、7 章の `for (key, &n) in counts` と同じ[パターン](../glossary.md#pattern)が書ける。
`collect` が、出てきたものを `Vec` に集める。

`sort_by` には、7 章では名前の付いた関数 `by_count_desc` を渡した。
ここでは `|a, b| ...` をその場で書いた。
`then_with` は、前の比較が `Equal`（7 章で見た `Ordering` という [enum](../glossary.md#enum) の値）のときだけ、渡したクロージャを呼んで次の比較をする。

`cargo run -p book-ch08` の出力の前半はこうなる。

```text
2 info
2 warn
1 debug
1 error
キーの順: {"debug": 1, "error": 1, "info": 2, "warn": 2}
2 件以上: ["info", "warn"]
合計: 6
Err(ParseIntError { kind: InvalidDigit })
```

順位は 7 章と同じである。
残りの関数は、節を追って見ていく。

## イテレータは、next を呼ぶまで何もしない

Rust のイテレータは、`next` というメソッドを持つ値である。
`next` は、次の要素があれば `Some(要素)` を、尽きていれば `None` を返す。
5 章の [`Option`](../glossary.md#option) である。
`for` 文は、この `next` を `None` が出るまで呼び続ける構文である。

`map` や `filter` のように、イテレータを包んで別のイテレータを返すものをアダプタと呼ぶ。
アダプタは、呼んだ時点では要素に触らない。
受け取ったクロージャを覚えておき、自分の `next` が呼ばれたときに初めて、包んだイテレータの `next` を呼ぶ。

だから、アダプタを呼んだだけで捨てると何も起きない。
コンパイラはそれを警告する。

```text
warning: unused `Map` that must be used
 --> src/main.rs:3:5
  |
3 |     v.iter().map(|n| n * 2);
  |     ^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: iterators are lazy and do nothing unless consumed
```

`next` を呼ぶ側に回るのが、`collect`・`sum`・`for` 文である。
この章のコードの `main` は、`map` と `filter` の中で表示して、呼ばれる順序を見せている。

```rust
let picked: Vec<u64> = [1_u64, 2, 3]
    .iter()
    .map(|n| {
        println!("map {n}");
        n * 10
    })
    .filter(|n| {
        println!("filter {n}");
        *n >= 20
    })
    .collect();
```

```text
map 1
filter 10
map 2
filter 20
map 3
filter 30
[20, 30]
```

`map` を全要素に当ててから `filter` に移るのではない。
1 つの要素が `map` と `filter` を通り抜けてから、次の要素に進む。
途中の結果を入れておく `Vec` は作られない。

C# の LINQ や Java の Stream を知っていれば、この遅延の仕方は同じである。
C++ の反復子とは形が違う。
C++ の反復子は「位置」で、終わりは別の反復子 `end()` と比べて知る。
Rust のイテレータは自分で終わりを知っており、`next` 1 つで「進む」「読む」「終わりか確かめる」を済ませる。
C++20 の ranges のビュー（`std::views::filter` など）が、遅延して段を重ねる点で近い。

## イテレータは、元のコレクションを借りている

`v.iter()` が返すイテレータは、`v` の要素への[共有参照](../glossary.md#references)を返す。
そのため、イテレータそのものが `v` を借りている。
借りているあいだに `v` を書き換えようとすると、2 章で見た[エラーコード](../glossary.md#error-code) `E0502` になる。

```rust
let mut v = vec![1, 2, 3];
let mut it = v.iter();
v.push(4);
println!("{:?}", it.next());
```

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:5
  |
3 |     let mut it = v.iter();
  |                  - immutable borrow occurs here
4 |     v.push(4);
  |     ^^^^^^^^^ mutable borrow occurs here
5 |     println!("{:?}", it.next());
  |                      -- immutable borrow later used here
```

C++ で同じことを書くと、コンパイルは通る。

```cpp
std::vector<int> v{1, 2, 3};
auto it = v.begin();
v.push_back(4);
std::printf("%d\n", *it);
```

`push_back` で領域を確保し直すと、規格はそれまでの[反復子をすべて無効](../glossary.md#iterator-invalidation)にする。
手元（Apple clang 21、libc++）では、`-Wall -Wextra` を付けても警告は出ず、実行すると `1` ではなく `2` が表示された。
[AddressSanitizer](../glossary.md#asan) を付けて走らせると、`heap-use-after-free`（解放済みの領域を読んだ）と報告された。

2 章の `prune` で `for` が `&self.counts` を借りたのも、同じ関係である。
`for` 文に `&v` を渡すと、中で `v.iter()` と同じイテレータが作られる。

イテレータの作り方は 3 つあり、要素を値で受け取るか、共有参照か可変参照で受け取るかに対応している。

| 書き方 | 出てくる要素 | `v` はどうなるか |
| --- | --- | --- |
| `v.iter()`、`for x in &v` | `&T` | 共有で借りられる |
| `v.iter_mut()`、`for x in &mut v` | `&mut T` | 可変で借りられる |
| `v.into_iter()`、`for x in v` | `T` | [move](../glossary.md#move) される |

1 章で `for (key, count) in other.counts` が `counts` を move したのは、3 行目に当たる。
`into_iter` を呼んだ時点で `v` は move されるので、イテレータを使わなくても、そのあとで `v` を読むと 1 章と同じ `E0382` になる。

## アダプタを重ねた型が、そのまま値の型になる

この章のコードの `main` は、アダプタを 2 段重ねた値の型名を表示している。

```rust
let doubled = [1_u64, 2, 3].iter().filter(|&&n| n >= 2).map(|n| n * 10);
println!("{}", type_name_of_val(&doubled));
```

```text
core::iter::adapters::map::Map<core::iter::adapters::filter::Filter<core::slice::iter::Iter<'_, u64>, book_ch08::main::{{closure}}>, book_ch08::main::{{closure}}>
```

`Iter` を `Filter` が包み、それを `Map` が包んでいる。
重ねた順に型が入れ子になり、クロージャもそれぞれが自分だけの型（`{{closure}}`）として型引数に入っている。
この値の大きさは、手元で 16 バイトだった。
`Iter` が持つ 2 つのポインタぶんで、何も閉じ込めていないクロージャは大きさを持たない。

C# の `IEnumerable<T>` や Java の `Stream<T>` では、段を重ねた結果の型はインタフェースであり、段ごとの呼び出しは型の上ではインタフェースを通る（実行時に JIT が呼び先を見抜いて展開することはある）。
Rust では、型が最後まで具体的なまま残り、コンパイラは `filter` に渡したクロージャがどれかを、コンパイルの時点で型から知っている。
型引数ごとにコードを作る仕組み（[単相化](../glossary.md#monomorphization)）は、9 章で扱う。

その結果を手元（Rust 1.97.1、aarch64、`-C opt-level=3`）で確かめた。
`u64` のスライスから「`min` 以上の要素を足す」関数を、`for` 文、添字のループ、`v.iter().filter(...).sum()` の 3 通りに書き、出てきた機械語を比べた。
3 つとも関数呼び出しを 1 つも含まず、1 周で 8 要素をまとめて処理するループになった。
命令列は完全には一致せず、違いはレジスタの割り当てとループの前後の計算にあった。
最適化を切ると（`-C opt-level=0`）、`filter` も `sum` も関数呼び出しのまま残った。
「抽象を重ねても遅くならない」は、最適化が掛かったビルドの話である。

## クロージャは、本体での使い方から借り方が決まる

C++ の[ラムダ式](../glossary.md#cpp-lambda)は、閉じ込める変数とその方法を `[&]`・`[=]`・`[&x, y]` のように書く。
Rust のクロージャには、この並び（キャプチャリスト）が無い。
コンパイラが本体を読み、変数ごとに、いちばん弱くて足りる方法を選ぶ。

- 読むだけなら、共有で借りる
- 書き換えるなら、可変で借りる
- 本体で move するなら、値ごと奪う

`at_least` の `filter` に渡したクロージャは、引数の `min` を読むだけなので、`min` を共有で借りている。

```rust
.filter(|&(_, &n)| n >= min)
```

クロージャの大きさは、閉じ込めたものの大きさになる。
この章のコードのテストで確かめている。
何も閉じ込めない `|| 1` は 0 バイト、`min` を借りる `|n: u64| n >= min` は参照 1 つぶんの 8 バイト、`String` を奪うクロージャは `String` と同じ 24 バイトだった。

借りているのだから、2 章の規則が掛かる。
変数を書き換えるクロージャが生きているあいだ、その変数は読めない。

```rust
let mut seen = 0;
let mut tick = || seen += 1;
println!("{seen}");
tick();
```

```text
error[E0502]: cannot borrow `seen` as immutable because it is also borrowed as mutable
 --> src/main.rs:4:16
  |
3 |     let mut tick = || seen += 1;
  |                    -- ---- first borrow occurs due to use of `seen` in closure
  |                    |
  |                    mutable borrow occurs here
4 |     println!("{seen}");
  |                ^^^^ immutable borrow occurs here
5 |     tick();
  |     ---- mutable borrow later used here
```

`println!` を最後の `tick()` の後ろに移せば通る。
`tick` の借用は、最後に呼ばれたところで終わる。
`tick` を `let mut` で宣言しているのは、呼ぶたびに中の可変参照を使うからで、`mut` を外すと `E0596` で断られる。

クロージャが借りている変数を、クロージャより先に手放すこともできない。
次の `drop` は、値を受け取ってその場で捨てる標準ライブラリの関数である。

```rust
let s = String::from("warn");
let len = || s.len();
drop(s);
println!("{}", len());
```

```text
error[E0505]: cannot move out of `s` because it is borrowed
 --> src/main.rs:4:10
  |
3 |     let len = || s.len();
  |               -- - borrow occurs due to use in closure
  |               |
  |               borrow of `s` occurs here
4 |     drop(s);
  |          ^ move out of `s` occurs here
5 |     println!("{}", len());
  |                    --- borrow later used here
```

C++ で、参照で閉じ込めたラムダを、変数の[スコープ](../glossary.md#scope)の外へ持ち出すと、こうなる。

```cpp
std::function<std::size_t()> len;
{
    std::string s = "a long string that does not fit in SSO buffer";
    len = [&] { return s.size(); };
}
std::printf("%zu\n", len());
```

手元では、`-Wall -Wextra` で警告は出ず、実行すると `45` と正しそうな値が表示された。
AddressSanitizer を付けると、`stack-use-after-scope`（スコープを抜けた変数を読んだ）と報告された。
[未定義動作](../glossary.md#undefined-behavior)なので、正しそうな値が出たのは偶然である。

C# や Java は、別の方法でこの問題を避けている。
C# は、ラムダが閉じ込めたローカル変数をコンパイラが[ヒープ](../glossary.md#heap)上のオブジェクトに移し、デリゲートが生きているあいだ生かしておく。
Java は、ラムダから使えるローカル変数を実質的に final なものに限り、値を写す。
Rust はどちらもしない。
変数は元の場所に置いたまま借り、借りた側が長生きしないことをコンパイラが確かめる。

もう 1 つ、借りる単位が細かい。
構造体のフィールドを 1 つだけ使うクロージャは、構造体全体ではなくそのフィールドだけを借りる。

```rust
let mut bump = || t.total += 1;
assert_eq!(t.keys.len(), 1);   // t.keys は借りられていないので読める
bump();
```

Rust には、互換性を壊す変更をまとめて入れる「版」（edition）があり、クレートごとに `Cargo.toml` の `edition` で選ぶ。
この本のコードは 2024 年版である。
フィールド単位で借りる振る舞いは 2021 年版から入ったもので、2018 年版としてコンパイルすると、`t` 全体を借りたことになって `E0502` で断られる。

## move を付けると、借りる代わりに奪う

`move ||` と書くと、クロージャは本体で使う変数を、使い方によらず値で受け取る。
`String` なら move され、元の変数は使えなくなる。

```rust
let s = String::from("warn");
let len = move || s.len();
drop(s);   // error[E0382]: use of moved value: `s`
```

[`Copy`](../glossary.md#copy-clone) な型なら、値が写される。
写したあとは、元の変数とクロージャの中の値は別物である。

```rust
let mut n = 1_u64;
let mut add = move || {
    n += 1;
    n
};
assert_eq!((add(), add()), (2, 3));
assert_eq!(n, 1);   // 元の n は変わっていない
```

`move` が要るのは、クロージャが、借りようとする変数より長生きするときである。
典型は、クロージャを返すクロージャである。
次の例は、キーごとに「`line` がそのキーで始まるか」を確かめるジョブを作る。

```rust
let keys = vec![String::from("warn"), String::from("info")];
let line = String::from("warn: disk full");
let jobs: Vec<_> = keys
    .iter()
    .cloned()
    .map(|key| || line.starts_with(key.as_str()))
    .collect();
```

`cloned` は、借りた要素を複製して渡すアダプタである。
外側のクロージャは、`key` を `String` の値で受け取る。
内側のクロージャは `key` を借りようとするが、`key` は外側のクロージャが返ると捨てられる。

```text
error[E0373]: closure may outlive the current function, but it borrows `key`, which is owned by the current function
```

内側に `move` を付ければ `key` は奪えるが、今度は `line` まで奪おうとして、別のエラーになる。

```text
error[E0507]: cannot move out of `line`, a captured variable in an `FnMut` closure
```

`line` はジョブの数だけ使いたいので、1 つ目のジョブに奪われると困る。
C++ なら `[key, &line]` と、変数ごとに方法を書き分けるところである。
Rust では、借りたい変数の参照を先に変数へ入れ直し、その参照を奪わせる。

```rust
.map(|key| {
    let line = &line;
    move || line.starts_with(key.as_str())
})
```

内側が奪うのは `key` の値と、`line` への参照の複製である。
参照は `Copy` なので、ジョブがいくつあっても同じ `line` を指せる。
もう 1 つの典型であるスレッドに渡すクロージャは、12 章で扱う。

## 呼べる回数で、クロージャは 3 種類に分かれる

本体で閉じ込めた値を move するクロージャは、1 回しか呼べない。
2 回目には、奪った値がもう無いからである。

```rust
let s = String::from("warn");
let take = move || s;
let a = take();
let b = take();
```

```text
error[E0382]: use of moved value: `take`
 --> src/main.rs:5:13
  |
4 |     let a = take();
  |             ------ `take` moved due to this call
5 |     let b = take();
  |             ^^^^ value used here after move
  |
note: closure cannot be invoked more than once because it moves the variable `s` out of its environment
```

Rust は、クロージャを呼べる回数と条件で 3 つに分けている。
エラーメッセージや標準ライブラリの説明に出てくる [`Fn`・`FnMut`・`FnOnce`](../glossary.md#fn-traits) が、その名前である。

- `FnOnce` は、1 回は呼べる。本体で値を move するクロージャはこれだけを満たす
- `FnMut` は、何度でも呼べるが、呼ぶたびに中身を書き換えうる。`tick` がこれに当たる
- `Fn` は、何度でも、共有のまま呼べる。読むだけのクロージャがこれに当たる

`Fn` を満たすクロージャは `FnMut` も `FnOnce` も満たし、`FnMut` を満たすものは `FnOnce` も満たす。
どれになるかを決めるのは本体での使い方で、`move` キーワードではない。
`move || s.len()` は `s` を奪って閉じ込めるが、本体は読むだけなので `Fn` である。

関数の側は、受け取るクロージャにどれを求めるかを宣言している。
`sort_by` や 2 章の `retain` は `FnMut` を求める。
何度も呼ぶが、呼ぶあいだにクロージャが状態を書き換えても構わないからである。
読むだけのクロージャも、`FnMut` を満たすので渡せる。
前の節の `E0507` は、`map` が求める `FnMut` を、内側の `move` が満たせなくしたために出た。
外側のクロージャが `line` を奪って渡すと、2 回目に渡すものが無いからである。

C++ のラムダにも、近い区別はある。
`mutable` を付けないラムダの `operator()` は `const` で、値で閉じ込めた変数を書き換えられない。
しかし [`std::function`](../glossary.md#std-function) の型には、この区別が無い。
何度でも呼べる前提で、中のラムダが閉じ込めた値を move してしまうかどうかは、書く側が気をつける
（C++23 の `std::move_only_function` は、`const` や `&&` の修飾で一部を型に書ける）。
自分の関数でクロージャを受け取り、どれを求めるかを書く方法は、9 章で扱う。

## collect は、作るものを戻り値の型から決める

`collect` は、何に集めるかを引数では受け取らない。
受け取る側の型を見て決める。
型が決まらなければ、コンパイラは断る。

```rust
let w = v.iter().map(|n| n * 2).collect();
```

```text
error[E0283]: type annotations needed
 --> src/main.rs:3:9
  |
3 |     let w = v.iter().map(|n| n * 2).collect();
  |         ^                           ------- type must be known at this point
  |
help: consider giving `w` an explicit type
  |
3 |     let w: Vec<_> = v.iter().map(|n| n * 2).collect();
  |          ++++++++
```

`Vec<_>` の `_` は「要素の型は推論に任せる」という意味である。
変数に型を書く代わりに、`collect::<Vec<_>>()` と呼ぶ側に書いてもよい。

同じ `collect` が、`by_key` では [`BTreeMap`](../glossary.md#btreemap) を作る。
戻り値の型が `BTreeMap<&str, u64>` で、組 `(キー, 値)` を出すイテレータからなら、そのまま表が作れるからである。

```rust
fn by_key(counts: &HashMap<String, u64>) -> BTreeMap<&str, u64> {
    counts.iter().map(|(key, &n)| (key.as_str(), n)).collect()
}
```

いちばん効くのは、[`Result`](../glossary.md#result) に集める形である。
`parse_all` は、文字列を 1 つずつ数に直し、その `Result` の列を `Result<Vec<u64>, _>` に集める。

```rust
fn parse_all(fields: &[&str]) -> Result<Vec<u64>, ParseIntError> {
    fields.iter().map(|field| field.parse::<u64>()).collect()
}
```

全部が `Ok` なら、中身を並べた `Vec` が `Ok` に包まれて返る。
1 つでも `Err` があれば、最初の `Err` が返る。
そして、最初の `Err` が出た時点で、残りの要素には触らない。
テストでは `["1", "x", "y"]` を渡し、`parse` が `1` と `x` にしか呼ばれないことを確かめている。
5 章で `?` を `loop` の中で使ったときと同じく、最初の失敗で抜ける。
空の列を渡すと `Ok` に包まれた空の `Vec` が返る。

## 本物ではどうなっているか

7 章で抜き出した `tally-core` の `report` は、この章の `ranked` とほとんど同じ形をしている。

```rust
let mut entries: Vec<Entry> = self
    .counts
    .iter()
    .map(|(key, &count)| Entry {
        key: key.clone(),
        count,
    })
    .collect();

entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
```

違いは 2 つある。
`map` で作るのが `Entry` という構造体であることと、キーを借りずに `clone` で複製していることである。
`tally` の `main.rs` は `Ok(counter.report(cli.limit))` と書き、`counter` を捨てたあとも結果を返り値として使う。
だから結果は `Counter` から借りたままにはできない。

`move` と借り直しは、`tally` の `main.rs` にある。
入力のファイルごとに「開いて集計する」ジョブをクロージャで作り、並列に走らせる部分である。

```rust
let jobs: Vec<_> = cli
    .inputs
    .iter()
    .map(|path| {
        let selector = &selector;
        let keep = &keep;
        move || -> Result<Counter, CliError> {
            tracing::debug!(path = %path.display(), "ファイルから読み込みます");
            input::tally_input(path.as_path(), cli.counter(), selector, keep)
        }
    })
    .collect();
```

前の節で見た形そのものである。
`-> Result<Counter, CliError>` は、クロージャの戻り値の型を明示する書き方である。
`path` は、入力のファイルのパスへの参照である。
`keep` も、`main.rs` の少し上で作ったクロージャで、行を残すかどうかを決める。
内側のクロージャは、そのクロージャへの参照を閉じ込めていることになる。

ただし、手元でこの借り直しの 2 行と `move` を外し、呼び出しの `selector` を `&selector` に直しても、コンパイルは通った。
前の節の例と違い、ここでの `path` は `cli.inputs` の要素への参照で、外側のクロージャが値で持っているわけではない。
2021 年版からは、内側のクロージャは変数 `path` そのものではなく、その先の要素を借りる。
その先は `cli.inputs` の中にあり、外側のクロージャが返っても消えない。
2018 年版では変数 `path` を借りたので、同じ形が `E0373` で断られた。
`.iter()` を `.iter().cloned()` にして `path` を値で渡すと、`move` が無ければ `E0373` で断られるようになる。

ジョブを受け取って並列に走らせる側は、受け取るクロージャに `Fn` と、スレッドをまたいでよいという条件を求めている。
その書き方は 9 章と 12 章で扱う。

## 演習

答えは [`book/code/ch08/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch08/src/exercises.rs) にある。
`cargo test -p book-ch08` で確かめられる。

**演習 1**

7 章の演習 1 をもう一度書く。
キーごとに、最初に現れた行の番号（1 から数える）を返す関数を、`for` を使わずに書け。

```rust
fn first_seen(lines: &[&str]) -> HashMap<String, usize>
```

`["b", "a", "b"]` なら、`b` は 1、`a` は 2 である。
要素に番号を振るアダプタ `enumerate` と、逆順に辿るアダプタ `rev` を使ってよい。
`HashMap` に `collect` すると、同じキーが 2 度出たときにどちらの値が残るかを、先に確かめておくとよい。

**演習 2**

空でない行だけを残し、捨てた行の数も返す関数を書け。
`filter` に渡すクロージャの中で、捨てた数を数えること。

```rust
fn non_empty(text: &str) -> (Vec<&str>, usize)
```

`"a\n\nb\n\n"` なら、`(vec!["a", "b"], 2)` である。
書けたら、`collect` を呼ぶ前に捨てた数を読もうとすると、どのエラーになるかを試してほしい。
