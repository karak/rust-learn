---
title: 1 所有権と move
sidebar_position: 1
---

# 1 所有権と move

`tally` は複数のファイルを同時に読む。
ファイルごとに別のカウンタを作って数え、最後に一つへ合流させる。
その合流の手続きが、この章の題材になる。

```rust
fn merge(&mut self, other: Tally) {
    for (key, count) in other.counts {
        *self.counts.entry(key).or_insert(0) += count;
    }
    self.total += other.total;
}
```

引数が `&Tally` でも `&mut Tally` でもなく `Tally` であることに目を留めてほしい。
呼び出した側では、渡したカウンタがその行を境に使えなくなる。

```rust
left.merge(right);
println!("{right:?}");  // ここでコンパイルエラー
```

```
error[E0382]: borrow of moved value: `right`
```

C++ であれば `Tally&&` を受け取って中身を奪い、呼び出し側には空になったオブジェクトが残る。
Rust は残さない。
なぜそうなっているのか、そしてそれが書く側に何をもたらすのかを見ていく。

## 所有権の規則

Rust の値には、必ず一人の**所有者**がいる。
所有者は変数であり、所有者がスコープを抜けると値は解放される。

ここまでは C++ の RAII と同じ話である。
`std::unique_ptr` も所有者は一つで、デストラクタが解放する。

違いは、所有権の移動をコンパイラが追いかけることにある。
`unique_ptr` は所有権を移したあとも変数として存在し、`nullptr` を指す有効なオブジェクトとして残る。
Rust はそれを **move** と呼び、move された変数をそれ以降使わせない。

```rust
let right = Tally::new();
left.merge(right);   // right の所有権が merge に移る
// right という名前は、ここから先は使えない
```

これは実行時の検査ではない。
`right` に何かフラグが立つのではなく、コンパイラが「この変数はここで move された」と記録し、以降の使用を拒否する。

## move したあとに何が残るか

C++ の move では、移動元のオブジェクトは「有効だが未規定の状態」になる。
標準ライブラリの型なら空になることが多いが、規格が保証するのは「デストラクタを呼んでも安全」ということだけである。
だから移動元を再利用するコードは、何が入っているかを確かめてから書くことになる。

Rust には、この「移動後の状態」という概念がない。
move された変数は状態を持たないのではなく、変数として存在しなくなる。

この差は、書くコードの形に出る。
C++ では move コンストラクタと move 代入演算子を書き、移動元を安全な状態に整える責任がある。
Rust にはそれに当たるものがない。
move はメモリ上のビット列をそのまま移すだけで、利用者が書いたコードは一切走らない。

デストラクタについても同じことが起きる。
move された変数のデストラクタは走らない。
走ってしまえば、移動先が持っている値を二重に解放することになる。
コンパイラは move されたかどうかを追えているので、解放する側を一つに決められる。

## 構造体の一部だけを move する

先ほどの `merge` をもう一度見ると、`other` の扱いが二段になっている。

```rust
for (key, count) in other.counts {   // counts を move する
    *self.counts.entry(key).or_insert(0) += count;
}
self.total += other.total;           // total はまだ読める
```

`for` に `other.counts` をそのまま渡すと、`HashMap` が move されて反復に消費される。
それでも次の行で `other.total` を読める。
move されたのは `counts` というフィールドだけで、`other` 全体ではないからだ。

これを**部分 move** と呼ぶ。
一度フィールドを move した構造体は、そのフィールドを読めないが、残りのフィールドは読める。
ただし構造体全体を別の場所に渡すことはもうできない。
欠けた部分があるものを、完全な `Tally` として扱わせるわけにはいかない。

## 値で受け取ると何が得か

`merge` が借りるのではなく受け取るのは、所有権の規則を練習するためではない。
キーの `String` を作り直さずに済ませるためである。

`&Tally` を受け取る版を書いてみると、違いがはっきりする。

```rust
fn merge_borrowed(&mut self, other: &Tally) {
    for (key, count) in &other.counts {
        *self.counts.entry(key.clone()).or_insert(0) += count;
    }
}
```

`key.clone()` が増えた。
借りた `HashMap` を反復すると得られるのは `&String` であり、`entry` はキーの所有権を要求するので、複製するほかない。
キーの種類が 1 万あれば 1 万回の確保が走る。

値で受け取る版では、この複製が一度も起きない。
`other` の `HashMap` を消費して反復すると、`String` が所有権ごと手に入る。
それをそのまま `entry` に渡せる。

引数を `Tally` にするという一語の選択が、「呼び手はもうこのカウンタを使わない」という約束をコンパイラに伝え、その約束と引き換えに複製を省いている。

## 複製される型

ここまで move の話をしてきたが、すべての型が move されるわけではない。

```rust
let count: u64 = 3;
let copied = count;
println!("{count}");  // 問題なく動く
```

`u64` のように、ビット列をそのまま複製しても意味が壊れない型は、代入のときに複製される。
こうした型は `Copy` という性質を持つ。
整数、浮動小数点数、`bool`、`char`、それに `Copy` な型だけでできたタプルや配列が当てはまる。

`String` や `HashMap` はヒープ上の領域を所有しているので、ビット列の複製では所有者が二人になってしまう。
だから `Copy` ではなく、move される。

C++ で言えば、`Copy` な型はコピーコンストラクタが自明に `memcpy` で済む型に近い。
ただし C++ はどの型にもコピーを許すのが既定で、禁じたいときに `= delete` を書く。
Rust は逆で、複製は `Copy` か `Clone` を実装した型にだけ許される。
`Clone` は `.clone()` と書いたときだけ働くので、確保を伴う複製はコード上に必ず現れる。

## 演習

答えは [`book/code/ch01/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch01/src/exercises.rs) にある。
`cargo t -p book-ch01` で確かめられる。

**演習 1**

`Tally` に、カウンタへ名前を付ける `label` を足したい。
次のようにつなげて書けるようにするには、`label` と `push` の引数と戻り値をどうすればよいか。

```rust
let built = Labeled::new().label("app.log").push("info").push("warn");
```

`&mut self` を受け取る形でも動く。
その形ではなぜつなげて書けないのかを、エラーメッセージを読んで確かめてほしい。

**演習 2**

`merge` を、借りて受け取る `merge_borrowed` に書き換えよ。
`key.clone()` を書かずにコンパイルを通せるか試し、通らない理由をコンパイラの指摘から説明してみてほしい。
