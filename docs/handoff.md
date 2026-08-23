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

> **⚠ 最優先の作業がある。** 未 push のコミットが積まれており、
> **`main` はまだ 1 つも進んでいない。** 下の
> 「最優先: 未 push のコミットを `main` に載せる」を先に読むこと。
> 段階 6 はそのあと。

- **実装は段階 5 まで完了**（2026-08-19）。**段階 6（並行・並列）はコード未着手。**
  **各段階の実績は `docs/stage-log.md` が正本**（節がある段階が完了した段階）。
  段階の定義・完了条件は `docs/curriculum.md`
- **ワークスペースのメンバは 2 つになった。**
  `crates/tally-core`（集計コア。`clap` も `anyhow` も持たない）と
  `crates/tally`（薄い CLI）
- **[ADR-0004](adr/0004-error-type-shape.md) と
  [ADR-0005](adr/0005-selector-public-api.md) は `accepted` になった。**
  実装・実測・Confirmation をすべて終えた。**この 2 つに残作業は無い**
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

## クレートの構造

**コードの配置と設計判断は、それぞれのクレートの `docs/layout.md` が正本。**

- `crates/tally-core/docs/layout.md` — 集計コア
- `crates/tally/docs/layout.md` — CLI

この文書には書かない（クレートの性質であって、セッションの状態ではないため）。
読む順序は学習者への指示なので `docs/curriculum.md` 段階 0 にある。

新しいコードをどこに置くか迷ったら、まず **どちらのクレートの話かを決める。**
`tally` 側の `layout.md` の「新しいコードをどこに置くか」がその問いから始まる。

## 最優先: 未 push のコミットを `main` に載せる

**2026-08-19 のセッションはここで終わっている。** コミットは全て済んでいるが、
**コンテナに GitHub の資格情報が無く push できなかった。**

### いまの状態

**ブランチが 3 本、下から順に積まれている。**

```
chore/container-ssh          ← 作業ツリーはこれをチェックアウト中
  └ chore/git-secrets
      └ stage-5/module-design
          └ main             = origin/main（2026-08-19 時点で動いていない）
```

- **リモートには 1 本も push されていない**（`origin` にあるのは `main` だけ）
- 未コミット無し、マージコミット 0 件

**SHA と件数はここに書かない**（`CLAUDE.md` の「コミット履歴・SHA は `git log` が答える」）。
実際の位置はこれで見る。

```bash
git log --oneline --graph --decorate main..chore/container-ssh
git branch -a
git status --short
```

### なぜ止まったか

**コンテナに credential helper / トークン / SSH 鍵 / agent のいずれも無い。**
`git fetch` が通るのは public リポジトリで読み取りが匿名でできるため。

対策は実装済みだが **コンテナを再ビルドしないと効かない** —
`devcontainer.json` がホストの `~/.ssh` を読み取り専用で `~/.ssh-host` にマウントし、
`scripts/setup-git-auth.sh` が正しいパーミッションで複製する。
設計の理由は `CLAUDE.md`「コンテナから GitHub へ push する」。

### 手順

**どこで実行するかが分かれる。** `/workspaces/rust-learn` は
macOS からの virtiofs バインドで、**作業ツリーと `.git` はホストと共有**。
どちらから触っても同じものを見ている。

| # | やること | どこで |
| --- | --- | --- |
| 1 | コンテナを再ビルド（`Rebuild Container`、または `devcontainer up --remove-existing-container`） | ホスト |
| 2 | `postCreateCommand` の出力に `git 認証: Hi karak — push できます` が出ることを確認 | ホスト（ログ） |
| 3 | 全検査を通す（下記） | コンテナ |
| 4 | 3 本を push し、CI が緑になるのを見る | どちらでも |
| 5 | `main` に `--ff-only` で載せる | どちらでも |
| 6 | ADR-0006 の Confirmation を実施して `accepted` にする | コンテナ |

**手順 1 が済むまで push はできない。**
ホスト側に資格情報があるなら、**ホストから push するだけでも 4・5 は進む**
（`.git` が共有なのでコミットはもう見えている）。その場合も
**手順 1 は別途やること** — Dockerfile の git-secrets 導入がまだ実地検証されていない。

#### 手順 3: 通すもの

```bash
scripts/check-docs.sh
cargo fmt --all --check && cargo lint && cargo t
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo deny check
git secrets --scan && git secrets --scan-history
```

**2026-08-19 時点では全て通っていた。** それでも自分で回すこと。

#### 手順 4・5: 積んだ 3 本を下から載せる

**`main` が動いていなければ rebase は不要。** 動いていたら
`CLAUDE.md`「ブランチ運用」の手順に従う（下から順に rebase する）。

```bash
git fetch
git push -u origin stage-5/module-design    # ← CI が回る
git push -u origin chore/git-secrets
git push -u origin chore/container-ssh
# 3 本とも CI が緑になってから
git switch main && git pull --ff-only
git merge --ff-only stage-5/module-design && git push
git merge --ff-only chore/git-secrets       && git push
git merge --ff-only chore/container-ssh     && git push
git branch -d stage-5/module-design chore/git-secrets chore/container-ssh
git push origin --delete stage-5/module-design chore/git-secrets chore/container-ssh
```

#### 手順 6: ADR-0006 を `accepted` にする

[ADR-0006](adr/0006-branching-strategy.md) の Confirmation 2〜5 が
**この作業でしか確かめられない。** 実施して結果を書き、`status` を `accepted` にする。
未決事項 7 も消す。**これ自体が 1 ブランチ 1 単位**（`adr-0006/confirmation` など）。

### 注意

- **CI が緑になる前に `main` へ載せない。** ADR-0006 の決定はそこが要点
- **`--force` を使わない。** rebase して push し直すなら `--force-with-lease`
- **`main` のマージコミットは 0 件を保つ。** `git log --merges --oneline | wc -l`
- **`.git/hooks` と `.git/config` はホストと共有されている。**
  `secrets.patterns` 11 件はホスト側の commit にも効く。
  **ホストで commit するなら、ホストにも `git-secrets` が必要**
  （無いと `git: 'secrets' is not a git command` で止まる）
- **`gh` はコンテナに無い。** CI の確認は GitHub Actions のページで行う

---

## 次の作業: 段階 6（並行・並列）

課題の内容は `docs/curriculum.md` の段階 6 が正本。ここには着手手順だけを書く。

**着手前にやること**:

1. **完了条件を先に `docs/curriculum.md` に書く。** 段階 4・5 と同じ手順。
   **とくにベンチの合格条件を先に決める** — 「速くなること」ではなく
   「どの条件で速くならないことを確認するか」を書く
2. **公開 API が変わるので、ADR が要るか判断する。** 段階 6 は
   「複数ファイルを引数に取る」ため、次の 2 つに触れる:
   - **`CliErrorKind::Tally` が `#[error(transparent)]` で path を持たない件。**
     ADR-0004 の Confirmation に「複数入力を扱うようになったら見直す」と書いた。
     **入力が複数になった時点で、どのファイルの失敗かが言えなくなる**
   - **`Counter` のマージ**（`Extend` か専用の `merge`）。
     `Counter` は非公開フィールドなので、公開 API が 1 つ増える
3. **`tally_reader` の形を見直す好機。** 段階 5 では
   `tally_reader(counter, reader, selector, keep, limit)` に留めたが、
   ADR-0005 の「実装時に決めたこと」に **複数入力を 1 つの集計に合流させたく
   なったら見直す**と書いた。段階 6 がまさにそれ

**進め方**: テストを先に書く。**コンパイルエラーは red ではない** —
型の骨格だけ足して通し、アサーションが落ちることを確認してから実装する。
振る舞いが複数あるならサイクルを分ける（段階 2 は 3 サイクルに分けた）。

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
- **コンテナからの push にはホストの SSH 鍵のマウントが要る。**
  `devcontainer.json` の mounts と `scripts/setup-git-auth.sh`。
  **`.git` はホストと共有されているので remote URL は書き換えない**
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
5. **複数入力になったとき、どのファイルの失敗かを言えない。**
   段階 5 で `CliErrorKind::Tally` を `#[error(transparent)]` にし、
   path の文脈を被せないと決めた（`TallyError::OpenInput` と二重になるため）。
   **入力が最大 1 つという前提に乗っている。** 段階 6 で崩れる
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

CI の稼働状況はここに書かない（腐るため）。
GitHub Actions の実行履歴を見ること（**`gh` は devcontainer に入っていない**）。
