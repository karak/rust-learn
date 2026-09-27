#!/usr/bin/env bash
# 段階 7 の測定用に、大きな入力を再現可能に作る。
#
# **同じ入力を作り直せることが目的。** 改善の前後を比べるとき、
# 入力が変わっていたら数値の差が何に由来するか分からない。
# 乱数を使わず、行番号から決定的に作る。
#
# 使い方:
#   scripts/gen-input.sh <出力先ディレクトリ> [行数]
#
# 作るもの（既定は 1000 万行）:
#   <出力先>/json.log   1 行 1 JSON。`--field lvl` で集計する（パースが重い）
#   <出力先>/plain.log  素の行。行全体をキーにする（パースが無い）
#
# **`target/` の下に作ること。** git に載せない。
set -euo pipefail

out=${1:?出力先ディレクトリを指定すること}
lines=${2:-10000000}

mkdir -p "$out"

# awk で作る。shell のループでは 1000 万行に耐えない。
# レベルは 4 種類を循環させる（キーの種類が少ない側）。
awk -v n="$lines" 'BEGIN {
    split("info warn error debug", level, " ")
    for (i = 0; i < n; i++) {
        printf "{\"lvl\":\"%s\",\"seq\":%d}\n", level[(i % 4) + 1], i
    }
}' > "$out/json.log"

awk -v n="$lines" 'BEGIN {
    split("info warn error debug", level, " ")
    for (i = 0; i < n; i++) {
        print level[(i % 4) + 1]
    }
}' > "$out/plain.log"

printf '%s に %s 行を 2 種類作った:\n' "$out" "$lines"
ls -lh "$out/json.log" "$out/plain.log"
