#!/usr/bin/env bash
# モジュール間の依存規則を機械的に検査する。
#
# **なぜ要るのか。** [ADR-0007](../docs/adr/0007-multi-input-aggregation.md) 論点 5 は、
# 並列化ポリシーを **入力の種類からも集計の中身からも独立した部品** として
# `crates/tally/src/aggregate.rs` に置くと決めた。CLI クレートの中にあるので、
# **放っておけば `clap` や `std::fs` が混入し、切り出せない部品に退化する。**
#
# 「移せるように書く」は守り忘れられる規約である。だから検査にする
# （CLAUDE.md「規約ではなく検査にする」）。
#
# **規則の正本は `crates/tally/docs/layout.md` の層の表。** ここはその実装。
#
# **限界（意図的）**:
#   - **コメント行は見ない。** doc コメント内のコード例に禁止された依存が
#     書かれていても拾えない（doc テストはコンパイルされるので、本来は拾いたい）
#   - 依存の向きは **識別子の出現**で見る。マクロ展開後に現れるものは拾えない
#
# 使い方: scripts/check-module-deps.sh
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0

report() {
    local file="$1" banned="$2" why="$3"
    shift 3
    printf '\n\033[31mNG\033[0m %s が %s に依存している\n   → %s\n' "$file" "$banned" "$why"
    printf '%s\n' "$@" | sed 's/^/   /'
    fail=1
}

# コメント行と空行を落とし、行番号を保ったまま出す。
# `//` `///` `//!` で始まる行だけを落とす（行末コメントは残るが、
# そこに依存が書かれていれば本体にも書かれている）。
code_lines() {
    grep -nv '^[[:space:]]*//' "$1"
}

# $1 = ファイル, $2 = 禁止の正規表現, $3 = 人間向けの名前, $4 = 理由
deny() {
    local file="$1" pattern="$2" name="$3" why="$4"
    [ -f "$file" ] || return 0
    local hits
    hits=$(code_lines "$file" | grep -E "$pattern" || true)
    [ -n "$hits" ] && report "$file" "$name" "$why" "$hits"
    return 0
}

# --- 層 2: aggregate は「並列化ポリシー」だけを持つ ----------------------
#
# **切り出せる状態を保つことが目的。** ファイルも引数も整形も知らない。

AGG=crates/tally/src/aggregate.rs
deny "$AGG" '\bclap\b' 'clap' \
    '引数の形が変わっても aggregate は変わらないこと。Cli 型を受け取らない（ADR-0007 論点 5）'
deny "$AGG" '\bregex\b|\bRegex\b' 'regex' \
    '行を通すかどうかは述語として渡す。正規表現の実装を持ち込まない'
deny "$AGG" 'std::fs|File::open|\bBufReader\b' 'ファイルを開く操作' \
    '開くのは呼び出し側（ADR-0007 論点 3）。ジョブは閉包として渡される'
deny "$AGG" 'crate::cli|crate::format|super::cli|super::format' '上位の層' \
    '層 2 は層 3 を知らない。知ると tally から切り出せなくなる'

# --- 層 1: error は他の tally モジュールを知らない ------------------------

ERR=crates/tally/src/error.rs
deny "$ERR" '\bclap\b' 'clap' \
    '終了コードと hint の決定に引数解釈は要らない'
deny "$ERR" '\brayon\b' 'rayon' \
    '失敗の分類は実行戦略を知らない'
deny "$ERR" 'crate::(cli|format|aggregate)|super::(cli|format|aggregate)' '上位の層' \
    '層 1 は最下層。上を知ると依存が循環する'

# --- 層 3: input は I/O の境界。引数と並列化は知らない -------------------
#
# **ファイルを開いてよい唯一の lib モジュール。** 逆に、引数の形（clap）と
# 実行戦略（rayon）からは独立している。ベンチとテストから呼ぶために lib にある。

IN=crates/tally/src/input.rs
deny "$IN" '\bclap\b' 'clap' \
    '引数の形が変わっても、入力を開いて数える手順は変わらない'
deny "$IN" '\brayon\b|crate::aggregate' '並列化' \
    '1 単位の集計は、何単位を同時に走らせるかを知らない'
deny "$IN" 'crate::cli|crate::format' '上位の層' \
    '層 3 は層 3 を横断しない。Cli 型ではなく素の引数を受ける'

# --- 層 3: cli / format はファイルを開かず、並列化も知らない --------------

for f in crates/tally/src/cli.rs crates/tally/src/format.rs; do
    deny "$f" 'std::fs|File::open' 'ファイルを開く操作' \
        'I/O を開くのは main.rs（layout.md のファイルの配置）'
    deny "$f" '\brayon\b|crate::aggregate' '並列化' \
        '整形と引数解釈は実行戦略を知らない'
done

# --- 層 0: tally-core は CLI とも並列化とも無縁 ---------------------------
#
# クレート単位の依存は cargo deny と cargo tree が見るが、
# **「まだ Cargo.toml に無い依存を書こうとした」段階で落としたい。**

for f in crates/tally-core/src/*.rs; do
    deny "$f" '\bclap\b|\banyhow\b|\brayon\b' 'CLI / 実行戦略のクレート' \
        'tally-core は行を数えるだけ（crates/tally-core/docs/layout.md の不変条件）'
    deny "$f" 'std::fs|File::open' 'ファイルを開く操作' \
        'このクレートはファイルを開かない。BufRead を受け取るのが唯一の I/O 境界'
done

# --- 結果 ---------------------------------------------------------------

if [ "$fail" -eq 0 ]; then
    echo "モジュール間の依存: OK"
else
    printf '\n規則の正本は crates/tally/docs/layout.md の層の表。\n'
    printf '根拠は docs/adr/0007-multi-input-aggregation.md 論点 5。\n'
fi
exit "$fail"
