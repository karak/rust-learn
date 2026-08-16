//! 集計結果の整形。
//!
//! **`main.rs` ではなくここに置く。** 整形は `(Report, Format) → バイト列` の
//! 純粋関数であり、`W: Write` を受け取る形にしておけば `Vec<u8>` に書いて
//! 内容を検証できる。`main.rs` に置くとプロセス起動でしか確かめられなくなる。
//!
//! 書き込み先を `impl Write` で受けるのが要点。ここが `Stdout` に固定されていると、
//! テストのために標準出力を差し替える仕掛けが必要になる。
//!
//! # ここに 3 つの実装が並んでいる理由
//!
//! [ADR-0002] の比較のため、**同じ出力を作る 3 案を同時に生かしてある。**
//!
//! | 案 | 置き場所 | 形 |
//! | --- | --- | --- |
//! | A | このファイルの [`write_report`] | enum + 自由関数 + `match` |
//! | C | [`dyn_variant`] | `trait Formatter` + `Box<dyn Formatter>`（動的 dispatch） |
//! | E | [`static_variant`] | `trait Formatter` + 網羅的 `match`（静的 dispatch） |
//!
//! **これは恒久的な構成ではない。** ADR-0002 の決定後、採らなかった案は削除する。
//! 3 案が重複を含むのは意図的で、「CSV を 1 つ足す差分」を案ごとに測るには、
//! 共通化してしまうと差が消えるため。
//!
//! `main.rs` が呼ぶのは A のみ。C と E は比較対象としてテストからだけ叩かれる。
//!
//! [ADR-0002]: ../../../../docs/adr/0002-output-format-abstraction.md

use std::borrow::Cow;
use std::io::{self, Write};

use crate::core::Report;

pub mod dyn_variant;
pub mod static_variant;

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
    /// [ADR-0003]: ../../../../docs/adr/0003-csv-output-contract.md
    Csv,
}

/// 集計結果を指定形式で書き出す（**案 A**: enum + 自由関数 + `match`）。
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
/// **3 案で共有する。** 引用規則は dispatch の形と無関係で、どの案でも同じものが
/// 要る。ここを案ごとに複写すると、比較したいのは dispatch の差なのに、
/// 定数分の差が 3 倍に見えてしまう。
///
/// [ADR-0003]: ../../../../docs/adr/0003-csv-output-contract.md
fn quote_field(value: &str) -> Cow<'_, str> {
    // 配列を渡すと「いずれかの文字を含むか」になる（`Pattern` の実装）。
    // 4 回 contains を書くより速く、意図も直接読める。
    if value.contains([',', '"', '\r', '\n']) {
        Cow::Owned(format!("\"{}\"", value.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(value)
    }
}

/// 3 案に共通のテスト素材。
///
/// **標本と期待値を 1 箇所に置く。** 案ごとに書くと、比較しているつもりで
/// 別々のものを検証してしまう。ここを共有することで「3 案が同じ契約を満たす」
/// ことがテストで保証される。
///
/// 実装そのものは共通化しない（[モジュールの説明](self)を参照）。
#[cfg(test)]
pub(crate) mod test_support {
    use super::{Report, io};
    use crate::core::Entry;

    /// `--format text` の期待出力。件数の降順で並ぶ。
    pub(crate) const TEXT_EXPECTED: &str = "2\ta\n1\tb\n";

    pub(crate) fn sample() -> Report {
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
    /// 呼び出し方が案ごとに違う（自由関数 / `Box<dyn>` / 静的 dispatch）ので、
    /// 書き出しはクロージャで受ける。
    pub(crate) fn rendered(write: impl FnOnce(&mut Vec<u8>) -> io::Result<()>) -> String {
        let mut buf = Vec::new();
        write(&mut buf).expect("Vec への書き込みは失敗しない");
        String::from_utf8(buf).expect("UTF-8 のはず")
    }

    /// 引用が要るキーを含む標本。
    ///
    /// [`sample`] と分けてあるのは、text と JSON の期待値を単純に保ちたいから。
    /// **CSV は引用規則を通ることまで確かめないと、案ごとに
    /// `quote_field` を呼び忘れても気づけない。**
    pub(crate) fn quoting_sample() -> Report {
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
    pub(crate) const CSV_EXPECTED: &str = "2,plain\n1,\"a,b\"\n1,\"say \"\"hi\"\"\"\n";

    /// JSON 出力が [`sample`] の集計値を表していることを確かめる。
    ///
    /// 文字列の部分一致ではなく、JSON として解釈して値を見る。
    /// 整形（空白や順序）が変わっても壊れず、中身の誤りは捕まえられる。
    pub(crate) fn assert_json(out: &str) {
        let parsed: serde_json::Value = serde_json::from_str(out).expect("JSON として読めるはず");
        assert_eq!(parsed["total"], 6);
        assert_eq!(parsed["skipped"], 3);
        assert_eq!(parsed["entries"][0]["key"], "a");
        assert_eq!(parsed["entries"][0]["count"], 2);
        // 改行が無いと、パイプで繋いだ次の出力と行が繋がる。
        assert!(out.ends_with('\n'), "末尾に改行が無い: {out:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{
        CSV_EXPECTED, TEXT_EXPECTED, assert_json, quoting_sample, rendered, sample,
    };
    use super::{Cow, Format, quote_field, write_report};

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
