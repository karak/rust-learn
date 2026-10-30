# 引き継ぎ（セッション再開用）

作業を中断・再開するときに最初に読む文書。
**現在地・再開手順・未決事項の正本はこのファイル。**

## 何をどこに書くか

**`CLAUDE.md` の「文書の書き分け」が正本。** ここには複製しない
（この文書自身が、複製で矛盾を起こした当事者なので）。

要点だけ: **この文書は「状態」だけを持つ。** 現在地、再開手順、未決事項。
規約は `CLAUDE.md`、クレートの構造は各クレートの `docs/layout.md`、
学習の進め方（予）は `docs/curriculum.md`、
**各段階の実績（実）は `docs/stage-log.md`**、学びは `docs/learning-log.md`。

**「完了した実績」はここに書かない。** `stage-log.md` の領分である。
ここに書くのは「いまどこにいるか」だけ。

## 現在地

- **段階 9 まで実施し、公開した**（2026-10-17）。
  **`v0.1.0` が正式版として GitHub Releases に出ている**（2026-10-28）。
  `v0.1.0-rc.2` もプレリリースとして残っている。
  **`v0.1.0-rc.1` は落ちたまま残っている**（Release は作られていない）。
  **[ADR-0009](adr/0009-release-distribution.md) は `accepted`** — 確認 10 項目すべて済み。
  **段階 10 を立てた**（2026-10-28、[ADR-0012](adr/0012-stage-10-associated-types.md)）。
  **各段階の実績は `docs/stage-log.md` が正本**（節がある段階が完了した段階）。
  段階の定義・完了条件は `docs/curriculum.md`
- **ワークスペースのメンバは 2 つになった。**
  `crates/tally-core`（集計コア。`clap` も `anyhow` も持たない）と
  `crates/tally`（薄い CLI）
- **[ADR-0004](adr/0004-error-type-shape.md) と
  [ADR-0005](adr/0005-selector-public-api.md) は `accepted` になった。**
  実装・実測・Confirmation をすべて終えた。**この 2 つに残作業は無い**
- **[ADR-0006](adr/0006-branching-strategy.md) も `accepted` になった**（2026-08-23）。
  rebase と `--force-with-lease` を含む経路も 2026-09-29 に通した
- **[ADR-0007](adr/0007-multi-input-aggregation.md) も `accepted` になった**（2026-09-24）。
  複数入力・マージ・失敗の文脈・並列化の置き場所を決め、実装と実測まで終えた。
  **ADR-0004 と ADR-0005 を部分的に supersede している**（未決事項 10）
- **`tally` は複数ファイルを取り、ファイル単位で並列に集計する。**
  `-j` / `--jobs` で並列度を指定でき、`-j 1` は逐次（検証の経路）
- **モジュール間の依存規則を `scripts/check-module-deps.sh` が検査する。**
  規則の正本は `crates/tally/docs/layout.md` の層の表
- **push はホストから行う。** コンテナへ資格情報を渡さないと決めた
  （2026-09-25、[ADR-0008](adr/0008-container-push-credentials.md)）
- **段階 7 で性能を測って直した**（1000 万行で JSON 2.29 倍、行全体 1.80 倍）。
  **実測と「効かなかった改善」は `docs/stage-log.md` 段階 7 が正本**
- **`foldhash` を依存に足した**（推移的依存ゼロ）。`rustc-hash` は
  固定シードなので採らなかった（`learning-log.md` 13-5）
- **段階 8 で依存クレートの `unsafe` を 3 か所読んだ**（`foldhash` / `serde_json` /
  `rayon-core`）。**コードは変えていない。`unsafe` は `deny` のまま**
- **`miri` を使えるようにした。** nightly を別に入れ、
  `cargo +nightly miri test -p tally-core` で当てる。
  **`rust-toolchain.toml` の 1.97.1 固定は変えていない**
- **ワークスペースのメンバが 3 つになった。** `crates/xtask` を足した
  （補完と man の生成器。**`tally` の依存にはしない**）
- **[ADR-0010](adr/0010-json-pointer-key.md) は `accepted`**（2026-10-20）。
  `--pointer` で入れ子と配列を辿れる。**`Key` のバリアントは 3 つになった。**
  **Confirmation 1 件が未実施**（既存 `--field` の性能。比べる相手を作っていない）
- **[ADR-0012](adr/0012-stage-10-associated-types.md) は `accepted`**（2026-10-28）。
  **段階 10 を実施した。** `trait Input { type Reader: BufRead; … }` を
  `tally` 層に定義し、`input.rs` の入口を 1 つにした。
  **このリポジトリで初めて自分で定義した trait である**
- **カリキュラムが挙げた概念は、すべて実コードで消化された**（2026-10-28）。
  **`curriculum.md` 末尾の終了基準を満たしている**
- **[ADR-0011](adr/0011-v0-1-0-promise.md) は `accepted`**（2026-10-28）。
  **`v0.1.0` が約束する範囲は、`output-format.md` と `distribution.md` の
  2 文書に書いてあることだけ。** 書いていないものは予告なく変わる。
  **`0.x` でも破壊的変更は minor を上げる**
- **`v0.1.0` を公開した**（2026-10-28）。**プレリリースではない。**
  2 つの契約文書は、打つ前に読み直して穴を 3 つ塞いである
- **[ADR-0009](adr/0009-release-distribution.md) は `proposed` のまま。**
  決定は 4 論点すべて済み、実装も済んでいるが、
  **Confirmation に未検証が 3 件残る**（未決事項 16）
- **`LICENSE` を置いた**（MIT、2026-10-12）。
  `Cargo.toml` は段階 0 から MIT を宣言していたが本文が無かった
- **配布物の契約は `crates/tally/docs/distribution.md` が正本。**
  `output-format.md`（出力の契約）とは別の文書である
- **道具が 3 つ増えた** — `samply`（プロファイラ）、`hyperfine`（時間測定）、
  `cargo-semver-checks`。**いずれもホストに入れたもので、リポジトリには入っていない**
- **`tally` は `anyhow` を使わない。** `CLAUDE.md` の方針 3 を段階 5 で書き換えた
- **[ADR-0013](adr/0013-book-phase.md) を `proposed` で立てた**（2026-10-31）。
  **学ぶフェーズを終え、学びを人間向けのハンズオンの本にするフェーズに入る。**
  本は `book/` に置き、`tally` を題材にする。**本は正本ではない**
- 公開済み: <https://github.com/karak/rust-learn>（public）

`docs/learning-log.md` は節ごとに対象ファイルを明記してある（「読み方」の表）。
**その表が唯一の索引である。** 見出しにも同じことを書いていたが、
段階 5 のファイル移動で片方だけ腐ったので見出しから落とした。

## 再開の手順

1. **`cargo lint` と `cargo t` と `cargo test --workspace --doc` を実行して、
   現在の状態を自分で確認する。** この文書の記述を信じない
   （nextest はドキュメントテストを実行しないので 3 つ目が要る）
2. `docs/curriculum.md` 段階 0 の「読む順序」に従い、次に触る範囲を読む
3. `docs/curriculum.md` の該当段階を読む
4. **ブランチを切る。** `main` に直接コミットしない。
   手順とブランチ名の規則は `CLAUDE.md`「ブランチ運用」が正本
   （[ADR-0006](adr/0006-branching-strategy.md)）

セットアップとコマンドは `README.md` を参照。

## 次の作業: 本を作る

**Rust を学ぶフェーズは終わった**（2026-10-28、終了基準は `docs/curriculum.md` の末尾）。
**次は [ADR-0013](adr/0013-book-phase.md) の本のフェーズ。**

**ADR-0013 は `proposed`。** 8 論点とも案で、Confirmation は 1 件も済んでいない。
**実装しながら確かめ、確かめ終えてから `accepted` にする**（ADR-0012 と同じ進め方）。

最初の 1 単位:

1. `book/` に Docusaurus の骨組みを置く
2. `.docscheckignore` を作り、`scripts/check-docs.sh` と
   `scripts/check-module-deps.sh` の両方に読ませる（Confirmation 2）
3. `CLAUDE.md`「文書の書き分け」の表に `book/` の行を足す
4. CI に `book` ジョブを足し、GitHub Pages への配置は別ワークフローにする
5. 第 1 章（所有権と move）と `book/code/ch01`
6. **GitHub Pages へ 1 回配置して経路を通す**（Confirmation 4）。
   **Pages はまだ有効になっていない**

**`docs/curriculum.md` と `docs/stage-log.md` は凍結する。**
段階を足さない。本の素材として読む。

## クレートの構造

**コードの配置と設計判断は、それぞれのクレートの `docs/layout.md` が正本。**

- `crates/tally-core/docs/layout.md` — 集計コア
- `crates/tally/docs/layout.md` — CLI

この文書には書かない（クレートの性質であって、セッションの状態ではないため）。
読む順序は学習者への指示なので `docs/curriculum.md` 段階 0 にある。

新しいコードをどこに置くか迷ったら、まず **どちらのクレートの話かを決める。**
`tally` 側の `layout.md` の「新しいコードをどこに置くか」がその問いから始まる。

## 既知の落とし穴

- **`clippy.toml` の `allow-expect-in-tests` は `#[cfg(test)]` にしか効かない。**
  `tests/` 配下は通常のクレートなので、ファイル先頭で明示的に `allow` する
- **nextest はドキュメントテストを実行しない。** `cargo test --workspace --doc` を別途回す
- **`missing_docs` が有効。** `pub` を足したら doc コメントが要る
- **文書にも検査がある。** `scripts/check-docs.sh`（CI で回る）。
  `curriculum.md` に進捗を書く、テスト名を転記する、リンクを切る、で落ちる
- **`scripts/check-docs.sh` は追跡済みのファイルしか見ない。**
  一部の検査が `git ls-files '*.md'` で対象を集めるので、
  **新しく作った文書は `git add` するまで検査を素通りする。**
  2026-10-31 に ADR-0013 でこれに引っかかった（手元は OK、CI で落ちた）。
  **新規文書を書いたら、`git add` してから検査を回す**
- **rustdoc の警告は `cargo lint` では出ない。**
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` が別に要る
  （CI には入っている）。**公開項目から非公開項目へのリンク**はここでしか拾えない
- **コンテナから push はできない。設計上そうしてある**
  （[ADR-0008](adr/0008-container-push-credentials.md)、2026-09-29 に実地確認）。
  **commit はコンテナ、push はホスト。** `.git` が共有なので成立する。
  push を試みると、**端末が無ければ終了コード 128、端末があれば
  `Username for` を聞いて止まる**（静かには成功しない）
- **コンテナでの commit は `.git/config` の identity に乗っている。**
  `user.name` / `user.email` はリポジトリ固有で、**git の追跡外。**
  clone し直すと消え、`Author identity unknown` で commit が止まる
- **devcontainer は git のワークツリーでは検証できない。**
  ワークツリーの `.git` は親リポジトリを指すファイルで、
  **コンテナ内からその絶対パスを辿れない。** 別ブランチを試すならクローンを作る
- **git-secrets のパターンとフックは git の追跡外**（`.git/config` と `.git/hooks`）。
  clone やコンテナ再作成で消える。`scripts/setup-git-secrets.sh` を回す
  （devcontainer は自動）。**パターンに literal な空白を書かない** —
  git-secrets は連結時に空白で単語分割する
- **`cargo deny` は `path` だけの依存を wildcard と見なす。**
  ワークスペース内のクレートを足すときは `version` も書く
- **ツールチェーンのバージョンが 2 箇所にある**（`rust-toolchain.toml` と `.devcontainer/Dockerfile`）。
  片方を変えたらもう片方も変える
- **コンテナの `target/` はホストと別物**（名前付きボリューム）。
  「ホストでは通るがコンテナで落ちる」の切り分け時に注意
- **`HashMap` の反復順に依存したテストは書かない。** プロセスごとにシードが変わる
- **実行ファイルのサイズや asm の総行数で実装案を比べても差が出ない。**
  lib には案が全て入り、LTO 無しではリンク量が変わらないため。
  比べるなら**当該関数の機械語を読む**（段階 3 でこれに引っかかった）
- **複数のワークツリーが同じ `main` を並行して進めることがある。**
  **これは [ADR-0006](adr/0006-branching-strategy.md) の「ブランチ必須」で
  構造的に閉じた**（各ワークツリーが別 ref を進めるため衝突しない）。
  手順は `CLAUDE.md`「ブランチ運用」が正本。
  **残るのは `git merge --ff-only` の前の rebase で衝突する場合。**
  そのときは force push や reset をせず、
  **まず双方が触ったファイルが重なっているかを突き合わせてから** rebase する

  ```bash
  comm -12 <(git show --name-only --format= <相手のコミット> | sort -u) \
           <(git show --name-only --format= <自分のコミット> | sort -u)
  ```

  （2026-08-17 に双方向で 2 回衝突した。詳細は `docs/journal/2026-08-17.md`）

## 未決事項

1. **解決済み（2026-10-28）。** 完了条件は
   **「各段階の『扱う概念』に挙げた概念を、すべて実コードで消化したとき」**と定めた
   （[ADR-0012](adr/0012-stage-10-associated-types.md) 論点 2。
   基準の本文は `docs/curriculum.md` の末尾）。
   **満たすのは段階 10 を終えたときである** — 残っているのは関連型 1 つ。
   **項目は番号を保つために残す**（未決事項 15）
2. **`Format` を trait に割る条件は決めたが、その監視をしていない。**
   [ADR-0002](adr/0002-output-format-abstraction.md) が「形式 1 つあたり
   概ね 20 行超」「操作が 2 つ以上」を閾値として定めた。
   **形式や操作を足すときに、この閾値を超えていないか見ること。**
   移行するなら新しい ADR で supersede する（ADR-0002 は書き換えない）
3. **「JSON の情報量を上げる」は二段あり、段 1 は済んだ。段 2 は別 ADR が要る。**
   [ADR-0005](adr/0005-selector-public-api.md) 論点 3 で調べた結果:
   - **段 1: JSON Pointer** — **2026-10-20 に実装した**
     （[ADR-0010](adr/0010-json-pointer-key.md)）。検証つき newtype で
     「先頭が `/` でない」を引数解釈の時点で弾く。
     **`Value::pointer` は使っていない** — 段階 7 の改善と衝突するため、
     ストリーミングの seed を入れ子に一般化した
   - **段 2: JSONPath** — 戻り値が 0..n で「1 行 = 1 カウント」が崩れる。
     `Report` の合計・`skipped` の意味・
     `crates/tally/docs/output-format.md` の外部契約に触れる。
     **採るなら新規 ADR。** クレートは jsonpath-rust か serde_json_path。
     **その ADR を書くときに 2 点を再確認する** —
     jsonpath-rust が RFC 9535 準拠かどうか（ADR-0005 論点 3 の表と食い違う情報がある）、
     および `jsonpath_rust::parser::model::JpQuery` が `Eq` を実装しないこと
     （採ると `Key` の `Eq` を手書きするか外すことになり、後者は破壊的変更）
4. **`--strict` は最初の 1 件で止まる（fail fast）。** 入力検査用途では
   「全件報告してから失敗」のほうが有用な場面がある。段階 2 では意図的に見送った。
   必要になれば `--max-errors` を足す
5. **解決済み（2026-09-24）。** 「複数入力でどのファイルの失敗か言えない」件は
   [ADR-0007](adr/0007-multi-input-aggregation.md) 論点 3 で解いた。
   `TallyError::OpenInput` を CLI へ移し、`CliErrorKind::Input` が
   入力の名前を前置する。**項目は番号を保つために残す**（下の番号がずれると
   他の文書からの参照が壊れる）
6. **`cargo-semver-checks` は試したが、CI に入れていない**（2026-09-29 に実施）。
   段階 6 の破壊的変更 2 件（バリアント削除、引数の個数）は検出したが、
   **同じ関数の戻り値と引数の型の変更は検出しなかった**（`stage-log.md` 段階 7）。
   **「検出されなかったこと」を安全の根拠にしない。**
   CI に入れるかは未決 — **baseline は `v0.1.0` が使える**ようになったが
   （2026-10-28。プレリリースは baseline に向かない）、
   **実行に git clone とビルド 2 回が要る**ため。
   **手で通したのは 2026-10-20 と 2026-10-28 の 2 回**
7. **文書に SHA を書かない規約に、機械検査が無い。**
   `CLAUDE.md` は「コミット履歴・SHA は `git log` が答える。文書に書かない」と
   定めているが、`scripts/check-docs.sh` はこれを見ていない。
   **2026-08-19 に handoff へ SHA を 4 件書きかけた**（commit 前に気づいて消した）。
   これは失敗事例 1（`progress.md` の SHA 4 件が全部無効になった）と同じ形。
   **検査に足すなら、`docs/adr/` の日付つき観測**
   （「変更前の値は `<sha>` を取り出して測った」）を誤検知しない形にする必要がある
8. **解決済み（2026-09-29）。** 「`--ff-only` の rebase 強制の経路を通っていない」件は、
   ADR-0008 の Confirmation を載せるときに `main` が先行していて、
   rebase → `--force-with-lease` → CI 再実行 → `--ff-only` が実際に通った
   （[ADR-0006](adr/0006-branching-strategy.md)「3 回目の適用で分かったこと」）。
   **`--ff-only` が rebase 忘れを拒む場面そのものは意図的には試していない。**
   **項目は番号を保つために残す**（下の番号がずれると他の文書からの参照が壊れる）
9. **解決済み（2026-10-28）。** 関連型は `trait Input` で消化した
   （段階 10、[ADR-0012](adr/0012-stage-10-associated-types.md)、
   `learning-log.md` 4-13）。
   **「必要になる課題を用意しないと消化されない」は、そのとおりだった** —
   段階を 1 つ立てるまで出番が来なかった。
   **項目は番号を保つために残す**（未決事項 15）
10. **ADR-0001 が「部分的な supersede」を定めていない。**
    ステータスは `Accepted` → `Superseded` の全体遷移しかなく、
    **「一部の論点だけ置き換わった」状態を表せない。**
    段階 6 では frontmatter に `partially-superseded-by` を足し、
    該当箇所に指し先を書く形で運用した（ADR-0004 と ADR-0005）。
    **これは ADR-0001 の「明確化」の枠を広げて使っている。**
    枠を正式にするなら ADR-0001 を supersede する新しい ADR が要る
11. **依存規則の検査がコメント行を見ない。**
    `scripts/check-module-deps.sh` は `//` で始まる行を落とすので、
    **doc コメント内のコード例に禁止された依存が書かれていても拾えない。**
    doc テストはコンパイルされるので、本来は拾いたい。
    `cargo expand` か rustdoc の JSON を使う案があるが、どちらも重い
12. **`Execution` を trait にする条件を決めたが、監視していない。**
    ADR-0007 論点 5 が「実装が 3 つ以上」「`tally` 以外の消費者」「エラー型の総称化」を
    挙げた。**戦略を足すときにこの条件を見ること**
13. **性能の測定環境が固定されていない。** 段階 7 の数値はホストの
    macOS で取ったもので、**別セッションの負荷で 1.7 倍ぶれた**
    （`stage-log.md` 段階 7）。`uptime` を見てから測り、
    **改善前後のバイナリを 1 回の `hyperfine` で並べる**のが今の対処。
    CI で回す形にはしていない（GitHub の runner は共有なのでさらにぶれる）
14. **一部解決（2026-10-04）。** `samply` / `hyperfine` /
    `cargo-semver-checks` / `miri` は **`README.md` に書いた**（任意の道具として）。
    **版は固定していない。** devcontainer にも CI にも入れていない。
    **理由は道具ごとに違う** — `samply` と `hyperfine` は
    runner が共有でぶれる（未決事項 13）ので CI に意味が薄い。
    **`miri` と `cargo-semver-checks` にはその理由が当たらない** —
    こちらは時間とビルド回数の問題で、入れるかはまだ決めていない
15. **番号の繰り上げに検査が無い。** 解決済みの項目を削って
    繰り上げたとき、**参照する側を全部直せたかを見る仕組みが無い。**
    2026-10-04 に 13 番が欠番になっているのを見つけた（別セッションの
    繰り上げが後ろ 2 件に届いていなかった）。
    **未決事項 7（文書に SHA を書かない規約に検査が無い）と同じ形の穴。**
    番号をやめて見出し文字列で参照する案があるが、参照側の書き換えが要る
16. **解決（2026-10-17）。** ADR-0009 の未検証 3 件は
    `v0.1.0-rc.2` を打って確認した。`status` は `accepted`。
    **番号は繰り上げない**（未決事項 15）。
    **残した教訓**: 走らせたら `v0.1.0-rc.1` が落ちた。
    **「未検証」と書いておいたので嘘にならずに済んだ** —
    「動くはず」と書いていれば 5 日間嘘が残っていた
17. **宣言と実物の一致を見る検査が無い**（2026-10-12 に気づいた）。
    `Cargo.toml` が `license = "MIT"` と宣言しながら
    **`LICENSE` が 8 段階にわたって無かった。**
    `missing_docs` も `check-docs.sh` も「書いたか」しか見ない。
    **足すなら、何と何を突き合わせるかを列挙する必要がある**

18. **本の journal と検査 4 が衝突する**（2026-10-31、[ADR-0013](adr/0013-book-phase.md) 論点 8）。
    `scripts/check-docs.sh` の検査 4 は「journal に出た rustc のエラーコードは
    `learning-log.md` にもある」ことを要求する。
    **一方 ADR-0013 は、Rust 言語の学びを `learning-log.md` に足さないと決めた。**
    **本を書く過程の journal がエラーコードに触れた時点で CI が落ちる。**
    **落ちてから決める** — 検査を `book/` 由来の journal に当てないか、
    例外的に `learning-log.md` へ足すか

CI の稼働状況はここに書かない（腐るため）。
GitHub Actions の実行履歴を見ること。
**`gh` は devcontainer には入っていないが、ホストには入っている** —
`gh run list` / `gh run watch` が使える（2026-08-23 に確認）。
