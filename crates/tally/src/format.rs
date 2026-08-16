//! 集計結果の整形。
//!
//! **`main.rs` ではなくここに置く。** 整形は `(Report, Format) → バイト列` の
//! 純粋関数であり、`W: Write` を受け取る形にしておけば `Vec<u8>` に書いて
//! 内容を検証できる。`main.rs` に置くとプロセス起動でしか確かめられなくなる。
//!
//! 書き込み先を `impl Write` で受けるのが要点。ここが `Stdout` に固定されていると、
//! テストのために標準出力を差し替える仕掛けが必要になる。

use std::io::{self, Write};

use crate::core::Report;

/// 出力形式。
///
/// clap の `ValueEnum` をここで derive している。整形の関心事である
/// 「どの形式か」と、CLI の関心事である「文字列からどう解釈するか」が
/// 同じ型に乗るが、**enum を 2 つ持って同期させるほうが害が大きい**
/// （同じ事実が 2 箇所になる）ため、こちらを選んだ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// `件数<TAB>キー` のタブ区切り。他の Unix ツールに繋ぐ前提。
    Text,
    /// JSON。機械可読な連携用。
    Json,
}

/// 集計結果を指定形式で書き出す。
///
/// 戻り値が `io::Result` であって [`crate::Result`][] でないのは、
/// ここで起きうる失敗が **書き込み先の I/O 失敗だけ** だから。
/// `TallyError` は入力の解釈に関する失敗を表す型なので、混ぜない。
pub fn write_report<W: Write>(out: &mut W, report: &Report, format: Format) -> io::Result<()> {
    match format {
        Format::Text => {
            for entry in &report.entries {
                writeln!(out, "{}\t{}", entry.count, entry.key)?;
            }
        }
        Format::Json => {
            serde_json::to_writer_pretty(&mut *out, report)?;
            // 末尾に改行を足すのは、パイプで繋いだ次の出力と行が繋がらないようにするため。
            writeln!(out)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Entry;

    fn sample() -> Report {
        Report {
            entries: vec![
                Entry {
                    key: "a".to_owned(),
                    count: 2,
                },
                Entry {
                    key: "b".to_owned(),
                    count: 1,
                },
            ],
            skipped: 3,
            total: 6,
        }
    }

    /// `Vec<u8>` に書いて文字列として取り出す。
    ///
    /// **プロセスを起動せずに出力を検証できる**のが、この関数を lib に置く目的。
    fn rendered(format: Format) -> String {
        let mut buf = Vec::new();
        write_report(&mut buf, &sample(), format).expect("Vec への書き込みは失敗しない");
        String::from_utf8(buf).expect("UTF-8 のはず")
    }

    #[test]
    fn text_形式は件数とキーをタブ区切りで出す() {
        assert_eq!(rendered(Format::Text), "2\ta\n1\tb\n");
    }

    #[test]
    fn json_形式は集計値を含む() {
        // 文字列の部分一致ではなく、JSON として解釈して値を見る。
        // 整形（空白や順序）が変わっても壊れず、中身の誤りは捕まえられる。
        let out = rendered(Format::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON として読めるはず");
        assert_eq!(parsed["total"], 6);
        assert_eq!(parsed["skipped"], 3);
        assert_eq!(parsed["entries"][0]["key"], "a");
        assert_eq!(parsed["entries"][0]["count"], 2);
    }

    #[test]
    fn json_形式は末尾に改行を付ける() {
        // 改行が無いと、パイプで繋いだ次の出力と行が繋がる。
        let out = rendered(Format::Json);
        assert!(out.ends_with('\n'), "末尾に改行が無い: {out:?}");
    }
}
