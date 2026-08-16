//! CLI の統合テスト。
//!
//! ここでしか検証できないこと（終了コード・stdout と stderr の分離・引数の実配線）
//! だけを置く。プロセス起動は遅いため。
//!
//! **ロジックのテストは `src/**` の `#[cfg(test)] mod tests` に置く**
//! （`src/core.rs` と `src/cli.rs`）。このファイルからは非公開項目が見えないので、
//! そもそも書けない。理由は `docs/layout.md` の
//! 「テストが 2 箇所に分かれるのは、選択ではなく制約」を参照。
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

#[test]
fn field_指定で_json_の値を集計する() {
    let input = "{\"lvl\":\"info\"}\n{\"lvl\":\"error\"}\n{\"lvl\":\"info\"}\n";
    tally()
        .args(["--field", "lvl"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("2\tinfo\n1\terror\n");
}

#[test]
fn 存在しないファイルは失敗して原因を示す() {
    tally()
        .arg("/definitely/not/here.log")
        .assert()
        .failure()
        .stderr(predicate::str::contains("入力を読めません"));
}

#[test]
fn 壊れた_json_は行番号を示して失敗する() {
    tally()
        .args(["--field", "lvl"])
        .write_stdin("{\"lvl\":\"info\"}\nnot json\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("2 行目"));
}

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

#[test]
fn ignore_case_で大文字小文字をまとめて集計する() {
    tally()
        .arg("--ignore-case")
        .write_stdin("Info\nINFO\nwarn\ninfo\n")
        .assert()
        .success()
        .stdout("3\tinfo\n1\twarn\n");
}

#[test]
fn ignore_case_なしでは大文字小文字が分かれる() {
    tally()
        .write_stdin("Info\ninfo\n")
        .assert()
        .success()
        .stdout("1\tInfo\n1\tinfo\n");
}

#[test]
fn ignore_case_は_json_の値にも効く() {
    tally()
        .args(["--field", "lvl", "-i"])
        .write_stdin("{\"lvl\":\"INFO\"}\n{\"lvl\":\"info\"}\n")
        .assert()
        .success()
        .stdout("2\tinfo\n");
}

#[test]
fn strict_はフィールド欠損で失敗し行番号と抜粋を示す() {
    tally()
        .args(["--field", "lvl", "--strict"])
        .write_stdin("{\"lvl\":\"info\"}\n{\"other\":1}\n")
        .assert()
        // 完了条件は「終了コードが 1」。failure() は非ゼロしか見ないので code(1) を使う。
        .code(1)
        .stderr(predicate::str::contains("2 行目"))
        .stderr(predicate::str::contains("other"));
}

#[test]
fn strict_なしでは欠損行をスキップして成功する() {
    tally()
        .args(["--field", "lvl"])
        .write_stdin("{\"lvl\":\"info\"}\n{\"other\":1}\n")
        .assert()
        .success()
        .stdout("1\tinfo\n");
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

// --- 段階 4: --filter ---

/// **上流で絞ってから渡した場合と、`--filter` で絞った場合の出力が一致する。**
///
/// `grep` を実際に起動して比べると、実装（BSD / GNU）と正規表現の方言に
/// 依存したテストになる。ここで確かめたいのは `tally` 側の性質
/// 「フィルタは集計の上流にある」なので、**入力を手で絞ったものと突き合わせる。**
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

/// フィルタで行が落ちても、エラーは **入力ファイルの行番号** を指す。
///
/// ここがずれると、利用者はエラーを見ても該当行を開けない。
#[test]
fn filter_で落ちた行があってもエラーは入力の行番号を指す() {
    // 3 行目が JSON として壊れている。2 行目はフィルタで落ちる。
    let input = "{\"lvl\":\"info\"}\nDROP ME\nnot json\n";
    tally()
        .args(["--field", "lvl", "--filter", "^[^D]"])
        .write_stdin(input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("3 行目"));
}

/// 壊れた正規表現は **集計を始める前に** 引数の誤りとして拒否される。
///
/// 終了コード 2 は clap の規約。入力を読んでから失敗すると、
/// パイプの上流が無駄に走る。
#[test]
fn 壊れた正規表現は終了コード_2_で拒否される() {
    tally()
        .args(["--filter", "["])
        .write_stdin("a\n")
        .assert()
        .code(2);
}
