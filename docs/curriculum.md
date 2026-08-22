# 学習ロードマップ

**Rust 固有の意思決定** に絞った順序。
各段階に「`tally` に加える機能」を割り当ててあり、読むだけで終わらないようにしてある。

各段階の完了条件は共通して:
`cargo fmt --all` / `cargo lint`（`pedantic` 込み）/ `cargo t` が通ること。

---

## この文書に書くもの / 書かないもの

**この文書は「予」だけを持つ。** 何をどう学ぶか、何ができたら完了か。

**「実」は `docs/stage-log.md` にある** — 実際に何を作り、何を測り、
予定と何が違ったか。2026-08-19 に分離した。
分離の理由と、それまでに実際に腐っていた記述は `stage-log.md` の冒頭にある。

| 書かないもの | 正しい場所 |
| --- | --- |
| **どの段階が完了したか** | `docs/stage-log.md`（節が存在する段階が完了した段階） |
| **実装の記録・実測値・予定との差** | `docs/stage-log.md` |
| いまどこにいるか・次に何をするか | `docs/handoff.md` |
| 進行中のチェックリストの状態 | `docs/handoff.md` |
| **テスト名・テスト件数** | `cargo t` / `cargo nextest list` が答える |
| 技術的な学び | `docs/learning-log.md` |
| 設計判断の過程・代替案 | `docs/adr/` |
| コードの配置ルール | そのクレートの `docs/layout.md` |

**完了条件はチェックボックスで書かない。** 状態を持たせると、
この文書が予実管理メモに戻る。**「満たされているべき条件」として書く。**

---

## 段階 0: 現状の把握

`crates/tally` と `crates/tally-core` は動く CLI とライブラリとして完成している。
まずこれを読む。

**読む順序**（ロジックから読み、I/O を最後に読む。この順で読めることが構成の目的でもある）。
**`tally-core` を先に読む。**

1. `crates/tally-core/src/lib.rs` — 公開面の宣言と、3 つの関心事の分け方
2. `crates/tally-core/src/select.rs` — キーの抽出と正規化
3. `crates/tally-core/src/count.rs` — 度数の集計と `tally_reader`
4. `crates/tally-core/src/error.rs` — 失敗の型と、`Display` / `source()` の合成規則
5. `crates/tally/src/cli.rs` — 引数定義
6. `crates/tally/src/format.rs` — 出力の整形
7. `crates/tally/src/error.rs` — CLI 固有の失敗、終了コード、hint
8. `crates/tally/src/main.rs` — I/O とログのみ
9. `crates/tally/tests/cli.rs` — 統合テスト

各ファイルに何を置いてよいかは、**そのクレートの** `docs/layout.md` を参照
（`crates/tally-core/docs/layout.md` と `crates/tally/docs/layout.md`）。

**問い**:

- `Key::extract` の戻り値が `Result<Option<Cow<str>>, LineErrorKind>` である理由を、
  4 つの型それぞれについて説明できるか。
  **`LineErrorKind` であって `LineError` でないのはなぜか。**
- `report()` で同数時のタイブレークを決めているのはなぜか。決めないと何が壊れるか。
- `main` が `Result` でなく `ExitCode` を返しているのはなぜか。
- **`tally-core` が `clap` に依存していないことを、どうやって確かめるか。**

---

## 段階 1: 所有権・借用・ライフタイム

**扱う概念**: move / `Copy`、`&` と `&mut` の排他性、ライフタイム注釈が
「関係の記述」であること、`Cow`、スライスと `Vec` の関係。

**課題**: `tally` に `--ignore-case` を足す。
`Cow` を維持したまま実装すること（大文字小文字が同じなら借用のまま返す）。
安易に全行 `to_lowercase()` すると、この課題の意味が消える。

**確認**: `cargo lint` を通し、アロケーションが増えないことを説明できる。

**参照**: `.claude/skills/rust-ownership-coach/`

---

## 段階 2: エラーモデル

**扱う概念**: `Result` と `?` の変換、`From`、`thiserror` の使い方、
エラーチェーン、`io::ErrorKind`。

**課題**: `--strict` フラグを足す。既定ではスキップしている
「フィールドが無い行」を、`--strict` 時はエラーにする。
エラーには行番号と、その行の先頭 40 文字を含める。

**確認**: 統合テストで終了コードと stderr の文言を検証する。

**参照**: `.claude/skills/rust-error-handling/`

### 完了条件

- `--strict` 指定時、キーを取り出せない行で**失敗し、終了コードが 1** になる
- エラーメッセージに **行番号** と **その行の先頭 40 文字** が含まれる
- 抜粋が**マルチバイト文字の途中で切れても panic しない**
- 抜粋に含まれる**制御文字がそのまま端末へ出ない**（ログインジェクション対策）
- **`--strict` 未指定時の挙動が変わらない**（既存テストが無修正で通る）
- `cargo fmt --all --check` / `cargo lint` / `cargo t` が通る

---

## 段階 3: トレイトとジェネリクス

**扱う概念**: トレイト境界、`impl Trait`（引数位置と戻り値位置の違い）、
`dyn Trait` と単相化のトレードオフ、関連型 vs 型パラメータ、
孤児ルール（orphan rule）、`From`/`Into`/`AsRef`/`Deref`。

**課題**: 出力形式を `Format` の `enum` + `match` から、
`trait Formatter { fn write(&self, out: &mut dyn Write, report: &Report) -> Result<()> }`
に切り替える。CSV 形式を追加する。

**問い**: `Box<dyn Formatter>` と `impl Formatter` のどちらを選ぶか。
この場面での動的ディスパッチのコストは実測でどれくらいか。
**enum + match のままのほうが良い可能性も検討すること**（Rust では
「トレイトにすべき」が常に正解ではない）。

### 完了条件

- **整形が `main.rs` から lib 側へ移り、`main.rs` にロジックが残らない**
- 各形式が **`Vec<u8>` への書き込みでユニットテストされている**（内容を検証する）
- **CSV 形式が追加**され、区切り文字・引用符・改行を含む値のエスケープが検査されている。
  **仕様は `crates/tally/docs/output-format.md` が正本**（ヘッダ行なし・LF・必要時のみ引用）
- **3 案を実際に書いて比較した記録がある**
  （A: `enum` + `match` / C: `Box<dyn Formatter>` / E: trait + 静的 dispatch）。
  **比較の基準は [ADR-0002](adr/0002-output-format-abstraction.md) の「確認方法」が正本。**
  形式を 1 つ足す差分・操作を 1 つ足す差分の行数と、
  **どの誤りがコンパイルエラーになるか** を見る
- **実行時コストを実測した記録がある。** ただし出力は 1 プロセス 1 回であり、
  **採否の根拠には使わない。** これは `dyn` と単相化の差を体感するための学習項目
- 採らなかった案を削り、**採らなかった理由が ADR に残っている**
- ADR-0002 のステータスが `accepted` になっている
- `cargo fmt --all --check` / `cargo lint` / `cargo t` が通る

**この段階の主眼は「trait に切り替えること」ではなく「切り替えるべきか判断できること」。**
enum のままを選ぶ結論もありうる。その場合も 3 案を書いた記録を残す。

**測り方の注意**: 実行ファイル全体のサイズや asm の総行数では案を判別できない。
lib には案が全て入り、LTO 無しではリンク量が変わらない。
**当該関数の機械語を読む。**

---

## 段階 4: イテレータとクロージャ

**扱う概念**: `Iterator` の遅延性、アダプタの合成、`FnOnce`/`FnMut`/`Fn` の違い、
`collect::<Result<Vec<_>, _>>()` の挙動、自前の `Iterator` 実装。

**課題**: `--filter <REGEX>` を足す。マッチする行だけを集計する。
実装は `tally_reader` のループではなく、**イテレータアダプタの合成** で表現する。

**設計上の制約**: `--filter` は `Selector` に入れない。理由は
`crates/tally-core/docs/layout.md` の「押さえておくべき設計判断」を参照。

**問い**: `for` ループ版とアダプタ版で、生成されるコードに差は出るか。
**段階 3 の測り方の注意を繰り返さないこと** — 当該関数の機械語を読む。
**陽性対照を用意する**（意図的に違う版を検出できるか確かめる）。

**着手前に決めること**: `--filter` と `total` / `skipped` の関係は
**利用者への契約**になる。`crates/tally/docs/output-format.md` に先に書く。

### 完了条件

- **`--filter <REGEX>` が動き、`Selector` には入っていない**
- **集計が `for` ループではなくイテレータアダプタの合成で書かれている**
- **`collect::<Result<Vec<_>, _>>()` を使っていない理由を説明できる。**
  短絡はするが成功分をすべて確保するため、行指向のストリーム処理では
  入力サイズのメモリを食う。**遅延性を壊さない畳み込みを選ぶ**
- **`Iterator<Item = Result<T, E>>` に対して、エラーを落とさずに
  中身だけで判定するフィルタが書けている**（`Err` は必ず下流へ流す）
- **`tally --filter X` と `grep X | tally` の出力一致が統合テストで検査されている**
- **フィルタで落ちた行があっても、エラーの行番号が入力の行番号と一致する**
  ことがテストされている
- **正規表現クレートの選定理由が記録されている**（代替候補と却下理由）。
  `cargo deny check` が通る
- **`for` ループ版とアダプタ版の生成コードを比較した記録がある**
- `cargo fmt --all --check` / `cargo lint` / `cargo t` が通る

---

## 段階 5: モジュール・クレート設計

**扱う概念**: `pub` / `pub(crate)` / `unreachable_pub`、モジュール階層と可視性、
ワークスペース分割、feature flag、セマンティックバージョニングと破壊的変更、
ドキュメントコメントとドキュメントテスト。

**課題**: 集計コアを `crates/tally-core` として別クレートに切り出す。
`tally` はそれに依存する薄い CLI にする。
`tally-core` の公開 API 全てにドキュメントコメントと **動くサンプル** を書く
（`cargo test --doc` で検証される）。

**確認**: `cargo doc` で読んで、外部の利用者が使えるか判断する。

### 着手前に決めること

**この段階には「切り出したら手遅れになるもの」が含まれる。**
公開 API の形は、切り出した時点で契約になる。**順序を守る。**

1. **公開する型の形を、切り出す前に ADR として決める。**
   `#[non_exhaustive]` の後付けは破壊的変更である。
   実際に決めたのは [ADR-0004](adr/0004-error-type-shape.md)（エラー型の形）と
   [ADR-0005](adr/0005-selector-public-api.md)（`Selector` と `Key` の公開 API）
2. **口頭で合意したことを文書化せずに持ち越さない。**
   段階 5 では実際にずれた —
   口頭で「`#[non_exhaustive]` + `Selector::new(key)`」まで合意していたが、
   ADR-0005 論点 2 は非公開フィールド + ビルダーを採り、
   `#[non_exhaustive]` は冗長として付けないと決めた
3. **「決定済み」は「実装が一意に決まる」ではない。**
   ADR を読み直して「これで実装が一意に決まるか」を問う工程を入れる
   （入れなかった結果は `docs/stage-log.md` 段階 5）

### 完了条件

**切り出し**

- `crates/tally-core` が独立したクレートとして存在し、
  `tally` はそれに依存する薄い CLI になっている
- **`tally-core` が `clap` にも `anyhow` にも依存していない**
  （依存の向きが逆流していないことを `cargo tree -p tally-core --edges normal` で示す）
- **公開したくない項目が公開されていない。**
  `unreachable_pub` を有効にし、`pub` と `pub(crate)` を意図して使い分けている

**エラー型（ADR-0004 の実装）**

- `LineError` / `LineErrorKind` / CLI 固有のエラー型が実装されている
- **ADR-0004 の Confirmation を全項目実施している。** とくに次の 2 つを落とさない:
  - **`Display` と `source()` の合成規則を決めている。**
    「抜粋の制御文字が単一エラーの `to_string()` に載る」検査が生きている
  - **`size_of` の実測値が ADR-0004 に記録されている**（測る前に数値を書かない）
- **終了コードの決定規則がユニットテストで網羅されている。**
  統合テストは配線の確認だけに減っている

**公開 API（ADR-0005 の実装）**

- **ADR-0005 の Confirmation の表を全項目実施している**

**ドキュメント**

- **`tally-core` の公開 API 全てにドキュメントコメントがある。**
  目視に頼らず `missing_docs` で機械的に担保する
- **動くサンプルが付いており、`cargo test --workspace --doc` が通る**
  （nextest はドキュメントテストを実行しないので別途回す）
- **rustdoc の警告検査が通る**
  （`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`）。
  **`cargo lint` では出ない。** 壊れた intra-doc link と、
  公開項目から非公開項目へのリンクはここでしか拾えない
- `cargo doc` で読み、**外部の利用者が使えるかを判断している**

**共通**

- `cargo fmt --all --check` / `cargo lint` / `cargo t` / `cargo deny check` が通る

### 検討する価値のあるもの（必須ではない）

- **`cargo-semver-checks`。** rustdoc の JSON を突き合わせて破壊的変更を検出する。
  **ただし万能ではなく**、フィールドや引数の**型**が変わった系は未カバーと
  明記されている。**「検出されなかったこと」を安全の根拠にしない**

---

## 段階 6: 並行・並列

**扱う概念**: `Send` / `Sync` が何を保証するか、`std::thread::scope`、
チャネル、`Arc<Mutex<T>>`、`rayon` のデータ並列、
そして **「並列化しても速くならない」典型パターン**。

**課題**: 複数ファイルを引数に取れるようにし、ファイル単位で `rayon` 並列化する。
`Counter` のマージ処理を実装する（`Extend` か、専用の `merge` メソッド）。

**確認**: **ベンチで実測する。** ファイル 1 個・10 個・1000 個で比較し、
I/O バウンドの場合に並列化が効かない（あるいは遅くなる）ことを確認する。
`divan` でベンチを書く。

**着手前に決めること**: **ベンチの合格条件を先に書く。**
「速くなること」ではなく「**どの条件で速くならないことを確認するか**」を書く。
また **公開 API が増えるので、ADR が要るか判断する**（`Counter` のマージ、
複数入力になったときのエラーの文脈）。未決事項は `docs/handoff.md`。

---

## 段階 7: 性能とメモリ

**扱う概念**: アロケーションの所在、`String` vs `&str` vs `Box<str>`、
`Vec::with_capacity`、`HashMap` のハッシュ関数差し替え（`ahash` 等）、
`#[inline]`、`profile.profiling` を使ったプロファイリング。

**課題**: 1000 万行の入力でプロファイルを取り、上位 3 つのホットスポットを特定して改善する。
**改善前後の数値を記録すること。** 推測で最適化しない。

---

## 段階 8: `unsafe` の境界（読むだけ）

**扱う概念**: `unsafe` が無効化するのは何か（借用検査ではない）、
未定義動作の実例、健全性（soundness）と安全な抽象、`miri`。

このリポジトリでは `unsafe` は `deny` のままにする。
**書くのではなく、標準ライブラリや依存クレートの `unsafe` を読んで、
どう安全性を担保しているかを説明できるようにする。**

```bash
rustup component add miri
cargo +nightly miri test -p tally
```

---

## 段階 9: 配布

**扱う概念**: クロスコンパイル、静的リンク（musl）、リリースプロファイル、
`cargo dist` / GitHub Releases、シェル補完の同梱、man ページ生成。

**課題**: `tally` をリリースするワークフローを CI に足す。
Linux（musl 静的）と macOS（aarch64）のバイナリを生成する。
