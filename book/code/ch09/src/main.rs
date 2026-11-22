//! 9 章「trait と境界、単相化と `dyn`」の実例。
//!
//! 状態コードとログの重さという別々の型を、同じ関数で数える。
//! 本物で同じ形をしているのは `crates/tally/src/input.rs` の `Input` と `tally_input`。
//!
//! 1 章から 8 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::collections::BTreeMap;
use std::fmt;
use std::num::ParseIntError;

/// 値を、どの欄に数えるか。
trait Bucket {
    /// 数える欄の名前。
    fn bucket(&self) -> String;

    /// 失敗として数えるか。
    ///
    /// 本体を書いておくと、実装する側は書かなくてよい（既定の実装）。
    fn is_error(&self) -> bool {
        false
    }
}

/// HTTP の状態コードは、百の位で欄を分ける。
///
/// `u16` は標準ライブラリの型だが、`Bucket` はこのクレートの trait なので実装できる。
impl Bucket for u16 {
    fn bucket(&self) -> String {
        format!("{}xx", self / 100)
    }

    fn is_error(&self) -> bool {
        *self >= 400
    }
}

/// ログの重さ。
///
/// `PartialOrd` と `Ord` を derive すると、判別値を明示しない限り、バリアントを書いた順に大小が決まる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        };
        f.write_str(name)
    }
}

/// `is_error` だけを上書きし、欄の名前は `Display` に任せる。
impl Bucket for Level {
    fn bucket(&self) -> String {
        self.to_string()
    }

    fn is_error(&self) -> bool {
        *self == Level::Error
    }
}

/// 欄ごとに数える。
///
/// `T: Bucket` が、`item.bucket()` を呼べる根拠になる。
/// 使われた `T` ごとに、別の関数としてコードが作られる。
fn tally<T: Bucket>(items: &[T]) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for item in items {
        *out.entry(item.bucket()).or_insert(0) += 1;
    }
    out
}

/// 失敗として数える値の個数。
fn errors<T>(items: &[T]) -> usize
where
    T: Bucket,
{
    items.iter().filter(|item| item.is_error()).count()
}

/// `keep` が真を返した行を数える。
///
/// `keep` は 1 行ごとに順に呼ぶだけなので、求めるのは `FnMut` で足りる。
/// `Fn` を求めると、中で変数を書き換えるクロージャを渡せなくなる。
fn count_if<F: FnMut(&str) -> bool>(lines: &[&str], mut keep: F) -> usize {
    let mut n = 0;
    for &line in lines {
        if keep(line) {
            n += 1;
        }
    }
    n
}

/// `min` より長い行を選ぶクロージャを返す。
///
/// クロージャの型には名前が無いので、戻り値は「`Fn(&str) -> bool` を満たす何か」と書く。
fn longer_than(min: usize) -> impl Fn(&str) -> bool {
    move |line| line.len() > min
}

/// 型の違う値を並べて、欄の名前を取り出す。
///
/// `&dyn Bucket` は、どの型の `bucket` を呼ぶかを実行時に表から引く。
fn buckets(items: &[&dyn Bucket]) -> Vec<String> {
    items.iter().map(|item| item.bucket()).collect()
}

/// 状態コードを読む失敗。
#[derive(Debug, PartialEq)]
enum CountError {
    Empty,
    BadNumber(ParseIntError),
}

/// これがあると、`?` が `ParseIntError` を `CountError` に包み直す。
impl From<ParseIntError> for CountError {
    fn from(source: ParseIntError) -> Self {
        CountError::BadNumber(source)
    }
}

/// 状態コードの列を読む。
fn parse_codes(fields: &[&str]) -> Result<Vec<u16>, CountError> {
    if fields.is_empty() {
        return Err(CountError::Empty);
    }
    let mut codes = Vec::new();
    for field in fields {
        codes.push(field.parse::<u16>()?);
    }
    Ok(codes)
}

fn main() {
    let codes = [200_u16, 404, 200, 503, 301];
    let levels = [
        Level::Warn,
        Level::Info,
        Level::Error,
        Level::Debug,
        Level::Info,
    ];

    println!("{:?}", tally(&codes));
    println!("{:?}", tally(&levels));
    println!("失敗: {} と {}", errors(&codes), errors(&levels));

    let mut sorted = levels;
    sorted.sort();
    println!("{sorted:?}");

    let lines = ["warn", "", "info", "error", ""];
    let mut empty = 0;
    let kept = count_if(&lines, |line| {
        if line.is_empty() {
            empty += 1;
        }
        !line.is_empty()
    });
    println!("残した {kept} 行、空行 {empty} 行");
    println!("4 文字より長い行: {}", count_if(&lines, longer_than(4)));

    println!("{:?}", buckets(&[&404_u16, &Level::Warn]));
    println!(
        "&u16: {} バイト、&dyn Bucket: {} バイト",
        size_of::<&u16>(),
        size_of::<&dyn Bucket>()
    );

    println!("{:?}", parse_codes(&["200", "404"]));
    println!("{:?}", parse_codes(&["200", "x"]));
}

#[cfg(test)]
mod tests {
    use super::{
        Bucket, CountError, Level, buckets, count_if, errors, longer_than, parse_codes, tally,
    };
    use std::collections::BTreeMap;

    #[test]
    fn 同じ関数で別の型を数えられる() {
        let by_code = tally(&[200_u16, 404, 200]);
        assert_eq!(
            by_code,
            BTreeMap::from([("2xx".to_owned(), 2), ("4xx".to_owned(), 1)])
        );
        let by_level = tally(&[Level::Info, Level::Info]);
        assert_eq!(by_level, BTreeMap::from([("info".to_owned(), 2)]));
        assert!(tally::<u16>(&[]).is_empty());
    }

    #[test]
    fn 上書きしなければ既定の実装が使われる() {
        struct Plain;
        impl Bucket for Plain {
            fn bucket(&self) -> String {
                "plain".to_owned()
            }
        }
        assert!(!Plain.is_error());
        assert_eq!(errors(&[Plain, Plain]), 0);
        assert_eq!(errors(&[200_u16, 404, 503]), 2);
    }

    #[test]
    fn derive_した_ord_はバリアントを書いた順になる() {
        let mut levels = [Level::Error, Level::Debug, Level::Warn, Level::Info];
        levels.sort();
        assert_eq!(
            levels,
            [Level::Debug, Level::Info, Level::Warn, Level::Error]
        );
        assert!(Level::Warn < Level::Error);
    }

    #[test]
    fn fn_mut_を求めれば書き換えるクロージャを渡せる() {
        let mut rejected = Vec::new();
        let kept = count_if(&["a", "", "b"], |line| {
            if line.is_empty() {
                rejected.push(line.len());
            }
            !line.is_empty()
        });
        assert_eq!((kept, rejected.len()), (2, 1));
        assert_eq!(count_if(&[], |_| true), 0);
    }

    #[test]
    fn 返したクロージャは引数を閉じ込めている() {
        let long = longer_than(3);
        assert!(long("warn"));
        assert!(!long("abc"));
        assert_eq!(count_if(&["warn", "info", "e"], longer_than(0)), 3);
    }

    #[test]
    fn dyn_なら型の違う値を同じ列に並べられる() {
        assert_eq!(
            buckets(&[&404_u16, &Level::Warn, &200_u16]),
            vec!["4xx", "warn", "2xx"]
        );
        assert!(buckets(&[]).is_empty());
    }

    #[test]
    fn dyn_への参照はポインタ2つぶんの大きさになる() {
        assert_eq!(size_of::<&u16>(), size_of::<usize>());
        assert_eq!(size_of::<&dyn Bucket>(), 2 * size_of::<usize>());
    }

    #[test]
    fn from_があれば_question_が失敗を包み直す() {
        assert_eq!(parse_codes(&["200", "404"]), Ok(vec![200, 404]));
        assert_eq!(parse_codes(&[]), Err(CountError::Empty));
        let Err(CountError::BadNumber(source)) = parse_codes(&["200", "x"]) else {
            panic!("BadNumber になるはず");
        };
        assert_eq!(source, "x".parse::<u16>().unwrap_err());
        // `u16` に収まらない数も、同じ経路で包まれる。
        assert!(matches!(
            parse_codes(&["70000"]),
            Err(CountError::BadNumber(_))
        ));
    }
}
