# ArcNet

A blockchain implementation in Rust featuring a distributed node network, cryptocurrency wallet, and mining capabilities. ArcNet implements a UTXO-based blockchain with transaction signing, network synchronization, and a terminal user interface.

## Project Overview

ArcNet is a complete blockchain ecosystem consisting of four main components:

- **lib**: Core library containing blockchain types, cryptographic operations, and network messaging
- **node**: Blockchain node that maintains and synchronizes the blockchain across the network
- **wallet**: Terminal-based wallet application with a TUI for managing keys, contacts, and transactions
- **miner**: Mining component for creating new blocks

## Features

- **UTXO-based Blockchain**: Unspent Transaction Output model for transaction management
- **Cryptographic Security**: ECDSA-based signing and verification using secp256k1
- **Network Synchronization**: Peer-to-peer node communication and blockchain synchronization
- **Wallet Application**: Terminal UI wallet with:
  - Balance display with ASCII art
  - Contact management
  - Transaction sending
  - Multiple key pair support
  - Configurable transaction fees (fixed or percentage-based)
- **Mining Support**: Block template generation and validation
- **Persistent Storage**: Blockchain state saved to disk in CBOR format

## Prerequisites

- **Rust**: Version 1.70 or later (Rust 2024 edition)
- **Cargo**: Rust's package manager (included with Rust)

### Installing Rust

If you don't have Rust installed, you can install it using [rustup](https://rustup.rs/):

```bash
# On Windows (PowerShell)
Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
.\rustup-init.exe

# On Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installation

1. **Clone the repository** (or navigate to the project directory):
   ```bash
   cd arcnet
   ```

2. **Build all components**:
   ```bash
   cargo build --release
   ```

   This will compile all workspace members (lib, node, wallet, miner) and create executables in `target/release/`.

3. **Build individual components** (optional):
   ```bash
   # Build only the wallet
   cargo build --release -p wallet
   
   # Build only the node
   cargo build --release -p node
   
   # Build only the miner
   cargo build --release -p miner
   ```

## Project Structure

```
arcnet/
├── lib/              # Core library (blockchain types, crypto, network)
│   └── src/
├── node/             # Blockchain node implementation
│   └── src/
├── wallet/           # Wallet application
│   └── src/
│       ├── main.rs   # Entry point
│       ├── core.rs   # Core wallet logic
│       ├── ui.rs     # Terminal UI
│       ├── tasks.rs  # Background tasks
│       └── utils.rs  # Utilities
├── miner/            # Mining component
│   └── src/
├── wallet_config.toml # Example wallet configuration
└── Cargo.toml        # Workspace configuration
```

## Usage

### Setting Up a Node

1. **Start a seed node** (first node in the network):
   ```bash
   cargo run --release -p node -- --port 9000
   ```

2. **Start additional nodes** connected to existing nodes:
   ```bash
   cargo run --release -p node -- --port 9001 127.0.0.1:9000
   cargo run --release -p node -- --port 9002 127.0.0.1:9000 127.0.0.1:9001
   ```

   **Options:**
   - `--port <PORT>`: Port number to listen on (default: 9000)
   - `--blockchain-file <FILE>`: Path to blockchain storage file (default: `./blockchain.cbor`)
   - `<nodes...>`: Space-separated list of node addresses to connect to

### Setting Up the Wallet

1. **Generate a wallet configuration**:
   ```bash
   cargo run --release -p wallet -- generate-config --output wallet_config.toml
   ```

2. **Edit `wallet_config.toml`** to add your keys and contacts:
   ```toml
   keys = [
       # Add your key pairs here
       # { public = "path/to/public.key", private = "path/to/private.key" }
   ]
   default_node = "127.0.0.1:9000"
   
   [[contacts]]
   name = "Alice"
   key = "path/to/alice.pub.poem"
   
   [[contacts]]
   name = "Bob"
   key = "path/to/bob.pub.poem"
   
   [fee_config]
   fee_type = "Percent"  # or "Fixed"
   value = 0.1           # 0.1% for Percent, or fixed amount for Fixed
   ```

3. **Generate key pairs** (using the lib binary tools):
   ```bash
   # Generate a key pair
   cargo run --release --bin key_gen -- --output my_key
   ```

4. **Run the wallet**:
   ```bash
   cargo run --release -p wallet -- --config wallet_config.toml --node 127.0.0.1:9000
   ```

   **Options:**
   - `--config <FILE>`: Path to wallet configuration file (default: `wallet_config.toml`)
   - `--node <ADDRESS>`: Override default node address

### Wallet UI Controls

- **Press `Esc`**: Access the menu bar
- **Press `q`**: Quit the application
- **Menu Options**:
  - **Send**: Send a transaction to a contact
  - **Quit**: Exit the wallet

### Mining

Run the miner to create new blocks:
```bash
cargo run --release -p miner -- --node 127.0.0.1:9000
```

## Configuration

### Wallet Configuration (`wallet_config.toml`)

```toml
# List of key pairs (public and private key file paths)
keys = []

# Default node address to connect to
default_node = "127.0.0.1:9000"

# Contact list for easy sending
[[contacts]]
name = "ContactName"
key = "path/to/public/key.pub.poem"

# Transaction fee configuration
[fee_config]
fee_type = "Percent"  # Options: "Percent" or "Fixed"
value = 0.1          # Percentage (0.1 = 0.1%) or fixed amount in satoshis
```

### Currency Units

- **Arcs**: Base unit (1 Arc = 100,000,000 satoshis)
- **Satoshis**: Smallest unit (like Bitcoin's satoshi)

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific package
cargo test -p lib
cargo test -p wallet
```

### Building for Development

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release
```

### Logging

The wallet application logs to `logs/wallet.log` with daily rotation. Log levels can be controlled via the `RUST_LOG` environment variable:

```bash
# Set log level
export RUST_LOG=debug  # or trace, info, warn, error

# Run wallet with logging
cargo run --release -p wallet
```

## Dependencies

### Core Dependencies
- **tokio**: Async runtime
- **serde**: Serialization framework
- **anyhow**: Error handling
- **tracing**: Structured logging

### Wallet Dependencies
- **cursive**: Terminal UI framework
- **clap**: Command-line argument parsing
- **kanal**: Async channels
- **text-to-ascii-art**: Balance display formatting

### Node Dependencies
- **argh**: Argument parsing
- **dashmap**: Concurrent hash map
- **static_init**: Static initialization

### Crypto Dependencies
- **k256**: secp256k1 elliptic curve cryptography
- **ecdsa**: Digital signature algorithms

## Network Protocol

The network uses a custom message protocol for communication between nodes, wallets, and miners:

- `FetchUTXOs`: Request UTXOs for a public key
- `UTXOs`: Response with UTXO list
- `SubmitTransaction`: Submit a transaction to the network
- `NewTransaction`: Broadcast a new transaction
- `FetchTemplate`: Request a block template for mining
- `Template`: Block template response
- `SubmitTemplate`: Submit a mined block
- `NewBlock`: Broadcast a new block
- `DiscoverNodes`: Discover other nodes in the network
- `NodeList`: List of known nodes
- `AskDifference`: Compare blockchain heights
- `FetchBlock`: Request a specific block

## Troubleshooting

### Node won't start
- Ensure the port is not already in use
- Check firewall settings
- Verify the blockchain file path is writable

### Wallet can't connect to node
- Verify the node is running and accessible
- Check the node address in `wallet_config.toml`
- Ensure network connectivity

### Transaction fails
- Verify sufficient balance (check UTXOs)
- Ensure recipient's public key is correct
- Check that the node is synchronized

### Build errors
- Ensure you're using Rust 2024 edition
- Run `cargo clean` and rebuild
- Check that all dependencies are available

## Contributing

Contributions are welcome! Please ensure:
- Code follows Rust conventions
- Tests are included for new features
- Documentation is updated


## Authors

- **James Muriuki** - geniusinrust@gmail.com

## Acknowledgments

Built with Rust and the amazing Rust ecosystem.
