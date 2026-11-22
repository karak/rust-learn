---
title: 用語集
---

# 用語集

本文に出てくる用語を、英語での呼び名と一緒に引けるようにしたものである。
説明は短くしてあり、詳しい話は「出てくる章」に置いてある。

各章からは、その章で用語を説明している箇所から、説明が無ければ地の文で初めて出る箇所からここへリンクしている。
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

出てくる章: [1](part1/ownership.md)、[4](part2/enums.md)、[6](part2/structs.md)、[8](part3/iterators.md)

### 部分 move（partial move） {/* #partial-move */}

構造体のフィールドの一部だけを move すること。
残りのフィールドは読めるが、構造体全体はもう渡せない。

出てくる章: [1](part1/ownership.md)

### `Copy` と `Clone` {/* #copy-clone */}

`Copy` は代入のときに暗黙に複製される型の性質で、`Clone` は `.clone()` と書いたときだけ複製できる型の性質。
どちらも trait である。

出てくる章: [1](part1/ownership.md)、[4](part2/enums.md)、[7](part3/collections.md)、[8](part3/iterators.md)

### drop {/* #drop */}

値の所有者がスコープを抜けたときに、その値の後始末を走らせて解放すること。
C++ のデストラクタの呼び出しに当たる。
move された変数については走らない（値が移った先で 1 回だけ走る）。

出てくる章: [1](part1/ownership.md)（デストラクタとして）、3（エラーメッセージの中）

### 借用（borrow） {/* #borrow */}

値を所有したまま、参照を作って貸すこと。

出てくる章: [1](part1/ownership.md)、[2](part1/borrowing.md)、[3](part1/strings.md)、[4](part2/enums.md)、[6](part2/structs.md)、[7](part3/collections.md)、[8](part3/iterators.md)

### 共有参照と可変参照（shared reference / mutable reference） {/* #references */}

`&T` が共有参照、`&mut T` が可変参照。
共有参照は同時にいくつでも作れるが、可変参照は同時に一つだけで、そのあいだは共有参照も作れない。
この性質から、可変参照は排他参照（exclusive reference）とも呼ばれる。

出てくる章: [2](part1/borrowing.md)、[6](part2/structs.md)、[7](part3/collections.md)、[8](part3/iterators.md)

### 内部可変性（interior mutability） {/* #interior-mutability */}

共有参照 `&T` しか持っていなくても中身を書き換えられるようにする仕組み。
書き換えが重ならないことは、型の側が受け持つ。

出てくる章: [2](part1/borrowing.md)、[6](part2/structs.md)

### 寿命（lifetime） {/* #lifetime */}

値や参照が有効でいられる期間。
Rust では、参照が指す先より長く生きないことをコンパイラが確かめる。

出てくる章: [3](part1/strings.md)

### `String` と `&str` {/* #string-str */}

`String` はヒープ上の UTF-8 のバイト列を所有する文字列で、`&str` はどこかにある UTF-8 のバイト列を眺める参照。

出てくる章: [1](part1/ownership.md)、[2](part1/borrowing.md)、[3](part1/strings.md)、[4](part2/enums.md)、[7](part3/collections.md)

### 文字列リテラル（string literal） {/* #string-literal */}

`"warn"` のようにコードに直接書いた文字列。
型は `&str` で、プログラムに埋め込まれたバイト列を指す。

出てくる章: [3](part1/strings.md)

### スライス（slice） {/* #slice */}

連続した要素の並びの全体または一部を指す参照で、`&[T]` と書く。
先頭へのポインタと長さを持つ。
`&str` は文字列のスライスである。

出てくる章: [3](part1/strings.md)、[5](part2/result.md)

### 文字の境界（char boundary） {/* #char-boundary */}

UTF-8 のバイト列のうち、1 文字の先頭に当たる位置（と末尾）。
`&str` を範囲で切るときは、範囲の両端がこの位置でなければならない。

出てくる章: [3](part1/strings.md)

### `Vec<T>` {/* #vec */}

要素をヒープ上に連続して並べる、長さを変えられる配列。
C++ の `std::vector` に当たる。

出てくる章: [2](part1/borrowing.md)、[3](part1/strings.md)、[7](part3/collections.md)、[8](part3/iterators.md)

### `HashMap` {/* #hashmap */}

キーから値を引くハッシュ表。
C++ の `std::unordered_map` に当たる。

出てくる章: [1](part1/ownership.md)、[3](part1/strings.md)、[7](part3/collections.md)、[8](part3/iterators.md)

### `BTreeMap` {/* #btreemap */}

キーの順序を保つ表。反復するとキーの昇順に出てくる。
C++ の `std::map` に当たる。

出てくる章: [7](part3/collections.md)、[8](part3/iterators.md)

### `Option` {/* #option */}

値が有るか無いかを表す型。
有れば `Some(値)`、無ければ `None` になる。

出てくる章: [3](part1/strings.md)、[4](part2/enums.md)、[5](part2/result.md)、[7](part3/collections.md)、[8](part3/iterators.md)

### `Result` {/* #result */}

成功の値か失敗の値のどちらかを持つ型。
成功なら `Ok(値)`、失敗なら `Err(エラー)` になる。

出てくる章: [2](part1/borrowing.md)、[5](part2/result.md)、[6](part2/structs.md)、[8](part3/iterators.md)

### enum {/* #enum */}

取りうる場合（バリアント）を並べて定義する型。
バリアントごとに別の値を持たせられる。

出てくる章: [4](part2/enums.md)、[5](part2/result.md)、[6](part2/structs.md)、[7](part3/collections.md)、[8](part3/iterators.md)

### パターン（pattern） {/* #pattern */}

値の形を書いて照らし合わせるための記法。
`match` の腕の左辺に書き、合ったときに中身へ名前を付けられる。

出てくる章: [4](part2/enums.md)、[5](part2/result.md)、[6](part2/structs.md)、[7](part3/collections.md)

### 網羅性検査（exhaustiveness checking） {/* #exhaustiveness */}

`match` がすべての場合を扱っているかを、コンパイラが確かめること。
漏れがあればコンパイルエラーになる。

出てくる章: [4](part2/enums.md)、[5](part2/result.md)、[7](part3/collections.md)

### `?` 演算子（question mark operator） {/* #question-mark */}

`Result` や `Option` の後ろに付け、成功なら中身を取り出し、失敗や `None` ならその場で呼び手へ返す演算子。

出てくる章: [5](part2/result.md)、[9](part3/traits.md)

### ユニット型（unit type） {/* #unit-type */}

`()` と書く、値を一つしか持たない型。
返すものが無いことを表す。

出てくる章: [5](part2/result.md)

### 型強制（coercion） {/* #coercion */}

ある型の値を、コンパイラが暗黙に別の型として扱うこと。
Rust では起きる場所と種類が限られている。

出てくる章: [3](part1/strings.md)、[9](part3/traits.md)

### 参照外し型強制（deref coercion） {/* #deref-coercion */}

型強制の一種。
`&String` を `&str` として渡せるように、行き先の型が決まっている場所でコンパイラが参照の型を変換すること。

出てくる章: [3](part1/strings.md)

### イテレータ（iterator） {/* #iterator */}

`next` を呼ぶたびに次の要素を `Some` に包んで返し、尽きたら `None` を返す値。
`map` や `filter` で包んでも、その処理は `next` が呼ばれたときに初めて走る。
C++ の反復子と違い、終わりを自分で知っている。

出てくる章: [8](part3/iterators.md)、[9](part3/traits.md)

### クロージャ（closure） {/* #closure */}

周りの変数を閉じ込めた、名前の無い関数。`|引数| 本体` と書く。
変数（やそのフィールド）ごとに、共有で借りるか、可変で借りるか、値で受け取るかを、本体での使い方からコンパイラが決める。
`move` を付けると、すべて値で受け取る。

出てくる章: [8](part3/iterators.md)、[9](part3/traits.md)

### `Fn` と `FnMut` と `FnOnce` {/* #fn-traits */}

クロージャを、呼べる回数と条件で分けた 3 つの trait。
`Fn` は共有のまま何度でも、`FnMut` は中身を書き換えながら何度でも、`FnOnce` は少なくとも 1 回呼べる。

出てくる章: [8](part3/iterators.md)、[9](part3/traits.md)

### panic {/* #panic */}

プログラムが誤りに当たったときに、実行を打ち切る仕組み。
既定では、スタックを巻き戻しながら値の後始末を走らせ、そのスレッドを終える。
main スレッドで起きれば、プログラム全体が終わる。

出てくる章: [3](part1/strings.md)、[5](part2/result.md)、[7](part3/collections.md)

### `unsafe` {/* #unsafe */}

コンパイラが安全性を確かめられない操作を、書く人の責任で許すための印。

出てくる章: [3](part1/strings.md)、[4](part2/enums.md)

### trait {/* #trait */}

型が持つ操作や性質をまとめて宣言するもの。

出てくる章: [はじめに](intro.md)、[5](part2/result.md)、[6](part2/structs.md)、[7](part3/collections.md)、[9](part3/traits.md)、3、4（エラーメッセージの中）

### 型引数（type parameter） {/* #type-parameter */}

`fn tally<T>(...)` や `Option<T>` の `T` のように、関数や型があとから受け取る型。
使われた型ごとに、別々のコードが作られる。

出てくる章: [8](part3/iterators.md)、[9](part3/traits.md)

### 境界（trait bound） {/* #trait-bound */}

型引数に入る型へ、実装していなければならない trait を求める条件。
`T: Ord` や `where T: Ord` と書く。
関数の本体で使える `T` の操作は、境界に書いた trait が持つものに限られる。

出てくる章: [9](part3/traits.md)

### 孤児ルール（orphan rule） {/* #orphan-rule */}

trait を型に実装するとき、trait か型の少なくとも一方が、実装を書くクレートのものでなければならないという規則。
同じ実装を 2 つ書くことを断る規則と合わせて、ある型にある trait の実装は、プログラム全体で多くても 1 つに決まる（コヒーレンス）。

出てくる章: [9](part3/traits.md)

### `impl Trait` {/* #impl-trait */}

引数や戻り値の型の位置に書く、「その trait を実装した、ある 1 つの型」。
クロージャのように名前の無い型を返すときに使う。
`dyn` と違い、実際の型はコンパイル時に決まっている。

出てくる章: [9](part3/traits.md)

### `dyn`（trait object） {/* #dyn */}

`&dyn Bucket` や `Box<dyn Bucket>` のように書き、trait を実装した何かの型の値を、型を消して扱う。
メソッドの呼び先は、値と一緒に持つ表（vtable）から実行時に引く。
`Self` を返すメソッドのように、表に載せられないメソッドを持つ trait は、そのままでは `dyn` にできない。

出てくる章: [9](part3/traits.md)

### derive {/* #derive */}

`#[derive(Debug, Clone)]` のように書き、trait の実装をコンパイラやライブラリに書かせる属性。
derive した実装は、フィールドの型の実装を使う。

出てくる章: [6](part2/structs.md)、[9](part3/traits.md)

### 関連型（associated type） {/* #associated-type */}

trait の中で `type Reader;` のように宣言し、実装ごとに 1 つに決める型。

出てくる章: [9](part3/traits.md)

### クレート（crate） {/* #crate */}

Rust のコンパイルの単位。
ライブラリ一つ、または実行ファイル一つに当たる。

出てくる章: [4](part2/enums.md)、[6](part2/structs.md)、[9](part3/traits.md)

### 属性（attribute） {/* #attribute */}

`#[...]` の形で項目に付ける、コンパイラへの指示や注記。

出てくる章: [4](part2/enums.md)、[6](part2/structs.md)

### 構造体（struct） {/* #struct */}

名前の付いたフィールドを並べた型。
値を作るときにフィールドを書き漏らすと、コンパイルが通らない。
フィールドに名前を付けず位置で並べる形（タプル構造体）もあり、中身は `.0`、`.1` で読む。

出てくる章: [6](part2/structs.md)

### メソッドと関連関数（method / associated function） {/* #method */}

`impl` の中に書く関数のうち、第 1 引数に `self`、`&self`、`&mut self` のどれかを取り、`値.名前(...)` で呼ぶもの。
`self` を取らず `型::名前(...)` で呼ぶものは関連関数と呼び、C++ の static メンバ関数に当たる。

出てくる章: [6](part2/structs.md)

### モジュール（module） {/* #module */}

名前をまとめる入れ物で、可視性の単位。
非公開の項目は、定義したモジュールとその子孫からだけ見える。
`mod 名前;` と書くと、別のファイル（たとえば `src/名前.rs`）の中身がそのモジュールになる。

出てくる章: [6](part2/structs.md)

### newtype {/* #newtype */}

既存の型を、フィールド 1 つのタプル構造体で包んで作る新しい型。
フィールドを非公開にして、作る関数で条件を確かめれば、モジュールの外では条件を満たす値しか作れなくなる。

出てくる章: [6](part2/structs.md)、[7](part3/collections.md)、[9](part3/traits.md)

### エラーコード（error code） {/* #error-code */}

コンパイルエラーに付く `E0382` のような番号。
`rustc --explain E0382` で、そのエラーの説明と例を読める。

出てくる章: 1、2、3、4（エラーメッセージの中）、[7](part3/collections.md)、[8](part3/iterators.md)、[9](part3/traits.md)。[rustc](#rustc) も参照

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

出てくる章: [3](part1/strings.md)、[4](part2/enums.md)、[5](part2/result.md)、[6](part2/structs.md)

### AddressSanitizer {/* #asan */}

C や C++ のプログラムに検査のコードを埋め込み、解放済みの領域やスコープを抜けた変数を読んだことを実行時に報告する道具。
clang と gcc では `-fsanitize=address` で有効にする。

出てくる章: [8](part3/iterators.md)

### rustup {/* #rustup */}

Rust のツールチェーン（コンパイラや `cargo` の版）を入れて切り替える道具。

出てくる章: [はじめに](intro.md)

## プログラミング言語の用語

### 例外（exception） {/* #exception */}

失敗を戻り値とは別の道で伝え、受け止める場所まで呼び出しをさかのぼらせる仕組み。
C++、Java、C# などが持つ。

出てくる章: [5](part2/result.md)、[6](part2/structs.md)、[7](part3/collections.md)

### 検査例外（checked exception） {/* #checked-exception */}

Java で、投げうることを `throws` として宣言に書かなければならない例外。
受け止めも宣言もせずに呼ぶとコンパイルエラーになる。

出てくる章: [5](part2/result.md)

### 参照外し（dereference） {/* #dereference */}

参照やポインタが指している先の値を読み書きすること。
Rust でも `*` と書く。

出てくる章: 1、2（コードの中）、[3](part1/strings.md)、[7](part3/collections.md)

### スコープ（scope） {/* #scope */}

変数の名前が有効な範囲。
Rust では、所有者の変数がスコープを抜けると値が解放される。

出てくる章: [1](part1/ownership.md)、[2](part1/borrowing.md)、[3](part1/strings.md)、[8](part3/iterators.md)

### ヒープ（heap） {/* #heap */}

実行中に大きさを決めて確保し、使い終わったら解放するメモリの領域。

出てくる章: [1](part1/ownership.md)、[3](part1/strings.md)、[8](part3/iterators.md)、[9](part3/traits.md)

### ファットポインタ（fat pointer） {/* #fat-pointer */}

アドレスに加えて、長さなどの情報を一緒に持つポインタ。
wide pointer とも呼ばれる。
3 章で「二語の値」と呼んだ `&str` やスライスがこれに当たる。

出てくる章: [3](part1/strings.md)、[9](part3/traits.md)

### スタックの巻き戻し（stack unwinding） {/* #stack-unwinding */}

例外や panic が起きたときに、呼び出しを逆にたどりながら、各関数が持っていた値の後始末を走らせること。

出てくる章: [3](part1/strings.md)

### 未定義動作（undefined behavior） {/* #undefined-behavior */}

言語の規格が結果を何も定めていない動作。
動いているように見えても、何が起きても文句は言えない。

出てくる章: [2](part1/borrowing.md)、[3](part1/strings.md)、[4](part2/enums.md)、[5](part2/result.md)、[7](part3/collections.md)、[8](part3/iterators.md)

### 安定ソート（stable sort） {/* #stable-sort */}

等しいと判定された要素どうしの前後を、並べる前のまま保つ並べ方。

出てくる章: [7](part3/collections.md)

### 単相化（monomorphization） {/* #monomorphization */}

型引数を持つ関数や型を、使われた型ごとに別々のコードとして生成すること。

出てくる章: [8](part3/iterators.md)、[9](part3/traits.md)

### vtable {/* #vtable */}

ある型が実装した trait のメソッドのアドレスを並べた表。
Rust では、`dyn` を書いたときだけ、呼び出しがこの表を引く。

出てくる章: [9](part3/traits.md)

### 型消去（type erasure） {/* #type-erasure */}

総称型の型引数を、コンパイルの後に消すこと。
Java の総称型がこの形で、`List<String>` と `List<Integer>` は実行時には同じクラスになる。

出てくる章: [9](part3/traits.md)

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

出てくる章: [1](part1/ownership.md)、[6](part2/structs.md)

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

出てくる章: [2](part1/borrowing.md)、[3](part1/strings.md)、[8](part3/iterators.md)

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

### `enum class`（scoped enumeration） {/* #enum-class */}

C++11 で入った列挙型。
基底型が決まっているので、基底型の値なら列挙子に無い値も持てる。

出てくる章: [4](part2/enums.md)

### `std::variant` {/* #std-variant */}

C++17 で入った、決めておいた型のどれか一つの値を持つ型。
どの型の値を持っているかを覚えていて、`std::visit` で場合分けできる。

出てくる章: [4](part2/enums.md)

### `std::optional` {/* #std-optional */}

C++17 で入った、値が有るか無いかを表す型。
空のまま `*` や `->` で中身を読むと未定義動作になる。

出てくる章: [5](part2/result.md)、[6](part2/structs.md)

### `std::expected` {/* #std-expected */}

C++23 で入った、成功の値か失敗の値のどちらかを持つ型。

出てくる章: [5](part2/result.md)、[6](part2/structs.md)

### `friend` {/* #friend */}

クラスの中で宣言して、外の関数やクラスに `private` のメンバを見せる仕組み。

出てくる章: [6](part2/structs.md)

### 参照修飾子（ref-qualifier） {/* #ref-qualifier */}

メンバ関数の後ろに付ける `&` や `&&`。
`&&` を付けたメンバ関数は、右辺値（`std::move` した値や一時オブジェクト）に対してだけ呼べる。

出てくる章: [6](part2/structs.md)

### 狭義の弱順序（strict weak ordering） {/* #strict-weak-ordering */}

`std::sort` などが比べ方に要求する条件。
`a < b` でも `b < a` でもない 2 つを等しいとみなしたとき、その関係が推移的であることを含む。
NaN を含む `double` の `<` はこれを満たさない。

出てくる章: [7](part3/collections.md)

### ラムダ式（lambda expression） {/* #cpp-lambda */}

C++ で、その場で書く名前の無い関数。
閉じ込める変数とその方法を、`[&]`・`[=]`・`[&x, y]` のようなキャプチャリストで書く。

出てくる章: [8](part3/iterators.md)

### `std::function` {/* #std-function */}

呼び出せるものなら何でも、同じ型に包んで持てる C++ の型。

出てくる章: [8](part3/iterators.md)
