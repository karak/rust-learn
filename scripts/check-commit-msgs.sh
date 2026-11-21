#!/usr/bin/env bash
# コミットのメッセージに、リポジトリに入れない語が無いかを検査する。
#
# scripts/check-docs.sh の検査 7 はファイルしか見ない。メッセージも履歴として公開されるので、
# 同じ語をここで検査する（2026-11-21、学習者の指示）。
#
# 使い方:
#   scripts/check-commit-msgs.sh          HEAD から辿れる全コミットを検査する（CI）
#   scripts/check-commit-msgs.sh <file>   メッセージのファイル 1 つを検査する（commit-msg フック）
#
# フックは scripts/setup-git-secrets.sh が入れる。
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

# パターンの正本は check-docs.sh の検査 7。ここに写すと 2 箇所になるので、読み出す。
# 読み出せなかったら、何も検査せずに通ってしまうので、落とす。
banned_re=$(sed -n "s/^banned_re='\(.*\)'\$/\1/p" "$root/scripts/check-docs.sh")
if [ -z "$banned_re" ]; then
    echo "scripts/check-docs.sh から banned_re を読み出せなかった。書き方が変わったなら、この行も直す" >&2
    exit 2
fi

hint='「連携先」と言い換える。理由は scripts/check-docs.sh の検査 7 のコメント'

if [ $# -ge 1 ]; then
    # git が編集画面に足す「#」の行は、メッセージに残らないので見ない。
    hits=$(grep -v '^#' "$1" | grep -nE "$banned_re" || true)
    if [ -n "$hits" ]; then
        printf '\033[31mNG\033[0m コミットのメッセージに、リポジトリに入れないと決めた語がある\n   → %s\n' "$hint" >&2
        printf '%s\n' "$hits" | sed 's/^/   /' >&2
        exit 1
    fi
    exit 0
fi

# 浅い clone だと古いコミットが見えず、見ていないのに OK と言ってしまう。
if [ "$(git -C "$root" rev-parse --is-shallow-repository)" = true ]; then
    echo "浅い clone では全履歴を検査できない。CI なら actions/checkout に fetch-depth: 0 を渡す" >&2
    exit 2
fi

hits=$(
    git -C "$root" rev-list HEAD | while read -r sha; do
        git -C "$root" log -1 --format=%B "$sha" | grep -nE "$banned_re" | sed "s/^/${sha:0:7}:/"
    done
)
if [ -n "$hits" ]; then
    printf '\033[31mNG\033[0m コミットのメッセージに、リポジトリに入れないと決めた語がある\n   → %s\n' "$hint"
    printf '%s\n' "$hits" | sed 's/^/   /'
    exit 1
fi
echo "コミットのメッセージ: OK（$(git -C "$root" rev-list --count HEAD) 件）"
