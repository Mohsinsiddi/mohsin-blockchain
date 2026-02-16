# MVM — Mohsin Virtual Machine

A custom Layer 1 blockchain built from scratch in Rust, featuring its own smart contract language (**Mosh**), token standard (MVM-20), and developer toolkit.

## Features

- **Custom Blockchain Core** — Proof-of-Authority consensus, 3-second blocks, RocksDB storage
- **Ed25519 Cryptography** — Keypair generation, transaction signing, bech32 addresses (`mvm1...`)
- **Mosh Smart Contracts** — Deploy and execute contracts with variables, mappings, functions, events, and control flow
- **MVM-20 Token Standard** — Create, transfer, and query custom tokens
- **REST + WebSocket API** — Full blockchain interaction via HTTP endpoints and real-time WS updates
- **Contract Events** — `signal`/`emit` events stored on-chain, queryable per contract
- **Leaderboard** — Top holders, token creators, contract deployers, and most active accounts

## The Mosh Language

Mosh blends Solidity's contract model with Rust's syntax, plus unique keywords:

```mosh
forge Counter {
    let count: u256 = 0;
    let owner: address = msg.sender;
    map balances: address => u256;

    fn increment() mut {
        count += 1;
        signal CountChanged(count);
    }

    fn deposit() vault {
        guard(msg.value > 0, "Must send tokens");
        balances[msg.sender] += msg.value;
    }

    fn getCount() pub -> u256 {
        return count;
    }
}
```

| Mosh | Replaces | Meaning |
|------|----------|---------|
| `forge` | `contract` | Define a contract |
| `fn` | `function` | Define a function |
| `let` | type decl | State variable |
| `map` | `mapping` | Key-value mapping |
| `guard` | `require` | Assertion check |
| `signal` | `emit` | Emit event |
| `vault` | `payable` | Accept tokens |
| `seal` | `onlyOwner` | Owner-only |
| `pub` | `view` | Read-only |
| `mut` | `write` | State-mutating |

## Quick Start

```bash
# Build
cargo build --release

# Run node
cargo run --release

# Run API tests
chmod +x test_api.sh
./test_api.sh
```

The node starts on `http://localhost:8545` with WebSocket at `ws://localhost:8545/ws`.

## API Endpoints

### Chain
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/` | Node info |
| GET | `/status` | Chain status (height, peers, pending txs) |
| GET | `/blocks?limit=N` | Recent blocks |
| GET | `/block/:height` | Block by height |
| GET | `/block/latest` | Latest block |
| GET | `/mempool` | Pending transactions |

### Transactions
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/txs?limit=N` | Recent transactions |
| GET | `/tx/:hash` | Transaction by hash |
| GET | `/txs/:address` | Transactions for address |
| POST | `/tx/sign` | Sign a transaction |
| POST | `/tx` | Submit signed transaction |

### Accounts
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/wallet/new` | Generate new wallet |
| POST | `/faucet/:address` | Get test tokens |
| GET | `/balance/:address` | Account balance |
| GET | `/nonce/:address` | Confirmed nonce |
| GET | `/nonce/pending/:address` | Pending nonce |
| GET | `/account/:address` | Full account info |

### Tokens (MVM-20)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/tokens` | All tokens |
| GET | `/token/:address` | Token details |
| GET | `/token/:addr/balance/:addr` | Token balance |

### Contracts
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/contracts` | All contracts |
| GET | `/contract/:address` | Contract details |
| GET | `/contract/:addr/mbi` | Contract ABI |
| GET | `/contract/:addr/var/:name` | Read variable (free) |
| GET | `/contract/:addr/mapping/:name/:key` | Read mapping (free) |
| GET | `/contract/:addr/call/:method` | Call view function (free) |
| GET | `/contract/:addr/events` | Contract events |

### Other
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/leaderboard` | Top accounts rankings |

## MVM Operations

The virtual machine supports these opcodes:

**Arithmetic:** `add`, `sub`, `mul`, `div`, `mod`
**Mapping Arithmetic:** `map_add`, `map_sub`, `map_mul`, `map_div`, `map_mod`, `map_set`
**Control:** `require` / `guard`, `if` (with else), `return`, `transfer`
**Events:** `emit` / `signal`
**Variables:** `set`

## Tech Stack

- **Language:** Rust
- **Web Framework:** Axum
- **Storage:** RocksDB
- **Crypto:** Ed25519 (ed25519-dalek), bech32 addresses
- **Serialization:** serde + serde_json
