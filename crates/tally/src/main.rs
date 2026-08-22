//! バイナリ本体。ロジックは持たず、境界の面倒だけを見る。

use std::fs::File;
use std::io::{self, BufReader, IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser as _;

use tally::cli::Cli;
use tally::error::{CliError, CliErrorKind, EXIT_OK};
use tally::{error, format};
use tally_core::{Report, TallyError, tally_reader};

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing();

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            let code = err.exit_code();
            // 成功として扱う失敗（パイプの下流が先に閉じた）では診断を出さない。
            // 出すと `tally big.log | head` が毎回 stderr を汚す。
            if code != EXIT_OK {
                report_error(&err);
            }
            ExitCode::from(code)
        }
    }
}

/// ログ出力の初期化。`RUST_LOG` で制御し、既定では何も出さない。
///
/// 標準出力ではなく標準エラーに出すのは、集計結果をパイプで繋いだときに
/// ログが混ざらないようにするため。
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .with_ansi(io::stderr().is_terminal())
        .try_init();
}

/// 失敗を stderr に出す。**stdout には何も書かない。**
///
/// hint を別行にしているのは、`error:` の行を機械的に拾う利用者を壊さないため。
fn report_error(err: &CliError) {
    eprintln!("error: {}", error::one_line(err));
    if let Some(hint) = err.hint() {
        eprintln!("hint: {hint}");
    }
}

fn run(cli: &Cli) -> Result<(), CliError> {
    let report = aggregate(cli).map_err(|source| {
        // **hint をここで打つ。** `From<TallyError> for CliError` を実装して
        // `?` に任せると、「この失敗に対して利用者は何ができるか」を
        // 問う瞬間が消える（ADR-0004 論点 6）。
        let hint = error::hint_for(&source);
        CliError::new(CliErrorKind::Tally(source), hint)
    })?;

    write_output(cli, &report)
        // 書き出しの失敗（ディスクフル、パイプ切断）に対して
        // 利用者が `tally` の使い方を変えてできることは無い。
        .map_err(|source| CliError::new(CliErrorKind::Write(source), None))?;

    if cli.stats {
        // 「読み」ではなく「集計し」。`--filter` で除外した行は total に入らないので、
        // 「読んだ行数」と言うと入力の行数と食い違う。
        eprintln!(
            "{} 行を集計し、{} 行をスキップしました",
            report.total, report.skipped
        );
    }
    Ok(())
}

/// 入力を開いて集計する。**I/O の面倒はここに閉じる。**
fn aggregate(cli: &Cli) -> Result<Report, TallyError> {
    let selector = cli.selector();
    let counter = cli.counter();
    // `--filter` 未指定なら全行を通す述語にする。core 側に `Option` を渡さないのは、
    // 「フィルタが無い」を分岐として core に持ち込まないため。
    let keep = |line: &str| cli.filter.as_ref().is_none_or(|re| re.is_match(line));

    // **2 つの枝を 1 本にまとめられない。** 読み手の型が
    // `BufReader<File>` と `StdinLock` で別物であり、`tally_reader` は
    // `R: BufRead` で単相化される。`Box<dyn BufRead>` にすれば 1 本になるが、
    // 行ごとに動的ディスパッチを払うことになる。
    if let Some(path) = cli.input.as_deref() {
        tracing::debug!(path = %path.display(), "ファイルから読み込みます");
        // **開くのはここ。** `tally_core` はファイルを開かない。
        let file = File::open(path).map_err(|source| TallyError::OpenInput {
            path: path.to_path_buf(),
            source,
        })?;
        tally_reader(counter, BufReader::new(file), &selector, keep, cli.limit)
    } else {
        tracing::debug!("標準入力から読み込みます");
        let stdin = io::stdin();
        tally_reader(counter, stdin.lock(), &selector, keep, cli.limit)
    }
}

/// 集計結果を stdout に書き出す。
fn write_output(cli: &Cli, report: &Report) -> io::Result<()> {
    // stdout は行バッファリングされるため、大量出力では明示的に BufWriter で包む。
    // 包まないと 1 行ごとに write(2) が走り、数十倍遅くなることがある。
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    format::write_report(&mut out, report, cli.format)?;
    // **flush を忘れると BufWriter の drop 時に落ち、失敗が捨てられる。**
    // パイプ切断はここで初めて観測されることが多い。
    out.flush()
}
