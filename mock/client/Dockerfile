# build with `docker build -t mock_client -f Dockerfile ..`
FROM rust as builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y musl-tools musl-dev protobuf-compiler

WORKDIR /usr/src/mock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

WORKDIR /usr/src/mock/client
COPY ./client/Cargo.toml ./Cargo.toml
COPY ./client/src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl
# RUN cargo build --target x86_64-unknown-linux-musl

FROM alpine
COPY --from=builder /usr/src/mock/client/target/x86_64-unknown-linux-musl/release/mock_client /mock_client
# COPY --from=builder /usr/src/mock/client/target/x86_64-unknown-linux-musl/debug/mock_client /mock_client
RUN mkdir -p /run/docker/plugins
ENTRYPOINT ["/mock_client"]

