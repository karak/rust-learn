//! 集計のコア。**I/O を一切持たない**ように切り出してある。
//!
//! この分離が学習上いちばん重要な点。`impl BufRead` を受け取る関数と、
//! `&str` を受け取る純粋関数を分けておくと、テストがファイルシステムに依存しなくなる。

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::BufRead;

use crate::error::{Result, TallyError};

/// 各行からどの値を取り出すか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// 行全体（前後の空白を除去したもの）をキーにする。
    WholeLine,
    /// 行を JSON オブジェクトとして解釈し、指定フィールドをキーにする。
    JsonField(String),
}

impl Key {
    /// 1 行からキーを取り出す。
    ///
    /// 戻り値が `Option` なのは「そのフィールドを持たない行」を
    /// エラーではなくスキップとして扱うため。ログ集計では欠損は日常的で、
    /// そこで全体を失敗させると使い物にならない。
    ///
    /// `Cow` を返しているのは、`WholeLine` の場合に借用のまま返せるから。
    /// ここで無条件に `String` を作ると、行数ぶんのアロケーションが増える。
    fn extract<'a>(&self, line: &'a str, line_no: usize) -> Result<Option<Cow<'a, str>>> {
        match self {
            Self::WholeLine => Ok(Some(Cow::Borrowed(line.trim()))),
            Self::JsonField(field) => {
                let value: serde_json::Value = serde_json::from_str(line)
                    .map_err(|source| TallyError::InvalidJson { line_no, source })?;

                let Some(found) = value.get(field) else {
                    return Ok(None);
                };

                let rendered = match found {
                    serde_json::Value::String(s) => Cow::Owned(s.clone()),
                    serde_json::Value::Number(n) => Cow::Owned(n.to_string()),
                    serde_json::Value::Bool(b) => Cow::Owned(b.to_string()),
                    serde_json::Value::Null => return Ok(None),
                    _ => {
                        return Err(TallyError::UnsupportedFieldType {
                            line_no,
                            field: field.clone(),
                        });
                    }
                };
                Ok(Some(rendered))
            }
        }
    }
}

/// 集計結果の 1 行。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Entry {
    pub key: String,
    pub count: u64,
}

/// 集計結果全体。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Report {
    pub entries: Vec<Entry>,
    /// キーを取り出せずスキップした行数。
    pub skipped: usize,
    /// 読んだ行の総数（空行を除く）。
    pub total: usize,
}

/// 度数カウンタ。
#[derive(Debug, Default)]
pub struct Counter {
    counts: HashMap<String, u64>,
    skipped: usize,
    total: usize,
}

impl Counter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 1 行を取り込む。空行は総数にも数えない。
    pub fn push_line(&mut self, key: &Key, line: &str, line_no: usize) -> Result<()> {
        if line.trim().is_empty() {
            return Ok(());
        }
        self.total += 1;

        match key.extract(line, line_no)? {
            Some(extracted) => {
                // entry API を使うと、存在チェックと挿入で 2 回ハッシュを引かずに済む。
                // `into_owned()` はここで初めて確定的にアロケーションする。
                *self.counts.entry(extracted.into_owned()).or_insert(0) += 1;
            }
            None => self.skipped += 1,
        }
        Ok(())
    }

    /// 上位 `limit` 件を返す。`None` なら全件。
    ///
    /// 件数の降順、同数ならキーの昇順。**同数時のタイブレークを決めておかないと
    /// `HashMap` の反復順に依存して出力が実行ごとに変わり、スナップショットテストが壊れる。**
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

/// `BufRead` を丸ごと集計する。I/O に触れるのはこの関数だけ。
pub fn tally_reader<R: BufRead>(reader: R, key: &Key, limit: Option<usize>) -> Result<Report> {
    let mut counter = Counter::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        counter.push_line(key, &line, index + 1)?;
    }
    Ok(counter.report(limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tally_str(input: &str, key: &Key) -> Report {
        tally_reader(input.as_bytes(), key, None).expect("集計に成功するはず")
    }

    #[test]
    fn 行全体をキーにして度数を数える() {
        let report = tally_str("a\nb\na\n", &Key::WholeLine);
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
        let report = tally_str("a\n\n   \na\n", &Key::WholeLine);
        assert_eq!(report.total, 2);
        assert_eq!(report.entries.len(), 1);
    }

    #[test]
    fn 同数のときはキーの昇順で安定する() {
        // HashMap の反復順に依存していれば、この assert はいずれ落ちる。
        let report = tally_str("b\nc\na\n", &Key::WholeLine);
        let keys: Vec<&str> = report.entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn json_フィールドを抽出する() {
        let input = r#"{"lvl":"info"}
{"lvl":"error"}
{"lvl":"info"}"#;
        let report = tally_str(input, &Key::JsonField("lvl".to_owned()));
        assert_eq!(report.entries[0].key, "info");
        assert_eq!(report.entries[0].count, 2);
    }

    #[test]
    fn フィールドがない行はスキップ扱いになる() {
        let input = "{\"lvl\":\"info\"}\n{\"other\":1}\n";
        let report = tally_str(input, &Key::JsonField("lvl".to_owned()));
        assert_eq!(report.skipped, 1);
        assert_eq!(report.total, 2);
    }

    #[test]
    fn json_として壊れている行は行番号つきで失敗する() {
        let input = "{\"lvl\":\"info\"}\nnot json\n";
        let err = tally_reader(input.as_bytes(), &Key::JsonField("lvl".to_owned()), None)
            .expect_err("2 行目で失敗するはず");
        assert!(
            matches!(err, TallyError::InvalidJson { line_no: 2, .. }),
            "実際のエラー: {err:?}"
        );
    }

    #[test]
    fn limit_は件数の多い順に切り詰める() {
        let report = tally_reader("a\na\nb\nc\n".as_bytes(), &Key::WholeLine, Some(2))
            .expect("集計に成功するはず");
        assert_eq!(report.entries.len(), 2);
        assert_eq!(report.entries[0].key, "a");
    }
}
