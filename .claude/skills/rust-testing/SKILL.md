---
name: rust-testing
description: Use when writing, reviewing, or debugging Rust tests — unit vs integration placement, cargo nextest, assert_cmd CLI tests, insta snapshots, proptest property tests, doc tests, test fixtures with tempfile, flaky ordering, or coverage gaps. Fires on #[test], cargo test, assert_eq, tests/ directory, and "add tests for this".
---

# Rust のテスト

参照実装: `crates/tally/src/core.rs`（ユニット）、`crates/tally/tests/cli.rs`（統合）。

## どこに置くか

| 種類 | 場所 | 見えるもの | 用途 |
| --- | --- | --- | --- |
| ユニット | `src/**` の `#[cfg(test)] mod tests` | **非公開項目も見える** | ロジックの大半 |
| 統合 | `tests/*.rs` | 公開 API のみ。別クレートとしてコンパイル | 配線・境界 |
| ドキュメント | `///` の ```` ``` ```` ブロック | 公開 API | 使用例が腐らないことの保証 |

**ロジックはユニットテストに寄せる。** 統合テストはプロセス起動を伴い数百倍遅い。
`tally` の比率（ユニット 11 / 統合 8）が目安。

**統合テストに置くべきものは限られる**: 終了コード、stdout と stderr の分離、
引数の実配線。「純粋関数の入出力」を統合テストで検証しているなら、それは置き場所の誤り。

## 実行

```bash
cargo t                          # = cargo nextest run --workspace
cargo nextest run -p tally core  # 名前でフィルタ
cargo test --workspace --doc     # nextest はドキュメントテストを実行しない
```

**nextest はテストごとにプロセスを分ける。** そのため：
- テスト間で `static` の状態が漏れない（`cargo test` はスレッド共有で漏れる）
- 個々のテストの panic が他を巻き込まない
- **ただしドキュメントテストは走らない。** CI は両方回している。

## 決定性

**`HashMap` / `HashSet` の反復順に依存したテストは、いつか落ちる。**
Rust の `HashMap` はプロセスごとにシード（DoS 対策）が変わるため、順序は保証されない。

```rust
// core.rs: 同数のときのタイブレークを実装側で決めてある
entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
```

対応は 2 つ。**実装側で全順序を決める**（推奨。出力の再現性は利用者の利益でもある）か、
テスト側で集合として比較するか。前者を選んだ理由をコメントに残すこと。

その他の非決定要因: 現在時刻、乱数、環境変数、カレントディレクトリ、
並行実行時のファイルパス衝突（`tempfile` で回避する）。

## 内容を検証する

**「エラーになること」ではなく「どう失敗したか」を検証する。**

```rust
// ✗ 何が起きても通ってしまう
assert!(result.is_err());

// ✓ 種類と、ユーザに必要な情報（行番号）まで見る
let err = tally_reader(...).expect_err("2 行目で失敗するはず");
assert!(matches!(err, TallyError::InvalidJson { line_no: 2, .. }), "実際: {err:?}");
```

同様に、`assert!(output.contains("error"))` は弱すぎる。
**エラーメッセージが「修正に必要な情報」を含むこと自体がテスト対象。**

`assert!` の第 2 引数に実際の値を入れる。入れないと、落ちたときにログを見ても原因が分からない。

## CLI の統合テスト（assert_cmd）

```rust
// tests/ は #[cfg(test)] ではないため、clippy.toml の許可が効かない
#![allow(clippy::expect_used, clippy::unwrap_used)]

tally()
    .arg("--stats")
    .write_stdin("a\na\n")
    .assert()
    .success()
    .stdout("2\ta\n")                                  // 完全一致
    .stderr(predicate::str::contains("2 行を読み"));   // 部分一致
```

- **`Command::cargo_bin("tally")`** はビルド済みバイナリを解決する。パスを手で書かない。
- **stdout は完全一致で assert する。** これが「stdout を汚さない」という契約を守らせる。
  stderr は文言が変わりやすいので部分一致でよい。
- ファイルが要るときは `tempfile::NamedTempFile`。**固定パスを使わない**
  （並行実行で衝突し、CI だけで落ちる典型）。

## スナップショット（insta）

出力が構造的で、手で書くと大きすぎる場合に使う。

```rust
insta::assert_json_snapshot!(report);
```

```bash
cargo insta review   # 差分を対話的に承認
```

**乱用しない。** スナップショットは「何を保証しているか」がテスト名からしか分からない。
不変条件が明確なものは通常の `assert_eq!` で書くほうが良いテストになる。
`*.snap.new`（未承認）は `.gitignore` 済み、`*.snap`（承認済み）はコミットする。

## プロパティテスト（proptest）

**入出力に代数的な関係があるとき**に有効。

```rust
proptest! {
    #[test]
    fn 集計の総和は行数に一致する(lines in prop::collection::vec("[a-z]{1,5}", 0..100)) {
        let input = lines.join("\n");
        let report = tally_reader(input.as_bytes(), &Key::WholeLine, None).unwrap();
        let sum: u64 = report.entries.iter().map(|e| e.count).sum();
        prop_assert_eq!(sum as usize, report.total - report.skipped);
    }
}
```

探す不変条件: ラウンドトリップ（`parse(render(x)) == x`）、
既知の素朴実装との一致、順序不変性、冪等性、境界（空・1 件・重複のみ）。

## テスト名

このリポジトリでは **日本語で「何が保証されるか」** を書く（既存コードがそう）。
`test_tally` ではなく `同数のときはキーの昇順で安定する`。
**落ちたときにテスト名だけで何が壊れたか分かることが目的。**

## レビュー時のチェック

- [ ] 統合テストに置かれたロジックのテストがユニットに移せないか
- [ ] `is_err()` / `is_some()` で止まっていないか（中身を見ているか）
- [ ] `HashMap` の反復順に依存していないか
- [ ] 一時ファイルが固定パスでなく `tempfile` か
- [ ] 境界（空入力・1 件・全件同数・不正入力）が網羅されているか
- [ ] エラーパスのテストがあるか（成功系だけになっていないか）
- [ ] `assert!` に実際の値を出すメッセージが付いているか
