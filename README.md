# MVM — Mohsin Virtual Machine

A custom Layer 1 blockchain built from scratch in Rust, featuring its own smart contract language (**Mosh**), token standard (MVM-20), and developer toolkit.

**Live API:** [mvm-chain.duckdns.org](https://mvm-chain.duckdns.org)
**Frontend Explorer:** [github.com/Mohsinsiddi/mvm-ui](https://github.com/Mohsinsiddi/mvm-ui)

## Features

- **Custom Blockchain Core** — Proof-of-Authority consensus, 3-second blocks, RocksDB storage
- **Ed25519 Cryptography** — Keypair generation, transaction signing, bech32 addresses (`mvm1...`)
- **Mosh Smart Contracts** — Deploy and execute contracts with variables, mappings, functions, events, and control flow
- **MVM-20 Token Standard** — Create, transfer, and query custom tokens
- **REST + WebSocket API** — Full blockchain interaction via HTTP endpoints and real-time WS updates
- **Contract Events** — `signal`/`emit` events stored on-chain, queryable per contract
- **Leaderboard** — Top holders, token creators, contract deployers, and most active accounts
- **Free Read Operations** — Read contract state, variables, mappings, and call view functions without gas

## Architecture

```
┌─────────────────────────────────────────┐
│              Axum HTTP Server            │
│          (REST API + WebSocket)          │
├────────────┬──────────┬─────────────────┤
│   Chain    │   MVM    │    State        │
│  (blocks,  │ (smart   │  (RocksDB,     │
│  consensus,│ contract │  accounts,     │
│  mempool)  │ executor)│  storage)      │
├────────────┴──────────┴─────────────────┤
│          Network (P2P Star Topology)     │
└─────────────────────────────────────────┘
```

### Source Files

| File | Lines | Description |
|------|-------|-------------|
| `src/main.rs` | 156 | Entry point, server init, block production loop |
| `src/api.rs` | 2,099 | All HTTP/WS endpoints and handlers |
| `src/chain.rs` | 799 | Blockchain core, block production, tx validation |
| `src/mvm.rs` | 852 | MVM smart contract executor |
| `src/state.rs` | 713 | RocksDB state management, storage layer |
| `src/address.rs` | 200 | Address generation, Ed25519 signing, bech32 encoding |
| `src/config.rs` | 116 | Configuration loading from TOML |
| `src/standards.rs` | 91 | MVM-20 token standard definitions |
| `src/network/` | — | Star topology P2P network |

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

### Language Limits

| Limit | Value |
|-------|-------|
| State variables | Max 10 |
| Mappings | Max 5 |
| Functions | Max 10 |
| Operations per function | Max 20 |
| String length | Max 256 chars |
| Identifier length | Max 32 chars |
| Nesting depth | Max 5 |

## Quick Start

### Local Development

```bash
# Clone
git clone https://github.com/Mohsinsiddi/mohsin-blockchain.git
cd mohsin-blockchain

# Build
cargo build --release

# Run node
cargo run --release

# Run API tests
chmod +x test_api.sh
./test_api.sh
```

The node starts on:
- **API:** `http://localhost:8545`
- **WebSocket:** `ws://localhost:8545/ws`
- **P2P:** `localhost:9000`

### Multi-Node Setup

```bash
# Terminal 1 — Master node
cargo run --release

# Terminal 2 — Slave node
cargo run --release -- --config node2.toml

# Terminal 3 — Another slave
cargo run --release -- --config node3.toml
```

## Deployment (DigitalOcean Droplet)

### First-Time Setup

```bash
# SSH into your droplet
ssh root@your-droplet-ip

# Install Docker & Docker Compose
curl -fsSL https://get.docker.com | sh
apt install docker-compose-plugin -y

# Clone the repo
git clone https://github.com/Mohsinsiddi/mohsin-blockchain.git
cd mohsin-blockchain

# Set up SSL (first time only)
mkdir -p certbot/conf certbot/www
docker compose run --rm certbot certonly \
  --webroot --webroot-path=/var/www/certbot \
  -d mvm-chain.duckdns.org

# Build and start
docker compose up -d --build
```

### Update After Code Changes (Pull & Rebuild)

```bash
# SSH into droplet
ssh root@your-droplet-ip
cd mohsin-blockchain

# Pull latest code
git pull origin main

# Rebuild and restart (chain data is preserved in Docker volume)
docker compose up -d --build

# Verify it's running
docker compose logs -f backend --tail 20

# Check health
curl https://mvm-chain.duckdns.org/status
```

### Useful Commands

```bash
# View logs
docker compose logs -f backend

# Restart without rebuilding
docker compose restart backend

# Stop everything
docker compose down

# Full rebuild (including Rust compilation)
docker compose up -d --build --force-recreate

# Reset chain data (WARNING: deletes all blockchain data)
docker compose down -v
docker compose up -d --build
```

### Docker Services

| Service | Description |
|---------|-------------|
| `backend` | Rust blockchain node (ports 8545, 8546, 9000) |
| `nginx` | Reverse proxy with SSL (ports 80, 443) |
| `certbot` | Let's Encrypt SSL certificate management |

## Configuration

The node is configured via `config.toml`:

```toml
[chain]
chain_id = "mvm-mainnet-1"
chain_name = "Mohsin Virtual Machine"

[block]
block_time = 3          # seconds
gas_limit = 1000000
max_txs_per_block = 100

[faucet]
enabled = true
amount = 1000           # MVM tokens per request
cooldown = 3600         # 1 hour between requests

[token]
name = "MVM"
symbol = "MVM"
decimals = 8

[network]
topology = "star"
api_port = 8545
ws_port = 8546
p2p_port = 9000
```

## API Endpoints

> For interactive API docs with "Try it" buttons, see the [API Reference](https://github.com/Mohsinsiddi/mvm-ui) in the frontend explorer.

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
| POST | `/faucet/:address` | Get test tokens (1,000 MVM) |
| GET | `/balance/:address` | Account balance |
| GET | `/nonce/:address` | Confirmed nonce |
| GET | `/nonce/pending/:address` | Pending nonce (for next tx) |
| GET | `/account/:address` | Full account info |

### Tokens (MVM-20)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/tokens` | All tokens |
| GET | `/tokens/creator/:address` | Tokens by creator |
| GET | `/tokens/holder/:address` | Token holdings for address |
| GET | `/token/:address` | Token details |
| GET | `/token/:addr/balance/:addr` | Token balance |
| GET | `/token/:addr/holders` | Token holders |

### Smart Contracts (Free Reads)
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/contracts` | All contracts |
| GET | `/contracts/creator/:address` | Contracts by creator |
| GET | `/contract/:address` | Contract details |
| GET | `/contract/:addr/mbi` | Contract MBI (ABI equivalent) |
| GET | `/contract/:addr/var/:name` | Read variable (free) |
| GET | `/contract/:addr/mapping/:name` | Read all mapping entries (free) |
| GET | `/contract/:addr/mapping/:name/:key` | Read mapping value (free) |
| GET | `/contract/:addr/call/:method` | Call view function (free) |
| GET | `/contract/:addr/events` | Contract events |

### Other
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/leaderboard` | Top accounts rankings |
| GET | `/ws` | WebSocket (real-time blocks & txs) |

### Transaction Signing Flow

All write operations use a 2-step sign-then-submit pattern:

```
1. GET  /nonce/pending/:address     → { pending_nonce }
2. POST /tx/sign                    → { tx_hash, signature, public_key }
3. POST /tx                         → { success, tx_hash }
4. GET  /tx/:hash                   → { status: "confirmed" }  (~3s)
```

### Transaction Types

| Type | Description | Gas |
|------|-------------|-----|
| `transfer` | Native MVM transfer | 21,000 |
| `create_token` | Deploy MVM-20 token | 100,000 |
| `transfer_token` | Transfer custom token | 65,000 |
| `deploy_contract` | Deploy Mosh contract | 200,000 |
| `call_contract` | Execute contract function | 100,000 |

## MVM Operations

The virtual machine supports these opcodes:

| Category | Operations |
|----------|-----------|
| Arithmetic | `add`, `sub`, `mul`, `div`, `mod` |
| Mapping Arithmetic | `map_add`, `map_sub`, `map_mul`, `map_div`, `map_mod`, `map_set` |
| Control | `require`/`guard`, `if` (with else), `return`, `transfer` |
| Events | `emit`/`signal` |
| Variables | `set` |

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust |
| Web Framework | Axum 0.7 |
| Storage | RocksDB |
| Cryptography | Ed25519 (ed25519-dalek), bech32 |
| Async Runtime | Tokio |
| WebSocket | tokio-tungstenite |
| Serialization | serde + serde_json |

## Related

- **Frontend Explorer:** [github.com/Mohsinsiddi/mvm-ui](https://github.com/Mohsinsiddi/mvm-ui) — React app with block explorer, Mosh IDE, wallet, token creator, and more
- **Live API:** [mvm-chain.duckdns.org](https://mvm-chain.duckdns.org) — Production API endpoint
