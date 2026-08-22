//! 度数の集計と結果の表現。

use std::collections::HashMap;
use std::io::BufRead;

use crate::error::{LineError, LineErrorKind, Result};
use crate::select::{Key, Selector};

/// 集計結果の 1 行。
///
/// ```
/// use tally_core::{Counter, Key, Selector, tally_reader};
///
/// let report = tally_reader(
///     Counter::new(),
///     "a\nb\na\n".as_bytes(),
///     &Selector::new(Key::WholeLine),
///     |_| true,
///     None,
/// )
/// .expect("集計できる");
///
/// assert_eq!(report.entries[0].key, "a");
/// assert_eq!(report.entries[0].count, 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Entry {
    /// 集計されたキー。
    pub key: String,
    /// 出現回数。
    pub count: u64,
}

/// 集計結果全体。
///
/// **並び順は契約である**（件数の降順、同数ならキーの昇順）。
/// 詳細は `crates/tally/docs/output-format.md`。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Report {
    /// 集計結果。並び順は件数の降順、同数ならキーの昇順。
    pub entries: Vec<Entry>,
    /// キーを取り出せずスキップした行数。
    pub skipped: usize,
    /// 集計に参加した行の総数（空行を除く）。上流の述語で落ちた行は含まない。
    pub total: usize,
}

/// 度数カウンタ。
///
/// # `strict` がここにある理由
///
/// 「キーを取り出せなかったときエラーにするか」は **抽出器の性質ではなく
/// 消費者の方針**である（[ADR-0005] 論点 1）。同じ [`Selector`] を、
/// 検査のために厳格に回す消費者と、集計のために緩く回す消費者が共有できる。
///
/// ```
/// use tally_core::{Counter, Key, Selector};
///
/// let selector = Selector::new(Key::JsonField("lvl".to_owned()));
///
/// // 既定は緩い。欠損行はスキップとして数える。
/// let mut lenient = Counter::new();
/// lenient.push_line(&selector, "{\"other\":1}", 1).expect("スキップされる");
/// assert_eq!(lenient.report(None).skipped, 1);
///
/// // 同じ Selector を厳格に回せる。
/// let mut strict = Counter::new().strict(true);
/// let err = strict
///     .push_line(&selector, "{\"other\":1}", 1)
///     .expect_err("strict なので失敗する");
/// assert_eq!(err.line_no, 1);
/// ```
///
/// [ADR-0005]: ../../../docs/adr/0005-selector-public-api.md
#[derive(Debug, Default)]
pub struct Counter {
    counts: HashMap<String, u64>,
    skipped: usize,
    total: usize,
    strict: bool,
}

impl Counter {
    /// 空のカウンタ。**既定はキーを取り出せない行をスキップする。**
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// キーを取り出せない行をエラーにする。
    ///
    /// **最初の 1 件で止まる**（fail fast）。全件報告してから失敗する挙動は
    /// 持っていない。
    #[must_use]
    pub fn strict(mut self, yes: bool) -> Self {
        self.strict = yes;
        self
    }

    /// 1 行を取り込む。**空行は総数にも数えない。**
    ///
    /// エラー型が [`LineError`] なのは、この関数が I/O で失敗しえないため
    /// （[ADR-0004] 論点 3）。
    ///
    /// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
    pub fn push_line(
        &mut self,
        selector: &Selector,
        line: &str,
        line_no: usize,
    ) -> Result<(), LineError> {
        if line.trim().is_empty() {
            return Ok(());
        }
        self.total += 1;

        let Some(extracted) = selector.select(line, line_no)? else {
            return self.on_missing(selector.key(), line, line_no);
        };

        // entry API を使うと、存在チェックと挿入で 2 回ハッシュを引かずに済む。
        // `into_owned()` はここで初めて確定的にアロケーションする。
        *self.counts.entry(extracted.into_owned()).or_insert(0) += 1;
        Ok(())
    }

    /// キーを取り出せなかった行の扱い。
    ///
    /// `_` を使わず全バリアントを書いているのは、[`Key`] に変種が増えたときに
    /// **黙って `strict` が無視される** のを防ぐため。コンパイルエラーになる。
    /// （`Key` は `#[non_exhaustive]` だが、**定義元クレートの中では属性が無効**なので
    /// この検査は保たれる。）
    ///
    /// `Key::WholeLine` は [`Selector::select`] が `None` を返さないので、
    /// `strict` が有効でもここには到達しない。
    fn on_missing(&mut self, key: &Key, line: &str, line_no: usize) -> Result<(), LineError> {
        match key {
            Key::JsonField(field) if self.strict => Err(LineError::new(
                line_no,
                line,
                LineErrorKind::MissingField {
                    field: field.as_str().into(),
                },
            )),
            Key::JsonField(_) | Key::WholeLine => {
                self.skipped += 1;
                Ok(())
            }
        }
    }

    /// 上位 `limit` 件を返す。`None` なら全件。
    ///
    /// 件数の降順、同数ならキーの昇順。**同数時のタイブレークを決めておかないと
    /// [`HashMap`] の反復順に依存して出力が実行ごとに変わり、
    /// スナップショットテストが壊れる。**
    ///
    /// ```
    /// use tally_core::{Counter, Key, Selector};
    ///
    /// let selector = Selector::new(Key::WholeLine);
    /// let mut counter = Counter::new();
    /// for (line_no, line) in ["b", "a", "a"].iter().enumerate() {
    ///     counter.push_line(&selector, line, line_no + 1).expect("失敗しない");
    /// }
    ///
    /// let report = counter.report(Some(1));
    /// assert_eq!(report.entries.len(), 1);
    /// assert_eq!(report.entries[0].key, "a");
    /// // limit は entries だけを切り詰める。total は変わらない。
    /// assert_eq!(report.total, 3);
    /// ```
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
}

/// `BufRead` を丸ごと集計する。**I/O に触れるのはこの関数だけ。**
///
/// `keep` が `false` を返した行は **集計にも総数にも入らない。**
/// フィルタは集計の上流にあるという扱いで、`tally --filter X` の出力が
/// `grep X | tally` と一致する。
///
/// **`Regex` ではなく述語を受け取る。** ここが `regex` に依存すると、
/// 「行を数える」という関心事に正規表現の実装が混ざる。
/// 述語なら呼び出し側が何で判定してもよく、テストも正規表現なしで書ける。
///
/// 境界が `FnMut` ではなく **`Fn`** なのは、判定に状態を持たせないため。
/// 行の順序で結果が変わる述語（「最初の 10 行だけ」など）を渡せてしまうと、
/// **フィルタが `limit` と競合する別種の機能になる。**
///
/// # `counter` を引数に取る理由
///
/// 厳格さ（`strict`）は [`Counter`] が持つ（[ADR-0005] 論点 1）ので、
/// **その方針を決めるのは呼び出し側**である。内部で `Counter::new()` を作ると
/// 方針を渡す手段が無くなる。
///
/// ```
/// use tally_core::{Counter, Key, Selector, tally_reader};
///
/// let input = "info\nwarn\ninfo\ndebug\n";
/// let report = tally_reader(
///     Counter::new(),
///     input.as_bytes(),
///     &Selector::new(Key::WholeLine),
///     // "debug" の行は読まなかったことにする。
///     |line| line != "debug",
///     None,
/// )
/// .expect("集計できる");
///
/// assert_eq!(report.entries[0].key, "info");
/// assert_eq!(report.entries[0].count, 2);
/// // 落ちた行は total にも skipped にも入らない。
/// assert_eq!(report.total, 3);
/// assert_eq!(report.skipped, 0);
/// ```
///
/// [ADR-0005]: ../../../docs/adr/0005-selector-public-api.md
pub fn tally_reader<R, F>(
    counter: Counter,
    reader: R,
    selector: &Selector,
    keep: F,
    limit: Option<usize>,
) -> Result<Report>
where
    R: BufRead,
    F: Fn(&str) -> bool,
{
    let counter = reader
        .lines()
        // **`filter` より前に置く。** ここを後ろにすると、落とした行のぶんだけ
        // 行番号がずれ、エラーが指す行と入力の行が食い違う。
        .enumerate()
        // `Result` の中身にだけ触る。`Result<T, E>` は `map` を持つので、
        // 成功のときだけ組を作り、失敗はそのまま素通しできる。
        .map(|(index, line)| line.map(|line| (index + 1, line)))
        .filter(|item| match item {
            Ok((_, line)) => keep(line),
            // **`Err` は必ず下流へ流す。** 述語で判定できないからと捨てると、
            // I/O 失敗が「フィルタに合わなかった行」と区別できなくなり、
            // 途中で読めなくなった入力が成功として集計される。
            Err(_) => true,
        })
        // **`collect::<Result<Vec<_>, _>>()` を使わない。** 短絡はするが、
        // 短絡するまでの成功分をすべて `Vec` に確保する。入力は標準入力の
        // ストリームでありうるので、入力サイズぶんのメモリを要求する形にしない。
        // `try_fold` なら 1 行ずつ畳み込み、最初のエラーで打ち切る。
        .try_fold(counter, |mut counter, item| -> Result<Counter> {
            // `item?` は `io::Error` → `TallyError::Read`、
            // `push_line?` は `LineError` → `TallyError::Line` と、
            // **別々の `From` 実装を経由して同じ型に合流する。**
            let (line_no, line) = item?;
            counter.push_line(selector, &line, line_no)?;
            Ok(counter)
        })?;

    Ok(counter.report(limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TallyError;

    fn json(field: &str) -> Selector {
        Selector::new(Key::JsonField(field.to_owned()))
    }

    /// 既定のカウンタで丸ごと集計する。
    fn tally_str(input: &str, selector: &Selector) -> Report {
        tally_reader(Counter::new(), input.as_bytes(), selector, |_| true, None)
            .expect("集計に成功するはず")
    }

    /// `strict` で丸ごと集計する。
    fn tally_strict(input: &str, selector: &Selector) -> Result<Report> {
        tally_reader(
            Counter::new().strict(true),
            input.as_bytes(),
            selector,
            |_| true,
            None,
        )
    }

    // --- 集計 ---

    #[test]
    fn 行全体をキーにして度数を数える() {
        let report = tally_str("a\nb\na\n", &Selector::new(Key::WholeLine));
        assert_eq!(
            report.entries,
            vec![
                Entry {
                    key: "a".to_owned(),
                    count: 2
                },
                Entry {
                    key: "b".to_owned(),
                    count: 1
                },
            ]
        );
        assert_eq!(report.total, 3);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn 空行は総数に含めない() {
        let report = tally_str("a\n\n   \na\n", &Selector::new(Key::WholeLine));
        assert_eq!(report.total, 2);
        assert_eq!(report.entries.len(), 1);
    }

    #[test]
    fn 同数のときはキーの昇順で安定する() {
        // HashMap の反復順に依存していれば、この assert はいずれ落ちる。
        let report = tally_str("b\nc\na\n", &Selector::new(Key::WholeLine));
        let keys: Vec<&str> = report.entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn json_フィールドを抽出する() {
        let input = "{\"lvl\":\"info\"}\n{\"lvl\":\"error\"}\n{\"lvl\":\"info\"}\n";
        let report = tally_str(input, &json("lvl"));
        assert_eq!(report.entries[0].key, "info");
        assert_eq!(report.entries[0].count, 2);
    }

    #[test]
    fn フィールドがない行はスキップ扱いになる() {
        let report = tally_str("{\"lvl\":\"info\"}\n{\"other\":1}\n", &json("lvl"));
        assert_eq!(report.skipped, 1);
        assert_eq!(report.total, 2);
    }

    #[test]
    fn json_として壊れている行は行番号つきで失敗する() {
        let err = tally_reader(
            Counter::new(),
            "{\"lvl\":\"info\"}\nnot json\n".as_bytes(),
            &json("lvl"),
            |_| true,
            None,
        )
        .expect_err("2 行目で失敗するはず");

        let TallyError::Line(line) = err else {
            panic!("行の失敗を期待した")
        };
        assert_eq!(line.line_no, 2);
        assert!(matches!(line.kind, LineErrorKind::InvalidJson { .. }));
    }

    #[test]
    fn limit_は件数の多い順に切り詰める() {
        let report = tally_reader(
            Counter::new(),
            "a\na\nb\nc\n".as_bytes(),
            &Selector::new(Key::WholeLine),
            |_| true,
            Some(2),
        )
        .expect("集計に成功するはず");
        assert_eq!(report.entries.len(), 2);
        assert_eq!(report.entries[0].key, "a");
    }

    #[test]
    fn ignore_case_で大文字小文字が同じキーに畳まれる() {
        let selector = Selector::new(Key::WholeLine).ignore_case(true);
        let report = tally_str("Info\nINFO\ninfo\n", &selector);
        assert_eq!(
            report.entries,
            vec![Entry {
                key: "info".to_owned(),
                count: 3
            }]
        );
    }

    #[test]
    fn ignore_case_なしなら大文字小文字は区別される() {
        let report = tally_str("Info\ninfo\n", &Selector::new(Key::WholeLine));
        assert_eq!(report.entries.len(), 2, "実際: {:?}", report.entries);
    }

    // --- strict ---

    #[test]
    fn 既定はキーを取り出せない行をスキップする() {
        let report = tally_str("{\"lvl\":\"info\"}\n{\"other\":1}\n", &json("lvl"));
        assert_eq!(report.skipped, 1);
    }

    #[test]
    fn strict_でフィールドが無い行はエラーになる() {
        let err = tally_strict("{\"lvl\":\"info\"}\n{\"other\":1}\n", &json("lvl"))
            .expect_err("strict なので失敗するはず");

        let TallyError::Line(line) = err else {
            panic!("行の失敗を期待した")
        };
        assert_eq!(line.line_no, 2, "2 行目で失敗するはず");
        assert!(matches!(
            line.kind,
            LineErrorKind::MissingField { ref field } if field.as_ref() == "lvl"
        ));
    }

    #[test]
    fn strict_では_null_値もエラーになる() {
        // 「取り出せたか否か」で一貫させる設計なので、null も欠損と同じ扱い。
        let err = tally_strict("{\"lvl\":null}\n", &json("lvl")).expect_err("失敗するはず");
        assert!(matches!(err, TallyError::Line(line) if line.line_no == 1));
    }

    #[test]
    fn strict_でも取り出せる行だけなら成功する() {
        let report = tally_strict("{\"lvl\":\"info\"}\n{\"lvl\":\"warn\"}\n", &json("lvl"))
            .expect("全行取り出せるので成功するはず");
        assert_eq!(report.total, 2);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn 行全体がキーなら_strict_でも失敗しない() {
        // WholeLine は None を返さないため、strict は到達しない。
        let report = tally_strict("a\nb\n", &Selector::new(Key::WholeLine)).expect("成功するはず");
        assert_eq!(report.total, 2);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn strict_の失敗は抜粋を持つ() {
        // 切り詰めの規則そのものは error モジュールで検査している。
        // ここで見るのは **抜粋が実際に配線されていること**。
        let long = "x".repeat(100);
        let err = tally_strict(&format!("{{\"pad\":\"{long}\"}}\n"), &json("lvl"))
            .expect_err("失敗するはず");

        let TallyError::Line(line) = err else {
            panic!("行の失敗を期待した")
        };
        assert!(line.snippet.ends_with('…'), "実際: {}", line.snippet);
        assert!(
            line.snippet.starts_with("{\"pad\""),
            "実際: {}",
            line.snippet
        );
    }

    #[test]
    fn 同じ_selector_を厳格にも緩くも回せる() {
        // strict を Selector から外した目的がこれ（ADR-0005 論点 1）。
        let selector = json("lvl");
        let input = "{\"other\":1}\n";
        assert_eq!(tally_str(input, &selector).skipped, 1);
        assert!(tally_strict(input, &selector).is_err());
    }

    // --- 述語（フィルタ） ---

    #[test]
    fn 述語で落ちた行は集計にも総数にも入らない() {
        // フィルタは集計の上流にある。落ちた行は「読まなかった」のと同じ扱いで、
        // これにより `tally --filter X` の出力が `grep X | tally` と一致する。
        let report = tally_reader(
            Counter::new(),
            "a\nb\na\n".as_bytes(),
            &Selector::new(Key::WholeLine),
            |line| line != "b",
            None,
        )
        .expect("集計に成功するはず");

        assert_eq!(
            report.entries,
            vec![Entry {
                key: "a".to_owned(),
                count: 2
            }]
        );
        assert_eq!(report.total, 2, "落ちた行を総数に入れてはいけない");
        assert_eq!(report.skipped, 0, "落ちた行はスキップでもない");
    }

    #[test]
    fn 述語で落ちた行も行番号に数える() {
        // 3 行目が壊れている。2 行目を落としても「3 行目」と言えなければ、
        // 利用者は入力ファイルの該当行を開けない。
        // enumerate を filter より前に置くことでこれを保証する。
        let err = tally_reader(
            Counter::new(),
            "{\"lvl\":\"a\"}\nDROP ME\nnot json\n".as_bytes(),
            &json("lvl"),
            |line| line != "DROP ME",
            None,
        )
        .expect_err("3 行目で失敗するはず");

        assert!(matches!(err, TallyError::Line(line) if line.line_no == 3));
    }

    // --- I/O 失敗の合流 ---

    #[test]
    fn 不正な_utf8_は読み取り失敗になる() {
        // `lines()` は InvalidData を返す。行の失敗（LineError）とは別の枝に入る。
        let err = tally_reader(
            Counter::new(),
            &b"ok\n\xff\xfe\n"[..],
            &Selector::new(Key::WholeLine),
            |_| true,
            None,
        )
        .expect_err("不正な UTF-8 で失敗するはず");

        assert!(matches!(err, TallyError::Read(_)), "実際: {err:?}");
    }

    // --- カウンタの再利用 ---

    #[test]
    fn 同じカウンタに複数回押し込める() {
        // `strict` を Counter が持つ形にした副産物。方針を保ったまま
        // 複数の入力を 1 つの集計に合流させられる。
        let selector = Selector::new(Key::WholeLine);
        let mut counter = Counter::new();
        counter.push_line(&selector, "a", 1).expect("成功する");
        counter.push_line(&selector, "a", 2).expect("成功する");
        counter.push_line(&selector, "b", 3).expect("成功する");

        let report = counter.report(None);
        assert_eq!(report.total, 3);
        assert_eq!(report.entries[0].count, 2);
    }
}
