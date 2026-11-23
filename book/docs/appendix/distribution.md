---
title: 付録 B 配る（クロスコンパイル・静的リンク・補完と man）
sidebar_position: 2
---

# 付録 B 配る（クロスコンパイル・静的リンク・補完と man）

自分の機械で動くバイナリを、そのまま他人の機械へ配っても動くとは限らない。
OS ごとのリンクの仕組み、シェル補完や man ページの同梱、アーカイブの構造まで含めて配布物になる。
この付録では、`tally` を Linux（musl）と macOS（Apple Silicon）向けに配布したときの仕組みと、実物を走らせて初めて分かった落とし穴をまとめる。

## 「静的リンク」の意味は Linux と macOS で違う

Linux と macOS では、配布バイナリの依存をどこまで切れるかが違う。

| 項目 | `x86_64-unknown-linux-musl` | `aarch64-apple-darwin` |
| --- | --- | --- |
| 実行ファイル（`v0.1.0-rc.2` 実測） | 3,066,080 バイト | 2,508,480 バイト |
| アーカイブ（`tar.gz`） | 1,288,803 バイト | 1,096,513 バイト |
| 動的リンク先 | なし（`ldd` は `statically linked`） | `libiconv.2.dylib`、`libSystem.B.dylib` |

Linux の `musl` ターゲットでは、システムコールを直接呼ぶ libc ごとバイナリに抱え込むため、共有ライブラリを一切要求しない完全な静的リンクができる。
libc を抱えるぶん、実行ファイルは macOS 版より約 22% 大きくなった。

一方、macOS には完全な静的リンクが存在しない。
`libSystem` がシステムコールの唯一の入口として定められているためである。
外部の共有ライブラリを同梱せず単体の実行ファイルとして配る場合、macOS で保証できるのは「`/usr/lib` や `/System` の外にあるライブラリへ依存しないこと」になる。

ここで 2 つ、事前の予想が外れた点がある。
1 つ目は、macOS の実物を `otool -L` で見たとき、`libSystem.B.dylib` だけでなく `libiconv.2.dylib` も付いていたことである。
`/usr/lib` にあるシステムライブラリなので実用上は問題ないが、検査条件を「`libSystem` だけ」に絞っていたら自分の CI で落ちていた。
2 つ目は、Linux の `ldd` の出力文言である。
静的バイナリなら `not a dynamic executable` と出る想定だったが、Ubuntu 上で生成された static-pie バイナリに対する `ldd` の出力は `statically linked` だった。
検査では両方を受け入れる分岐にしていたため通ったが、期待する文言を 1 つに決めていたら静的リンクに成功しているのに落ちていたことになる。

なお、`v0.1.0-rc.2` のビルドでは、`Cargo.toml` の `[profile.release]` に `lto = "thin"`、`codegen-units = 1`、`strip = "symbols"` を指定して最適化とシンボル削除を行っていた。

## 補完と man ページは、本体とは別の `xtask` クレートで生成する

CLI の配布物には、シェル補完と man ページを同梱したい。
`tally` では、これを本体とは別の `xtask` という[クレート](../glossary.md#crate)に切り出し、`cargo run -p xtask -- <出力先>` で生成する構成を採っている。

```rust
let mut cmd = tally::cli::Cli::command();
let name = cmd.get_name().to_string();

for &shell in &[Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
    clap_complete::generate_to(shell, &mut cmd, &name, &completions)?;
}

let mut buf = Vec::new();
clap_mangen::Man::new(cmd).render(&mut buf)?;
std::fs::write(&page, buf)?;
```

引数の定義を `tally::cli::Cli` の 1 箇所から引くため、オプションを足したときに補完や man を直し忘れる経路が無い。
また、`clap_complete` や `clap_mangen` は `xtask` だけが依存するため、集計だけを行う `tally` 本体の依存には乗らない。

可変個の位置引数を持つ CLI に必須引数つきのサブコマンド（たとえば `tally completions <shell>`）を同居させると、同じ名前空間を争う。
`completions` という名前のファイルを `--` 無しで渡したときにサブコマンドと解釈され、必須引数 `<SHELL>` が足りないという無関係なエラー（`clap::error::ErrorKind::MissingRequiredArgument`）になりうる（`clap` 4.6.6 で実測）。
別クレートの `xtask` に切り出しておけば、本体の引数空間を汚さずに済む（設計判断の経緯は [ADR-0009](https://github.com/karak/rust-learn/blob/main/docs/adr/0009-release-distribution.md) にある）。

## ツールチェーンの固定とプレリリース版には、固有の落とし穴がある

初めてプレリリースのタグ `v0.1.0-rc.1` を打つ前後で、2 つの落とし穴に当たった。

1 つ目は、タグを打つ前に版を書き換えた段階で起きたプレリリース版のバージョン解決である。
`crates/tally/Cargo.toml` の版を `0.1.0` から `0.1.0-rc.1` へ書き換えたところ、`tally = { path = "../tally", version = "0.1.0" }` と書いていた `xtask` のビルドが落ちた。
セマンティックバージョニング（semver）の順序では `0.1.0-rc.1` は `0.1.0` より手前の版であり、`"0.1.0"`（`^0.1.0` の範囲）はプレリリースを自動では拾わない。
[`cargo`](../glossary.md#cargo) は `path` で場所が確定しているワークスペース内依存でも、`version` が書かれていれば照合を省かない。
そのため、新しい版が元の semver 要求（プレリリース制約を含む）から外れる場合は参照側も更新する必要がある。

2 つ目は、タグを打ってリリースワークフローを走らせたときに起きた、[`rustup`](../glossary.md#rustup) のツールチェーン固定とターゲットの食い違いである。
リポジトリ直下に `rust-toolchain.toml` を置いてコンパイラの版を固定していると、ツールチェーンを明示指定しない呼び出しでは固定版が優先される。
ところが CI の設定で別の版（たとえば `stable`）へ `x86_64-unknown-linux-musl` ターゲットを足すと、実際にビルドに使う固定版にはターゲットが入っておらず、`` error[E0463]: can't find crate for `core` `` で落ちる。
macOS 側はホスト自身のターゲットが最初から入っているため通り、Linux 側だけ落ちる形になった。
版を固定しているときは、`--toolchain` を付けずに `rustup target add x86_64-unknown-linux-musl` を実行するか、`rust-toolchain.toml` の `targets` に書いて、固定版自身へターゲットを足す必要がある。

## 配布物の形は外部の契約であり、別の機械で展開して確かめる

一度リリースしたアーカイブのファイル名や中身の配置は、利用者の導入スクリプトに焼き付く。
`tally` では配布物の構造を [`crates/tally/docs/distribution.md`](https://github.com/karak/rust-learn/blob/main/crates/tally/docs/distribution.md) に契約として書き出している。

```text
tally-<版>-<ターゲット>/
├── tally
├── completions/
│   ├── tally.bash
│   ├── _tally
│   ├── tally.fish
│   └── _tally.ps1
├── man/
│   └── tally.1
├── LICENSE
└── README.md
```

展開したときにファイルがカレントディレクトリへ散らばらないよう、全体を 1 つのディレクトリに収めている。
また、アーカイブへ入れるファイルを数え上げる段になって、`Cargo.toml` で `license = "MIT"` と宣言しながら `LICENSE` ファイル自体を置き忘れていたことに初めて気づいた。
宣言と実物が揃っているかは、実際に配布物を組んで初めて露呈することがある。

最後に、「他人の機械で動くか」を確かめるため、ビルドしたジョブとは別の CI ジョブで検証を行う。
ソースコードを checkout せず、完成した `tar.gz` だけを受け取って展開し、`printf 'a\nb\na\n' | ./tally --format csv` を走らせて期待する出力（`2,a` と `1,b`）と突き合わせる。
`--version` が通るだけでは実行時の依存漏れや基本機能の破綻を拾いきれないため、集計を 1 回通すところまでを配布の検査に含めている。
