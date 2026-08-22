#!/usr/bin/env bash
# git-secrets のパターンとフックをこのリポジトリに入れる。
#
# **なぜスクリプトなのか。** git-secrets が守るものは 2 つとも
# **git が追跡しない場所**にある。
#
#   - フック   → .git/hooks/（追跡対象外）
#   - パターン → .git/config（追跡対象外）
#
# したがって clone しただけでは保護がゼロになる。**しかもフックは存在するので、
# 「入っている」ように見えて何も検査しない状態になりうる** — 実際 2026-08-19 に
# その状態を発見した（フックは 2026-08-15 から居たが、パターンが 0 件で
# `git secrets --scan` は全て通していた）。
#
# このスクリプトが「追跡される唯一の正本」である。
# バイナリ本体は .devcontainer/Dockerfile が入れる。
#
# 使い方: scripts/setup-git-secrets.sh
# devcontainer では postCreateCommand から自動で呼ばれる。
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v git-secrets >/dev/null 2>&1; then
    cat >&2 <<'MSG'
git-secrets が見つかりません。

  devcontainer:  イメージを再ビルドしてください（.devcontainer/Dockerfile が入れます）
  ホスト直実行:  https://github.com/awslabs/git-secrets の手順で入れてください

MSG
    exit 1
fi

# --- パターン -----------------------------------------------------------
#
# **冪等にするため、まず消してから入れ直す。** `git secrets --add` は
# 重複を検査しないので、2 回走らせると同じパターンが 2 つ並ぶ。
#
# **パターンに literal な空白を書かないこと。** git-secrets は登録された
# パターンを `|` で連結するとき **空白で単語分割する。** 2026-08-19 に
# `BEGIN [A-Z ]*PRIVATE KEY` が `BEGIN|[A-Z|]*PRIVATE|KEY` になり、
# 裸の `PRIVATE` が `C-STRUCT-PRIVATE` に当たる誤検知を起こした。
# 空白は `[[:space:]]` で書く。**同じ制約が `.gitallowed` にもある。**
#
# 許可（false positive の除外）は **`.gitallowed`**（追跡される）に書く。
# `git config secrets.allowed` は追跡されないので使わない。
git config --unset-all secrets.patterns 2>/dev/null || true
git config --unset-all secrets.allowed 2>/dev/null || true
git config --unset-all secrets.providers 2>/dev/null || true

# AWS の鍵（git-secrets 同梱のプロバイダ）。
git secrets --register-aws >/dev/null

# 接頭辞が長く一意なものだけを足す。**誤検知は規約を殺す** —
# 落ちるのが日常になると `--no-verify` が習慣になる。
git secrets --add -- 'BEGIN[A-Z[:space:]]*PRIVATE[[:space:]]KEY'  # 秘密鍵ブロック
git secrets --add -- 'gh[pousr]_[0-9A-Za-z]{36,}'                 # GitHub トークン
git secrets --add -- 'github_pat_[0-9A-Za-z_]{20,}'               # GitHub PAT（新形式）
git secrets --add -- 'sk-ant-[0-9A-Za-z_-]{20,}'                  # Anthropic
git secrets --add -- 'sk-proj-[0-9A-Za-z_-]{20,}'                 # OpenAI
git secrets --add -- 'xox[abprs]-[0-9A-Za-z-]{10,}'               # Slack

# --- フック -------------------------------------------------------------
#
# pre-commit / commit-msg / prepare-commit-msg の 3 本。-f で上書きする（冪等）。
git secrets --install -f >/dev/null

# --- 自己検査 -----------------------------------------------------------
#
# **入れただけでは動作の証拠にならない。** 両方向を見る。
probe=$(mktemp)
trap 'rm -f "$probe"' EXIT

# 陽性: 検出できるか
printf -- '-----BEGIN RSA PRIVATE KEY-----\n' > "$probe"
if git secrets --scan --no-index "$probe" >/dev/null 2>&1; then
    echo "検出できていない。パターンの登録に失敗している" >&2
    exit 1
fi

# 陰性: 作業ツリーで誤検知しないか
if ! git secrets --scan >/dev/null 2>&1; then
    echo "作業ツリーで検出された。内容を確認すること:" >&2
    git secrets --scan >&2 || true
    exit 1
fi

printf 'git-secrets: パターン %s 件、フック 3 本を設定した\n' \
    "$(git config --get-all secrets.patterns | wc -l)"
