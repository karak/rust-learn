//! **案 E**: `trait Formatter` が操作を定義し、enum の網羅的 `match` が
//! 具体型を選ぶ（静的 dispatch）。
//!
//! 比較用の実装。位置づけは[親モジュール](super)を参照。

use std::io::{self, Write};

use super::{Format, Report};

/// 1 つの出力形式。
///
/// **書き込み先をジェネリックに取れる。** 動的 dispatch を捨てた見返りで、
/// オブジェクト安全性の制約が外れる（案 C ではこの形が書けない）。
/// 呼び出しは単相化され、vtable 経由の間接参照が消える。
pub trait Formatter {
    fn write<W: Write + ?Sized>(&self, out: &mut W, report: &Report) -> io::Result<()>;
}

struct Text;

impl Formatter for Text {
    fn write<W: Write + ?Sized>(&self, out: &mut W, report: &Report) -> io::Result<()> {
        for entry in &report.entries {
            writeln!(out, "{}\t{}", entry.count, entry.key)?;
        }
        Ok(())
    }
}

struct Json;

impl Formatter for Json {
    fn write<W: Write + ?Sized>(&self, out: &mut W, report: &Report) -> io::Result<()> {
        serde_json::to_writer_pretty(&mut *out, report)?;
        // 末尾に改行を足すのは、パイプで繋いだ次の出力と行が繋がらないようにするため。
        writeln!(out)
    }
}

/// 案 A の [`write_report`](super::write_report) と同じ契約の入口。
///
/// `_` を使わず全バリアントを列挙するので、形式を足したときに
/// ここがコンパイルエラーになる（案 A・C と同じ保証）。
pub fn write_report<W: Write>(out: &mut W, report: &Report, format: Format) -> io::Result<()> {
    match format {
        Format::Text => Text.write(out, report),
        Format::Json => Json.write(out, report),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{TEXT_EXPECTED, assert_json, rendered, sample};
    use super::{Format, write_report};

    #[test]
    fn text_形式は件数とキーをタブ区切りで出す() {
        let out = rendered(|buf| write_report(buf, &sample(), Format::Text));
        assert_eq!(out, TEXT_EXPECTED);
    }

    #[test]
    fn json_形式は集計値を含み末尾に改行を付ける() {
        assert_json(&rendered(|buf| write_report(buf, &sample(), Format::Json)));
    }
}
