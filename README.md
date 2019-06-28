# turing-node

A Parity Substrate node implementing Turnetwork. And the DAO on Substrate inspired by [slockit/**DAO**](<https://github.com/slockit/DAO>).

# Building

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
./scripts/init.sh
```

Build the WebAssembly binary:

```bash
./scripts/build.sh
```

Build all native code: 

```bash
cargo build --release
```

# Run

## Run Development Chain

You can start a development chain with:

```bash
cargo run -- --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units. Give each node a name and expose them so they are listed on the Polkadot [telemetry site](https://telemetry.polkadot.io/#/Local%20Testnet). You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN \
  --chain=local \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.

## Run Development Substrate Node

```bash
./target/release/turing-node --dev
```

If you have run the node before, running `./target/release/turing-node purge-chain --dev` will ensure that you start a new node.

## Run Turing Testnet Node

```bash
./target/release/turing-node \
  --base-path /tmp/alice \
  --chain=turing \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

```bash
./target/release/turing-node \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN \
  --chain=turing \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

```bash
./target/release/turing-node \
  --base-path /tmp/charlie \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN \
  --chain=turing \
  --charlie \
  --port 30335 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```



# UI

[Substrate Portal](https://polkadot.js.org/apps/)

```
Settings > General > remote node/endpoint to connect to > Local Node (127.0.0.1:9944)
Settings > Developer > Manually enter your custom type definitions as valid JSON > 
{
  "TokenBalance": "u128",
  "Bytes": "Vec<u8>",
  "Moment": "u64",
  "Balance_in_Token": "u128"
}
```

