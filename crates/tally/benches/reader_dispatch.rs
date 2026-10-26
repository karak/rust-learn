//! `impl BufRead`（静的 dispatch）と `Box<dyn BufRead>`（動的 dispatch）の差を測る。
//!
//! **測る理由は `docs/learning-log.md` 4-11 にある。** そこには
//! 「`Box<dyn BufRead>` にすると `read_line` が行ごとに間接呼び出しになる。
//! このプログラムでは埋もれる程度だが、まとめるのに値段が付いているのが差分」と
//! 書いたが、**埋もれるかどうかを測っていなかった**（16 節の未解決の論点）。
//!
//! **結果は `docs/learning-log.md` に書く。このファイルに数値を書かない。**
//!
//! # 測り方の固定
//!
//! - **差を dispatch だけに絞る。** 同じ入力、同じ [`tally_reader`]、同じ
//!   [`Selector`] を通し、**reader の包み方だけ**を変える
//! - **2 通りの土台で測る。** `Cursor`（メモリ）と `BufReader<File>`（ファイル）。
//!   前者は **間接呼び出しの割合が最も大きくなる条件**で、上限を与える。
//!   後者が実際のプログラムの形である
//! - **ページキャッシュは温まった状態**（同じファイルを繰り返し読む）。
//!   冷えた状態は `divan` の反復の中では作れない
//! - **reader を `black_box` に通してから渡す。** 通さないと、`Box` を作った場所と
//!   使う場所が同じ関数にあるため、**LLVM が具体型を知って devirtualize しうる。**
//!   そうなると「動的 dispatch を測っているつもりで、何も測っていない」ことになる。
//!   実際のプログラムでは `Box` は分岐から来るので具体型は見えない
//!
//! # 予測
//!
//! **差は出ない。** 1 行あたりの仕事（`read_line` の中の走査とコピー、JSON の解釈、
//! ハッシュ挿入）に対して、間接呼び出し 1 回は桁が違う。
//! **出たとすれば、それは dispatch ではなくインライン展開の差である** —
//! `dyn` にすると `read_line` の本体が呼び出し側に展開されなくなる。

// **`tests/` と同じ理由で明示的に許可する。** `benches/` は `#[cfg(test)]` ではないので、
// `clippy.toml` の `allow-expect-in-tests` が効かない。
#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Cursor};
use std::path::PathBuf;
use std::sync::OnceLock;

use divan::Bencher;
use tally_core::{Counter, Key, Selector, tally_reader};

fn main() {
    divan::main();
}

/// 行数。**`aggregate` のベンチと揃える必要は無い。**
/// ここで見たいのは 1 行あたりの費用の差なので、反復が安定する程度にあれば足りる。
const LINES: usize = 50_000;

/// 入力を 1 度だけ作る。**測定のたびに作り直すと、生成が混ざる。**
struct Corpus {
    text: String,
    /// **`TempDir` を保持し続ける必要がある。** drop されるとファイルごと消える。
    dir: tempfile::TempDir,
}

impl Corpus {
    fn path(&self) -> PathBuf {
        self.dir.path().join("json.log")
    }
}

fn corpus() -> &'static Corpus {
    static CORPUS: OnceLock<Corpus> = OnceLock::new();
    CORPUS.get_or_init(|| {
        let mut text = String::new();
        for serial in 0..LINES {
            let level = ["info", "warn", "error", "debug"][serial % 4];
            writeln!(text, "{{\"lvl\":\"{level}\",\"seq\":{serial}}}")
                .expect("String への書き込みは失敗しない");
        }
        let dir = tempfile::tempdir().expect("一時ディレクトリを作れる");
        let corpus = Corpus { text, dir };
        fs::write(corpus.path(), &corpus.text).expect("書ける");
        corpus
    })
}

/// 1 本の reader を集計する。**ベンチの本体はこれだけ。**
fn run<R: BufRead>(reader: R) {
    let selector = Selector::new(Key::JsonField("lvl".to_owned()));
    let mut counter = Counter::new();
    tally_reader(&mut counter, reader, &selector, |_| true).expect("集計できる");
    divan::black_box(counter.report(None));
}

#[divan::bench(sample_count = 20)]
fn メモリ_静的(bencher: Bencher) {
    let text = &corpus().text;
    bencher.bench_local(|| run(divan::black_box(Cursor::new(text.as_bytes()))));
}

#[divan::bench(sample_count = 20)]
fn メモリ_動的(bencher: Bencher) {
    let text = &corpus().text;
    bencher.bench_local(|| {
        let reader: Box<dyn BufRead> = Box::new(Cursor::new(text.as_bytes()));
        run(divan::black_box(reader));
    });
}

#[divan::bench(sample_count = 20)]
fn ファイル_静的(bencher: Bencher) {
    let path = corpus().path();
    bencher.bench_local(|| {
        let file = File::open(&path).expect("開ける");
        run(divan::black_box(BufReader::new(file)));
    });
}

#[divan::bench(sample_count = 20)]
fn ファイル_動的(bencher: Bencher) {
    let path = corpus().path();
    bencher.bench_local(|| {
        let file = File::open(&path).expect("開ける");
        let reader: Box<dyn BufRead> = Box::new(BufReader::new(file));
        run(divan::black_box(reader));
    });
}
