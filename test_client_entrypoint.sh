#!/bin/sh
set -euo pipefail
alias docker=docker-entrypoint.sh

echo installing rust
apk add curl build-base
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup update
rustup target add x86_64-unknown-linux-musl

echo copy to /app because /app_origin dir is read only
rm -rf /app
cp -r /app_origin /app
cd /app

echo check we can connect to dind
docker version

if ! docker plugin ls | grep logdna:latest; then
    echo creating logdna plugin
    mkdir ./plugin/rootfs
    docker build -t rootfsimage .
    docker export $(docker create rootfsimage true) | tar -x -C ./plugin/rootfs
    docker plugin create logdna ./plugin
    rm -r ./plugin/rootfs
    docker plugin enable logdna
    echo installed logdna plugin

    # heuristic to not compile this every time
    echo building mock client container
    cd /app/mock/client
    docker build -t mock_client -f Dockerfile ..
else
    echo logdna plugin already installed, remove and restart dind container to rebuild
fi


echo building and running dind test driver
cd /app/dind_test_driver
export IN_DIND=true
cargo run --target x86_64-unknown-linux-musl --release

echo all done
echo have a nice day

