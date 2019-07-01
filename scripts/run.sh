#!/usr/bin/env bash

set -e

if [ $# -lt 3 ]; then
    echo "Please input seed, port, name!"
    echo "Example : .run.sh 0x00000000000 30333 alice"
    exit 1
fi


KEY=$1
PORT=$2
NODE_NAME=$3
LOG_FILE="$NODE_NAME.log"
PROJECT_ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null && pwd )"
EXE_PATH="../target/release/turing-node"
BASE_PATH="$PWD/$NODE_NAME"

RUST_LOG='info' $EXE_PATH --chain=turing --base-path=$BASE_PATH --key=$KEY --name=$NODE_NAME --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN --port $PORT --validator --telemetry-url ws://telemetry.polkadot.io:1024 > $LOG_FILE 2>&1 &

echo "Node run with $NODE_NAME, Log in $LOG_FILE file"