# build wity `docker build -t mock_server -f Dockerfile ..`
FROM rust as builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev protobuf-compiler

WORKDIR /usr/src/mock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

WORKDIR /usr/src/mock/server
COPY ./server/Cargo.toml ./Cargo.toml
COPY ./server/src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl
# RUN cargo build --target x86_64-unknown-linux-musl

FROM alpine
COPY --from=builder /usr/src/mock/server/target/x86_64-unknown-linux-musl/release/mock_server /mock_server
# COPY --from=builder /usr/src/mock/server/target/x86_64-unknown-linux-musl/debug/mock_server /mock_server
RUN mkdir -p /run/docker/plugins
ENTRYPOINT ["/mock_server"]

