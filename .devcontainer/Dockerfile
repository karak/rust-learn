# ツールチェーンをイメージに焼き込む。
# rust-toolchain.toml と **同じバージョン** を指定すること。
# ずれていると、コンテナ起動後に rustup が 1.97.1 を追加ダウンロードし、
# 「固定した」はずの環境が実行時のネットワークに依存してしまう。
FROM rust:1.97.1-bookworm

ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

# 非 root ユーザで作業する。root で作ったファイルはホスト側から
# 触れなくなる（bind mount 経由で uid が漏れる）ため。
#
# lldb は **入れない**。デバッグは VS Code 拡張 CodeLLDB（vadimcn.vscode-lldb）が
# 自前の lldb を同梱するため不要で、apt から入れると LLVM 一式で
# イメージが 550MB 増える（実測: 2.38GB → 3.09GB）。
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m -s /bin/bash $USERNAME \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        sudo \
        git \
        curl \
        ca-certificates \
        less \
        pkg-config \
        libssl-dev \
    && echo "$USERNAME ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME \
    && rm -rf /var/lib/apt/lists/*

# ツールチェーンと開発ツールを 1 レイヤで入れる。
#
# - rustup component add: rust-toolchain.toml が要求するものを先に入れておく。
#   入れておかないと、初回の cargo 実行時にダウンロードが走る。
# - cargo-binstall: cargo install はソースからのビルドで 10 分以上かかるため、
#   ビルド済みバイナリを取得する。
# - chmod は **ここで追加したファイル** のために CARGO_HOME にだけ当てる。
#   公式 rust イメージは RUSTUP_HOME / CARGO_HOME の両方を既に開放済みなので、
#   base 由来のファイルに再度 chmod する必要はない。むしろ RUSTUP_HOME まで
#   chmod -R すると、変更のないファイルまでこのレイヤに複製される（実測 +280MB）。
#   一方 cargo binstall が root で置いたバイナリは 755 で入るため、
#   vscode から cargo install で上書きできるようここだけ開けておく。
#
# 注意: バージョン固定を担保しているのは **イメージタグと rust-toolchain.toml** であって、
# ファイルパーミッションではない。コンテナ内から rustup でツールチェーンを
# 差し替えることは技術的には可能（curriculum 段階 8 の miri 追加はこれを使う）。
RUN rustup component add rustfmt clippy rust-src rust-analyzer \
    && curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
        | bash \
    && cargo binstall -y cargo-nextest cargo-deny cargo-expand \
    && chmod -R a+w "$CARGO_HOME"

# git-secrets（コミット前の秘密情報スキャン）。
#
# **apt には無い**ので単一の bash スクリプトを直接置く。13KB なのでレイヤは無視できる。
#
# バージョンは **コミット SHA で固定する。** 最新リリースの 1.3.0 は 2019 年で、
# 秘密検出のパターンが古い（Bedrock のキー形式などが入っていない）。
# raw の URL は SHA でコンテンツアドレスされるが、**sha256 も検証する** —
# 「固定した」を配信側の善意に依存させない。
#
# パターンとフックはイメージには入らない（リポジトリごとの設定なので）。
# `scripts/setup-git-secrets.sh` が入れる。devcontainer.json の
# postCreateCommand から呼んでいる。
ARG GIT_SECRETS_SHA=7d6b970cbd3c216353cb22b383b70c150140662e
ARG GIT_SECRETS_SHA256=775e3f0d2f2f8e4b5922115f1235f7369805de76f4efc69cb674bcba5590d0fa
RUN curl -fsSL --proto '=https' --tlsv1.2 \
        "https://raw.githubusercontent.com/awslabs/git-secrets/${GIT_SECRETS_SHA}/git-secrets" \
        -o /tmp/git-secrets \
    && echo "${GIT_SECRETS_SHA256}  /tmp/git-secrets" | sha256sum -c - \
    && install -m 0755 /tmp/git-secrets /usr/local/bin/git-secrets \
    && rm -f /tmp/git-secrets

USER $USERNAME
