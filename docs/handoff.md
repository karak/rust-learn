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

- **段階 8 まで完了**（2026-10-04）。**段階 9（配布）は未着手。**
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
- **道具が 3 つ増えた** — `samply`（プロファイラ）、`hyperfine`（時間測定）、
  `cargo-semver-checks`。**いずれもホストに入れたもので、リポジトリには入っていない**
- **`tally` は `anyhow` を使わない。** `CLAUDE.md` の方針 3 を段階 5 で書き換えた
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

## 次の作業: 段階 9（配布）

課題の内容は `docs/curriculum.md` の段階 9 が正本。ここには着手手順だけを書く。

**着手前にやること**:

1. **完了条件を先に `docs/curriculum.md` に書く。** 段階 6・7・8 と同じ手順
2. **これがカリキュラム名目上の最終段階である。** 未決事項 1
   （「カリキュラムの完了条件が未定義」）に答えを出す機会になる

**積み残しが 2 件ある**（どちらも段階 6 由来）:

- **キャッシュが冷えた状態での測定。** `sudo purge` が要るので
  **Claude からは実行できない**（端末がパスワードを要求する）。
  ホストのターミナルで次を実行して記録する:
  `hyperfine --runs 3 -p 'sudo purge' 'target/release/tally --field lvl target/perf/json.log' 'target/release/tally -j 1 --field lvl target/perf/json.log'`
  測定用の入力は `scripts/gen-input.sh target/perf 10000000` で作れる
- **`Box<dyn BufRead>` の間接呼び出しのコスト**（`learning-log.md` 15 節）

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

1. **カリキュラムの完了条件が未定義。** 名目上の終点は段階 9 だが、
   「役目を終えた」とする基準は決めていない
2. **`Format` を trait に割る条件は決めたが、その監視をしていない。**
   [ADR-0002](adr/0002-output-format-abstraction.md) が「形式 1 つあたり
   概ね 20 行超」「操作が 2 つ以上」を閾値として定めた。
   **形式や操作を足すときに、この閾値を超えていないか見ること。**
   移行するなら新しい ADR で supersede する（ADR-0002 は書き換えない）
3. **「JSON の情報量を上げる」は二段あり、段 2 は別 ADR が要る。**
   [ADR-0005](adr/0005-selector-public-api.md) 論点 3 で調べた結果:
   - **段 1: JSON Pointer** — `serde_json` の `Value::pointer` に既にあり**依存ゼロ**。
     戻り値が 0..1 なので `extract` も `Counter` も無変更。
     **`Key::JsonPointer` として足せる**（`#[non_exhaustive]` により非破壊）。
     足すときは「先頭が `/` でない」を黙って `None` にしないよう、
     **検証つき newtype を検討する**
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
   CI に入れるかは未決 — baseline をどこに取るか（直前のタグが無い）と、
   **実行に git clone とビルド 2 回が要る**ため
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
9. **`AsRef` / `Deref` と関連型を、実際のコードで一度も使っていない。**
   段階 5 で回収を試みたが、**trait を自分で定義するまで出番が来ない**と分かった。
   段階 6 でも来ない見込み。**必要になる課題を用意しないと消化されない**
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

CI の稼働状況はここに書かない（腐るため）。
GitHub Actions の実行履歴を見ること。
**`gh` は devcontainer には入っていないが、ホストには入っている** —
`gh run list` / `gh run watch` が使える（2026-08-23 に確認）。
