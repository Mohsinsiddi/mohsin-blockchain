# MOHSIN VIRTUAL MACHINE (MVM)

A simple blockchain with custom virtual machine and MVM-20 token standard.

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
- ✅ Native MVM token
- ✅ MVM-20 token standard (create your own tokens!)
- ✅ Smart contract getter/setter support
- ✅ WebSocket for browsers
- ✅ P2P for full nodes
- ✅ REST API
- ✅ Faucet
- ✅ RocksDB storage
- ✅ State pruning for light sync

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Build

```bash
cargo build --release
```

### Run Master Node

```bash
./target/release/mvm --config config.toml
```

### Run Additional Nodes (Different Terminals)

```bash
# Terminal 2
./target/release/mvm --config node2.toml

# Terminal 3
./target/release/mvm --config node3.toml
```

## Test on Same Machine

```bash
# Terminal 1: Master
./target/release/mvm --config config.toml

# Terminal 2: Node 2 (different ports)
./target/release/mvm --config node2.toml

# Terminal 3: Node 3 (different ports)
./target/release/mvm --config node3.toml
```

## Test with ngrok (Remote Access)

```bash
# Terminal 1: Run master
./target/release/mvm --config config.toml

# Terminal 2: Expose with ngrok
ngrok http 8545

# Share the ngrok URL with others
# They can connect by editing node2.toml:
# master_url = "ws://abc123.ngrok.io/p2p"
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | API info |
| `/status` | GET | Chain status |
| `/block/:height` | GET | Get block by height |
| `/block/latest` | GET | Get latest block |
| `/balance/:address` | GET | Get MVM balance |
| `/faucet/:address` | POST | Claim faucet tokens |
| `/tx` | POST | Submit transaction |
| `/tokens` | GET | List all MVM-20 tokens |
| `/token/:address` | GET | Get token info |
| `/ws` | WebSocket | Browser connection |
| `/p2p` | WebSocket | Full node connection |

## Example API Calls

```bash
# Get status
curl http://localhost:8545/status

# Get balance
curl http://localhost:8545/balance/mvm1abc...

# Claim from faucet
curl -X POST http://localhost:8545/faucet/mvm1abc...

# Create token
curl -X POST http://localhost:8545/tx \
  -H "Content-Type: application/json" \
  -d '{
    "tx_type": "create_token",
    "from": "mvm1abc...",
    "data": {
      "name": "Pepe Token",
      "symbol": "PEPE",
      "total_supply": 1000000
    },
    "signature": "0x..."
  }'
```

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

## Architecture

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

## Rewards

| Recipient | Percentage | Per Block |
|-----------|------------|-----------|
| Validator (block producer) | 70% | 7 MVM |
| Top 3 Service Nodes | 30% | 3 MVM total |
| - Rank 1 | 50% of pool | 1.5 MVM |
| - Rank 2 | 33% of pool | 1.0 MVM |
| - Rank 3 | 17% of pool | 0.5 MVM |

## License

MIT

## Author

Mohsin - [0xmohsin.dev](https://0xmohsin.dev)
# mohsin-blockchain
