FROM rust:1.81-alpine3.20

RUN apk add --no-cache \
        zsh \
        musl-dev \
        musl \
        protobuf-dev \
        sccache \
        make \
        perl \
        git-lfs \
        cosign \
        less \
        zig && \
    apk add --no-cache --repository=http://dl-cdn.alpinelinux.org/alpine/edge/testing/ \
        grcov && \
    cargo install \
        cargo-zigbuild \
        semantic-release-cargo && \
    rm -rf $HOME/.cargo/git && \
    rm -rf /usr/local/cargo/registry

RUN rustup component add \
        clippy \
        rustfmt-preview \
        llvm-tools-preview

RUN rustup target add \
        x86_64-unknown-linux-musl \
        aarch64-unknown-linux-musl \
        x86_64-unknown-linux-gnu \
        aarch64-unknown-linux-gnu \
        x86_64-pc-windows-gnu

RUN sh -c "$(wget -O- https://github.com/deluan/zsh-in-docker/releases/download/v1.1.5/zsh-in-docker.sh)" -- \
    -t robbyrussell
