#!/usr/bin/env bash
# コンテナから GitHub へ push できるようにする。
#
# **前提が 2 つある。**
#
#   1. /workspaces/rust-learn はホストからの bind マウントで、**.git は共有**。
#      remote URL を書き換えるとホスト側の push にも影響するので **触らない**
#   2. /home/vscode はボリュームではなく **再ビルドで消える**。
#      鍵も設定も「置く」のではなく、作成のたびに入れ直す
#
# したがって:
#   - ホストの ~/.ssh は **読み取り専用で別パス**（~/.ssh-host）にマウントする
#     （devcontainer.json）。macOS からのマウントは ssh が要求する 600 を
#     満たさないことが多く、読み取り専用では chmod もできないため、複製する
#   - HTTPS の remote を SSH で扱うのは `url.insteadOf`。
#     **コンテナ内の ~/.gitconfig にだけ書く**ので、共有された .git は無傷
#
# 使い方: scripts/setup-git-auth.sh
# devcontainer では postCreateCommand から自動で呼ばれる。
set -uo pipefail

host_ssh="$HOME/.ssh-host"
mine="$HOME/.ssh"

if [ ! -d "$host_ssh" ]; then
    cat >&2 <<'MSG'
[skip] ホストの ~/.ssh がマウントされていません。push はできません（fetch は可能）。

  有効にするには devcontainer.json の mounts を確認し、コンテナを作り直してください。
  ホスト側に ~/.ssh が無い場合は、先に GitHub 用の鍵を作って登録してください。

MSG
    # **ここでは失敗させない。** 資格情報が無くても読み取りと開発は成立する。
    # 気づかないまま進む心配も無い — push すれば git がその場で明確に失敗する。
    # （git-secrets と違い「静かに無防備」にはならないので、警告で足りる）
    exit 0
fi

# --- 鍵を複製する（パーミッションを ssh の要求に合わせる） --------------
mkdir -p "$mine"
chmod 700 "$mine"
# ディレクトリは辿らない。鍵と設定だけを取る。
find "$host_ssh" -maxdepth 1 -type f -exec cp -f {} "$mine/" \;
chmod 600 "$mine"/* 2>/dev/null || true
# 公開鍵と known_hosts は 644 でよい（600 でも動くのでそのままにする）。

# --- github.com のホスト鍵 ---------------------------------------------
#
# ホストの known_hosts に入っていれば上の複製で来ている。無ければ取りに行く。
# **取りに行くのは初回だけ**にして、以後は固定されたものを使う。
if ! ssh-keygen -F github.com -f "$mine/known_hosts" >/dev/null 2>&1; then
    echo "known_hosts に github.com が無いので ssh-keyscan で取得します" >&2
    ssh-keyscan -t rsa,ecdsa,ed25519 github.com >> "$mine/known_hosts" 2>/dev/null
fi

# --- HTTPS の remote を SSH で扱う --------------------------------------
#
# **コンテナ内の ~/.gitconfig にだけ書く。** リポジトリの .git/config は
# ホストと共有されているので触らない。
git config --global url."git@github.com:".insteadOf "https://github.com/"

# --- 疎通確認 -----------------------------------------------------------
#
# GitHub は shell を提供しないので `ssh -T` は **終了コード 1 を返して成功**する。
# 終了コードではなくメッセージを見る。
reply=$(ssh -o BatchMode=yes -o StrictHostKeyChecking=yes -T git@github.com 2>&1 || true)
case "$reply" in
    *"successfully authenticated"*)
        echo "git 認証: ${reply%%.*} — push できます"
        ;;
    *)
        printf '[warn] GitHub への SSH 認証を確認できませんでした:\n  %s\n' "$reply" >&2
        echo "  鍵が GitHub に登録されているか確認してください" >&2
        ;;
esac
