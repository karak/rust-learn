//! 引数定義。
//!
//! `main.rs` に置かず独立させているのは、`Cli::try_parse_from(...)` で
//! **プロセスを起動せずに** 引数解釈をテストできるようにするため。

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::core::{Key, Selector};

/// 出力形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// `件数<TAB>キー` のタブ区切り。他の Unix ツールに繋ぐ前提。
    Text,
    /// JSON。機械可読な連携用。
    Json,
}

/// 行指向データの度数を集計する。
#[derive(Debug, Parser)]
#[command(name = "tally", version, about, long_about = None)]
pub struct Cli {
    /// 入力ファイル。省略時は標準入力を読む。
    pub input: Option<PathBuf>,

    /// 各行を JSON として解釈し、このフィールドの値を集計する。
    ///
    /// 省略時は行全体をキーにする。
    #[arg(short, long, value_name = "NAME")]
    pub field: Option<String>,

    /// 上位 N 件だけ出力する。
    #[arg(short = 'n', long, value_name = "N")]
    pub limit: Option<usize>,

    /// 出力形式。
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,

    /// 大文字小文字を区別せずに集計する。
    ///
    /// 正規化されるのは集計対象の値のみ。`--field` で指定するフィールド名の
    /// 一致判定は厳密なままである点に注意。
    #[arg(short = 'i', long)]
    pub ignore_case: bool,

    /// キーを取り出せない行があればエラーにする。
    ///
    /// 既定では、フィールドを持たない行は黙ってスキップする。
    /// 入力の健全性を検査したいときに指定する。
    #[arg(long)]
    pub strict: bool,

    /// 集計対象の行数・スキップ行数を標準エラーに出す。
    #[arg(long)]
    pub stats: bool,
}

impl Cli {
    /// 引数から集計キーを決める。
    #[must_use]
    pub fn key(&self) -> Key {
        match &self.field {
            Some(name) => Key::JsonField(name.clone()),
            None => Key::WholeLine,
        }
    }

    /// 引数から「どこから取り、どう正規化するか」を決める。
    #[must_use]
    pub fn selector(&self) -> Selector {
        Selector {
            key: self.key(),
            ignore_case: self.ignore_case,
            strict: self.strict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_の定義自体が矛盾していない() {
        // 短縮フラグの重複などを clap が検証してくれる。CLI が増えたら効いてくる。
        Cli::command().debug_assert();
    }

    #[test]
    fn field_省略時は行全体がキーになる() {
        let cli = Cli::try_parse_from(["tally"]).expect("引数なしで解釈できるはず");
        assert_eq!(cli.key(), Key::WholeLine);
        assert_eq!(cli.format, Format::Text);
    }

    #[test]
    fn field_指定時は_json_フィールドがキーになる() {
        let cli = Cli::try_parse_from(["tally", "--field", "lvl"]).expect("解釈できるはず");
        assert_eq!(cli.key(), Key::JsonField("lvl".to_owned()));
    }

    #[test]
    fn ignore_case_は既定で無効() {
        let cli = Cli::try_parse_from(["tally"]).expect("解釈できるはず");
        assert!(!cli.ignore_case);
        assert!(!cli.selector().ignore_case);
    }

    #[test]
    fn ignore_case_は短縮形でも指定できる() {
        for args in [["tally", "-i"], ["tally", "--ignore-case"]] {
            let cli = Cli::try_parse_from(args).expect("解釈できるはず");
            assert!(cli.selector().ignore_case, "失敗した引数: {args:?}");
        }
    }

    #[test]
    fn selector_は_field_と_ignore_case_の両方を反映する() {
        let cli = Cli::try_parse_from(["tally", "--field", "lvl", "-i"]).expect("解釈できるはず");
        assert_eq!(
            cli.selector(),
            Selector {
                key: Key::JsonField("lvl".to_owned()),
                ignore_case: true,
                strict: false,
            }
        );
    }

    #[test]
    fn strict_は既定で無効() {
        let cli = Cli::try_parse_from(["tally"]).expect("解釈できるはず");
        assert!(!cli.selector().strict);
    }

    #[test]
    fn strict_指定が_selector_に反映される() {
        let cli = Cli::try_parse_from(["tally", "--strict"]).expect("解釈できるはず");
        assert!(
            cli.selector().strict,
            "--strict が selector に伝わっていない"
        );
    }

    #[test]
    fn 未知のフラグは失敗する() {
        Cli::try_parse_from(["tally", "--nope"]).expect_err("未知のフラグは拒否されるはず");
    }
}
