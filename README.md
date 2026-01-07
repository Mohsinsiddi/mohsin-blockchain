# MOHSIN VIRTUAL MACHINE (MVM)

A custom Layer 1 blockchain with the **Mosh** smart contract language, MVM-20 tokens, and Proof of Authority consensus.

```
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   ███╗   ███╗ ██╗   ██╗ ███╗   ███╗                          ║
║   ████╗ ████║ ██║   ██║ ████╗ ████║                          ║
║   ██╔████╔██║ ██║   ██║ ██╔████╔██║                          ║
║   ██║╚██╔╝██║ ╚██╗ ██╔╝ ██║╚██╔╝██║                          ║
║   ██║ ╚═╝ ██║  ╚████╔╝  ██║ ╚═╝ ██║                          ║
║   ╚═╝     ╚═╝   ╚═══╝   ╚═╝     ╚═╝                          ║
║                                                               ║
║   MOHSIN VIRTUAL MACHINE                                      ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
```

## Features

- ✅ Proof of Authority (PoA) consensus
- ✅ 3-second block time
- ✅ Custom `mvm1...` addresses (Bech32)
- ✅ Native MVM token (18 decimals)
- ✅ MVM-20 token standard (create your own tokens!)
- ✅ **Mosh** smart contract language
- ✅ Auto-generated getters/setters
- ✅ View functions (FREE reads)
- ✅ Payable functions
- ✅ Access control (onlyOwner)
- ✅ MBI (Mosh Binary Interface)
- ✅ Transaction signing & verification
- ✅ WebSocket for real-time updates
- ✅ P2P for full nodes
- ✅ REST API (32 endpoints)
- ✅ Faucet (testnet)
- ✅ RocksDB storage
- ✅ State pruning for light sync

---

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [API Reference](#api-reference)
- [Transaction Signing](#transaction-signing)
- [Gas Fees](#gas-fees)
- [MVM-20 Tokens](#mvm-20-tokens)
- [Mosh Contract Language](#mosh-contract-language)
- [MBI (Mosh Binary Interface)](#mbi-mosh-binary-interface)
- [WebSocket](#websocket)
- [Examples](#examples)
- [Configuration](#configuration)
- [Rewards](#rewards)

---

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Build & Run

```bash
# Build
cargo build --release

# Run node
./target/release/mvm

# API available at http://localhost:8545
```

### Test All APIs

```bash
chmod +x test-all-apis.sh
./test-all-apis.sh
```

### Run Multiple Nodes

```bash
# Terminal 1: Master
./target/release/mvm --config config.toml

# Terminal 2: Node 2
./target/release/mvm --config node2.toml

# Terminal 3: Node 3
./target/release/mvm --config node3.toml
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         MVM Node                             │
├─────────────────────────────────────────────────────────────┤
│  REST API (:8545)  │  WebSocket (:8545/ws)  │  P2P (:30303) │
├─────────────────────────────────────────────────────────────┤
│                      Chain Module                            │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐ │
│  │ Blocks  │  │   Txs   │  │ Mempool │  │ PoA Consensus   │ │
│  └─────────┘  └─────────┘  └─────────┘  └─────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                        MVM Engine                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ MVM-20      │  │ Mosh        │  │ Operations VM       │  │
│  │ Tokens      │  │ Contracts   │  │ (set,add,require..) │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    State (RocksDB)                           │
│  Accounts │ Balances │ Tokens │ Contracts │ Variables       │
└─────────────────────────────────────────────────────────────┘
```

### Network Topology

```
                    MASTER NODE
                   (Block Producer)
                         │
          ┌──────────────┼──────────────┐
          │              │              │
          ▼              ▼              ▼
     Full Node 1    Full Node 2    Full Node 3
          │              │              │
          ▼              ▼              ▼
     Browsers       Browsers       Browsers
```

### Key Components

| Component | Description |
|-----------|-------------|
| **Consensus** | Proof of Authority (PoA) - 3 second blocks |
| **Address Format** | `mvm1...` (Bech32 encoded) |
| **Native Token** | MVM (18 decimals) |
| **Smart Contracts** | Mosh language (JSON-based) |
| **Token Standard** | MVM-20 (similar to ERC-20) |

---

## API Reference

Base URL: `http://localhost:8545`

### Chain Info

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | API info & endpoints |
| `/status` | GET | Chain status |
| `/blocks?limit=10` | GET | Recent blocks |
| `/block/:height` | GET | Block by height |
| `/block/latest` | GET | Latest block |
| `/txs?limit=20` | GET | Recent transactions |
| `/tx/:hash` | GET | Transaction by hash |

```bash
# Get status
curl http://localhost:8545/status

# Get latest block
curl http://localhost:8545/block/latest

# Get recent transactions
curl "http://localhost:8545/txs?limit=10"
```

### Accounts & Wallets

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/wallet/new` | GET | Create new wallet |
| `/balance/:address` | GET | Native MVM balance |
| `/nonce/:address` | GET | Current nonce |
| `/account/:address` | GET | Full account info |
| `/txs/:address` | GET | Address transactions |
| `/faucet/:address` | POST | Get test tokens |

```bash
# Create wallet
curl http://localhost:8545/wallet/new

# Response:
# {
#   "address": "mvm1abc123...",
#   "public_key": "02abc123...",
#   "private_key": "deadbeef...",   ⚠️ SAVE THIS!
#   "warning": "Save your private key!"
# }

# Get balance
curl http://localhost:8545/balance/mvm1abc123...

# Get test tokens
curl -X POST http://localhost:8545/faucet/mvm1abc123...
```

### Transactions

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/tx/sign` | POST | Sign transaction |
| `/tx` | POST | Submit signed transaction |

### MVM-20 Tokens

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/tokens` | GET | List all tokens |
| `/token/:address` | GET | Token info |
| `/token/:contract/balance/:address` | GET | Token balance |
| `/tokens/creator/:address` | GET | Tokens by creator |
| `/tokens/holder/:address` | GET | Token holdings |

### Mosh Contracts

| Endpoint | Method | Description | Gas |
|----------|--------|-------------|-----|
| `/contracts` | GET | List all contracts | FREE |
| `/contract/:address` | GET | Contract info | FREE |
| `/contract/:address/mbi` | GET | Contract interface (MBI) | FREE |
| `/contract/:address/var/:name` | GET | Read variable | FREE |
| `/contract/:address/mapping/:name` | GET | List mapping entries | FREE |
| `/contract/:address/mapping/:name/:key` | GET | Read mapping value | FREE |
| `/contract/:address/call/:method?args=a,b,c` | GET | Call view function | FREE |

### WebSocket

| Endpoint | Description |
|----------|-------------|
| `/ws` | Browser connection (real-time updates) |
| `/p2p` | Full node connection |

---

## Transaction Signing

### Signature Verification Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Client     │     │   /tx/sign   │     │   /tx        │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       │ 1. Sign Request    │                    │
       │ (private_key, tx)  │                    │
       │───────────────────>│                    │
       │                    │                    │
       │ 2. Returns         │                    │
       │ (signature, pubkey)│                    │
       │<───────────────────│                    │
       │                    │                    │
       │ 3. Submit TX                            │
       │ (tx + signature + pubkey)               │
       │────────────────────────────────────────>│
       │                    │                    │
       │                    │    4. Verify:      │
       │                    │    - pubkey → addr │
       │                    │    - sig valid?    │
       │                    │    - addr == from? │
       │                    │                    │
       │ 5. Result (hash)                        │
       │<────────────────────────────────────────│
```

### Verification Rules

| Rule | Description |
|------|-------------|
| **Public Key → Address** | The public key must derive to the `from` address |
| **Signature Valid** | Signature must be valid for the transaction hash |
| **Signer == Payer** | The signer must match the `from` address |

### Sign Transaction

```bash
curl -X POST http://localhost:8545/tx/sign \
  -H "Content-Type: application/json" \
  -d '{
    "private_key": "your_private_key_hex",
    "tx_type": "transfer",
    "from": "mvm1sender...",
    "to": "mvm1receiver...",
    "value": 1000000000000000000,
    "nonce": 0
  }'
```

Response:
```json
{
  "success": true,
  "signature": "304402...",
  "public_key": "02abc123...",
  "tx_hash": "abc123..."
}
```

### Submit Transaction

```bash
curl -X POST http://localhost:8545/tx \
  -H "Content-Type: application/json" \
  -d '{
    "tx_type": "transfer",
    "from": "mvm1sender...",
    "to": "mvm1receiver...",
    "value": 1000000000000000000,
    "nonce": 0,
    "signature": "304402...",
    "public_key": "02abc123..."
  }'
```

Response:
```json
{
  "success": true,
  "hash": "def456...",
  "message": "Transaction submitted successfully"
}
```

### Transaction Types

| Type | Description | Data Fields |
|------|-------------|-------------|
| `transfer` | Send native MVM | `to`, `value` |
| `create_token` | Create MVM-20 token | `name`, `symbol`, `total_supply` |
| `transfer_token` | Send MVM-20 token | `contract`, `to`, `amount` |
| `deploy_contract` | Deploy Mosh contract | Contract JSON |
| `call_contract` | Call contract method | `contract`, `method`, `args`, `amount` |

---

## Gas Fees

### Transaction Gas Costs

| Operation | Gas Cost |
|-----------|----------|
| Base transaction | 21,000 |
| Transfer (native) | 21,000 |
| Create Token | 100,000 |
| Transfer Token | 50,000 |
| Deploy Contract | 200,000 + ops |
| Call Contract | 50,000 + ops |

### Contract Operation Gas

| Operation | Gas Cost | Description |
|-----------|----------|-------------|
| `set` | 5,000 | Set variable |
| `add` / `sub` | 5,000 | Arithmetic |
| `map_set` | 10,000 | Set mapping |
| `map_add` / `map_sub` | 10,000 | Mapping arithmetic |
| `require` | 1,000 | Condition check |
| `transfer` | 20,000 | Token transfer |
| `return` | 100 | Return value |
| `let` | 500 | Local variable |

### FREE Operations (No Gas!)

| Operation | Endpoint |
|-----------|----------|
| Read variable | `GET /contract/:addr/var/:name` |
| Read mapping | `GET /contract/:addr/mapping/:name/:key` |
| Call view function | `GET /contract/:addr/call/:method` |
| Get MBI | `GET /contract/:addr/mbi` |
| List contracts | `GET /contracts` |
| Contract info | `GET /contract/:addr` |

### Gas Calculation

```
Total Cost = Gas Used × Gas Price

Examples:
├── Transfer:        21,000 gas × 1 gwei = 0.000021 MVM
├── Create Token:   100,000 gas × 1 gwei = 0.0001 MVM
├── Deploy Contract: 250,000 gas × 1 gwei = 0.00025 MVM
└── Call Contract:   75,000 gas × 1 gwei = 0.000075 MVM
```

---

## MVM-20 Tokens

### Create Token

```bash
# 1. Sign
SIGN=$(curl -s -X POST http://localhost:8545/tx/sign \
  -H "Content-Type: application/json" \
  -d '{
    "private_key": "YOUR_PRIVATE_KEY",
    "tx_type": "create_token",
    "from": "mvm1creator...",
    "nonce": 0,
    "data": {
      "name": "Pepe Token",
      "symbol": "PEPE",
      "total_supply": 1000000000000000000000000
    }
  }')

SIG=$(echo $SIGN | jq -r '.signature')
PUB=$(echo $SIGN | jq -r '.public_key')

# 2. Submit
curl -X POST http://localhost:8545/tx \
  -H "Content-Type: application/json" \
  -d "{
    \"tx_type\": \"create_token\",
    \"from\": \"mvm1creator...\",
    \"nonce\": 0,
    \"data\": {
      \"name\": \"Pepe Token\",
      \"symbol\": \"PEPE\",
      \"total_supply\": 1000000000000000000000000
    },
    \"signature\": \"$SIG\",
    \"public_key\": \"$PUB\"
  }"
```

### Transfer Token

```bash
curl -X POST http://localhost:8545/tx \
  -H "Content-Type: application/json" \
  -d '{
    "tx_type": "transfer_token",
    "from": "mvm1sender...",
    "nonce": 1,
    "data": {
      "contract": "mvm1token...",
      "to": "mvm1receiver...",
      "amount": 1000000000000000000
    },
    "signature": "...",
    "public_key": "..."
  }'
```

### Query Tokens

```bash
# List all tokens
curl http://localhost:8545/tokens

# Token info
curl http://localhost:8545/token/mvm1token...

# Token balance
curl http://localhost:8545/token/mvm1token.../balance/mvm1holder...

# Tokens created by address
curl http://localhost:8545/tokens/creator/mvm1creator...

# Token holdings
curl http://localhost:8545/tokens/holder/mvm1holder...
```

---

## Mosh Contract Language

### Contract Structure

```json
{
  "name": "ContractName",
  "token": "mvm1token...",
  "variables": [
    {"name": "count", "type": "uint64", "default": "0"}
  ],
  "mappings": [
    {"name": "balances", "key_type": "address", "value_type": "uint64"}
  ],
  "functions": [
    {
      "name": "stake",
      "modifiers": ["payable"],
      "args": [],
      "body": [
        {"op": "require", "left": "msg.amount", "cmp": ">", "right": "0", "msg": "Amount=0"},
        {"op": "map_add", "map": "balances", "key": "msg.sender", "value": "msg.amount"}
      ]
    }
  ]
}
```

### Variable Types

| Type | Description | Example |
|------|-------------|---------|
| `uint64` | Unsigned 64-bit integer | `"0"`, `"1000000"` |
| `string` | UTF-8 string | `"Hello"` |
| `bool` | Boolean | `"true"`, `"false"` |
| `address` | MVM address | `"mvm1abc..."` |

### Function Modifiers

| Modifier | Description | Gas |
|----------|-------------|-----|
| `view` | Read-only function | FREE |
| `write` | Modifies state | Paid |
| `payable` | Receives tokens | Paid |
| `onlyOwner` | Owner restricted | Paid |

### Operations Reference

#### Variable Operations

```json
{"op": "set", "var": "count", "value": "42"}
{"op": "add", "var": "count", "value": "1"}
{"op": "sub", "var": "count", "value": "1"}
```

#### Mapping Operations

```json
{"op": "map_set", "map": "balances", "key": "msg.sender", "value": "100"}
{"op": "map_add", "map": "balances", "key": "msg.sender", "value": "msg.amount"}
{"op": "map_sub", "map": "balances", "key": "msg.sender", "value": "amount"}
```

#### Control Flow

```json
{"op": "require", "left": "amount", "cmp": ">=", "right": "100", "msg": "Too low"}
```

Comparisons: `>`, `>=`, `<`, `<=`, `==`, `!=`

#### Token Transfer

```json
{"op": "transfer", "to": "msg.sender", "amount": "amount"}
```

#### Return Value

```json
{"op": "return", "value": "balances[user]"}
```

### Built-in Variables

| Variable | Description |
|----------|-------------|
| `msg.sender` | Transaction caller |
| `msg.amount` | Tokens sent (payable only) |
| `block.height` | Current block number |
| `block.timestamp` | Block timestamp (Unix) |
| `contract.owner` | Contract owner |
| `contract.address` | Contract address |

### Auto-Generated Methods

Every contract automatically gets:

```
┌─────────────────────────────────────────────────────────────┐
│ For each VARIABLE:                                          │
│   get_<name>()           → Returns value (FREE)             │
│   set_<name>(value)      → Sets value (owner only)          │
├─────────────────────────────────────────────────────────────┤
│ For each MAPPING:                                           │
│   get_<name>(key)        → Returns value (FREE)             │
│   set_<name>(key, value) → Sets value (owner only)          │
├─────────────────────────────────────────────────────────────┤
│ Reserved:                                                   │
│   get_owner()            → Returns owner (FREE)             │
│   set_owner(new_owner)   → Transfer ownership               │
│   get_creator()          → Returns creator (FREE)           │
│   get_token()            → Returns linked token (FREE)      │
│   get_address()          → Returns contract addr (FREE)     │
└─────────────────────────────────────────────────────────────┘
```

---

## Contract Examples

### Example 1: Simple Counter

```json
{
  "name": "Counter",
  "variables": [
    {"name": "count", "type": "uint64", "default": "0"}
  ],
  "mappings": [],
  "functions": []
}
```

**Auto-generated:**
- `get_count()` - FREE
- `set_count(value)` - Owner only

```bash
# Read (FREE)
curl http://localhost:8545/contract/mvm1contract.../call/get_count

# Write (requires signature)
# tx_type: call_contract, method: set_count, args: ["42"]
```

### Example 2: Whitelist

```json
{
  "name": "Whitelist",
  "variables": [
    {"name": "total", "type": "uint64", "default": "0"}
  ],
  "mappings": [
    {"name": "allowed", "key_type": "address", "value_type": "bool"}
  ],
  "functions": []
}
```

**Auto-generated:**
- `get_total()`, `set_total(value)`
- `get_allowed(address)`, `set_allowed(address, value)`

```bash
# Read mapping (FREE)
curl http://localhost:8545/contract/mvm1contract.../mapping/allowed/mvm1user...

# Or via call
curl "http://localhost:8545/contract/mvm1contract.../call/get_allowed?args=mvm1user..."
```

### Example 3: Staking Vault

```json
{
  "name": "Vault",
  "token": "mvm1token...",
  "variables": [
    {"name": "total_staked", "type": "uint64", "default": "0"}
  ],
  "mappings": [
    {"name": "balances", "key_type": "address", "value_type": "uint64"}
  ],
  "functions": [
    {
      "name": "stake",
      "modifiers": ["payable"],
      "args": [],
      "body": [
        {"op": "require", "left": "msg.amount", "cmp": ">", "right": "0", "msg": "Amount=0"},
        {"op": "map_add", "map": "balances", "key": "msg.sender", "value": "msg.amount"},
        {"op": "add", "var": "total_staked", "value": "msg.amount"}
      ]
    },
    {
      "name": "unstake",
      "modifiers": ["write"],
      "args": [{"name": "amount", "type": "uint64"}],
      "body": [
        {"op": "require", "left": "balances[msg.sender]", "cmp": ">=", "right": "amount", "msg": "Insufficient"},
        {"op": "map_sub", "map": "balances", "key": "msg.sender", "value": "amount"},
        {"op": "sub", "var": "total_staked", "value": "amount"},
        {"op": "transfer", "to": "msg.sender", "amount": "amount"}
      ]
    },
    {
      "name": "get_balance",
      "modifiers": ["view"],
      "args": [{"name": "user", "type": "address"}],
      "body": [
        {"op": "return", "value": "balances[user]"}
      ],
      "returns": "uint64"
    }
  ]
}
```

**Usage:**

```bash
# Stake tokens (payable)
curl -X POST http://localhost:8545/tx \
  -H "Content-Type: application/json" \
  -d '{
    "tx_type": "call_contract",
    "from": "mvm1user...",
    "nonce": 0,
    "data": {
      "contract": "mvm1contract...",
      "method": "stake",
      "args": [],
      "amount": 10000
    },
    "signature": "...",
    "public_key": "..."
  }'

# Check balance (FREE)
curl "http://localhost:8545/contract/mvm1contract.../call/get_balance?args=mvm1user..."

# Unstake
curl -X POST http://localhost:8545/tx \
  -H "Content-Type: application/json" \
  -d '{
    "tx_type": "call_contract",
    "from": "mvm1user...",
    "nonce": 1,
    "data": {
      "contract": "mvm1contract...",
      "method": "unstake",
      "args": ["5000"]
    },
    "signature": "...",
    "public_key": "..."
  }'
```

---

## MBI (Mosh Binary Interface)

Like Ethereum's ABI, MBI describes the contract interface.

```bash
curl http://localhost:8545/contract/mvm1contract.../mbi
```

Response:
```json
{
  "success": true,
  "mbi": {
    "name": "Vault",
    "address": "mvm1contract...",
    "owner": "mvm1owner...",
    "token": "mvm1token...",
    "variables": [
      {
        "name": "total_staked",
        "type": "Uint64",
        "read": "GET /contract/.../var/total_staked",
        "write": "POST /tx call_contract set_total_staked"
      }
    ],
    "mappings": [
      {
        "name": "balances",
        "key_type": "Address",
        "value_type": "Uint64",
        "read": "GET /contract/.../mapping/balances/{key}",
        "read_all": "GET /contract/.../mapping/balances"
      }
    ],
    "functions": [
      {
        "name": "stake",
        "modifiers": ["Payable"],
        "args": [],
        "free": false,
        "payable": true
      },
      {
        "name": "get_balance",
        "modifiers": ["View"],
        "args": [{"name": "user", "type": "Address"}],
        "returns": "Uint64",
        "free": true
      }
    ],
    "auto_getters": [...],
    "auto_setters": [...]
  }
}
```

---

## WebSocket

Connect to `ws://localhost:8545/ws` for real-time updates.

### Events

```json
// New block
{"type": "new_block", "data": {"height": 1234, "hash": "abc...", "transactions": 5}}

// New transaction
{"type": "new_transaction", "data": {"hash": "def...", "from": "mvm1...", "to": "mvm1..."}}
```

### JavaScript Example

```javascript
const ws = new WebSocket('ws://localhost:8545/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data.type, data.data);
};
```

---

## Examples

### Complete JavaScript SDK

```javascript
const BASE_URL = 'http://localhost:8545';

// Create wallet
async function createWallet() {
  const res = await fetch(`${BASE_URL}/wallet/new`);
  return res.json();
}

// Get balance
async function getBalance(address) {
  const res = await fetch(`${BASE_URL}/balance/${address}`);
  return res.json();
}

// Transfer MVM
async function transfer(privateKey, from, to, amount) {
  // Get nonce
  const { nonce } = await (await fetch(`${BASE_URL}/nonce/${from}`)).json();
  
  // Sign
  const signRes = await fetch(`${BASE_URL}/tx/sign`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      private_key: privateKey,
      tx_type: 'transfer',
      from, to, value: amount, nonce
    })
  });
  const { signature, public_key } = await signRes.json();
  
  // Submit
  const txRes = await fetch(`${BASE_URL}/tx`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      tx_type: 'transfer',
      from, to, value: amount, nonce,
      signature, public_key
    })
  });
  return txRes.json();
}

// Read contract (FREE)
async function readContract(contractAddr, varName) {
  const res = await fetch(`${BASE_URL}/contract/${contractAddr}/var/${varName}`);
  return res.json();
}

// Call view function (FREE)
async function callView(contractAddr, method, args = []) {
  const argsStr = args.join(',');
  const res = await fetch(`${BASE_URL}/contract/${contractAddr}/call/${method}?args=${argsStr}`);
  return res.json();
}

// Call write function (requires signature)
async function callContract(privateKey, from, contractAddr, method, args, amount = 0) {
  const { nonce } = await (await fetch(`${BASE_URL}/nonce/${from}`)).json();
  
  const signRes = await fetch(`${BASE_URL}/tx/sign`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      private_key: privateKey,
      tx_type: 'call_contract',
      from, nonce,
      data: { contract: contractAddr, method, args, amount }
    })
  });
  const { signature, public_key } = await signRes.json();
  
  const txRes = await fetch(`${BASE_URL}/tx`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      tx_type: 'call_contract',
      from, nonce,
      data: { contract: contractAddr, method, args, amount },
      signature, public_key
    })
  });
  return txRes.json();
}
```

---

## Configuration

### Master Node (config.toml)

```toml
[node]
id = "master"
type = "master"
data_dir = "./data"

[network]
p2p_port = 9000
ws_port = 8546
api_port = 8545
```

### Full Node (node2.toml)

```toml
[node]
id = "node-2"
type = "full"
data_dir = "./data-node2"

[network]
p2p_port = 9001
ws_port = 8547
api_port = 8555

[network.star]
master_url = "ws://localhost:8546/p2p"
```

### Test with ngrok

```bash
# Terminal 1: Run master
./target/release/mvm --config config.toml

# Terminal 2: Expose with ngrok
ngrok http 8545

# Share ngrok URL for remote access
```

---

## Rewards

| Recipient | Percentage | Per Block |
|-----------|------------|-----------|
| Validator (block producer) | 70% | 7 MVM |
| Top 3 Service Nodes | 30% | 3 MVM total |
| - Rank 1 | 50% of pool | 1.5 MVM |
| - Rank 2 | 33% of pool | 1.0 MVM |
| - Rank 3 | 17% of pool | 0.5 MVM |

---

## Error Codes

| Error | Description |
|-------|-------------|
| `invalid_signature` | Signature doesn't match sender |
| `invalid_nonce` | Wrong nonce (get current nonce first) |
| `insufficient_balance` | Not enough balance |
| `contract_not_found` | Contract doesn't exist |
| `method_not_found` | Method doesn't exist |
| `only_owner` | Only owner can call |
| `not_view_function` | Can't call write fn via read endpoint |
| `require_failed` | Contract require() failed |

---

## API Endpoint Summary

| Category | Count | Description |
|----------|-------|-------------|
| Chain | 7 | Status, blocks, transactions |
| Accounts | 6 | Wallets, balances, faucet |
| Tokens | 5 | MVM-20 operations |
| Contracts | 7 | Deploy, call, read (5 FREE) |
| Transactions | 2 | Sign, submit |
| WebSocket | 2 | Real-time updates |
| **Total** | **32** | |

---

## License

MIT

---

## Author

Mohsin - [0xmohsin.dev](https://0xmohsin.dev)

---

## Links

- GitHub: [mohsin-blockchain](https://github.com/user/mohsin-blockchain)
- Documentation: [docs.mvm.network](https://docs.mvm.network)
- Explorer: [explorer.mvm.network](https://explorer.mvm.network)