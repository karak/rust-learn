//! 集計結果の整形。
//!
//! **`main.rs` ではなくここに置く。** 整形は `(Report, Format) → バイト列` の
//! 純粋関数であり、`W: Write` を受け取る形にしておけば `Vec<u8>` に書いて
//! 内容を検証できる。`main.rs` に置くとプロセス起動でしか確かめられなくなる。
//!
//! 書き込み先を `impl Write` で受けるのが要点。ここが `Stdout` に固定されていると、
//! テストのために標準出力を差し替える仕掛けが必要になる。
//!
//! # `trait Formatter` にしていない理由
//!
//! **`Box<dyn Formatter>` と「trait + 静的 dispatch」も実装して比べたうえで、
//! この形（enum + 自由関数 + `match`）を選んでいる。** 判断の過程と実測値は
//! [ADR-0002] が正本。3 案が同居していた状態はタグ `adr-0002-baseline` に残る。
//!
//! **形式ごとの本体が長くなったら再評価する。** trait が有利になるのは
//! 形式ごとの実装が型として分離するに足る大きさになったときで、
//! 現状（1 形式あたり 3〜5 行）はその水準にない。
//!
//! [ADR-0002]: ../../../docs/adr/0002-output-format-abstraction.md

use std::borrow::Cow;
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
    /// CSV。ヘッダ行なし・LF・必要なときだけ引用（[ADR-0003]）。
    ///
    /// [ADR-0003]: ../../../docs/adr/0003-csv-output-contract.md
    Csv,
}

/// 集計結果を指定形式で書き出す。
///
/// **`_` を使わず全バリアントを列挙する。** 形式を足したときにここが
/// `E0004` になることが、この形を選んだ理由の一つ（[ADR-0002]）。
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
        Format::Csv => {
            for entry in &report.entries {
                writeln!(out, "{},{}", entry.count, quote_field(&entry.key))?;
            }
        }
    }
    Ok(())
}

/// CSV の 1 フィールドを、必要なときだけ二重引用符で囲む。
///
/// 規則は [ADR-0003] が正本。区切り `,`・引用符 `"`・CR・LF の
/// いずれかを含むときだけ囲み、含まれる `"` は `""` に倍化する。
///
/// **戻り値が [`Cow`] なのは、大半の値が引用不要だから。** 囲む必要がなければ
/// 入力をそのまま借用して返し、確保も複写も起きない。`String` を返す設計だと
/// 「何もしない」場合にまで確保が生じる。
///
/// **[`write_report`] に埋め込まず関数に切ってある。** 引用規則は CSV の外部契約
/// そのもので、境界値（`,` `"` CR LF）を関数単位で検査したい。埋め込むと
/// 「集計結果を丸ごと整形して文字列を突き合わせる」形でしか確かめられない。
///
/// [ADR-0003]: ../../../docs/adr/0003-csv-output-contract.md
fn quote_field(value: &str) -> Cow<'_, str> {
    // 配列を渡すと「いずれかの文字を含むか」になる（`Pattern` の実装）。
    // 4 回 contains を書くより速く、意図も直接読める。
    if value.contains([',', '"', '\r', '\n']) {
        Cow::Owned(format!("\"{}\"", value.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cow, Format, Report, io, quote_field, write_report};
    use crate::core::Entry;

    /// `--format text` の期待出力。件数の降順で並ぶ。
    const TEXT_EXPECTED: &str = "2\ta\n1\tb\n";

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
    /// **プロセスを起動せずに出力を検証できる**のが、整形を lib に置く目的。
    /// 書き出しをクロージャで受けるのは、`Format` ごとに呼び分けを書かずに
    /// 済ませるため。
    fn rendered(write: impl FnOnce(&mut Vec<u8>) -> io::Result<()>) -> String {
        let mut buf = Vec::new();
        write(&mut buf).expect("Vec への書き込みは失敗しない");
        String::from_utf8(buf).expect("UTF-8 のはず")
    }

    /// 引用が要るキーを含む標本。
    ///
    /// [`sample`] と分けてあるのは、text と JSON の期待値を単純に保ちたいから。
    /// **CSV は引用規則を通ることまで確かめないと、
    /// `quote_field` を呼び忘れても気づけない。**
    fn quoting_sample() -> Report {
        Report {
            entries: vec![
                Entry {
                    key: "plain".to_owned(),
                    count: 2,
                },
                Entry {
                    key: "a,b".to_owned(),
                    count: 1,
                },
                Entry {
                    key: "say \"hi\"".to_owned(),
                    count: 1,
                },
            ],
            skipped: 0,
            total: 4,
        }
    }

    /// [`quoting_sample`] を `--format csv` で出したときの期待出力。
    ///
    /// ヘッダ行が無いこと・改行が LF であることも、この文字列が保証している。
    const CSV_EXPECTED: &str = "2,plain\n1,\"a,b\"\n1,\"say \"\"hi\"\"\"\n";

    /// JSON 出力が [`sample`] の集計値を表していることを確かめる。
    ///
    /// 文字列の部分一致ではなく、JSON として解釈して値を見る。
    /// 整形（空白や順序）が変わっても壊れず、中身の誤りは捕まえられる。
    fn assert_json(out: &str) {
        let parsed: serde_json::Value = serde_json::from_str(out).expect("JSON として読めるはず");
        assert_eq!(parsed["total"], 6);
        assert_eq!(parsed["skipped"], 3);
        assert_eq!(parsed["entries"][0]["key"], "a");
        assert_eq!(parsed["entries"][0]["count"], 2);
        // 改行が無いと、パイプで繋いだ次の出力と行が繋がる。
        assert!(out.ends_with('\n'), "末尾に改行が無い: {out:?}");
    }

    #[test]
    fn text_形式は件数とキーをタブ区切りで出す() {
        let out = rendered(|buf| write_report(buf, &sample(), Format::Text));
        assert_eq!(out, TEXT_EXPECTED);
    }

    #[test]
    fn json_形式は集計値を含み末尾に改行を付ける() {
        assert_json(&rendered(|buf| write_report(buf, &sample(), Format::Json)));
    }

    #[test]
    fn csv_形式はヘッダ無しで必要なときだけ引用する() {
        let out = rendered(|buf| write_report(buf, &quoting_sample(), Format::Csv));
        assert_eq!(out, CSV_EXPECTED);
    }

    #[test]
    fn csv_の引用は必要なときだけ行う() {
        assert_eq!(quote_field("plain"), "plain");
        assert_eq!(quote_field("key,with,commas"), "\"key,with,commas\"");
        assert_eq!(quote_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(quote_field("two\nlines"), "\"two\nlines\"");
        assert_eq!(quote_field("cr\rhere"), "\"cr\rhere\"");
    }

    #[test]
    fn 引用が不要な値は借用のまま返る() {
        // Cow を返す意味はここにある。確保が起きないことを型で確かめる。
        assert!(matches!(quote_field("plain"), Cow::Borrowed(_)));
    }
}
