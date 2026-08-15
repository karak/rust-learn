---
name: rust-error-handling
description: Use when designing, reviewing, or debugging error types in Rust — choosing between thiserror and anyhow, deciding what belongs in a public error enum, adding context, handling io::ErrorKind, or converting between error types. Fires on `Result`, `?`, `Box<dyn Error>`, `unwrap`, `expect`, panic-vs-Result decisions, and "how should this fail".
---

# Rust のエラー設計

## 中心となる判断: 誰が分岐するのか

**呼び出し側がエラーの種類で分岐する可能性があるか。**

| 状況 | 使うもの |
| --- | --- |
| ライブラリ / 公開 API | `thiserror` で具体的な `enum` |
| バイナリの `main` 付近 | `anyhow::Error` |
| 内部のヘルパー関数 | 呼び出し元と同じ型をそのまま流す |

**`anyhow::Error` を公開 API の戻り値に出さない。**
出した瞬間、呼び出し側は「ファイルが無い」と「JSON が壊れている」を区別できなくなる。
文字列マッチで分岐するコードが生まれたら、それは設計の失敗のサイン。

逆に、**バイナリで `enum` を作り込むのは過剰。** `main` は結局全部を
「エラーメッセージを出して終了コード 1」に潰すので、区別する意味がない。

## ライブラリ層: thiserror

参照実装は `crates/tally/src/error.rs`。

```rust
#[derive(Debug, thiserror::Error)]
pub enum TallyError {
    // #[source] で原因を保持する。表示には含めず、チェーンとして辿らせる。
    #[error("入力を読めません: {path}")]
    OpenInput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    // #[from] は source + From 実装を同時に生成する。`?` で自動変換される。
    #[error("入力の読み取りに失敗しました")]
    Read(#[from] std::io::Error),
}
```

### 守ること

- **`#[error("...")]` に原因を埋め込まない。** `#[error("読めません: {source}")]` と書くと、
  上位が `{:#}` で表示したときに同じ文言が二重に出る。`#[source]` に任せる。
- **エラーメッセージは「どこを直せばいいか」を含む。** 行番号、パス、フィールド名。
  `TallyError::InvalidJson { line_no, .. }` が行番号を持つのはこのため。
- **`#[from]` は 1 つの型につき 1 バリアントまで。** 同じ型から 2 つは生成できない。
  複数箇所で `io::Error` を区別したいなら `#[from]` をやめて `map_err` で明示する。
- **エラー型は `Send + Sync + 'static` に保つ。** そうでないとスレッドを跨げず、
  `anyhow` にも入らない。エラーの中に `Rc` や生ポインタを入れない。

## バイナリ層: anyhow

```rust
use anyhow::Context as _;

let file = File::open(path)
    .with_context(|| format!("{} の集計に失敗しました", path.display()))?;
```

- **`context` は「何をしようとしていたか」を足す。** 「失敗した」ではなく「何の途中で失敗したか」。
- **`with_context` はクロージャ。** 成功時にフォーマットのコストを払わない。
  文字列リテラルだけなら `context` でよい。
- **表示は `{:#}`。** `{}` は最上位のメッセージしか出さない。
  `eprintln!("error: {err:#}")` でチェーン全体が `: ` 区切りで出る。

## 型を跨いで見分ける

`anyhow` に包んだ後でも、チェーンを辿れば具体型に戻せる。
`crates/tally/src/main.rs` の broken pipe 判定がその実例：

```rust
err.chain().any(|cause| {
    cause.downcast_ref::<io::Error>()
        .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
})
```

`kind()` で分岐すること。**`io::Error` を文字列比較しない** — OS とロケールで変わる。

## panic してよい場合

`Result` を返すのが原則だが、例外がある。

| panic してよい | 理由 |
| --- | --- |
| テストコード | 失敗＝テスト失敗でよい（`clippy.toml` で許可済み） |
| 不変条件の破れ（プログラマのバグ） | 回復しても無意味。`unreachable!` / `assert!` |
| `const` 的に絶対成立する初期化 | 例: 固定の正規表現のコンパイル |

**回復不能かどうかで決める。ユーザ入力・ファイル・ネットワークは常に `Result`。**

`expect` を使うなら、メッセージは **「なぜ成立するはずなのか」** を書く。
「失敗した」ではなく「ここでは必ずソート済みのはず」。

## 他言語との差分

- **Go の `if err != nil` と違い、`?` は型変換を伴う。** `From` 実装が変換経路。
  だから「どの型に変換されるか」がエラー設計の中心になる。
- **例外と違い、スタックトレースは自動で付かない。** 位置情報が欲しければ
  `context` を積むか、`RUST_BACKTRACE=1` + `anyhow` のバックトレース機能を使う。
  **`context` を積む習慣がスタックトレースの代わり。**
- **`?` は早期リターンであってロールバックではない。** 途中まで変更した状態は戻らない。
  失敗しうる処理は「全部準備してから最後にコミット」の順に組む。

## レビュー時のチェック

- [ ] 公開 API の `Result` の `E` が `anyhow::Error` になっていないか
- [ ] エラーメッセージに、修正に必要な情報（パス・行・名前）が入っているか
- [ ] `#[error(...)]` と `#[source]` でメッセージが二重になっていないか
- [ ] 非テストコードの `unwrap` / `expect` に正当化のコメントがあるか
- [ ] `io::Error` を文字列でなく `kind()` で判定しているか
- [ ] エラーを握り潰して `Ok(())` や既定値を返している箇所がないか
