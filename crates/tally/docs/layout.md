# tally のコード配置

**このクレートのどこに何を置くか、なぜそこなのか** の正本。
コードを足す・動かす前にここを読む。

**ここに書くのはクレートの性質だけ。** 次のものは書かない。

- 進捗・現在地 → `docs/handoff.md`
- 学習の進め方、読む順序といった **学習者への指示** → `docs/curriculum.md`
- 他言語との差分などの **学びの内容** → `docs/learning-log.md`
- **集計そのものの配置** → `crates/tally-core/docs/layout.md`
- **利用者に対する出力の外部契約** → `docs/output-format.md`

---

## このクレートの境界

**`tally` は CLI の関心事だけを持つ。** 集計は `tally-core` にある。

| 関心事 | クレート |
| --- | --- |
| 引数の解釈、出力の整形、終了コード、hint、ログ、ファイルを開く | **`tally`** |
| キーの抽出、正規化、度数の集計、行に紐づく失敗 | **`tally-core`** |

**依存の向きは `tally` → `tally-core` の一方向。**
`tally-core` に `clap` や `anyhow` を持ち込む変更は入れない
（`cargo tree -p tally-core` で確かめられる）。

---

## ファイルの配置

| パス | 役割 | 置いてよいもの | 置いてはいけないもの |
| --- | --- | --- | --- |
| `src/cli.rs` | 引数定義 | `clap` の型、引数から `tally_core` の型への変換 | 集計ロジック、I/O |
| `src/format.rs` | 出力の整形 | `(Report, Format) → バイト列` の純粋関数 | 書き込み先の決定、終了コード |
| `src/error.rs` | CLI 固有の失敗 | `CliError`、終了コード、hint、チェーンの表示 | 集計の失敗の分類（`tally-core` 側） |
| `src/main.rs` | 実行の外枠 | 引数解釈の呼び出し、I/O、終了コード、ログ初期化 | **ロジック全般。** 純粋関数にできるものは lib へ |
| `src/lib.rs` | 公開面の宣言 | `pub mod`、クレートの doc | 実装 |
| `tests/cli.rs` | 統合テスト | 終了コード（clap 由来）、stdout と stderr の分離、引数の実配線 | 純粋関数の入出力検査 |
| `docs/` | このクレートの構造と外部契約 | 配置・設計判断・その理由、出力仕様 | 進捗、日付つきの記録 |

| `src/aggregate.rs` | 並列化ポリシー | ジョブの列の走らせ方、合流、失敗の選び方 | **ファイルを開く操作、`clap`、`regex`、整形** |

---

## 層（モジュール間で許される依存）

**下の層は上の層を知らない。** 根拠は
[ADR-0007](../../../docs/adr/0007-multi-input-aggregation.md) 論点 5。

| 層 | モジュール | 依存してよいもの | 依存してはいけないもの |
| --- | --- | --- | --- |
| 0 | （別クレート）`tally-core` | `std`、`thiserror`、`serde` | `clap`、`anyhow`、`rayon`、ファイルを開く操作 |
| 1 | `src/error.rs` | `std`、`thiserror`、`tally_core` | `clap`、`regex`、`rayon`、他の `tally` モジュール |
| 2 | **`src/aggregate.rs`** | `std`、`rayon`、`tally_core`、`crate::error` | **`clap`、`regex`、ファイルを開く操作、`crate::cli`、`crate::format`** |
| 3 | `src/cli.rs`、`src/format.rs` | `clap` / `regex` / `serde_json`、`tally_core`、`crate::error` | ファイルを開く操作、`rayon`、`crate::aggregate` |
| 4 | `src/main.rs` | すべて | — |

**層 2 の目的は「切り出せる状態を保つ」こと。** 並列化ポリシーは
入力の種類にも集計の中身にも依存しないので、
**`tally` 以外の消費者が現れたら別クレートへ移せる**（移行条件は ADR-0007 論点 5）。

**`scripts/check-module-deps.sh` がこの表を機械的に検査し、CI で回る。**
**限界がある** — コメント行は見ないので、doc コメント内のコード例は拾えない。

---

**判断に迷ったら**: それは「プロセスを起動しないと確かめられないこと」か。
そうでなければ純粋関数として lib 側に置き、ユニットテストで検査する。

**さらに問う**: それは CLI でなくても意味を持つか。
意味を持つなら `tally-core` の話であって、ここではない。

---

## テストが 2 箇所に分かれるのは、選択ではなく制約

| 種別 | 置き場所 | 見えるもの |
| --- | --- | --- |
| ユニット | `src/**` の `#[cfg(test)] mod tests` | **非公開項目を含む全て** |
| 統合 | `tests/*.rs` | **公開 API のみ** |

`tests/` 配下の各ファイルは **独立したクレートとしてコンパイルされ**、
このライブラリを外部依存として読み込む。したがって非公開項目は見えない。

再現できる。`tests/` に次を置いてビルドすると失敗する。

```rust
#[test]
fn 非公開項目に触れるか() {
    let _ = tally::format::quote_field("x");
}
```

```text
error[E0603]: function `quote_field` is private
```

**この制約はクレートを分けたあとも同じ。** `tally-core` の非公開項目
（`Key::extract` / `fold_case` / `snippet`）は `tally` からも見えない。
だから `tally-core` のテストは `tally-core` の中にある。

**テストのために `pub` を付けない。** 公開 API は semver の対象であり、
検査の都合で広げると約束が増える。検査したい対象が非公開なら、テストを同じモジュールに置く。

### 統合テストを書いてよい条件

**プロセスを起動しないと観測できないものだけ。** プロセス起動は遅く、
同じことをユニットで書けるなら常にそちらが正しい。

- **終了コードのうち clap が返すもの**（引数の誤り = `2`）。
  実行時エラーの終了コードは `CliError::exit_code` がユニットテストで網羅する
- stdout と stderr が混ざっていないこと（hint を含む）
- 引数が実際に配線されていること（**1 フラグ 1 件に絞る。**
  振る舞いはユニットテストが持っている）

なお `clippy.toml` の `allow-expect-in-tests` は `#[cfg(test)]` にしか効かない。
`tests/` 配下は通常のクレートなので、ファイル先頭で明示的に `allow` する。

---

## 押さえておくべき設計判断

壊さないために、変更前に理由を把握しておくもの。

| 判断 | 理由 |
| --- | --- |
| **`anyhow` を使わず CLI 固有のエラー型に一本化** | 不透明な型では「エラー → 終了コード」を網羅性検査つきの `match` で書けない。併用すると表現が 2 系統になる（[ADR-0004](../../../docs/adr/0004-error-type-shape.md) 論点 5、理由は `src/error.rs`） |
| **終了コードが純粋関数（`CliError::exit_code`）** | プロセスを起動せずに網羅できる。`main` に述語を散らすと、網羅数がプロセス起動数に比例し、書き忘れが検出されない |
| **終了コード `2` は clap 側に残る** | `Cli::parse()` が `run` より前に返すので `exit_code` を通らない。**プロセスを起動しないと観測できない唯一の終了コード** |
| **hint が型のフィールドで、既定値を持つコンストラクタが無い** | 「この失敗に対して利用者は何ができるか」を打鍵時に問わせる。既定化すると価値がゼロになる |
| **hint は stderr にのみ、`error:` とは別の行に出す** | stdout はデータ専用。`error:` 行を機械的に拾う利用者を壊さない |
| **`CliErrorKind::Input` が入力の名前を前置する** | 行番号は入力ごとに 1 から数えるので、**名前が無いと行番号まで意味を失う**（[ADR-0007](../../../docs/adr/0007-multi-input-aggregation.md) 論点 3）。段階 5 の `Tally(#[error(transparent)])` は、`TallyError::OpenInput` と path が二重になるのを避けた形だった |
| **開けなかった失敗（`CliErrorKind::Open`）が CLI にある** | `tally_core` はファイルを開かない。**開く主体が名前を知っている**（ADR-0007 論点 3。段階 5 までは `tally_core` 側にあった） |
| **`InputName` が `Option<PathBuf>` ではない** | 標準入力には path が無い。`None` で表すと「持つが空」の状態が生まれ、表示の場合分けが呼び出し側に漏れる |
| `Format` に `clap::ValueEnum` を derive | enum を 2 つ持って同期させるほうが害が大きい（同じ事実が 2 箇所になる） |
| `Format` を trait にしていない | [ADR-0002](../../../docs/adr/0002-output-format-abstraction.md)。形式ごとの本体が「概ね 20 行超」かつ「操作が 2 つ以上」になったら再評価する |
| `write_report` が `impl Write` を受ける | `Vec<u8>` に書いて内容を検証できる。`Stdout` 固定だと差し替えの仕掛けが要る |
| `quote_field` が関数に切られている | 引用規則は CSV の外部契約そのもの。境界値（`,` `"` CR LF）を関数単位で検査したい |
| **正規表現は `regex` クレート**（`fancy-regex` を採らない） | `--filter` は利用者の任意入力を受ける。先読み・後方参照と引き換えに入力長への線形時間を失うと、ReDoS が成立しうる。`regex-lite` はバイナリは小さいが実行が遅く、この用途で得がない |
| `--filter` を `Option<String>` でなく `Option<Regex>` で受ける | 不正な正規表現を引数解釈の時点（終了コード 2）で拒否する。入力を読み始めてから失敗させない |
| `--strict` が clap の `requires = "field"` を持つ | `--field` 無しでは意味を持たない。実行時に黙って無視するより引数解釈で拒否する |
| **`--strict` が `Cli::counter()` に乗り、`Cli::selector()` には乗らない** | 「産まなかったときどうするか」は消費者の方針（[ADR-0005](../../../docs/adr/0005-selector-public-api.md) 論点 1） |
| `main` が `ExitCode` を返す | `Result` を返すとエラーが `Debug` 表示になり読めない |
| **書き出し後に明示的に `flush()` する** | 忘れると `BufWriter` の drop 時に落ち、失敗が捨てられる。パイプ切断はここで初めて観測されることが多い |
| stdout はデータ専用 | 統合テストが stdout の完全一致を検査しており、この契約を守らせている |

---

## 新しいコードをどこに置くか

上から順に問う。**最初に「はい」になったところが置き場所。**

1. **CLI でなくても意味を持つか**（集計・抽出・行の失敗）→ **`tally-core`**
2. **プロセスを起動しないと観測できないか** → `tests/cli.rs`
3. **引数の形の話か** → `src/cli.rs`
4. **出力の整形か** → `src/format.rs`
5. **CLI 固有の失敗・終了コード・hint か** → `src/error.rs`
6. **I/O・ログの初期化か** → `src/main.rs`

6 に落ちた場合、**純粋な部分を切り出せないか**を先に検討する。
`main.rs` にロジックが溜まると、検証手段がプロセス起動しか無くなる。
