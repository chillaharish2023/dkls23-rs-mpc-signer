# Network Architecture

## Overview

The system consists of multiple components that work together to enable secure multi-party computation:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        MPC NETWORK ARCHITECTURE                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                 │
│     │   Party 0   │     │   Party 1   │     │   Party 2   │                 │
│     │ (dkls-party)│     │ (dkls-party)│     │ (dkls-party)│                 │
│     └──────┬──────┘     └──────┬──────┘     └──────┬──────┘                 │
│            │                   │                   │                        │
│            └───────────────────┼───────────────────┘                        │
│                                │                                            │
│                       ┌────────▼────────┐                                   │
│                       │  Message Relay  │                                   │
│                       │  (msg-relay-svc)│                                   │
│                       │                 │                                   │
│                       │ • HTTP/WS API   │                                   │
│                       │ • Msg Caching   │                                   │
│                       │ • Peer Routing  │                                   │
│                       └────────┬────────┘                                   │
│                                │                                            │
│                    ┌───────────┴───────────┐                                │
│                    │                       │                                │
│             ┌──────▼──────┐         ┌──────▼──────┐                         │
│             │ Peer Relay  │         │ Peer Relay  │                         │
│             │  (backup)   │         │  (backup)   │                         │
│             └─────────────┘         └─────────────┘                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Components

### dkls23-core

The cryptographic core library containing:
- DKG implementation
- DSG (signing) implementation
- OT primitives (Endemic OT, SoftSpokenOT)
- Key derivation (BIP32)

### msg-relay-svc

HTTP/WebSocket service for MPC message routing:
- Flexible pub/sub-like messaging
- Message caching for offline parties
- Peer relay support for redundancy

### msg-relay-client

Client library for communicating with relay service:
- Implements the `Relay` trait
- HTTP-based message passing
- Timeout and retry handling

### dkls-party

CLI application for running MPC operations:
- DKG for key generation
- DSG for signing
- Key refresh
- BIP32 derivation

## Message Flow

### DKG Message Flow

```
Party 0              Relay              Party 1              Party 2
   │                   │                   │                   │
   │─── Round 1 ──────►│                   │                   │
   │                   │◄── Round 1 ───────│                   │
   │                   │◄── Round 1 ────────────────────────── │
   │                   │                   │                   │
   │◄── Collect ───────│                   │                   │
   │                   │────► Collect ─────│                   │
   │                   │────► Collect ─────────────────────────│
   │                   │                   │                   │
   │─── Round 2 (P2P) ─────────────────────►                   │
   │─── Round 2 (P2P) ─────────────────────────────────────────►
   │                   │                   │                   │
   ...
```

### DSG Message Flow

```
Party 0              Relay              Party 1
   │                   │                   │
   │─── Commitment ───►│                   │
   │                   │◄── Commitment ────│
   │                   │                   │
   │◄── Collect ───────│                   │
   │                   │────► Collect ─────│
   │                   │                   │
   │─── MtA Data ─────────────────────────►│
   │◄── MtA Data ──────────────────────────│
   │                   │                   │
   │─── Partial Sig ──►│                   │
   │                   │◄── Partial Sig ───│
   │                   │                   │
   │◄── Final Sig ─────│                   │
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `DEST` | Data directory | `./data` |
| `RELAY_URL` | Relay service URL | `http://127.0.0.1:8080` |
| `PARTY_ID` | This party's ID | Required |
| `RAYON_NUM_THREADS` | Parallel threads | CPU count |
| `TOKIO_WORKER_THREADS` | Async threads | CPU count |

### Performance Tuning

For optimal performance:
- Set `RAYON_NUM_THREADS + TOKIO_WORKER_THREADS = CPU cores`
- Use `--release` builds for production
- Deploy relay service close to parties (low latency)
