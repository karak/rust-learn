---
title: 付録 C unsafe の境界を読む（不変条件と miri）
sidebar_position: 3
---

# 付録 C `unsafe` の境界を読む（不変条件と `miri`）

3〜5 章では「[`unsafe`](../glossary.md#unsafe) を使わない限り〜」という留保を置き、12 章でも手動の `Send`/`Sync` 実装には `unsafe impl` が要ると触れた。
`tally` のワークスペースでは `Cargo.toml` の lint 設定（`unsafe_code = "deny"`）で `unsafe` をコンパイルエラーにしていたが、依存している[クレート](../glossary.md#crate)や標準ライブラリの内側には `unsafe` がある。

この付録では、`unsafe` を書く手順ではなく、**依存クレートの実物の `unsafe` を読んで安全性の根拠を確かめる視点**を扱う。
題材は、2026 年 10 月時点で `tally` が依存していた 3 つのクレート（`foldhash` 0.2.0、`serde_json` 1.0.151、`rayon-core` 1.13.0）である。

## `unsafe` が外すのは借用検査ではなく、限られた操作だけ

`unsafe` ブロックや `unsafe impl` を書いても、参照（`&T` / `&mut T`）に対する 2 章の[借用](../glossary.md#borrow)検査は止まらない。
借用規則も型の整合もそのまま効いたうえで、コンパイラが静的に確かめられない一部の操作だけが許される。
代表的には次の 5 つである。

- 生ポインタ（`*const T` / `*mut T`）の[参照外し](../glossary.md#dereference)（生ポインタ自体は借用規則で追跡されず、null やダングリングになりうる）
- `unsafe fn` の呼び出し（外部言語の関数を含む）
- 可変な静的変数（`static mut`）へのアクセス
- 共用体（`union`）のフィールド読み
- `unsafe trait`（[`Send` や `Sync`](../glossary.md#send-sync) など）の実装（`unsafe impl` 宣言）

C++ には構文上の境界が無い。
生ポインタの演算も型変換も地の文にそのまま混ざるため、どこから先が人間の証明責任なのかを機械的に抜き出しにくい。
Rust は境界を `unsafe` で囲うため、ソースを `grep` しておおよその件数（識別子やコメントのヒットを含む）を当たり、読み切れる範囲へ対象を絞れる。
この読み進めでは、`foldhash` 0.2.0 の `seed.rs`（`unsafe` 構文 3 件）、`serde_json` 1.0.151 の `read.rs`（3 件）、そして全体で 80 件超ヒットした `rayon-core` 1.13.0 から `ScopePtr` 周辺を抜き出して読んだ。

## 依存クレートの実物 3 か所を読むと、前提を作る場所が全部違った

読む前の予想は外れた。
「`unsafe` の直前に `if` で検査が書いてあるはずだ」と見込んでいたが、3 か所とも安全性の前提（不変条件）を作っている場所が違っていた。

| 対象（2026 年 10 月時点の版） | `unsafe` の前提 | 誰が前提を作っているか |
| --- | --- | --- |
| `foldhash` 0.2.0 `GlobalSeed::get` | 大域の種が初期化済みである | 型と可視性（コンストラクタ経由でしか作れないサイズ 0 の型） |
| `serde_json` 1.0.151 `StrRead::parse_str` | バイト列が正しい [UTF-8](../glossary.md#utf-8) である | 入力の型（[`&str`](../glossary.md#string-str) として入ってきた事実） |
| `rayon-core` 1.13.0 `ScopePtr::as_ref` | ポインタの指す `Scope` が生きている | 呼び出し側の時間順序（全タスクの完了をラッチで待つ） |

### 1. `foldhash`: 型と可視性で「初期化済み」を証明する

`foldhash` の `GlobalSeed` は、サイズ 0 の[ユニット型](../glossary.md#unit-type) `()` を非公開フィールドとして 1 つだけ持つ[構造体](../glossary.md#struct)である。

```rust
pub struct GlobalSeed {
    // So we can't accidentally type GlobalSeed { } within this crate.
    _no_accidental_unsafe_init: (),
}
```

14 章で見た「非公開フィールド＋コンストラクタ」と同じ仕組みである。
`GlobalSeed` は `seed.rs` 内の `mod global` に置かれ、[モジュール](../glossary.md#module)の外からはリテラルで直接組み立てられない。
必ず初期化を通すコンストラクタ経由でしか手に入らないため、`GlobalSeed` の値を持っていること自体が「初期化済みである」という証明書になる。
`get()` の `unsafe` はその型を根拠にしており、毎回「初期化済みか」を実行時に問い直さずに済ませている。
非公開の壁が効くのは定義元モジュールの外に対してまでであり、内側ではあくまで「うっかり `GlobalSeed {}` と書かないための段差」にとどまる。

### 2. `serde_json`: 入力の型が持つ約束をそのまま使う

付録 A で扱った `Deserializer::from_str` の経路では、`serde_json` の `StrRead::parse_str` が `str::from_utf8_unchecked(bytes)` を呼んでいる。

```rust
// The deserialization input came in as &str with a UTF-8 guarantee,
// and the \u-escapes are checked along the way, so don't need to
// check here.
Ok(unsafe { str::from_utf8_unchecked(bytes) })
```

直前に UTF-8 の検証をやり直していない。
3 章で見た「`&str` はすでに正しい UTF-8 である」という型の約束をそのまま使っている。
入力が `&[u8]` の経路ではこの近道は取れない。
安全 Rust の型が入口で守った約束が、内側の `unsafe` の検査省略につながっている。

### 3. `rayon-core`: 寿命を型から消し、実行時の順序で埋め合わせる

12 章で見た `thread::scope` と同様のスコープ並列が `rayon-core` にもある。
`ScopePtr<T>` は、スコープを管理する構造体 `Scope` への参照を `*const T` に変えて[寿命](../glossary.md#lifetime)を型から消す。
代わりに、全タスクの完了を数えて待つ同期機構（ラッチ）によって「`Scope` がまだ生きている」ことを実行時の順序で保証している。

`ScopePtr` では、`unsafe` の証明責任が 2 か所に分かれている。

```rust
// SAFETY: !Send for raw pointers is not for safety, just as a lint
unsafe impl<T: Sync> Send for ScopePtr<T> {}
```

`unsafe impl Send` は「`T: Sync` ならこのポインタを別スレッドへ送ってよい」と宣言しているだけで、送った先で安全に参照外しできることまでは約束していない。
指す先が生きている前提は、実際に `*const T` を `&T` に戻す別の `unsafe fn as_ref` を呼び出す側に事前条件として課されている。

## 前提が破れたときに起きるのは「落ちる」ではなく未定義動作

上の 3 か所で前提が破れても、プログラムが親切にエラーで止まるとは限らない。
いずれも[未定義動作](../glossary.md#undefined-behavior)につながる。

- `GlobalSeed::get` を初期化完了前に呼ぶと、他スレッドの書き込みと重なって[データ競合](../glossary.md#data-race)になりうる
- 不正なバイト列から `from_utf8_unchecked` で `&str` を作ると、事前条件と `&str` の型の約束を破り、その値を受け取る安全 Rust の操作が未定義動作を起こしうる状態になる
- スコープ終了後に `ScopePtr::as_ref` を呼ぶと、関数から抜けて無効になったスタック領域を指す参照（ダングリング参照）になる

テストの条件や最適化のかかり方しだいで、未定義動作は「たまたま動いて見える」ことがある。
だからこそ、安全な API の内側に `unsafe` を閉じ込めるときは、外側のどの操作からも前提を破れないこと（健全性）を確かめる必要がある。

## `miri` は未定義動作を解釈実行で見つけるが、時間と対象に限界がある

`miri` は Rust の中間表現（MIR）を解釈実行し、未定義動作やメモリリークを検出する道具である。
実行には nightly のツールチェーンが要る。

```bash
cargo +nightly miri test -p tally-core
```

2026 年 10 月時点で `tally-core` の単体テスト 64 件に当てたところ、通常実行の 0.675 秒に対して 10.56 秒（およそ 16 倍）掛かり、全件通った。
`tally-core` はファイルを開かずメモリ上の読み取りだけを行う設計なので `miri` の下でも回る。
一方、`miri` は既定では OS 呼び出しやファイル・プロセス起動を制限しており、外部の C ライブラリ（FFI）も解釈できない。
そのため、プロセス起動とファイル I/O を含む `tally` 本体には当てていない。

また、`miri` は動的検査であり、テストが実際に通った実行経路だけを確かめる。
上の実行では、自分のコードに `unsafe` を書いていないため想定どおり通過し、あわせてテストが通った範囲の依存クレートの処理にも問題が出ないことを確かめられた。
依存クレートの挙動を疑ったときや、将来やむを得ず `unsafe` を書く場面が来たときに、目視だけに頼らず未定義動作を確かめる手段になる。
