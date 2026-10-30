#!/usr/bin/env bash
# .docscheckignore を読む。検査スクリプトから source して使う。
#
# **なぜ共通化するのか。** 除外もまた規約であり、
# **2 箇所に書けば片方が腐る**（CLAUDE.md「同じ事実を 2 箇所に書かない」）。
# 除外の理由を 1 ファイルに集め、検査の側はそれを読むだけにする。
#
# 使い方:
#   . "$(dirname "$0")/ignore.sh"
#   git ls-files '*.md' $(ignore_pathspec)   # 除外つきで列挙する
#   is_ignored "$f" && continue              # ループの中で弾く

# 空行とコメントを落とし、末尾の `/` を外したパターンを 1 行ずつ出す。
ignore_patterns() {
    [ -f .docscheckignore ] || return 0
    sed -e 's/#.*//' -e 's#/*[[:space:]]*$##' -e '/^[[:space:]]*$/d' .docscheckignore
}

# git の pathspec の除外指定に変換する。`git ls-files` の引数にそのまま渡す。
ignore_pathspec() {
    ignore_patterns | sed 's#^#:(exclude)#;s#$#/*#'
}

# $1 が除外対象なら 0 を返す。
is_ignored() {
    local path="$1" pat
    while IFS= read -r pat; do
        [ -n "$pat" ] || continue
        # shellcheck disable=SC2254  # パターンは glob として展開させる
        case "$path" in
        $pat | $pat/*) return 0 ;;
        esac
    done < <(ignore_patterns)
    return 1
}
