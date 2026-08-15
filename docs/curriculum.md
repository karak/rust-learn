# 学習ロードマップ

**Rust 固有の意思決定** に絞った順序。
各段階に「`tally` に加える機能」を割り当ててあり、読むだけで終わらないようにしてある。

各段階の完了条件は共通して:
`cargo fmt --all` / `cargo lint`（`pedantic` 込み）/ `cargo t` が通ること。

---

## 段階 0: 現状の把握（済）

`crates/tally` は動く CLI として完成している。まずこれを読む。

読む順序: `src/core.rs` → `src/error.rs` → `src/cli.rs` → `src/main.rs`。
**ロジックから読み、I/O を最後に読む。** この順で読めることが、この構成の目的でもある。

問い:
- `Key::extract` の戻り値が `Result<Option<Cow<str>>>` である理由を、
  3 つの型それぞれについて説明できるか。
- `report()` で同数時のタイブレークを決めているのはなぜか。決めないと何が壊れるか。
- `main` が `Result` でなく `ExitCode` を返しているのはなぜか。

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

**扱う概念**: `Result` と `?` の変換、`From`、`thiserror` と `anyhow` の役割分担、
エラーチェーン、`io::ErrorKind`。

**課題**: `--strict` フラグを足す。既定ではスキップしている
「フィールドが無い行」を、`--strict` 時はエラーにする。
エラーには行番号と、その行の先頭 40 文字を含める。

**確認**: 統合テストで終了コードと stderr の文言を検証する。

**参照**: `.claude/skills/rust-error-handling/`

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

---

## 段階 4: イテレータとクロージャ

**扱う概念**: `Iterator` の遅延性、アダプタの合成、`FnOnce`/`FnMut`/`Fn` の違い、
`collect::<Result<Vec<_>, _>>()` の挙動、自前の `Iterator` 実装。

**課題**: `--filter <REGEX>` を足す。マッチする行だけを集計する。
実装は `tally_reader` のループではなく、**イテレータアダプタの合成** で表現する。

**問い**: `for` ループ版とアダプタ版で、生成されるコードに差は出るか
（`cargo asm` や `--release` でのベンチで確認する）。

---

## 段階 5: モジュール・クレート設計

**扱う概念**: `pub` / `pub(crate)` / `unreachable_pub`、モジュール階層と可視性、
ワークスペース分割、feature flag、セマンティックバージョニングと破壊的変更、
ドキュメントコメントとドキュメントテスト。

**課題**: 集計コアを `crates/tally-core` として別クレートに切り出す。
`tally` はそれに依存する薄い CLI にする。
`tally-core` の公開 API 全てにドキュメントコメントと **動くサンプル** を書く
（`cargo test --doc` で検証される）。

**確認**: `cargo doc --open` で読んで、外部の利用者が使えるか判断する。

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
