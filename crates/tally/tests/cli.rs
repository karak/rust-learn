//! CLI の統合テスト。
//!
//! **ここに置いてよいのは、プロセスを起動しないと観測できないものだけ。**
//!
//! - 終了コードのうち **clap が返すもの**（引数の誤り = `2`）
//! - stdout と stderr が混ざっていないこと
//! - 引数が実際に配線されていること
//!
//! **終了コードの決定規則そのものはここでは網羅しない。**
//! `src/error.rs` の `CliError::exit_code` がユニットテストで網羅している
//! （ADR-0004 論点 5）。プロセス起動の数に比例させると遅く、しかも
//! **「テストを書き忘れた」ことが検出されない。**
//!
//! **ロジックのテストは各クレートの `#[cfg(test)] mod tests` に置く**
//! （`tally-core` の `select` / `count` / `error`、`tally` の `cli` / `format`）。
//! このファイルからは非公開項目が見えないので、そもそも書けない。理由は
//! `docs/layout.md` の「テストが 2 箇所に分かれるのは、選択ではなく制約」を参照。
//!
//! `clippy.toml` の `allow-expect-in-tests` は `#[cfg(test)]` モジュールにしか効かない。
//! `tests/` 配下は通常のクレートとしてコンパイルされるため、ここで明示的に許可する。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::io::Write as _;

use assert_cmd::Command;
use predicates::prelude::*;

fn tally() -> Command {
    Command::cargo_bin("tally").expect("バイナリがビルドされているはず")
}

fn fixture(contents: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().expect("一時ファイルを作れるはず");
    file.write_all(contents.as_bytes()).expect("書き込めるはず");
    file.flush().expect("flush できるはず");
    file
}

// --- 入力の配線 ---

#[test]
fn 標準入力を集計してタブ区切りで出す() {
    tally()
        .write_stdin("a\nb\na\n")
        .assert()
        .success()
        .stdout("2\ta\n1\tb\n");
}

#[test]
fn ファイル引数を集計する() {
    let file = fixture("x\ny\nx\nx\n");
    tally()
        .arg(file.path())
        .assert()
        .success()
        .stdout("3\tx\n1\ty\n");
}

// --- 引数の配線 ---
//
// 各フラグの**振る舞い**は tally-core と cli.rs のユニットテストが持っている。
// ここで見るのは「clap の値が実際に届いているか」だけなので、1 フラグ 1 件に絞る。

#[test]
fn field_指定で_json_の値を集計する() {
    tally()
        .args(["--field", "lvl"])
        .write_stdin("{\"lvl\":\"info\"}\n{\"lvl\":\"error\"}\n{\"lvl\":\"info\"}\n")
        .assert()
        .success()
        .stdout("2\tinfo\n1\terror\n");
}

#[test]
fn ignore_case_が配線されている() {
    tally()
        .arg("--ignore-case")
        .write_stdin("Info\nINFO\nwarn\ninfo\n")
        .assert()
        .success()
        .stdout("3\tinfo\n1\twarn\n");
}

#[test]
fn limit_で上位だけに絞る() {
    tally()
        .args(["-n", "1"])
        .write_stdin("a\na\nb\n")
        .assert()
        .success()
        .stdout("2\ta\n");
}

#[test]
fn json_出力は機械可読な形になる() {
    tally()
        .args(["--format", "json"])
        .write_stdin("a\na\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\": 2"))
        .stdout(predicate::str::contains("\"total\": 2"));
}

/// CSV にヘッダ行が無いことを、**stdout の完全一致**で検査する。
///
/// 部分一致では「余計な行が出ていない」ことを言えない。ヘッダの有無は
/// [ADR-0003] が定めた利用者への契約なので、ここは完全一致で押さえる。
///
/// [ADR-0003]: ../../../docs/adr/0003-csv-output-contract.md
#[test]
fn csv_出力はヘッダ行を持たない() {
    tally()
        .args(["--format", "csv"])
        .write_stdin("a\na\nb\n")
        .assert()
        .success()
        .stdout("2,a\n1,b\n");
}

/// **上流で絞ってから渡した場合と、`--filter` で絞った場合の出力が一致する。**
///
/// `grep` を実際に起動して比べると、実装（BSD / GNU）と正規表現の方言に
/// 依存したテストになる。ここで確かめたいのは `tally` 側の性質
/// 「フィルタは集計の上流にある」なので、**入力を手で絞ったものと突き合わせる。**
///
/// **バイト単位の一致を見るのでここでしか書けない。** `total` と `skipped` を
/// 含む出力全体が一致することが `output-format.md` の契約である。
#[test]
fn filter_の結果は事前に絞った入力と一致する() {
    let full = "info: a\nwarn: b\ninfo: c\ndebug: d\ninfo: a\n";
    let prefiltered = "info: a\ninfo: c\ninfo: a\n";

    let with_filter = tally()
        .args(["--filter", "^info", "--format", "json"])
        .write_stdin(full)
        .assert()
        .success();
    let without_filter = tally()
        .args(["--format", "json"])
        .write_stdin(prefiltered)
        .assert()
        .success();

    assert_eq!(
        with_filter.get_output().stdout,
        without_filter.get_output().stdout,
        "--filter は上流で絞るのと同じでなければならない（total と skipped も含めて）"
    );
}

// --- stdout と stderr の分離 ---

#[test]
fn stats_は標準出力を汚さず標準エラーに出る() {
    tally()
        .arg("--stats")
        .write_stdin("a\na\n")
        .assert()
        .success()
        // stdout は集計結果だけ。パイプで繋いだ先が壊れないことの保証。
        .stdout("2\ta\n")
        .stderr(predicate::str::contains("2 行を集計し"));
}

/// 診断は stderr にのみ出る。**stdout は 1 バイトも出ない。**
///
/// 集計が失敗した時点で書き出しに進まないので、部分的な結果も残らない。
#[test]
fn 失敗時の診断は_stdout_に混ざらない() {
    tally()
        .args(["--field", "lvl"])
        .write_stdin("{\"lvl\":\"info\"}\nnot json\n")
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("2 行目"));
}

/// hint は **stderr にのみ** 出る。
///
/// **2 方向とも見る**（ADR-0004 の Confirmation）。
///
/// - stdout の**完全一致**で「混ざっていない」ことを言う
/// - stderr の**部分一致**で「実際に出ている」ことを言う
///
/// 片方だけでは足りない。stdout だけ見れば「hint を実装し忘れた」でも通り、
/// stderr だけ見れば「stdout にも出した」でも通る。
#[test]
fn hint_は_stderr_にのみ出る() {
    tally()
        .args(["--field", "lvl", "--strict"])
        .write_stdin("{\"lvl\":\"info\"}\n{\"other\":1}\n")
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("hint: "))
        .stderr(predicate::str::contains("--strict を外す"));
}

/// **示唆の無い失敗では `hint:` 行を出さない。**
///
/// 既定を `None` にした意味がここにある。「JSON を直せ」のような
/// 情報量ゼロの助言で診断を埋めない（ADR-0004 論点 6 の「範囲を絞る」）。
#[test]
fn 示唆の無い失敗では_hint_行が出ない() {
    tally()
        .args(["--field", "lvl"])
        .write_stdin("not json\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("hint:").not());
}

/// 入力を開けない場合は、**どのパスを開けなかったか**を出す。
///
/// `anyhow` の `.context()` を落としたので、path はエラー型
/// （`CliErrorKind::Open`。ADR-0007 論点 3 で `tally_core` から移した）が
/// 持っている。ここはその配線の確認。
#[test]
fn 存在しないファイルは失敗して原因とパスを示す() {
    tally()
        .arg("/definitely/not/here.log")
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("入力を読めません"))
        .stderr(predicate::str::contains("/definitely/not/here.log"));
}

/// 読み取り途中で失敗した場合も、**どの入力かが出る。**
///
/// **包む責任は呼び出し側にある**（ADR-0007 論点 3 が 3b に負けている軸）。
/// 包み忘れても型検査は通るので、ここで押さえる。
/// **行番号は入力ごとに 1 から数える**ため、名前が無いと行番号まで意味を失う。
#[test]
fn 集計に失敗した入力の名前が診断に出る() {
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れるはず");
    let path = dir.path().join("broken.log");
    std::fs::write(&path, "{\"lvl\":\"info\"}\n{\"other\":1}\n").expect("書けるはず");

    tally()
        .args(["--field", "lvl", "--strict"])
        .arg(&path)
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("broken.log の集計に失敗しました"))
        .stderr(predicate::str::contains("2 行目"));
}

/// 標準入力の失敗にも呼び名が付く。**`Option<PathBuf>` の `None` にしない。**
#[test]
fn 標準入力の失敗は標準入力という名前で出る() {
    tally()
        .args(["--field", "lvl", "--strict"])
        .write_stdin("{\"other\":1}\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("標準入力 の集計に失敗しました"));
}

// --- 複数入力（ADR-0007） ---

/// 複数のファイルを 1 つの集計に合流させる。
#[test]
fn 複数のファイルが_1_つの集計に合流する() {
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れるはず");
    let first = dir.path().join("first.log");
    let second = dir.path().join("second.log");
    std::fs::write(&first, "info\nwarn\n").expect("書けるはず");
    std::fs::write(&second, "info\n").expect("書けるはず");

    tally()
        .arg(&first)
        .arg(&second)
        .assert()
        .success()
        .stdout("2\tinfo\n1\twarn\n");
}

/// **引数の順序を入れ替えても、同じバイト列が出る**（段階 6 の完了条件）。
#[test]
fn 入力の順序は出力を変えない() {
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れるはず");
    let first = dir.path().join("a.log");
    let second = dir.path().join("b.log");
    std::fs::write(&first, "x\ny\n").expect("書けるはず");
    std::fs::write(&second, "y\n").expect("書けるはず");

    let forward = tally().arg(&first).arg(&second).assert().success();
    let backward = tally().arg(&second).arg(&first).assert().success();

    assert_eq!(
        forward.get_output().stdout,
        backward.get_output().stdout,
        "順序で出力が変わってはいけない"
    );
}

/// **複数が失敗したときは、引数順で最初のものを報告する**（ADR-0007 論点 4）。
///
/// 並列に回すと「最初に見つかった失敗」は実行ごとに変わりうる。
/// 規則が引数順であることを示すため、**順序を入れ替えると報告も入れ替わる**ことまで見る。
#[test]
fn 複数が失敗したときは引数順で最初のものを報告する() {
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れるはず");
    let first = dir.path().join("first.log");
    let second = dir.path().join("second.log");
    std::fs::write(&first, "{\"other\":1}\n").expect("書けるはず");
    std::fs::write(&second, "{\"other\":2}\n").expect("書けるはず");

    tally()
        .args(["--field", "lvl", "--strict"])
        .arg(&first)
        .arg(&second)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("first.log"))
        .stderr(predicate::str::contains("second.log").not());

    tally()
        .args(["--field", "lvl", "--strict"])
        .arg(&second)
        .arg(&first)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("second.log"))
        .stderr(predicate::str::contains("first.log").not());
}

/// `-j 1` は逐次実行。**検証経路として残している**（結果は並列と一致する）。
#[test]
fn jobs_1_でも同じ結果になる() {
    let dir = tempfile::tempdir().expect("一時ディレクトリを作れるはず");
    let path = dir.path().join("a.log");
    std::fs::write(&path, "a\nb\na\n").expect("書けるはず");

    let parallel = tally().arg(&path).assert().success();
    let sequential = tally().args(["-j", "1"]).arg(&path).assert().success();

    assert_eq!(parallel.get_output().stdout, sequential.get_output().stdout);
}

/// `-j 0` は **引数の誤り**として集計前に拒否する。
#[test]
fn jobs_0_は終了コード_2_で拒否される() {
    tally()
        .args(["-j", "0"])
        .write_stdin("a\n")
        .assert()
        .code(2);
}

// --- clap が返す終了コード ---

/// 壊れた正規表現は **集計を始める前に** 引数の誤りとして拒否される。
///
/// **終了コード `2` は `CliError::exit_code` を通らない。** `Cli::parse()` が
/// `run` より前に返すので、clap 側に残る。**プロセスを起動しないと観測できない
/// 唯一の終了コードであり、ここに置く理由がある。**
#[test]
fn 壊れた正規表現は終了コード_2_で拒否される() {
    tally()
        .args(["--filter", "["])
        .write_stdin("a\n")
        .assert()
        .code(2);
}

/// `--strict` は `--field` を伴わないと clap が拒否する。これも終了コード `2`。
#[test]
fn field_なしの_strict_は終了コード_2_で拒否される() {
    tally().args(["--strict"]).assert().code(2);
}
