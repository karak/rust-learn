# 文書の検査

`scripts/check-docs.sh` が何を検査し、どこで走るか。
**検査の実体はスクリプトである。** この一覧と食い違ったら、スクリプトが正しい。

## 書かないもの

- **判定の細かな条件と実装** — `scripts/check-docs.sh` 本体とそのコメント
- **検査の根拠になった失敗の経緯** — [doc-rules-history.md](doc-rules-history.md)
- **書き分けの規則そのもの** — `CLAUDE.md`「文書の書き分け」
- **実行コマンド** — `CLAUDE.md`「コマンド」

## 何を検査するか

`scripts/check-docs.sh` が次を機械的に検査し、**CI で回る**。

| 検査 | 根拠 |
| --- | --- |
| `curriculum.md` の見出しに（済）が無い | 完了状況は `stage-log.md` |
| `curriculum.md` にチェックボックスが無い | 状態を持たせると予実管理メモに戻る |
| `curriculum.md` に実績の節が無い | 「実」は `stage-log.md` |
| **`curriculum.md` が `docs/handoff.md` を指さない** | 持ち越し・未決事項は handoff の側だけが持つ。予に書くと「自分の課題」と読まれる |
| **どの文書にもテストのフルパス（`::tests::`）が無い** | `cargo nextest list` が答える |
| **`aggregate` が `clap` / `regex` / ファイル操作に触れない** | 並列化ポリシーを切り出せる状態に保つ（ADR-0007 論点 5） |
| `stage-log.md` に現在地・次の作業の節が無い | 状態は `handoff.md` の 1 箇所 |
| **journal に出た rustc のエラーコードが `learning-log.md` にもある** | 言語の性質は学びの側。journal は参照するだけ |
| md の相対リンクが切れていない | [経緯](doc-rules-history.md) 3 の再発防止 |
| **`book/` がリポジトリの運用に触れない** | 読者のリポジトリの話ではない（ADR-0013 論点 6） |
| **`book/` が却下した設計案に触れない** | 検討の過程は `docs/adr/` が正本 |
| **`book/` から `docs/adr/` への相対リンクが無い** | サイトのビルドが落ちる（ADR-0013 論点 7） |
| `book/` で太字がリンクの外側に無い | 和文に接すると強調にならず、ビルドは通る（2026-11-21 に起きた） |
| 9 章までのコード（章のクレートと本文のコードブロック）にライフタイム注釈が無い | 注釈は 10 章で出す（ADR-0013 論点 4c） |
| **まだ `git add` していない文書も、名前に空白や非 ASCII を含む文書も検査する** | どちらも、何も言わずに検査から外れていた |

## どこで走るか

検査は **3 層で走る。**

| 層 | いつ | 設定場所 |
| --- | --- | --- |
| **Claude のフック** | 文書を書き換えた直後 | `.claude/settings.json` の `PostToolUse`（`Write` / `Edit`） |
| 手で実行 | コミット前 | `CLAUDE.md`「コマンド」 |
| CI | push のたび | `.github/workflows/ci.yml` の `docs` ジョブ |

