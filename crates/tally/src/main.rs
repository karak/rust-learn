//! バイナリ本体。ロジックは持たず、境界の面倒だけを見る。

use std::fs::File;
use std::io::{self, BufReader, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Context as _;
use clap::Parser as _;

use tally::cli::Cli;
use tally::core::{Report, Selector, tally_reader};
use tally::error::TallyError;

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing();

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        // パイプの下流が先に閉じた場合（`tally big.log | head`）。
        // これは異常ではないので、静かに成功終了する。Unix ツールの作法。
        Err(err) if is_broken_pipe(&err) => ExitCode::SUCCESS,
        Err(err) => {
            // `{:#}` は anyhow の context チェーンを 1 行に連結して表示する。
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
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

fn run(cli: &Cli) -> anyhow::Result<()> {
    let selector = cli.selector();
    // `--filter` 未指定なら全行を通す述語にする。core 側に `Option` を渡さないのは、
    // 「フィルタが無い」を分岐として core に持ち込まないため。
    let keep = |line: &str| cli.filter.as_ref().is_none_or(|re| re.is_match(line));

    let report = if let Some(path) = cli.input.as_deref() {
        tracing::debug!(path = %path.display(), "ファイルから読み込みます");
        read_file(path, &selector, keep, cli.limit)?
    } else {
        tracing::debug!("標準入力から読み込みます");
        let stdin = io::stdin();
        tally_reader(stdin.lock(), &selector, keep, cli.limit)
            .context("標準入力の集計に失敗しました")?
    };

    // stdout は行バッファリングされるため、大量出力では明示的に BufWriter で包む。
    // 包まないと 1 行ごとに write(2) が走り、数十倍遅くなることがある。
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    tally::format::write_report(&mut out, &report, cli.format)?;
    out.flush()?;

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

fn read_file<F>(
    path: &Path,
    selector: &Selector,
    keep: F,
    limit: Option<usize>,
) -> anyhow::Result<Report>
where
    F: Fn(&str) -> bool,
{
    let file = File::open(path).map_err(|source| TallyError::OpenInput {
        path: path.to_path_buf(),
        source,
    })?;
    tally_reader(BufReader::new(file), selector, keep, limit)
        .with_context(|| format!("{} の集計に失敗しました", path.display()))
}

/// エラーチェーンのどこかに `BrokenPipe` があるか。
///
/// `anyhow::Error::chain()` で原因を辿れる点が、`Box<dyn Error>` を手で扱うより楽なところ。
fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|io_err| io_err.kind() == io::ErrorKind::BrokenPipe)
    })
}
