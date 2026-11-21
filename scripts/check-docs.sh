#!/usr/bin/env bash
# 文書の書き分けを機械的に検査する。
#
# 検査の根拠は CLAUDE.md「文書の書き分け」と docs/stage-log.md の除外リスト。
#
# 使い方: scripts/check-docs.sh
set -uo pipefail
cd "$(dirname "$0")/.."

# 除外設定（.docscheckignore）を読む。本は docs/ の規律では検査しない。
# shellcheck source=scripts/ignore.sh
. "$(dirname "$0")/ignore.sh"

fail=0

# 検査する文書を列挙する。引数は git ls-files の pathspec。
#
# **追跡済みだけでなく、まだ git add していない新しいファイルも含める。**
# 含めないと、書いたばかりの章が add する前は検査されず、
# 何も言わずに OK になる（.gitignore にあるものは含めない）。
#
# 名前に空白や非 ASCII を含むファイルも 1 行 1 件で出す（quotePath を切る）。
# 受け取る側は xargs ではなく grep_each を使う。xargs は空白で割り、
# grep の失敗は 2>/dev/null で消えるので、そのファイルが何も言わずに検査から外れる。
ls_md() {
    git -c core.quotePath=false ls-files --cached --others --exclude-standard -- "$@"
}

# 標準入力の 1 行を 1 ファイルとして grep に渡す。-H でファイルが 1 つでも名前を出す。
grep_each() {
    tr '\n' '\0' | xargs -0 grep -H "$@"
}

# 違反を報告する。$1 = 説明、$2 = 正しい置き場所、以降 = grep の出力
report() {
    local what="$1" where="$2"
    shift 2
    printf '\n\033[31mNG\033[0m %s\n   → %s\n' "$what" "$where"
    printf '%s\n' "$@" | sed 's/^/   /'
    fail=1
}

# 1. curriculum.md に「実」を書かない

hits=$(grep -nE '^#+ .*（済）' docs/curriculum.md || true)
[ -n "$hits" ] && report \
    'curriculum.md の見出しに（済）がある' \
    'どの段階が完了したかは docs/stage-log.md（節の存在が答える）' \
    "$hits"

hits=$(grep -nE '^[[:space:]]*- \[[ x]\]' docs/curriculum.md || true)
[ -n "$hits" ] && report \
    'curriculum.md にチェックボックスがある' \
    '完了条件は「満たされているべき条件」として書く。進行中の状態は docs/handoff.md' \
    "$hits"

hits=$(grep -nE '^#+ .*(実装の記録|証跡|実績)' docs/curriculum.md || true)
[ -n "$hits" ] && report \
    'curriculum.md に実績の節がある' \
    'docs/stage-log.md' \
    "$hits"

# curriculum.md から handoff.md を指さない。
#
# 積み残しや未決事項は **「次に何をするか」で、handoff の領分である。**
# カリキュラム（予）に持ち越しを書くと、**段階を始める人が
# 「自分の課題」と読む**（docs/doc-rules-history.md の 4 と同じ形）。
#
# **handoff を指すこと自体が「状態を指している」の安い代理変数になる。**
# 上の（済）やチェックボックスと違い、**言い換えで逃げられない** —
# 積み残しや未決事項を書けば、必ずその正本を指したくなるため。
#
# 例外は「何をどこに書くか」の表だけ（正しい置き場所の宣言であって、参照ではない）。
hits=$(grep -nE 'handoff' docs/curriculum.md | grep -vE '^[0-9]+:\|' || true)
[ -n "$hits" ] && report \
    'curriculum.md が docs/handoff.md を指している' \
    '持ち越し・未決事項・現在地は docs/handoff.md の側だけが持つ。予には書かない' \
    "$hits"

# 2. テスト名・件数を文書に転記しない
#
# CLAUDE.md「テストが通るか・件数 → cargo t を実行する — 文書に書かない」。
# 段階 5 のクレート分割で、実際に curriculum.md の表が全件腐った。

# 識別子が前置されているものだけを拾う。`::tests::` 単体（この検査自身の
# 説明が docs/doc-checks.md にある）を誤検知しないため。
# shellcheck disable=SC2046  # pathspec を複数の引数として渡す
hits=$(ls_md '*.md' $(ignore_pathspec) | grep_each -nE '[[:alnum:]_]::tests::[^`]' 2>/dev/null || true)
[ -n "$hits" ] && report \
    '文書にテストのフルパスが転記されている' \
    'cargo nextest list が答える。文書には書かない' \
    "$hits"

# 3. stage-log.md に「状態」を書かない
#
# stage-log.md は「完了した実績」だけを持ち、現在地は持たない。

hits=$(grep -nE '^#+ .*(現在地|次の作業|再開)' docs/stage-log.md || true)
[ -n "$hits" ] && report \
    'stage-log.md に現在地・次の作業の節がある' \
    'docs/handoff.md。状態を 2 箇所に置くと片方が腐る' \
    "$hits"

# 4. 言語の学びを journal に閉じ込めない
#
# journal は進め方の反省、learning-log は技術的な学び（CLAUDE.md の 9 と 10）。
# 境界を機械的に見る手がかりとして rustc のエラーコードを使う。
# **journal がコードに触れるのは構わないが、その知識は learning-log にも要る。**

missing=""
for code in $(grep -ohE 'E0[0-9]{3}' docs/journal/*.md 2>/dev/null | sort -u); do
    grep -q "$code" docs/learning-log.md || missing="$missing$code が journal にしか無い"$'\n'
done
[ -n "$missing" ] && report \
    'rustc のエラーコードが journal にしか無い' \
    'コードが示す言語の性質は docs/learning-log.md に書く。journal からは参照する' \
    "$missing"

# 5. 相対リンクの切れ

broken=$(
    # shellcheck disable=SC2046  # pathspec を複数の引数として渡す
    ls_md '*.md' $(ignore_pathspec) | while read -r f; do
        dir=$(dirname "$f")
        grep -oE '\]\([^)#:]+\.md' "$f" 2>/dev/null | sed 's/^](//' | while read -r link; do
            [ -e "$dir/$link" ] || echo "$f -> $link"
        done
    done
)
[ -n "$broken" ] && report \
    'md の相対リンクが切れている' \
    'リンク先を直すか、節を移動したなら参照も直す' \
    "$broken"

# 6. 本に書かないもの
#
# 本は正本ではない（docs/adr/0013-book-phase.md 論点 6）。
# **3 つのうち 2 つは機械的に見える。**
# 残る 1 つ（tally の現在の実装状態）は文字列で見分けられないので検査していない。
# 理由は ADR-0013 論点 6 に書いてある。
#
# **ここだけは book/ を対象にする。** 除外設定は docs/ 向けの検査のためにあり、
# 本には本の検査が要る。

book_md=$(ls_md 'book/*.md' 'book/*.mdx' || true)

if [ -n "$book_md" ]; then
    hits=$(printf '%s\n' "$book_md" | grep_each -nE 'ff-only|force-with-lease|handoff|git switch|git rebase' 2>/dev/null || true)
    [ -n "$hits" ] && report \
        '本がリポジトリの運用に触れている' \
        '読者のリポジトリの話ではない。運用は CLAUDE.md と docs/handoff.md' \
        "$hits"

    hits=$(printf '%s\n' "$book_md" | grep_each -nE '却下|代替案|検討した案' 2>/dev/null || true)
    [ -n "$hits" ] && report \
        '本が却下した設計案に触れている' \
        '検討の過程は docs/adr/ が正本。本には結論と、それが使う言語機能だけを書く' \
        "$hits"

    # ADR-0013 論点 7。本から docs/adr を指すときは GitHub の URL を使う。
    # 相対リンクは下の検査 5 では拾えない（book/ を除外しているため）。
    hits=$(printf '%s\n' "$book_md" | grep_each -nE '\]\([^)]*\.\./[^)]*docs/adr' 2>/dev/null || true)
    [ -n "$hits" ] && report \
        '本から docs/adr へ相対リンクを張っている' \
        'GitHub の URL で指す（ADR-0013 論点 7）。相対リンクはサイトのビルドが落ちる' \
        "$hits"
    # 太字はリンクの内側に書く（[**語**](…)）。
    # 外側（**[語](…)** や **語は[x](y)**）だと、和文に接したところで
    # CommonMark の強調にならない。リンクの直後から太字を始める書き方も拾うので、
    # そのときは間に語を挟む。
    # **ビルドは通るので、生成された HTML を見ないと気づけない。**
    hits=$(printf '%s\n' "$book_md" | grep_each -nE '\*\*\[|\]\([^)]*\)\*\*' 2>/dev/null || true)
    [ -n "$hits" ] && report \
        '本で太字がリンクの外側にある' \
        '[**語**](…) の形にする。和文に接すると **[語](…)** は強調にならない' \
        "$hits"

    # ADR-0013 論点 4c。9 章までのコードは、ライフタイム注釈を 1 つも書かずに通す
    # （注釈は 10 章で、省略が効かない例に当たってから出す）。
    # Confirmation 7 の「10 章まで」は、その 10 章で初めて出すという意味で読む。
    #
    # 見るのはコードだけである。章のクレートの .rs と、本文の rust のコードブロック。
    # 言語の無いブロック（コンパイラの出力）は見ない。rustc の help が `'static` を出すため。
    # 本文の地の文で「10 章で扱う」と名前に触れるのは妨げない。
    # 章の番号は front matter の title の先頭から取る（引用符で囲んだ title も読む）。
    # 文字リテラル（'a' や , 'b'）を拾わないよう、名前の直後の ' を除く。
    # 文字列の中の「, 'word」「: 'word」は誤検知しうる。そのときは書き方を変える。
    lifetime_re="'static|'_([^']|\$)|(&|<|,|\+|:)[[:space:]]*'[a-z][a-z0-9_]*([^a-z0-9_']|\$)|^[[:space:]]*'[a-z][a-z0-9_]*,"
    early_md=$(printf '%s\n' "$book_md" | grep_each -lE "^title: [\"']?[1-9] " 2>/dev/null || true)
    early_rs=$(ls_md 'book/code/ch0[1-9]/*.rs' || true)
    hits=$(
        printf '%s\n' "$early_rs" | grep . | grep_each -nE "$lifetime_re" 2>/dev/null
        # コードブロックの中の行だけを「ファイル:行:中身」で出してから絞る。
        printf '%s\n' "$early_md" | grep . | while IFS= read -r f; do
            awk -v f="$f" '
                /^[[:space:]]*```/ { if (inside) inside = 0; else { inside = 1; rust = ($0 ~ /```(rust|rs)/) }; next }
                inside && rust { print f ":" FNR ":" $0 }' "$f"
        done | grep -E ":[0-9]+:.*($lifetime_re)"
        true
    )
    [ -n "$hits" ] && report \
        '9 章までのコードにライフタイム注釈がある' \
        '注釈なしで通る形に直す。通らないなら ADR-0013 論点 4 の前提が崩れたので、章の順序を見直す' \
        "$hits"
fi

if [ "$fail" -eq 0 ]; then
    echo "文書の書き分け: OK"
else
    printf '\n検査の根拠は CLAUDE.md「文書の書き分け」。\n'
fi
exit "$fail"
