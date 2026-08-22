//! 行指向データの度数集計コア。
//!
//! `tally` CLI の中身を、**引数解釈と I/O から切り離した**ライブラリ。
//! 貫かれている不変条件は 2 つ。
//!
//! - **`clap` にも `anyhow` にも依存しない。** 依存の向きは
//!   `tally` → `tally-core` の一方向で、逆流していないことは
//!   `cargo tree -p tally-core` で確かめられる
//! - **ファイルを開かない。** [`tally_reader`] が [`BufRead`][std::io::BufRead] を
//!   受け取るのが唯一の I/O 境界で、開くのは呼び出し側の仕事
//!
//! # 使い方
//!
//! ```
//! use tally_core::{Counter, Key, Selector, tally_reader};
//!
//! let input = "{\"lvl\":\"info\"}\n{\"lvl\":\"error\"}\n{\"lvl\":\"INFO\"}\n";
//!
//! let report = tally_reader(
//!     // 「産まなかったときどうするか」はカウンタの方針。
//!     Counter::new().strict(false),
//!     input.as_bytes(),
//!     // 「どこから取り、どう正規化するか」は Selector の方針。
//!     &Selector::new(Key::JsonField("lvl".to_owned())).ignore_case(true),
//!     // 「どの行が参加するか」は述語。
//!     |_| true,
//!     None,
//! )
//! .expect("集計できる");
//!
//! assert_eq!(report.entries[0].key, "info");
//! assert_eq!(report.entries[0].count, 2);
//! assert_eq!(report.total, 3);
//! ```
//!
//! # 関心事の分け方
//!
//! **3 つの問いが 3 つの場所に分かれている。** どれをどこに足すかで迷ったら
//! この表を見る。
//!
//! | 問い | 置き場所 |
//! | --- | --- |
//! | どの行が集計に参加するか | [`tally_reader`] の述語（呼び出し側） |
//! | 参加する行がどうキーを産むか | [`Selector`] |
//! | キーを産まなかったときどうするか | [`Counter::strict`] |
//!
//! この分割の過程は [ADR-0005] が正本。
//!
//! # モジュールを公開していない理由
//!
//! `pub mod` ではなく **非公開モジュール + ルートでの再エクスポート**にしている。
//! 同じ型に至る経路が 1 つになるので、doc と semver の対象が曖昧にならない。
//! 副作用として、`pub` を付けたまま再エクスポートを忘れた項目は
//! `unreachable_pub` が拾う。
//!
//! [ADR-0005]: ../../docs/adr/0005-selector-public-api.md

mod count;
mod error;
mod select;

pub use count::{Counter, Entry, Report, tally_reader};
pub use error::{JsonError, LineError, LineErrorKind, Result, TallyError};
pub use select::{Key, Selector};
