---
slug: /
title: はじめに
sidebar_position: 1
---

# はじめに

この本は、`tally` という小さなコマンドラインツールを題材に、Rust の言語機能を一つずつ辿る。

`tally` は標準入力かファイルから行を読み、キーごとに件数を数えて出力する。
`sort | uniq -c | sort -rn` を 1 本のコマンドにしたものだと思えばよい。

```
$ tally --field level app.log
42	info
7	warn
3	error
```

題材が小さいのには理由がある。
所有権も、エラー型の設計も、並列化も、このツール一つの中で実際に判断を迫られた。
作り話の例ではなく、実際に動いているコードの中で各機能がどう効いているのかを見ていく。

## 想定する読者

仕事でコードを書いて届けた経験がある人を想定している。
C++ を実務で触ったことがあれば、所有権の章は読みやすいはずだ。

一方で、型システムの理論的な語彙は前提にしない。
変性、型クラス、高階型といった語は、必要になった箇所でその場で説明する。

Rust のインストールと `cargo` の基本操作は済んでいるものとする。
まだなら [rustup](https://rustup.rs/) から入れてほしい。

## 読み方

各章は独立している。
章を順に読めば難易度が緩やかに上がるように並べたが、必要な章だけを拾っても詰まらないようにしてある。

章ごとに小さなコードが付いている。
実際に動くものが [GitHub の `book/code/`](https://github.com/karak/rust-learn/tree/main/book/code) にあり、章の末尾の演習の答えも同じ場所にある。

```bash
git clone https://github.com/karak/rust-learn.git
cd rust-learn
cargo run -p book-ch01
```

## この本が扱わないもの

**設計判断の経緯はこの本の正本ではない。**
「なぜ `Selector` を trait にしなかったのか」のような、設計に至るまでの判断は [ADR](https://github.com/karak/rust-learn/tree/main/docs/adr) に記録してある。
この本が設計判断に触れるときは、結論と、その結論が言語機能のどこを使っているかだけを書く。

`tally` の現在の実装状態も書かない。
版が進めば変わるので、確かめたいときは[ソース](https://github.com/karak/rust-learn/tree/main/crates/tally)を読んでほしい。
