---
title: 2 借用
sidebar_position: 2
---

# 2 借用

1 章の `merge` は、相手のカウンタを値で受け取った。
渡した側はそれを失い、二度と使えなくなった。

ほとんどの関数はそういう振る舞いを望まない。
数えた結果を表示するたびにカウンタが消えてしまっては困る。
値を渡す代わりに貸すのが、この章の話である。

この章で使うカウンタには、貸し方の違う二つのメソッドがある。

```rust
fn sorted(&self) -> Vec<(&str, u64)>

fn prune(&mut self, min: u64)
```

`sorted` は並べて返すだけで、カウンタを変えない。
`prune` は件数の少ないキーを落とすので、カウンタを変える。
この違いが `&self` と `&mut self` に出ている。

## 二種類の貸し方

`&T` を共有参照、`&mut T` を可変参照と呼ぶ。
C++ の `const T&` と `T&` に、それぞれ対応する。

対応はするが、コンパイラが保証する範囲が違う。
C++ の `const T&` は「この参照からは書き換えない」という約束でしかなく、同じオブジェクトを指す別の参照が書き換えることを妨げない。
Rust の `&T` は、借りているあいだ誰も書き換えないことまで保証する。

ただし、この約束を型の側で引き受け直す仕組みがある。
`&` で受け取りながら中身を書き換えられる型がいくつかあり、そのときは書き換えが重ならないことを型が自分で面倒を見る。
スレッドをまたいで値を共有するときに使うもので、12 章で出てくる。
それまでの章に出てくる型には当てはまらない。

## 規則は二つ

Rust が課す規則は、次の二つだけである。

一つの値に対して、共有参照 `&T` は同時にいくつでも作れる。
可変参照 `&mut T` は同時に一つしか作れず、それが存在するあいだは共有参照も作れない。

読む人は何人いてもよいが、書く人がいるあいだは他の誰もその値を見られない。
この二つから、借用に関するコンパイルエラーのほとんどが導ける。

## 規則に当たってみる

`prune` を素直に書くと、規則に当たる。
件数の少ないキーを探して消す、という手続きをそのまま書いた形である。

```rust
fn prune(&mut self, min: u64) {
    for (key, count) in &self.counts {
        if *count < min {
            self.counts.remove(key);
        }
    }
}
```

これはコンパイルを通らない。

```
error[E0502]: cannot borrow `self.counts` as mutable because it is also borrowed as immutable
   |
11 |         for (key, count) in &self.counts {
   |                             ------------
   |                             |
   |                             immutable borrow occurs here
   |                             immutable borrow later used here
12 |             if *count < min {
13 |                 self.counts.remove(key);
   |                 ^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
```

`for` に渡した `&self.counts` は、ループが終わるまで生き続ける共有参照である。
一方 `remove` は `counts` を書き換えるので、可変参照を要求する。
二つ目の規則のとおり、共有参照があるあいだは可変参照を作れない。

この形なら通る。

```rust
fn prune(&mut self, min: u64) {
    self.counts.retain(|_, count| *count >= min);
}
```

`retain` は条件を受け取って、自分で反復しながら消す。
借り手が一つになるので規則を満たす。

## コンパイラは何を守ったのか

落ちたコードを C++ で書くと、こうなる。

```cpp
for (auto& [key, count] : counts) {
    if (count < min) {
        counts.erase(key);   // ここで反復子が無効になる
    }
}
```

これは未定義動作である。
`erase` は消した要素を指す反復子を無効にするので、範囲 for が次に進むときに無効な反復子を進めることになる。

ところが、次のように書けば正しく動く。

```cpp
for (auto it = counts.begin(); it != counts.end(); ) {
    if (it->second < min) {
        it = counts.erase(it);   // 次の要素を指す反復子が返る
    } else {
        ++it;
    }
}
```

`unordered_map` の `erase` が無効にするのは消した要素の反復子だけで、他の要素の反復子は生きている。
再ハッシュを起こすのは `insert` のほうである。

つまり C++ には、見た目のよく似た正しい書き方と壊れた書き方の両方がある。
そして、どちらであるかをコンパイラは教えてくれない。
壊れたほうは、要素が一つだけなら動いてしまうことも多い。

Rust は、形のほうを拒む。
「反復しながら同じコンテナを書き換える」という形が規則に反するので、正しく書けたはずの場合もまとめて通らない。
その代わり、通ったコードはこの種の誤りを含まない。

他の言語では書く側が気をつけていた部分を、型システムに移したのだと考えるとよい。
気をつけ方を覚える必要は無くなるが、コンパイラが許す形に合わせて書く必要が生まれる。

## 「可変」ではなく「排他」

`&mut` という記法は mutable から来ているが、規則として効いているのは排他性のほうである。

同じ値に書き手が一人しかいないなら、読んでいる最中に書き換えられることはない。
これはデータ競合が成立しないための条件そのものである。
並行処理の話は 12 章で扱うが、その土台はここで作られている。

書き換えないのに `&mut` が必要になる場面がある、という逆の見方もできる。
排他が欲しいだけのときにも `&mut` を書く。

## 借用はいつ終わるか

借用は、最後に使った場所で終わる。
宣言したブロックの終わりまで続くのではない。

次のコードは通らない。

```rust
let top = tally.sorted();
tally.push("warn");
println!("{top:?}");
```

```
error[E0502]: cannot borrow `tally` as mutable because it is also borrowed as immutable
   |
24 |     let top = tally.sorted();
   |               ----- immutable borrow occurs here
25 |     tally.push("warn");
   |     ^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
26 |     println!("{top:?}");
   |                --- immutable borrow later used here
```

`top` を最後に使うのが `println!` なので、借用はそこまで続いている。
その途中に `push` が入っている。

二行を入れ替えるだけで通る。

```rust
let top = tally.sorted();
println!("{top:?}");
tally.push("warn");
```

`top` はまだ変数として残っているが、もう使われないので借用は終わっている。
C++ の参照は宣言したスコープのあいだ生き続けるので、ここは直感が食い違う。
Rust で「変数が生きている範囲」と「借りている範囲」は別物である。

## 借りたものを返り値で持ち出す

`sorted` の戻り値をもう一度見てほしい。

```rust
fn sorted(&self) -> Vec<(&str, u64)>
```

返している `&str` は、カウンタが持っている `String` を指している。
新しく文字列を作ってはいない。
だからこの `Vec` を持っているあいだ、カウンタ全体が借りられたままになる。
先ほどのエラーが起きたのはそのためである。

ここで一つ、書かずに済んでいるものがある。
返り値の `&str` が引数の `&self` から来ていることを、コードのどこにも書いていない。
引数が `&self` 一つだけなので、規則で決まってしまうからである。
規則で決まらない形を書くときは注釈が要るが、それは 10 章まで出てこない。

## 本物ではどうなっているか

`tally` が入力を読む関数は、先頭の二つの引数が今回の話に当たる。

```rust
pub fn tally_reader<R, F>(
    counter: &mut Counter,
    mut reader: R,
    selector: &Selector,
    keep: F,
) -> Result<()>
```

`counter` は数を積み上げるので `&mut`、`selector` は設定を読むだけなので `&` である。
呼び出した側はどちらも手放さない。
同じ `counter` に別のファイルを続けて流し込めるのは、値で渡していないからである。

残りは後の章になる。
`R` と `F` のような型の受け取り方は 9 章、`keep` に渡す関数そのものは 8 章、`Result` は 5 章で扱う。

## 演習

答えは [`book/code/ch02/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch02/src/exercises.rs) にある。
`cargo test -p book-ch02` で確かめられる。

**演習 1**

`retain` を使わずに `prune` を書け。
落とすキーを先に集めておけば、二つの借用が重ならない。
集めたものを `Vec<&String>` にすると通らないので、なぜ通らないかをエラーから読み取ってほしい。

**演習 2**

`sorted` を、カウンタを借りない形に変えよ。
戻り値を受け取ったまま `push` を呼べるようになる。
その代わり何が増えるかを、1 章の `merge` の話と見比べて確かめてほしい。
