# DKLs23 MPC Signer

A Rust-based multiparty threshold ECDSA signing service using the DKLs23 protocol with SoftSpokenOT-style oblivious transfer.

## Overview

This project implements a production-grade threshold ECDSA signing system that enables distributed key generation and transaction signing without ever reconstructing the private key. It's designed for secure wallet backends, custody infrastructure, and any application requiring distributed key management.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        DKLs23 MPC SIGNER                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────┐     │
│  │                         Workspace Crates                           │     │
│  ├────────────────────────────────────────────────────────────────────┤     │
│  │                                                                    │     │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │     │
│  │  │ dkls23-core │  │  msg-relay  │  │ msg-relay-  │                 │     │
│  │  │             │  │             │  │    svc      │                 │     │
│  │  │ • DKG       │  │ • Store     │  │             │                 │     │
│  │  │ • DSG       │  │ • Messages  │  │ • HTTP API  │                 │     │
│  │  │ • OT        │  │ • Peers     │  │ • WebSocket │                 │     │
│  │  │ • BIP32     │  │             │  │ • Caching   │                 │     │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │     │
│  │                                                                    │     │
│  │  ┌─────────────┐  ┌─────────────┐                                  │     │
│  │  │ msg-relay-  │  │ dkls-party  │                                  │     │
│  │  │   client    │  │             │                                  │     │
│  │  │             │  │ • CLI       │                                  │     │
│  │  │ • Relay     │  │ • Keygen    │                                  │     │
│  │  │   trait     │  │ • Sign      │                                  │     │
│  │  │ • HTTP      │  │ • Derive    │                                  │     │
│  │  └─────────────┘  └─────────────┘                                  │     │
│  │                                                                    │     │
│  └────────────────────────────────────────────────────────────────────┘     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Features

### Cryptographic Operations

- **Distributed Key Generation (DKG)**: Generate threshold ECDSA keys among N parties
- **Distributed Signature Generation (DSG)**: Sign messages with T-of-N parties
- **Key Refresh**: Proactively refresh shares without changing the public key
- **BIP32 Derivation**: Non-hardened child key derivation for wallet compatibility

### Network Layer

- **Message Relay Service**: Flexible message routing for MPC communication
- **Offline Support**: Message caching for temporarily disconnected parties
- **Peer Routing**: Redundant relay infrastructure support

### Security

- **No Key Reconstruction**: Private key is never assembled in any location
- **UC Security**: Universally Composable security guarantees
- **Three-Round Signing**: Optimal round complexity matching Schnorr

## Quick Start

### Prerequisites

- Rust 1.75+ (stable)
- Cargo

### Build

```bash
# Clone the repository
git clone https://github.com/Kazopl/dkls23-mpc-signer.git
cd dkls23-mpc-signer

# Build all crates
cargo build --release
```

### Run Message Relay

```bash
# Terminal 1: Start the relay service
./scripts/run-relay.sh 8080
```

### Run DKG (3 parties, threshold 2)

```bash
# Terminal 2: Run DKG
DEST=./data ./scripts/dkg.sh 3 2
```

### Sign a Message

```bash
# Sign with parties 0, 1, 2
MESSAGE="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
DEST=./data ./scripts/dsg.sh $MESSAGE 0 1 2
```

## Usage

### CLI Reference

```bash
# Distributed Key Generation
dkls-party --party-id 0 keygen --n 3 --t 2

# Sign a message
dkls-party --party-id 0 sign \
    --message "abcd..." \
    --parties "0,1,2"

# Derive child key
dkls-party --party-id 0 derive --path "m/0/1/42"

# Show key share info
dkls-party --party-id 0 info
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (debug, info, warn, error) | `info` |
| `DEST` | Directory for key share storage | `./data` |
| `RELAY_URL` | Message relay service URL | `http://127.0.0.1:8080` |
| `PARTY_ID` | This party's identifier | Required |
| `RAYON_NUM_THREADS` | Rayon thread pool size | CPU cores |
| `TOKIO_WORKER_THREADS` | Tokio worker threads | CPU cores |

## Project Structure

```
dkls23-mpc-signer/
├── Cargo.toml              # Workspace definition
├── crates/
│   ├── dkls23-core/        # Cryptographic core
│   │   └── src/
│   │       ├── keygen/     # DKG implementation
│   │       ├── sign/       # DSG implementation
│   │       ├── mpc/        # Relay trait & utils
│   │       └── oblivious/  # OT primitives
│   ├── msg-relay/          # Message store library
│   ├── msg-relay-svc/      # HTTP relay service
│   ├── msg-relay-client/   # Relay client library
│   └── dkls-party/         # CLI party node
├── scripts/
│   ├── dkg.sh              # DKG orchestration
│   ├── dsg.sh              # DSG orchestration
│   └── run-relay.sh        # Relay service launcher
├── docs/
│   ├── protocol.md         # Protocol documentation
│   └── architecture.md     # Architecture guide
└── data/                   # Key share storage
```

## Protocol

The implementation is based on:

- **DKLs23**: [Threshold ECDSA in Three Rounds](https://eprint.iacr.org/2023/765.pdf)
- **SoftSpokenOT**: [Oblivious Transfer Extension](https://eprint.iacr.org/2022/192.pdf)
- **Endemic OT**: [Base Oblivious Transfer](https://eprint.iacr.org/2019/706.pdf)

### DKG Flow

1. **Round 1**: Each party commits to a random polynomial (Feldman VSS)
2. **Round 2**: Parties exchange secret shares
3. **Round 3**: Verify shares and compute public key

### DSG Flow

1. **Round 1**: Generate and commit to nonce shares
2. **Round 2**: Run MtA protocol for multiplicative shares
3. **Round 3**: Compute and combine partial signatures

## Security Considerations

- **Threshold Security**: T-of-N parties required to sign
- **No Single Point of Failure**: Key is never in one place
- **Proactive Security**: Regular key refresh recommended
- **Message Authentication**: All MPC messages are authenticated

## Performance

Typical performance on modern hardware (Apple M1):

| Operation | Parties | Time |
|-----------|---------|------|
| DKG | 3 | ~500ms |
| DSG | 3 | ~200ms |
| Key Derivation | 1 | <1ms |

## References

- [DKLs23 Paper](https://eprint.iacr.org/2023/765.pdf)
- [SoftSpokenOT Paper](https://eprint.iacr.org/2022/192.pdf)
- [Endemic OT Paper](https://eprint.iacr.org/2019/706.pdf)
- [Feldman VSS](https://www.cs.umd.edu/~gasarch/TOPICS/secretsharing/feldmanVSS.pdf)

## License

MIT
