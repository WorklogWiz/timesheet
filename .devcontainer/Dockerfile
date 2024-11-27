FROM rust:1.82-alpine3.20

RUN apk add --no-cache \
        cosign \
        git-lfs \
        less \
        make \
        musl \
        musl-dev \
        perl \
        protobuf-dev \
        sccache \
        zig \
        zsh && \
    apk add --no-cache --repository=https://dl-cdn.alpinelinux.org/alpine/edge/testing/ \
        grcov && \
    cargo install \
        cargo-zigbuild \
        semantic-release-cargo && \
    rm -rf "$HOME"/.cargo/git && \
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

RUN sh -c "$(wget -O- https://raw.githubusercontent.com/deluan/zsh-in-docker/cb315e12a047ba60fc32164bfc071bd858b60e39/zsh-in-docker.sh)" -- \
        -t robbyrussell
