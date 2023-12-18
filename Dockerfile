FROM rust as builder

RUN apt-get update && apt-get install -y musl-tools musl-dev protobuf-compiler
RUN rustup target add x86_64-unknown-linux-musl && rustup component add clippy

WORKDIR /usr/src/docker_logdna/mock/client
COPY ./mock/client/Cargo.toml ./Cargo.toml
COPY ./mock/client/src ./src

WORKDIR /usr/src/docker_logdna/mock
COPY ./mock/Cargo.toml ./Cargo.toml
COPY ./mock/src ./src

WORKDIR /usr/src/docker_logdna
COPY ./Cargo.toml ./Cargo.toml
COPY ./build.rs ./build.rs
COPY ./src ./src

RUN cargo clippy -- -W clippy::unwrap_used -A clippy::useless_format -D warnings && \
    cargo test && \
    cargo test --release && \
    cargo build --release --target x86_64-unknown-linux-musl
# RUN cargo build --target x86_64-unknown-linux-musl

FROM alpine
COPY --from=builder /usr/src/docker_logdna/target/x86_64-unknown-linux-musl/release/docker_logdna /docker_logdna
RUN mkdir -p /run/docker/plugins
# overwritten by config.json
ENTRYPOINT ["/docker_logdna"]

LABEL org.opencontainers.image.source=https://github.com/ibm/docker_logdna

