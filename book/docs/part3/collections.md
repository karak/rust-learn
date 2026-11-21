---
title: 7 コレクションと順序
sidebar_position: 7
---

# 7 コレクションと順序

`tally` は、数えた結果を件数の多い順に出す。
件数が同じなら、キーの昇順に出す。
同じ入力を渡せば、何度走らせても同じバイト列が出る。

数えている途中の入れ物は [`HashMap`](../glossary.md#hashmap) で、こちらには順序が無い。
しかも Rust の `HashMap` は、反復する順序が実行のたびに変わりうる。
C++ の `std::unordered_map` に慣れていると、ここで足をすくわれる。

この章では、[`Vec`](../glossary.md#vec)・`HashMap`・`BTreeMap` を C++ の対応物と比べながら、出力の順序を自分で決める書き方を扱う。
III 部の最初の章である。

## この章のコードは、数えてから並べ直す

この章のコード（`book/code/ch07`）は、行を数えて `HashMap<String, u64>` に入れ、表示する前に 3 通りに並べ直す。

```rust
fn count(lines: &[&str]) -> HashMap<String, u64>
fn ranked(counts: &HashMap<String, u64>) -> Vec<(&str, u64)>
fn by_key(counts: &HashMap<String, u64>) -> BTreeMap<&str, u64>
fn ranked_by_count_only(counts: &HashMap<String, u64>) -> Vec<(&str, u64)>
```

`ranked` が本物の `tally` と同じ順序（件数の降順、同数ならキーの昇順）で並べる。
`by_key` はキーの順に並べる。
`ranked_by_count_only` はわざと件数しか見ない版で、比べるために置いてある。

`cargo run -p book-ch07` を 2 回走らせた結果が次である。

```text
HashMap:  {"info": 2, "error": 1, "warn": 2, "debug": 1}
BTreeMap: {"debug": 1, "error": 1, "info": 2, "warn": 2}
件数だけ: [("info", 2), ("warn", 2), ("error", 1), ("debug", 1)]
2 info
2 warn
1 debug
1 error
```

```text
HashMap:  {"warn": 2, "debug": 1, "error": 1, "info": 2}
BTreeMap: {"debug": 1, "error": 1, "info": 2, "warn": 2}
件数だけ: [("warn", 2), ("info", 2), ("debug", 1), ("error", 1)]
2 info
2 warn
1 debug
1 error
```

1 行目と 3 行目が変わり、2 行目と最後の 4 行は変わらなかった。
この章は、この違いがどこから来るかの話である。

## HashMap の反復順は、実行するたびに変わりうる

1 行目は、`HashMap` を `{:?}` でそのまま表示したものである。
手元で 30 回走らせると、4 つのキーの並びが 16 通り出た。

C++ で同じことを試すと、結果が違う。
`std::unordered_map<std::string, int>` に 6 つのキー（`warn`、`info`、`error`、`debug`、`trace`、`fatal`）を入れて反復するプログラムを手元（Apple clang 21、libc++）で 3 回走らせると、3 回とも同じ順序だった。
規格は反復の順序を約束していないので、これは手元の libc++ での振る舞いである。

Rust の `HashMap` が既定で使うハッシュ関数は、乱数の種を混ぜる。
種は `HashMap::new()` でマップを作るたびに変わる（`clone` で複製したマップは種を引き継ぐ）。
手元では、この 6 つのキーを同じ順に入れたマップを 1 つのプログラムの中で 2 つ作っても、反復の順序が食い違った。

```text
["warn", "info", "error", "debug", "trace", "fatal"]
["warn", "info", "error", "trace", "fatal", "debug"]
```

標準ライブラリの説明には、マップごとに違う種を使うと書いてあり、理由は HashDoS への備えだとしている。
ハッシュ関数が固定だと、衝突するキーを前もって計算した入力で、表の操作を遅くできる。
`tally` のキーは利用者の入力そのものなので、この備えが要る。
6 章で見た本物の `Counter` は、標準とは別のハッシュ関数（`foldhash`）を選んでいるが、こちらもマップごとに種を持つ。

順序が変わっても、中身が同じマップは `==` で等しいと判定される。
テストでマップを比べるなら、`{:?}` の文字列ではなくマップそのものを比べる。

```rust
assert_eq!(count(&["a", "b", "a"]), count(&["b", "a", "a"]));
```

## 同数の並びまで決めないと、順序が漏れる

3 行目の `ranked_by_count_only` は、件数の降順に並べている。
それでも、同数の `info` と `warn`、`debug` と `error` の並びが実行ごとに入れ替わった。

```rust
fn ranked_by_count_only(counts: &HashMap<String, u64>) -> Vec<(&str, u64)> {
    let mut out = Vec::with_capacity(counts.len());
    for (key, &n) in counts {
        out.push((key.as_str(), n));
    }
    out.sort_by(by_count_desc);
    out
}

fn by_count_desc(a: &(&str, u64), b: &(&str, u64)) -> Ordering {
    b.1.cmp(&a.1)
}
```

コードに出てくる `Vec::with_capacity` と `&n` は、次の節で説明する。
`sort_by` に渡すのは、2 つの要素を比べる関数である。
ここでは関数の名前 `by_count_desc` をそのまま渡した。
その場で関数を書いて渡すこともでき、その書き方（クロージャ）は 8 章で扱う。

比べる関数は `std::cmp::Ordering` を返す。
`Less`・`Equal`・`Greater` の 3 つの値を持つ [enum](../glossary.md#enum) で、`cmp` は 2 つの数を比べてこのどれかを返す。
`b.1.cmp(&a.1)` と左右を入れ替えて書いたので、件数の多いほうが前に来る。

件数が同じ 2 つを渡すと、`by_count_desc` は `Equal` を返す。
`sort_by` は [安定ソート](../glossary.md#stable-sort)なので、`Equal` の要素は並べる前の順序のまま残る。
並べる前の順序は、`HashMap` から取り出した順序である。
こうして、実行ごとに変わる `HashMap` の順序が、同数の範囲で出力に漏れた。

**直し方は、同数のときに何を比べるかまで決めて、どの 2 つの要素も前後が決まるようにすることである。**
`tally` の出力の仕様が「同数ならキーの昇順」まで書いているのは、このためである。

## タプルと Reverse で、比べ方を書かずに並べる

比べる関数を書かずに同じ順序を作ったのが `ranked` である。

```rust
fn ranked(counts: &HashMap<String, u64>) -> Vec<(&str, u64)> {
    let mut keyed = Vec::with_capacity(counts.len());
    for (key, &n) in counts {
        keyed.push((Reverse(n), key.as_str()));
    }
    keyed.sort();

    let mut out = Vec::with_capacity(keyed.len());
    for (Reverse(n), key) in keyed {
        out.push((key, n));
    }
    out
}
```

タプルは、前の要素から順に比べる。
前の要素が等しいときだけ、次の要素を比べる。
C++ の `std::tuple` や `std::pair` の `operator<` と同じ規則である。
だから `(件数, キー)` のタプルを `sort` すれば、件数で並び、同数ならキーで並ぶ。

件数だけを逆順にするために、`std::cmp::Reverse` で包んだ。
`Reverse` は、中の値の比べ方を逆にするだけの型で、6 章で見た [newtype](../glossary.md#newtype) の形をしている。
フィールドが公開されているので、`for (Reverse(n), key) in keyed` のように[パターン](../glossary.md#pattern)で中身を取り出せる。

`for (key, &n) in counts` の `&n` もパターンである。
[借りた](../glossary.md#borrow) `HashMap` を反復すると `(&String, &u64)` の組が得られ、`&n` と書くと `&u64` の[参照](../glossary.md#references)を[外した](../glossary.md#dereference) `u64` が `n` に入る。
`u64` は[複製](../glossary.md#copy-clone)できる型なので、取り出しても `counts` は変わらない。

`Vec::with_capacity` で作ると、要素数ぶんの領域をあらかじめ確保した空の `Vec` になる。
C++ の `reserve` に当たる。

キーは `HashMap` の中で重複しないので、`keyed` に等しい要素は 2 つと無い。
等しい要素が無ければ、安定な `sort` でも、安定でない `sort_unstable` でも、結果は同じになる。

キーの昇順は、文字列をバイト列として比べた順序である。
ASCII の大文字は小文字より前に来るので、`Warn` は `error` より前に並ぶ。
言語や地域の慣習に合わせた並べ方ではない。

## 並べられない型は、コンパイルの時点で断られる

`f64` の `Vec` は `sort` できない。
この章のコードの `main` に、一時的に次の 2 行を置いてみる。

```rust
let mut levels = vec![0.5_f64, -1.0];
levels.sort();
```

コンパイラはこう断る。
この節から先のエラーと panic は、どれも同じように一時的に置いて出したもので、引用はその一部を抜き出した。

```text
error[E0277]: the trait bound `f64: Ord` is not satisfied
   --> src/main.rs:85:12
    |
 85 |     levels.sort();
    |            ^^^^ the trait `Ord` is not implemented for `f64`
```

`Ord` は「どの 2 つの値も前後が決まる」ことを表す [trait](../glossary.md#trait) で、`sort` はそれを要素の型に要求する。
`f64` には NaN があり、NaN はどの数と比べても大小が決まらないので、`Ord` を満たさない。
trait と、それを要求する書き方は 9 章で扱う。

C++ の `std::sort` は、`double` の配列をそのまま受け取る。
規格は、比べ方が[狭義の弱順序](../glossary.md#strict-weak-ordering)であることを要求している。
その条件の 1 つは、`a < b` でも `b < a` でもない 2 つを「等しい」とみなしたとき、その関係が推移的であることである。
NaN は 1 とも 2 とも「等しい」ことになるが、1 と 2 は等しくないので、条件が崩れる。
満たさない比べ方を渡したときの動作を、規格は定めていない。
つまり[未定義動作](../glossary.md#undefined-behavior)になる。
手元（Apple clang 21、libc++）で `{3, NaN, 1, 2, NaN, 0.5, 5, 4}` を並べると、次のように出た。

```text
3 nan 1 2 nan 0.5 4 5
```

`3` が `1` より前に残っている。
それでいて、同じ配列に `std::is_sorted` を当てると、`-O0` でも `-O2` でも真が返った。
要素を 500 個に増やした別の入力では、並び損ねたうえに `is_sorted` も偽になった。
同じ比べ方でも、入力によって振る舞いが変わる。

Rust で `f64` を並べたいときは、どの 2 つの値にも前後を決める（全順序の）比べ方をする関数 `f64::total_cmp` を、`sort_by` に渡す。

```rust
v.sort_by(f64::total_cmp);
```

`total_cmp` は NaN にも位置を決める。
NaN は符号で両端に分かれ、負の NaN が先頭に、正の NaN が末尾に来る。
`-0.0` は `0.0` より前に来る。

`HashMap` のキーにも、`f64` は使えない。
キーには `Eq`（等しいかどうかが必ず決まる）と `Hash`（ハッシュ値を計算できる）が要り、`f64` はどちらも持たない。
型を書かずに `HashMap::new()` で作って `insert(0.5_f64, 1)` を呼ぶと、同じ[エラーコード](../glossary.md#error-code) `E0277` が `Eq` と `Hash` について 1 つずつ出る。
`HashMap<f64, u64>` と型を書いておくと、エラーは `E0599` が 1 つになり、満たしていない条件として `Eq` と `Hash` が挙がる。

## キーの順が欲しいなら BTreeMap を使う

2 行目の `BTreeMap` は、何度走らせても同じだった。
[`BTreeMap`](../glossary.md#btreemap) はキーの順序を保つ表で、C++ の `std::map` に当たる。
反復すると、キーの昇順に出てくる。

```rust
fn by_key(counts: &HashMap<String, u64>) -> BTreeMap<&str, u64> {
    let mut sorted = BTreeMap::new();
    for (key, &n) in counts {
        sorted.insert(key.as_str(), n);
    }
    sorted
}
```

キーは `counts` から借りた `&str` なので、戻り値を持っているあいだ `counts` は書き換えられない。
2 章でカウンタから借りた `Vec` を返したのと同じ関係である。

ただし `BTreeMap` が保つのはキーの順序で、件数の順序ではない。
`tally` の出力の順序は、どちらの表に入れても最後に並べ直すことになる。
標準ライブラリの説明によれば、1 回の探索にかかる手間は、`HashMap` が期待値で要素数によらず一定、`BTreeMap` が要素数の対数に比例する（`HashMap` の挿入は、ならしたうえでの一定である）。
`tally` は数えるあいだは `HashMap` を使い、出す直前に 1 回だけ並べる。

集合の `HashSet` と `BTreeSet` も、同じ関係にある。
`HashSet` の反復順も、実行ごとに変わりうる。

## 添字は、無ければ止まる。マップの添字には書き込めない

`Vec` の添字が範囲の外なら、プログラムはその場で [panic](../glossary.md#panic) する。
C++ の `std::vector::operator[]` で範囲の外を読むと未定義動作になり、範囲を確かめる `at()` は[例外](../glossary.md#exception)を投げる。
Rust の添字は、C++ の `at()` の側に倒してある。

```text
thread 'main' (15480587) panicked at src/main.rs:85:25:
index out of bounds: the len is 4 but the index is 10
```

括弧の中の数はスレッドの番号で、実行ごとに変わる。

無いかもしれない位置を読むなら、`get` を使う。
`get` は、要素への参照を 5 章の [`Option`](../glossary.md#option) に包んで返し、無ければ `None` を返す。

`HashMap` にも添字があり、`counts["warn"]` と書ける。
キーが無ければ、これも panic する。

```text
thread 'main' (15480107) panicked at src/main.rs:84:26:
no entry found for key
```

C++ の `std::unordered_map::operator[]` は、キーが無ければ値を既定の値で作って挿入する。
読むつもりで `if (m["warn"] > 0)` と書いただけで、`m.size()` が 0 から 1 に増える。
`const` なマップには `operator[]` が無く、コンパイルが通らない。

Rust では、マップの添字で書き換えることもできない。
`let mut counts = counts;` に続けて `counts["warn"] += 1;` と書くと、こうなる。

```text
error[E0594]: cannot assign to data in an index of `HashMap<String, u64>`
  --> src/main.rs:85:5
   |
85 |     counts["warn"] += 1;
   |     ^^^^^^^^^^^^^^^^^^^ cannot assign
   |
   = help: trait `IndexMut` is required to modify indexed content, but it is not implemented for `HashMap<String, u64>`
   = help: to modify a `HashMap<String, u64>`, use `.get_mut()`, `.insert()` or the entry API
```

読むのは添字か `get`、書くのは `get_mut` か `entry` か `insert`、と道が分かれている。

### entry は「あった」と「無かった」を enum で返す

エラーの最後の行が勧める entry API が、1 章から使ってきた `entry(...).or_insert(0)` の正体である。
`count` は、それを省略せずに書いている。

```rust
match counts.entry(line.to_owned()) {
    Entry::Occupied(mut seen) => *seen.get_mut() += 1,
    Entry::Vacant(new) => {
        new.insert(1);
    }
}
```

`entry` は、キーを 1 回だけ探し、結果を `std::collections::hash_map::Entry` という enum で返す。
6 章で見た `tally` の集計結果の 1 行 `Entry` とは、名前が同じだけの別の型である。
`Occupied` はキーがあった場合で、`get_mut` で値を書き換えられる。
`Vacant` は無かった場合で、`insert` で値を入れる。
4 章の[網羅性検査](../glossary.md#exhaustiveness)が効くので、片方の場合を書き忘れるとコンパイルが通らない。

この `match` を 1 行に縮めたのが `or_insert(0)` である。
無ければ 0 を入れ、どちらの場合も値への可変参照を返す。

C++ にも 1 回の探索で済む `try_emplace` があり、挿入したかどうかを `bool` で返す。
違いは、Rust では 2 つの場合が別々の型の値として返り、`match` で両方を書くことになる点にある。

`entry` はキーを値で受け取る。
そのため `count` は、既に数えたキーでも行ごとに `to_owned` で [`String`](../glossary.md#string-str) を作っている。
これを避ける書き方は、3 章の演習 1 で扱った。

上書きの規則も C++ と違う。
Rust の `insert` は、キーが既にあれば値を上書きし、前の値を `Some` に包んで返す。
C++ の `insert` と `try_emplace` は、既にある値を残す。
Rust の `insert` に当たるのは、C++ の `insert_or_assign` である。

## 本物ではどうなっているか

`tally-core` の `Counter` は、数えるあいだ `HashMap` に入れ、`report` で `Vec` に写して並べる。

```rust
/// 上位 `limit` 件を返す。`None` なら全件。
///
/// 件数の降順、同数ならキーの昇順。**同数時のタイブレークを決めておかないと
/// [`HashMap`] の反復順に依存して出力が実行ごとに変わり、
/// スナップショットテストが壊れる。**
#[must_use]
pub fn report(&self, limit: Option<usize>) -> Report {
    let mut entries: Vec<Entry> = self
        .counts
        .iter()
        .map(|(key, &count)| Entry {
            key: key.clone(),
            count,
        })
        .collect();

    entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));

    if let Some(limit) = limit {
        entries.truncate(limit);
    }

    Report {
        entries,
        skipped: self.skipped,
        total: self.total,
    }
}
```

`|a, b| ...` が、8 章で扱うクロージャである。
`sort_by` に渡す比べ方を、名前を付けずにその場で書いている。
`then_with` は、前の比較が `Equal` のときだけ次の比較を使う。
件数で比べ、同数ならキーで比べるという順序は、この章の `ranked` と同じである。
`.iter().map(...).collect()` は、この章で `for` と `push` で書いた詰め替えを 1 つの式にしたもので、クロージャと同じく 8 章で扱う。
`if let Some(limit) = limit` は、パターンに合ったときだけ中を実行する書き方で、`match` の腕が 1 つで足りるときに使う。
`truncate` は、`Vec` を先頭から指定した数だけ残して切り詰める。
抜粋では、`///` で始まる説明の中の使用例を省いた。

この順序は、利用者への約束として[出力の仕様](https://github.com/karak/rust-learn/blob/main/crates/tally/docs/output-format.md)に書いてある。

> **並び順は契約である。** `HashMap` の反復順に依存させず、同数時のタイブレークまで決めてあるので、同じ入力に対して常に同じバイト列が出る。

テストも、同数の場合を名指しで確かめている。

```rust
#[test]
fn 同数のときはキーの昇順で安定する() {
    // HashMap の反復順に依存していれば、この assert はいずれ落ちる。
    let report = tally_str("b\nc\na\n", &Selector::new(Key::WholeLine));
    let keys: Vec<&str> = report.entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["a", "b", "c"]);
}
```

テストの中の `tally_str` は、文字列を 1 つ集計して `Report` を返す補助関数である。
コメントが「いずれ落ちる」と書いているのは、出力例の「件数だけ」の行で見たとおり、同数の組がたまたま正しい順に並ぶ実行もあるからである。

## 演習

答えは [`book/code/ch07/src/exercises.rs`](https://github.com/karak/rust-learn/blob/main/book/code/ch07/src/exercises.rs) にある。
`cargo test -p book-ch07` で確かめられる。

**演習 1**

キーごとに、最初に現れた行の番号（1 から数える）を返す関数を書け。

```rust
fn first_seen(lines: &[&str]) -> HashMap<String, usize>
```

`["b", "a", "b"]` なら、`b` は 1、`a` は 2 である。
書けたら、`entry` の代わりに `insert` で書いた場合に何が残るかを考えてほしい。

**演習 2**

件数の昇順、同数ならキーの降順に並べる関数を書け。
比べる関数は書かず、タプルと `Reverse` だけで済ませること。

```rust
fn ranked_ascending(counts: &HashMap<String, u64>) -> Vec<(&str, u64)>
```
