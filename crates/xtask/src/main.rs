//! 配布物に同梱するシェル補完と man ページを生成する。
//!
//! **なぜ `tally` のサブコマンドではないのか。** `tally` は入力ファイルを
//! 可変個の位置引数で取る。`completions` サブコマンドを足すと、
//! **`completions` という名前のファイルが `--` なしで渡せなくなり、
//! 「シェル名が足りない」という無関係な診断が出る**
//! （clap 4.6.6 で実測。[ADR-0009] 論点 2）。
//!
//! **なぜ `build.rs` ではないのか。** 成果物が `OUT_DIR` に出ると
//! ハッシュ入りの path を探すことになる。
//! また `clap_complete` が `tally` の依存になり、
//! **集計しかしない利用者のビルドにも乗る。**
//!
//! 使い方:
//!
//! ```text
//! cargo run -p xtask -- <出力先ディレクトリ>
//! ```
//!
//! [ADR-0009]: ../../../docs/adr/0009-release-distribution.md

use std::io;
use std::path::Path;
use std::process::ExitCode;

use clap::CommandFactory as _;
use clap_complete::Shell;

/// 配布物に入れる補完の対象シェル。
///
/// **`Shell` の全要素を使わない。** `elvish` は配布物の需要が薄く、
/// **入れると「どのシェルを約束したか」が曖昧になる**ので明示的に並べる。
const SHELLS: &[Shell] = &[Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell];

/// 生成物を入れる相対パス。**アーカイブの構造は外部の契約**なので、
/// ここを変えると利用者の手順が壊れる（`crates/tally/docs/output-format.md`）。
const COMPLETIONS_DIR: &str = "completions";
const MAN_DIR: &str = "man";

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let Some(out) = args.next() else {
        eprintln!("出力先ディレクトリを指定すること: cargo run -p xtask -- <dir>");
        return ExitCode::FAILURE;
    };
    if args.next().is_some() {
        eprintln!("引数は出力先ディレクトリ 1 つだけ");
        return ExitCode::FAILURE;
    }

    match generate(Path::new(&out)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("生成に失敗した: {err}");
            ExitCode::FAILURE
        }
    }
}

/// 補完と man を `out` の下に書き出す。
fn generate(out: &Path) -> io::Result<()> {
    let completions = out.join(COMPLETIONS_DIR);
    let man = out.join(MAN_DIR);
    std::fs::create_dir_all(&completions)?;
    std::fs::create_dir_all(&man)?;

    // **`Cli` の定義を 1 箇所から引く。** 引数を足したときに
    // 補完と man を直し忘れる経路を作らない。
    let mut cmd = tally::cli::Cli::command();
    // `clap` は実行時の argv[0] から名前を取ることがあるので、明示する。
    // **ここがずれると補完関数の名前がずれ、補完が効かない。**
    let name = cmd.get_name().to_string();

    for &shell in SHELLS {
        let path = clap_complete::generate_to(shell, &mut cmd, &name, &completions)?;
        println!("{}", path.display());
    }

    let page = man.join(format!("{name}.1"));
    let mut buf = Vec::new();
    clap_mangen::Man::new(cmd).render(&mut buf)?;
    std::fs::write(&page, buf)?;
    println!("{}", page.display());

    Ok(())
}
