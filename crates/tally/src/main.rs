//! バイナリ本体。ロジックは持たず、境界の面倒だけを見る。

use std::fs::File;
use std::io::{self, BufReader, IsTerminal, Write};
use std::num::NonZeroUsize;
use std::process::ExitCode;

use clap::Parser as _;

use tally::aggregate::{Execution, aggregate_all};
use tally::cli::Cli;
use tally::error::{CliError, CliErrorKind, EXIT_OK, InputName};
use tally::{error, format};
use tally_core::{Counter, Report, TallyError, tally_reader};

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
    let report = aggregate(cli)?;

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

/// 集計の失敗に入力の名前を被せる。
///
/// **包む責任は呼び出し側にある**（ADR-0007 論点 3）。包み忘れても型検査は通るので、
/// **`tally_reader` を呼ぶ経路をこの関数 1 つに集約する。**
///
/// **hint もここで打つ。** `From<TallyError> for CliError` を実装して `?` に任せると、
/// 「この失敗に対して利用者は何ができるか」を問う瞬間が消える（ADR-0004 論点 6）。
fn input_error(name: InputName, source: TallyError) -> CliError {
    let hint = error::hint_for(&source);
    CliError::new(CliErrorKind::Input { name, source }, hint)
}

/// 引数から実行戦略を決める。
///
/// **この対応づけは `cli` にも `aggregate` にも置けない。** `cli` は層 3 で
/// `aggregate` を知らず、`aggregate` は層 2 で `clap` を知らない
/// （`crates/tally/docs/layout.md` の層の表）。**繋ぐのは最上層の仕事。**
fn execution_of(cli: &Cli) -> Execution {
    match cli.jobs.map(NonZeroUsize::get) {
        // **1 は「スレッド 1 本の並列」ではなく逐次。** rayon を経由しない経路を
        // 通すことに意味がある（検証経路として `-j 1` を残した理由）。
        Some(1) => Execution::Sequential,
        _ => Execution::Parallel { threads: cli.jobs },
    }
}

/// 入力を開いて集計する。**I/O の面倒はここに閉じる。**
fn aggregate(cli: &Cli) -> Result<Report, CliError> {
    let selector = cli.selector();
    // `--filter` 未指定なら全行を通す述語にする。core 側に `Option` を渡さないのは、
    // 「フィルタが無い」を分岐として core に持ち込まないため。
    let keep = |line: &str| cli.filter.as_ref().is_none_or(|re| re.is_match(line));

    let counter = if cli.inputs.is_empty() {
        tracing::debug!("標準入力から読み込みます");
        // **標準入力は並列化しない。** 単位が 1 つしかなく、`StdinLock` は
        // 複数のジョブへ分けられない。
        let mut counter = cli.counter();
        let stdin = io::stdin();
        tally_reader(&mut counter, stdin.lock(), &selector, keep)
            .map_err(|source| input_error(InputName::Stdin, source))?;
        counter
    } else {
        // **ジョブは「1 ファイルを開いて集計する」閉包。**
        // 並列化ポリシー（`aggregate` モジュール）はファイルを知らないので、
        // ここで閉じ込める（ADR-0007 論点 5）。
        let jobs: Vec<_> = cli
            .inputs
            .iter()
            .map(|path| {
                let selector = &selector;
                let keep = &keep;
                move || -> Result<Counter, CliError> {
                    tracing::debug!(path = %path.display(), "ファイルから読み込みます");
                    // **開くのはここ。** `tally_core` はファイルを開かないので、
                    // 開けなかった失敗も CLI の型になる（ADR-0007 論点 3）。
                    let file = File::open(path).map_err(|source| {
                        // 開けない理由（パス・権限）に対して、`tally` の使い方を変えて
                        // できることは無い。だから hint は `None`。
                        CliError::new(
                            CliErrorKind::Open {
                                path: path.clone(),
                                source,
                            },
                            None,
                        )
                    })?;
                    let mut counter = cli.counter();
                    tally_reader(&mut counter, BufReader::new(file), selector, keep)
                        .map_err(|source| input_error(InputName::Path(path.clone()), source))?;
                    Ok(counter)
                }
            })
            .collect();

        aggregate_all(&jobs, || cli.counter(), execution_of(cli))?
    };

    // **順位づけはここで初めて起きる**（ADR-0007 論点 2）。`tally_reader` が返すのは
    // 順位づけ前の状態で、`limit` はその切り詰めなので `report` が持つ。
    // **失敗したときはここに来ない。** 途中まで積まれた `counter` は `?` とともに捨てられる。
    Ok(counter.report(cli.limit))
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
