---
title: 用語集
---

# 用語集

本文に出てくる用語を、英語での呼び名と一緒に引けるようにしたものである。
説明は短くしてあり、詳しい話は「出てくる章」に置いてある。

各章からは、その章で用語を説明している箇所へ、説明が無ければ地の文で初めて出る箇所からここへリンクしている。
コードやエラーメッセージの中にだけ出る用語にはリンクしていない。

`tally` そのものの語彙（出力の形式やオプションの意味）はここには入れない。
それは [`tally` の出力の仕様](https://github.com/karak/rust-learn/blob/main/crates/tally/docs/output-format.md)の側にある。

## Rust の用語

### 所有権・所有者（ownership / owner） {/* #ownership */}

Rust の値には必ず一人の所有者がいて、所有者がスコープを抜けると値が解放される、という規則。

出てくる章: [はじめに](intro.md)、[1](part1/ownership.md)、[3](part1/strings.md)

### move {/* #move */}

値の所有権を別の変数や関数へ移すこと。
移したあとの元の変数は、コンパイラが使わせない。

出てくる章: [1](part1/ownership.md)

### 部分 move（partial move） {/* #partial-move */}

構造体のフィールドの一部だけを move すること。
残りのフィールドは読めるが、構造体全体はもう渡せない。

出てくる章: [1](part1/ownership.md)

### `Copy` と `Clone` {/* #copy-clone */}

`Copy` は代入のときに暗黙に複製される型の性質で、`Clone` は `.clone()` と書いたときだけ複製できる型の性質。
どちらも trait である。

出てくる章: [1](part1/ownership.md)

### drop {/* #drop */}

値の所有者がスコープを抜けたときに、その値の後始末を走らせて解放すること。
C++ のデストラクタの呼び出しに当たる。
move された変数については走らない（値が移った先で 1 回だけ走る）。

出てくる章: [1](part1/ownership.md)（デストラクタとして）、3（エラーメッセージの中）

### 借用（borrow） {/* #borrow */}

値を所有したまま、参照を作って貸すこと。

出てくる章: [1](part1/ownership.md)、[2](part1/borrowing.md)、[3](part1/strings.md)

### 共有参照と可変参照（shared reference / mutable reference） {/* #references */}

`&T` が共有参照、`&mut T` が可変参照。
共有参照は同時にいくつでも作れるが、可変参照は同時に一つだけで、そのあいだは共有参照も作れない。
この性質から、可変参照は排他参照（exclusive reference）とも呼ばれる。

出てくる章: [2](part1/borrowing.md)

### 内部可変性（interior mutability） {/* #interior-mutability */}

共有参照 `&T` しか持っていなくても中身を書き換えられるようにする仕組み。
書き換えが重ならないことは、型の側が受け持つ。

出てくる章: [2](part1/borrowing.md)

### 寿命（lifetime） {/* #lifetime */}

値や参照が有効でいられる期間。
Rust では、参照が指す先より長く生きないことをコンパイラが確かめる。

出てくる章: [3](part1/strings.md)

### `String` と `&str` {/* #string-str */}

`String` はヒープ上の UTF-8 のバイト列を所有する文字列で、`&str` はどこかにある UTF-8 のバイト列を眺める参照。

出てくる章: [1](part1/ownership.md)、[2](part1/borrowing.md)、[3](part1/strings.md)

### 文字列リテラル（string literal） {/* #string-literal */}

`"warn"` のようにコードに直接書いた文字列。
型は `&str` で、プログラムに埋め込まれたバイト列を指す。

出てくる章: [3](part1/strings.md)

### スライス（slice） {/* #slice */}

連続した要素の並びの全体または一部を指す参照で、`&[T]` と書く。
先頭へのポインタと長さを持つ。
`&str` は文字列のスライスである。

出てくる章: [3](part1/strings.md)

### 文字の境界（char boundary） {/* #char-boundary */}

UTF-8 のバイト列のうち、1 文字の先頭に当たる位置（と末尾）。
`&str` を範囲で切るときは、範囲の両端がこの位置でなければならない。

出てくる章: [3](part1/strings.md)

### `Vec<T>` {/* #vec */}

要素をヒープ上に連続して並べる、長さを変えられる配列。
C++ の `std::vector` に当たる。

出てくる章: [2](part1/borrowing.md)、[3](part1/strings.md)

### `HashMap` {/* #hashmap */}

キーから値を引くハッシュ表。
C++ の `std::unordered_map` に当たる。

出てくる章: [1](part1/ownership.md)、[3](part1/strings.md)

### `Option` {/* #option */}

値が有るか無いかを表す型。
有れば `Some(値)`、無ければ `None` になる。

出てくる章: [3](part1/strings.md)

### `Result` {/* #result */}

成功の値か失敗の値のどちらかを持つ型。
成功なら `Ok(値)`、失敗なら `Err(エラー)` になる。

出てくる章: [2](part1/borrowing.md)

### 型強制（coercion） {/* #coercion */}

ある型の値を、コンパイラが暗黙に別の型として扱うこと。
Rust では起きる場所と種類が限られている。

出てくる章: [3](part1/strings.md)

### 参照外し型強制（deref coercion） {/* #deref-coercion */}

型強制の一種。
`&String` を `&str` として渡せるように、行き先の型が決まっている場所でコンパイラが参照の型を変換すること。

出てくる章: [3](part1/strings.md)

### panic {/* #panic */}

プログラムが誤りに当たったときに、実行を打ち切る仕組み。
既定では、スタックを巻き戻しながら値の後始末を走らせ、そのスレッドを終える。
main スレッドで起きれば、プログラム全体が終わる。

出てくる章: [3](part1/strings.md)

### `unsafe` {/* #unsafe */}

コンパイラが安全性を確かめられない操作を、書く人の責任で許すための印。

出てくる章: [3](part1/strings.md)

### trait {/* #trait */}

型が持つ操作や性質をまとめて宣言するもの。

出てくる章: [はじめに](intro.md)、3（エラーメッセージの中）

### エラーコード（error code） {/* #error-code */}

コンパイルエラーに付く `E0382` のような番号。
`rustc --explain E0382` で、そのエラーの説明と例を読める。

出てくる章: 1、2、3（エラーメッセージの中）。[rustc](#rustc) も参照

## ツール

### cargo {/* #cargo */}

Rust のビルドとパッケージ管理の道具。
`cargo run`、`cargo test` などでビルド・実行・テストを行う。

出てくる章: [はじめに](intro.md)、各章の演習（コードの中）

### rustc {/* #rustc */}

Rust のコンパイラ。
ふだんは `cargo` が呼び出すので、直接打つことは少ない。

出てくる章: 本文には出ない。[エラーコード](#error-code)の項目で使う

### clippy {/* #clippy */}

Rust の lint（コードの書き方の点検）を行う道具。
`cargo clippy` で走らせる。

出てくる章: [3](part1/strings.md)

### rustup {/* #rustup */}

Rust のツールチェーン（コンパイラや `cargo` の版）を入れて切り替える道具。

出てくる章: [はじめに](intro.md)

## プログラミング言語の用語

### 参照外し（dereference） {/* #dereference */}

参照やポインタが指している先の値を読み書きすること。
Rust でも `*` と書く。

出てくる章: 1、2（コードの中）、[3](part1/strings.md)

### スコープ（scope） {/* #scope */}

変数の名前が有効な範囲。
Rust では、所有者の変数がスコープを抜けると値が解放される。

出てくる章: [1](part1/ownership.md)、[2](part1/borrowing.md)、[3](part1/strings.md)

### ヒープ（heap） {/* #heap */}

実行中に大きさを決めて確保し、使い終わったら解放するメモリの領域。

出てくる章: [1](part1/ownership.md)、[3](part1/strings.md)

### ファットポインタ（fat pointer） {/* #fat-pointer */}

アドレスに加えて、長さなどの情報を一緒に持つポインタ。
wide pointer とも呼ばれる。
3 章で「二語の値」と呼んだ `&str` やスライスがこれに当たる。

出てくる章: [3](part1/strings.md)

### スタックの巻き戻し（stack unwinding） {/* #stack-unwinding */}

例外や panic が起きたときに、呼び出しを逆にたどりながら、各関数が持っていた値の後始末を走らせること。

出てくる章: [3](part1/strings.md)

### 未定義動作（undefined behavior） {/* #undefined-behavior */}

言語の規格が結果を何も定めていない動作。
動いているように見えても、何が起きても文句は言えない。

出てくる章: [2](part1/borrowing.md)、[3](part1/strings.md)

### データ競合（data race） {/* #data-race */}

複数のスレッドが同じ場所に同時にアクセスし、少なくとも一方が書き込みで、間に同期が無い状態。

出てくる章: [2](part1/borrowing.md)

### UTF-8 {/* #utf-8 */}

Unicode の文字を 1 から 4 バイトの可変長で表す符号化方式。
ASCII の文字は 1 バイトで、日本語の多くの文字は 3 バイトになる。

出てくる章: [3](part1/strings.md)

### 符号点（code point） {/* #code-point */}

Unicode が文字に割り当てるための番号。
U+0000 から U+10FFFF まであり、そのうちサロゲートと呼ばれる範囲は文字を表さない。

出てくる章: [3](part1/strings.md)

### Unicode スカラ値（Unicode scalar value） {/* #unicode-scalar-value */}

符号点のうち、サロゲートの範囲を除いたもの。
Rust の `char` 1 つがこれ 1 つに当たる。

出てくる章: [3](part1/strings.md)

### 書記素クラスタ（grapheme cluster） {/* #grapheme-cluster */}

人が 1 文字と見なす単位。
前の文字に付く結合文字や、組み合わせの絵文字では、複数の Unicode スカラ値が 1 つの書記素クラスタになる。

出てくる章: [3](part1/strings.md)

### 置換文字（replacement character） {/* #replacement-character */}

U+FFFD。
壊れたバイト列を文字列に直すときに、読めなかった部分の代わりに置く文字。

出てくる章: [3](part1/strings.md)

## C++ の用語

### RAII {/* #raii */}

Resource Acquisition Is Initialization。
資源の解放をオブジェクトのデストラクタに任せ、スコープを抜けたときに確実に解放させる書き方。

出てくる章: [1](part1/ownership.md)

### `std::unique_ptr` {/* #unique-ptr */}

指す先のオブジェクトを一つだけの持ち主として管理するスマートポインタ。
move すると、元の `unique_ptr` は `nullptr` を持つ有効なオブジェクトとして残る。

出てくる章: [1](part1/ownership.md)

### 右辺値参照（rvalue reference） {/* #rvalue-reference */}

型 `T` に対して `T&&` と書く参照。
一時オブジェクトや `std::move` した値に束縛でき、move の受け口になる。

出てくる章: [1](part1/ownership.md)

### コピーコンストラクタ（copy constructor） {/* #copy-constructor */}

同じ型のオブジェクトから複製を作る特別なメンバ関数。
書かなければ、多くの場合コンパイラが用意する。

出てくる章: [1](part1/ownership.md)

### move コンストラクタと move 代入演算子（move constructor / move assignment operator） {/* #move-constructor */}

右辺値参照を受け取り、移動元の資源を引き取るための特別なメンバ関数。
Rust にはこれに当たるものが無い。

出てくる章: [1](part1/ownership.md)

### 有効だが未規定の状態（valid but unspecified state） {/* #valid-but-unspecified */}

標準ライブラリの型を move したあとの、移動元のオブジェクトの状態。
壊れてはいないが、中身が何であるかは規格が定めていない。
型ごとに別の定めがある場合はそれに従う（`std::unique_ptr` は `nullptr` になる）。

出てくる章: [1](part1/ownership.md)

### `= delete`（deleted function） {/* #deleted-function */}

関数を、呼べないように宣言する書き方。
コピーコンストラクタに付けて、コピーを禁じるときによく使う。

出てくる章: [1](part1/ownership.md)

### `const T&` と `T&` {/* #cpp-references */}

C++ の const 参照と非 const 参照。
Rust の `&T` と `&mut T` に対応するが、コンパイラが保証する範囲が違う。

出てくる章: [2](part1/borrowing.md)

### 範囲 for（range-based for） {/* #range-for */}

`for (auto& x : container)` の形で、コンテナの要素を順に辿る構文。

出てくる章: [2](part1/borrowing.md)

### 反復子とその無効化（iterator / iterator invalidation） {/* #iterator-invalidation */}

反復子は、コンテナの要素を指して順に辿るためのオブジェクトである。
コンテナを書き換える操作のあとで、反復子や参照が使えなくなることを無効化と呼ぶ。
どの操作で何が無効になるかは、コンテナと操作ごとに規格が定めている。

出てくる章: [2](part1/borrowing.md)、[3](part1/strings.md)

### 再ハッシュ（rehash） {/* #rehash */}

ハッシュ表が要素を収める区画の数を増やし、要素を並べ直すこと。

出てくる章: [2](part1/borrowing.md)

### `std::string_view` {/* #string-view */}

C++17 で入った、文字列を所有せずに眺める型。
指している先の寿命を知らない。

出てくる章: [3](part1/strings.md)

### SSO（small string optimization） {/* #sso */}

短い文字列を、ヒープに確保せず `std::string` オブジェクトの中に直接しまう実装上の工夫。
規格の要求ではない。

出てくる章: [3](part1/strings.md)

### 一時オブジェクトの延命（temporary lifetime extension） {/* #lifetime-extension */}

一時オブジェクトを const 参照に束縛すると、参照の寿命まで一時オブジェクトが生き延びる規則。
Rust にも、`let` の右辺で直接 `&` を付けた一時的な値について似た規則がある。

出てくる章: [3](part1/strings.md)
