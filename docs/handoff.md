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

- **実装は段階 6 まで完了**（2026-09-24）。**段階 7（性能とメモリ）は未着手。**
  **各段階の実績は `docs/stage-log.md` が正本**（節がある段階が完了した段階）。
  段階の定義・完了条件は `docs/curriculum.md`
- **ワークスペースのメンバは 2 つになった。**
  `crates/tally-core`（集計コア。`clap` も `anyhow` も持たない）と
  `crates/tally`（薄い CLI）
- **[ADR-0004](adr/0004-error-type-shape.md) と
  [ADR-0005](adr/0005-selector-public-api.md) は `accepted` になった。**
  実装・実測・Confirmation をすべて終えた。**この 2 つに残作業は無い**
- **[ADR-0006](adr/0006-branching-strategy.md) も `accepted` になった**（2026-08-23）。
  **ただし決定の全経路を通したわけではない** — 未決事項 8
- **[ADR-0007](adr/0007-multi-input-aggregation.md) も `accepted` になった**（2026-09-24）。
  複数入力・マージ・失敗の文脈・並列化の置き場所を決め、実装と実測まで終えた。
  **ADR-0004 と ADR-0005 を部分的に supersede している**（未決事項 11）
- **`tally` は複数ファイルを取り、ファイル単位で並列に集計する。**
  `-j` / `--jobs` で並列度を指定でき、`-j 1` は逐次（検証の経路）
- **モジュール間の依存規則を `scripts/check-module-deps.sh` が検査する。**
  規則の正本は `crates/tally/docs/layout.md` の層の表
- **未 push のブランチは無い**（2026-08-23 時点）。
  **ただし push できるのはホストからだけ** — コンテナからの push は動いていない（未決事項 10）
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

## 次の作業: 段階 7（性能とメモリ）

課題の内容は `docs/curriculum.md` の段階 7 が正本。ここには着手手順だけを書く。

**着手前にやること**:

1. **完了条件を先に `docs/curriculum.md` に書く。** 段階 6 と同じく、
   **測る前に予測を書く。** 段階 6 では予測 4 件が当たったが、
   **当たったこと自体より「予測を先に書いたので切り分けができた」ことが効いた**
   （同じ総行数でファイル数だけ動かしたので、`open` の費用と並列化の効果を分離できた）
2. **段階 6 で積み残した測定が 2 件ある。**
   - **キャッシュが冷えた状態での測定**（`sudo purge` + `hyperfine`）。
     `divan` の反復の中では作れない
   - **`Box<dyn BufRead>` の間接呼び出しのコスト**（`learning-log.md` 13 節）
3. **`cargo-semver-checks` を試す好機。** 段階 6 で公開 API を実際に壊したので、
   **検出されるはずの変更が検出されるか**を確かめられる（未決事項 6）

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
- **コンテナからの push は動かない**（2026-08-23 に確認）。
  `devcontainer.json` の mounts と `scripts/setup-git-auth.sh` は
  ホストの SSH 鍵を複製する設計だが、**鍵を持っていることと使えることは別**だった。
  詳細と代替案は未決事項 10。
  **当面はコンテナで commit し、ホストから push する**（`.git` は共有なので成立する）。
  **remote URL は書き換えない** — `.git/config` がホストと共有されているため
  （`url.insteadOf` をコンテナ内の `~/.gitconfig` にだけ置く）
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
6. **`cargo-semver-checks` を導入していない。** 段階 5 の
   「検討する価値のあるもの」に挙げたまま。公開 API が固まった今が試し時だが、
   **「検出されなかったこと」を安全の根拠にしない**（フィールドや引数の
   **型**が変わった系は未カバーと明記されている）
7. **文書に SHA を書かない規約に、機械検査が無い。**
   `CLAUDE.md` は「コミット履歴・SHA は `git log` が答える。文書に書かない」と
   定めているが、`scripts/check-docs.sh` はこれを見ていない。
   **2026-08-19 に handoff へ SHA を 4 件書きかけた**（commit 前に気づいて消した）。
   これは失敗事例 1（`progress.md` の SHA 4 件が全部無効になった）と同じ形。
   **検査に足すなら、`docs/adr/` の日付つき観測**
   （「変更前の値は `<sha>` を取り出して測った」）を誤検知しない形にする必要がある
8. **`--ff-only` の「rebase を機械的に強制する」経路を通っていない。**
   [ADR-0006](adr/0006-branching-strategy.md) は 2026-08-23 に `accepted` になったが、
   **そのとき `main` が動いていなかったので rebase も `--force-with-lease` も要らなかった。**
   決め手として挙げた強制が発動する場面は、**次に `main` が先行したときが初回**になる
9. **`AsRef` / `Deref` と関連型を、実際のコードで一度も使っていない。**
   段階 5 で回収を試みたが、**trait を自分で定義するまで出番が来ない**と分かった。
   段階 6 でも来ない見込み。**必要になる課題を用意しないと消化されない**
10. **コンテナから push する方法が決まっていない。**
    2026-08-23 にホストで確認した事実: `~/.ssh/config` が `github.com` に指定している
    鍵は**パスフレーズ付き**で、**ssh-agent には identity が 0 件**。
    ホストで `ssh -T git@github.com` 自体が通らない。
    **鍵ファイルを複製する現行設計は、複製先でも解錠できない。**
    検討した候補は 3 つ。**(a) ssh-agent をフォワードする**
    — 鍵の実体がコンテナに入らないので最も安全。ホストで一度
    `ssh-add --apple-use-keychain` する必要があり、`devcontainer up` CLI では
    socket の bind mount が別途要る（VS Code の拡張は自動でやる）。
    **(b) パスフレーズ無しの ed25519 を deploy key として登録する**
    — 非対話で通るが平文の秘密鍵が残る。被害はこのリポジトリに閉じる。
    **(c) token を渡す** — 最も速いが、classic PAT の `repo` は全リポジトリに及ぶ。
    採るなら fine-grained + 期限つきに絞ること。
    **決めるまでは push をホストから行う**（落とし穴の項も参照）

CI の稼働状況はここに書かない（腐るため）。
GitHub Actions の実行履歴を見ること。
**`gh` は devcontainer には入っていないが、ホストには入っている** —
`gh run list` / `gh run watch` が使える（2026-08-23 に確認）。

11. **ADR-0001 が「部分的な supersede」を定めていない。**
    ステータスは `Accepted` → `Superseded` の全体遷移しかなく、
    **「一部の論点だけ置き換わった」状態を表せない。**
    段階 6 では frontmatter に `partially-superseded-by` を足し、
    該当箇所に指し先を書く形で運用した（ADR-0004 と ADR-0005）。
    **これは ADR-0001 の「明確化」の枠を広げて使っている。**
    枠を正式にするなら ADR-0001 を supersede する新しい ADR が要る
12. **依存規則の検査がコメント行を見ない。**
    `scripts/check-module-deps.sh` は `//` で始まる行を落とすので、
    **doc コメント内のコード例に禁止された依存が書かれていても拾えない。**
    doc テストはコンパイルされるので、本来は拾いたい。
    `cargo expand` か rustdoc の JSON を使う案があるが、どちらも重い
13. **`Execution` を trait にする条件を決めたが、監視していない。**
    ADR-0007 論点 5 が「実装が 3 つ以上」「`tally` 以外の消費者」「エラー型の総称化」を
    挙げた。**戦略を足すときにこの条件を見ること**
