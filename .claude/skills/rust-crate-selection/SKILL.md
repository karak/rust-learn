---
name: rust-crate-selection
description: Use when adding a dependency to a Rust project, choosing between competing crates, reviewing Cargo.toml, deciding on feature flags, evaluating whether the standard library suffices, or auditing supply chain with cargo-deny. Fires on cargo add, "which crate should I use", async runtime choice, and dependency bloat or compile time complaints.
---

# 依存クレートの選定

Rust は標準ライブラリが意図的に小さい。**「何を依存に入れるか」の判断が実務スキルの一部。**
このスキルは、追加前に必ず通す手順と、領域ごとの事実上の標準をまとめる。

## 追加前の手順

**`cargo add` を打つ前に、以下を答えてから提案する。**

1. **標準ライブラリで足りないか。** `std::collections`、`std::sync`、
   イテレータアダプタは想像より広い。数十行で済むなら依存を増やさない。
2. **代替候補を 2 つ挙げ、選定理由を述べる。**
3. **健全性を確認する** — 直近のリリース、メンテナ数、逆依存数（`crates.io` の
   "Dependents"）、`unsafe` の量、ドキュメントの有無。
4. **推移的依存の量を見る。** `cargo tree -p <crate>` でツリーを確認する。
   小さな便利機能のために 40 クレート増えるなら、自分で書く。
5. **feature を絞る。** `default-features = false` から始め、必要なものだけ足す。
6. 追加後に **`cargo deny check`** を通す。ライセンス許可リストは `deny.toml`。

**バージョンはワークスペースの `[workspace.dependencies]` に書く。**
個別クレートは `foo.workspace = true` で参照する。同一クレートの版が分岐すると、
「同名だが別の型」というコンパイルエラーに悩まされる。

## 領域別の事実上の標準

| 用途 | 既定の選択 | 補足 |
| --- | --- | --- |
| 引数解析 | `clap`（derive） | 小さなツールなら `lexopt`（依存が極小） |
| エラー（lib） | `thiserror` | |
| エラー（bin） | `anyhow` | 併用が正しい。`.claude/skills/rust-error-handling/` |
| シリアライズ | `serde` + `serde_json` | 事実上唯一。`toml` / `serde_yaml` も同系 |
| ログ | `tracing` + `tracing-subscriber` | 非同期・スパンが要らなければ `log` + `env_logger` |
| 日時 | `jiff`、または `chrono` | `time` も現役。新規は `jiff` が扱いやすい |
| 正規表現 | `regex` | 後方参照は無い（線形時間保証のため） |
| HTTP クライアント | `reqwest` | 軽くしたいなら `ureq`（同期・依存少） |
| 非同期ランタイム | `tokio` | 迷ったらこれ。`smol` は軽量 |
| データ並列 | `rayon` | `par_iter()` に置換するだけのことが多い |
| 一時ファイル | `tempfile` | テストでは必須 |
| パス | `camino`（UTF-8 パス） | `PathBuf` の `to_str()` 地獄を避けたいとき |
| CLI テスト | `assert_cmd` + `predicates` | |
| スナップショット | `insta` | |
| プロパティテスト | `proptest` | `quickcheck` より表現力が高い |
| ベンチ | `divan` または `criterion` | `divan` は軽く速い |

## 非同期を入れる判断

**`tokio` を入れると、依存とコンパイル時間と設計上の制約が一気に増える。**
CLI ツールでは多くの場合不要。

| 状況 | 判断 |
| --- | --- |
| ファイル処理・CPU 処理 | **同期でよい。** 並列化は `rayon` |
| HTTP リクエストが数個 | 同期の `ureq` で足りる |
| 数百の I/O を並行 | `tokio` が正当化される |
| ネットワークサーバ | `tokio` |

async を入れるなら、**「関数が async に染まる」ことを設計判断として受け入れる。**
同期関数から async を呼ぶには実行時ブロックが要り、これは容易にデッドロックする。

## feature flag

```toml
clap = { version = "4", features = ["derive", "env", "wrap_help"] }
```

- **既定 feature が何を含むか読む。** `reqwest` の既定は TLS スタックを丸ごと引く。
- **feature は加算的でなければならない。** feature を足して既存のコードが壊れる設計は誤り。
- 依存が重いと感じたら `cargo tree -e features` で誰が何を要求しているか辿る。

## コンパイル時間・バイナリサイズが問題になったら

```bash
cargo build --timings          # 何が時間を食っているか
cargo tree --duplicates        # 同一クレートの複数バージョン
cargo deny check bans          # 重複を検出（deny.toml で warn 設定）
```

- proc-macro（`serde` derive、`clap` derive）はコンパイル時間の主因。
  それでも大抵は手書きより価値がある。**測ってから外す。**
- バイナリサイズは `[profile.release]` の `lto = "thin"`、`codegen-units = 1`、
  `strip = "symbols"` で改善する（このリポジトリでは設定済み）。

## 避けること

- **`cargo add` を先に実行して、後から理由を説明する。** 順序が逆。
- 記憶で API を書く。**`context7` MCP で最新ドキュメントを引いてから書く。**
  Rust エコシステムは破壊的変更のペースが速い。
- `*` や `>=` のバージョン指定（`deny.toml` の `wildcards = "deny"` で禁止済み）。
- git 依存（`unknown-git = "deny"`）。学習リポジトリでは crates.io に限定する。
- 「便利だから」で入れる。**依存は資産ではなく負債**として扱う。
