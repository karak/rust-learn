//! 集計器と、数えるフィールドの名前。
//!
//! このモジュールは `lib.rs` から `pub use` されるものだけを外に出す。
//! フィールドはすべて非公開で、外からはメソッドを通してしか触れない。

use std::collections::HashMap;

/// 数えるフィールドの名前。
///
/// 空でなく、空白と `=` を含まない。
/// `FieldName::new` を通らずに作る道が無いので、この型の値はいつもこの条件を満たす。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldName(String);

/// `FieldName::new` が名前を受け付けなかった理由。
#[derive(Debug, PartialEq, Eq)]
pub enum FieldNameError {
    /// 空だった。
    Empty,
    /// 使えない文字を含んでいた。
    ///
    /// 行は空白で区切った `名前=値` の並びで、語が名前で始まり、そのすぐ後ろが `=` なら、残りを値とみなす。
    /// 空白を含む名前は、どの行とも一致しない。
    /// `=` を含む名前は、`a=b=c` を「`a` の値 `b=c`」とも「`a=b` の値 `c`」とも読ませてしまう。
    Forbidden(char),
}

impl FieldName {
    /// 名前を検査してから作る。
    pub fn new(raw: &str) -> Result<Self, FieldNameError> {
        if raw.is_empty() {
            return Err(FieldNameError::Empty);
        }
        for c in raw.chars() {
            if c == '=' || c.is_whitespace() {
                return Err(FieldNameError::Forbidden(c));
            }
        }
        Ok(Self(raw.to_owned()))
    }

    /// 名前を文字列として返す。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `strict` のとき、フィールドを持たない行があった。
#[derive(Debug, PartialEq, Eq)]
pub struct MissingField {
    /// 何行目か。1 から数える。
    pub line_no: usize,
}

/// 集計を終えたあとの結果。
///
/// 読み手が中身を直接見られるよう、フィールドはすべて公開する。
/// `#[non_exhaustive]` は、あとでフィールドを足しても、このクレートの外のコードが壊れないようにするためである。
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Report {
    /// 値ごとの出現回数。
    pub counts: HashMap<String, u64>,
    /// フィールドを持たずに飛ばした行の数。
    pub skipped: usize,
}

/// 行を 1 つずつ受け取って、フィールドの値を数える。
///
/// `lines` は、`counts` に数えた回数の合計と `skipped` の和にいつも等しい。
/// フィールドを公開しないのは、この等しさをモジュールの外から崩させないためである。
#[derive(Debug)]
pub struct Tally {
    field: FieldName,
    strict: bool,
    counts: HashMap<String, u64>,
    skipped: usize,
    lines: usize,
}

impl Tally {
    /// 空の集計器を作る。既定では、フィールドを持たない行を飛ばす。
    #[must_use]
    pub fn new(field: FieldName) -> Self {
        Self {
            field,
            strict: false,
            counts: HashMap::new(),
            skipped: 0,
            lines: 0,
        }
    }

    /// フィールドを持たない行で失敗するかどうかを決める。
    ///
    /// `self` を値で受け取って返すので、`Tally::new(f).strict(true)` と続けて書ける。
    #[must_use]
    pub fn strict(mut self, yes: bool) -> Self {
        self.strict = yes;
        self
    }

    /// 1 行を数える。
    ///
    /// 失敗したときは何も変えない。失敗した行は `lines` にも数えない。
    pub fn push_line(&mut self, line: &str) -> Result<(), MissingField> {
        let line_no = self.lines + 1;
        match find_field(line, &self.field) {
            Some(value) => *self.counts.entry(value).or_insert(0) += 1,
            None if self.strict => return Err(MissingField { line_no }),
            None => self.skipped += 1,
        }
        self.lines = line_no;
        Ok(())
    }

    /// 数えているフィールドの名前を返す。
    #[must_use]
    pub fn field(&self) -> &FieldName {
        &self.field
    }

    /// ある値がここまでに何回出たかを返す。
    #[must_use]
    pub fn count_of(&self, value: &str) -> u64 {
        self.counts.get(value).copied().unwrap_or(0)
    }

    /// 集計を終えて結果を取り出す。
    ///
    /// `self` を値で受け取るので、`counts` を複製せずに `Report` へ移せる。
    /// 呼んだあとの `Tally` は使えない。
    #[must_use]
    pub fn finish(self) -> Report {
        Report {
            counts: self.counts,
            skipped: self.skipped,
        }
    }
}

/// 1 行から `name=値` の値を探す。
///
/// メソッドではない、ただの関数である。
/// それでも同じモジュールの中にあるので、`FieldName` の非公開のフィールド `.0` を読める。
fn find_field(line: &str, name: &FieldName) -> Option<String> {
    let name = name.0.as_str();
    for token in line.split_whitespace() {
        if token.starts_with(name) && token[name.len()..].starts_with('=') {
            return Some(token[name.len() + 1..].to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level() -> FieldName {
        FieldName::new("level").expect("正しい名前")
    }

    #[test]
    fn 名前は空白と等号と空を受け付けない() {
        assert_eq!(FieldName::new(""), Err(FieldNameError::Empty));
        assert_eq!(FieldName::new("a b"), Err(FieldNameError::Forbidden(' ')));
        assert_eq!(FieldName::new("a=b"), Err(FieldNameError::Forbidden('=')));
        assert_eq!(FieldName::new("level").map(|f| f.0), Ok("level".to_owned()));
    }

    #[test]
    fn 子のモジュールからは非公開のフィールドが見える() {
        let mut tally = Tally::new(level());
        tally.push_line("level=warn").expect("緩いので失敗しない");
        tally.push_line("user=a").expect("緩いので失敗しない");
        // `tests` は `tally` モジュールの子なので、`lines` も `skipped` も読める。
        assert_eq!(tally.lines, 2);
        assert_eq!(tally.skipped, 1);
        assert_eq!(tally.counts.get("warn"), Some(&1));
    }

    #[test]
    fn strict_は続けて書ける() {
        let mut tally = Tally::new(level()).strict(true);
        tally.push_line("level=warn").expect("フィールドがある");
        assert_eq!(tally.push_line("user=a"), Err(MissingField { line_no: 2 }));
    }

    #[test]
    fn 失敗した行のあとも行数は数えた行と飛ばした行の和に等しい() {
        let mut tally = Tally::new(level()).strict(true);
        tally.push_line("level=warn").expect("フィールドがある");
        assert!(tally.push_line("user=a").is_err());
        tally.push_line("level=info").expect("フィールドがある");
        let counted: u64 = tally.counts.values().sum();
        assert_eq!(tally.lines, 2);
        assert_eq!(
            usize::try_from(counted).expect("小さい"),
            tally.lines - tally.skipped
        );
    }

    #[test]
    fn 等号を含む名前は語を二通りに読ませる() {
        // `FieldName::new` を通さずに作れるのは、このモジュールの中だからである。
        let odd = FieldName("a=b".to_owned());
        let plain = FieldName("a".to_owned());
        assert_eq!(find_field("a=b=c", &odd), Some("c".to_owned()));
        assert_eq!(find_field("a=b=c", &plain), Some("b=c".to_owned()));
    }

    #[test]
    fn 空白を含む名前はどの語とも一致しない() {
        let odd = FieldName("a b".to_owned());
        assert_eq!(find_field("a b=c", &odd), None);
    }

    #[test]
    fn finish_は数えた結果をそのまま移す() {
        let mut tally = Tally::new(level());
        for line in ["level=warn", "level=info", "level=warn", "user=a"] {
            tally.push_line(line).expect("緩いので失敗しない");
        }
        assert_eq!(tally.count_of("warn"), 2);
        assert_eq!(tally.count_of("error"), 0);

        let report = tally.finish();
        assert_eq!(report.counts.len(), 2);
        assert_eq!(report.counts.get("warn"), Some(&2));
        assert_eq!(report.skipped, 1);
    }

    #[test]
    fn クレートの中なら_non_exhaustive_な構造体も作れる() {
        let report = Report {
            counts: HashMap::new(),
            skipped: 0,
        };
        assert!(report.counts.is_empty());
    }

    #[test]
    fn 名前を返すメソッドは中身を見せる() {
        let tally = Tally::new(level());
        assert_eq!(tally.field().as_str(), "level");
    }
}
