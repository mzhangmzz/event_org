#!/bin/sh

./build.sh

echo ">> Deploying contract"

near deploy --wasmFile ./target/wasm32-unknown-unknown/release/contract.wasm --accountId test1.event_org.testnet
