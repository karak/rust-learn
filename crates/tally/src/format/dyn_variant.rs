//! **案 C**: `trait Formatter` + `Box<dyn Formatter>`（動的 dispatch）。
//!
//! 比較用の実装。位置づけは[親モジュール](super)を参照。

use std::io::{self, Write};

use super::{Format, Report};

/// 1 つの出力形式。
///
/// **`&mut dyn Write` を受ける。** ここを `W: Write` のジェネリックメソッドに
/// すると trait がオブジェクト安全でなくなり、`Box<dyn Formatter>` を作れない。
/// 案 E との本質的な差はここに出る。
pub trait Formatter {
    fn write(&self, out: &mut dyn Write, report: &Report) -> io::Result<()>;
}

struct Text;

impl Formatter for Text {
    fn write(&self, out: &mut dyn Write, report: &Report) -> io::Result<()> {
        for entry in &report.entries {
            writeln!(out, "{}\t{}", entry.count, entry.key)?;
        }
        Ok(())
    }
}

struct Json;

impl Formatter for Json {
    fn write(&self, out: &mut dyn Write, report: &Report) -> io::Result<()> {
        serde_json::to_writer_pretty(&mut *out, report)?;
        // 末尾に改行を足すのは、パイプで繋いだ次の出力と行が繋がらないようにするため。
        writeln!(out)
    }
}

/// 形式に対応する整形器を作る。
///
/// `Text` も `Json` も ZST なので、**この `Box::new` はアロケーションしない**
/// （`docs/learning-log.md` 節 4-7）。実コストは vtable 経由の間接参照 1 回だけ。
#[must_use]
pub fn formatter(format: Format) -> Box<dyn Formatter> {
    match format {
        Format::Text => Box::new(Text),
        Format::Json => Box::new(Json),
    }
}

/// 案 A の [`write_report`](super::write_report) と同じ契約の入口。
pub fn write_report(out: &mut dyn Write, report: &Report, format: Format) -> io::Result<()> {
    formatter(format).write(out, report)
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
