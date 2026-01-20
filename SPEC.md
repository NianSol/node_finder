# Node Finder - Technical Specification

A Telegram bot that discovers and validates public Ethereum-compatible RPC nodes via Shodan API.

## Overview

Node Finder queries Shodan for publicly accessible RPC nodes, validates their sync status and functionality, and returns working endpoints to users via Telegram.

---

## Supported Chains

| Chain | Chain ID (Hex) | Chain ID (Dec) | Default Reference RPC |
|-------|----------------|----------------|-----------------------|
| Ethereum | 0x1 | 1 | https://eth.llamarpc.com |
| BSC | 0x38 | 56 | https://bsc.meowrpc.com |
| Base | 0x2105 | 8453 | https://base-rpc.publicnode.com |
| Custom | User-defined | User-defined | User-provided |

### Genesis Block Hashes (Hardcoded)
- **ETH**: `0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3`
- **BSC**: `0x0d21840abff46b96c84b2ac9e10e4f5cdaeb5693cb665db62a2f3b02d2d57b5b`
- **Base**: `0xf712aa9241cc24369b143cf6dce85f0902a9731e70d66818a3a5845b296c73dd`

---

## Architecture

### Technology Stack
- **Language**: Rust
- **Async Runtime**: Tokio
- **Telegram Library**: teloxide
- **HTTP Client**: reqwest
- **WebSocket Client**: tokio-tungstenite
- **JSON**: serde_json

### Core Flow
1. Single Shodan API query per user request
2. Parallel node validation (HTTP unlimited, WS semaphore-limited to 25 concurrent)
3. Results sorted by latency (fastest first)
4. Respond only after full validation completes

---

## Shodan Integration

### Search Query Format
```
port:8545,8546 "Chain ID: 0x{hex_id}" OR "Chain ID: {decimal_id}" country:{country_code}
```

### Supported Locations
| Country | Code | Display |
|---------|------|---------|
| United States | US | 🇺🇸 United States |
| Germany | DE | 🇩🇪 Germany |
| Finland | FI | 🇫🇮 Finland |
| Canada | CA | 🇨🇦 Canada |
| Netherlands | NL | 🇳🇱 Netherlands |
| France | FR | 🇫🇷 France |
| Singapore | SG | 🇸🇬 Singapore |
| All | - | 🌍 All Locations |

### Query Strategy
- Single query fetches both ports (8545, 8546)
- Filter results locally based on user's HTTP/WS preference
- Search both hex AND decimal chain ID formats

---

## Node Validation

### Full Node Validation
1. **Connectivity Check**: HTTP/WS connection within 5 second timeout
2. **Chain ID Verification**: `eth_chainId` must match expected value
3. **Genesis Block Check**: Block 0 hash must match hardcoded genesis hash
4. **Sync Status Check**:
   - Query reference RPC for current block: `eth_blockNumber`
   - Query target node for current block: `eth_blockNumber`
   - Difference must be within user-configured tolerance (default: 50 blocks)

### Archive Node Validation
All Full Node checks PLUS:
1. Query `eth_getBlockByNumber` for block 1 (10s total budget)
2. Query `eth_getBlockByNumber` for block 100
3. Query `eth_getBlockByNumber` for block 1,000,000
4. **All three blocks must return valid data** to classify as archive

### Timeouts
| Operation | Timeout |
|-----------|---------|
| HTTP RPC call | 5 seconds |
| WS RPC call | 5 seconds |
| Archive block queries (total) | 10 seconds |

### WebSocket Handling
- Port 8546 assumes `ws://` protocol
- Maximum 25 concurrent WS connections (semaphore-limited)
- Connections closed immediately after validation

---

## Reference RPC Handling

### Failure Behavior
- If reference RPC is unreachable, **fail the entire request**
- User receives error message asking them to try again or configure custom RPC

### Custom Reference RPC
- Users can configure custom reference RPCs per chain in Config
- Custom RPC **replaces** the default (no fallback)
- Custom RPCs stored in user's config

---

## Node Types & Defaults

| Type | Description | Default Count |
|------|-------------|---------------|
| Full Node | Any synced node passing validation | 10 |
| Archive Node | Node returning data for early blocks (1, 100, 1M) | 10 |
| Bulk Node | JSON-formatted list of validated nodes | 50 |

---

## Telegram Bot Interface

### Commands
- `/start` - Main entry point, shows node type selection
- `/help` - Brief command list and usage

### Button Flow

```
/start
    │
    ├── 🔄 Full Node
    ├── 📚 Archive Node
    ├── 📦 Bulk Nodes
    └── ⚙️ Config
```

#### After Node Type Selection → Chain Selection
```
Select Chain:
    ├── Ξ Ethereum
    ├── ⛓️ BSC
    ├── 🔵 Base
    └── 🔧 Custom Chain
```

#### After Chain Selection → Location Selection
```
Select Location:
    ├── 🇺🇸 United States
    ├── 🇩🇪 Germany
    ├── 🇫🇮 Finland
    ├── 🇨🇦 Canada
    ├── 🇳🇱 Netherlands
    ├── 🇫🇷 France
    ├── 🇸🇬 Singapore
    └── 🌍 All Locations
```

### Custom Chain Wizard
Interactive sequence (no timeout - user can `/start` to reset):
1. Bot: "Enter the Chain ID (decimal, e.g., 137 for Polygon):"
2. User: `137`
3. Bot: "Enter a reference RPC URL for this chain:"
4. User: `https://polygon-rpc.com`
5. Proceed to location selection

### Config Menu
Single message with all settings, inline edit buttons:
```
⚙️ Configuration

📊 Default node count: 10 [Edit]
🔌 Preferred protocol: HTTP [Toggle]
🔄 Sync tolerance: 50 blocks [Edit]

Reference RPCs:
  • ETH: eth.llamarpc.com [Edit]
  • BSC: bsc.meowrpc.com [Edit]
  • Base: base-rpc.publicnode.com [Edit]

[Back to Main Menu]
```

### Search Progress
- Display static "🔍 Searching..." message
- Message is edited with results when validation completes

### Empty Results Handling
If no nodes pass validation for selected location:
1. Automatically retry with "All Locations"
2. Inform user: "No nodes found in [Location]. Expanded search to all locations."

---

## Output Format

### Individual Nodes (Full/Archive)
```
✅ Found 10 synced nodes:

1. http://203.0.113.45:8545
2. http://198.51.100.23:8545
3. http://192.0.2.89:8545
...
```

Minimal format: URL + implicit sync status (only synced nodes shown)

### Bulk Export
JSON array split across multiple messages if exceeds 4096 characters:
```json
[
  "http://203.0.113.45:8545",
  "http://198.51.100.23:8545",
  "http://192.0.2.89:8545"
]
```

### WebSocket Format
```
ws://203.0.113.45:8546
```

---

## User Configuration

### Storage
- JSON file: `config.json` in project directory
- Structure: `{ "user_id": { ...config } }`
- Overwrite directly on save (no backups)

### Config Schema
```json
{
  "12345678": {
    "default_count": 10,
    "protocol": "http",
    "sync_tolerance": 50,
    "reference_rpcs": {
      "1": "https://eth.llamarpc.com",
      "56": "https://bsc.meowrpc.com",
      "8453": "https://base-rpc.publicnode.com"
    }
  }
}
```

### Default Values (New Users)
- `default_count`: 10
- `protocol`: "http"
- `sync_tolerance`: 50 blocks
- `reference_rpcs`: Use hardcoded defaults

---

## Concurrency & Parallel Queries

### User Behavior
- `/start` during an active search shows new menu
- If user starts a second query, run it in parallel (if Shodan rate limit permits)
- Both results are sent when ready (separate messages)

### Shodan Rate Limiting
- Standard Shodan rate limit: 1 request/second
- Bot tracks last query timestamp
- If second query comes within rate limit window, queue it

### Validation Parallelism
- HTTP: Unlimited concurrent validations
- WebSocket: Max 25 concurrent (semaphore)

---

## Error Handling

### Shodan API Errors
- Return user-friendly error: "Shodan search failed. Please try again."

### Reference RPC Failure
- Return error: "Reference node unavailable. Configure a custom RPC in settings or try again later."

### All Nodes Failed Validation
- Auto-expand to All Locations
- If still no results: "No working nodes found. The network may be experiencing issues."

### Telegram API Errors
- Log to stdout, retry once, then silently fail

---

## Bot Behavior

### Startup
- Start accepting commands immediately upon Telegram API connection
- No pre-checks required

### Shutdown
- Immediate abort on SIGTERM/SIGINT
- No graceful drain of pending validations

### Chat Types
- Works in both direct messages and group chats
- Responds to commands and inline button callbacks in groups

### Logging
- Stdout only during runtime
- No persistent log files

---

## Environment Variables

```env
TELEGRAM_TOKEN=<bot token from @BotFather>
SHODAN_TOKEN=<Shodan API key>
```

Location: `/root/projects/node_finder/.env`

---

## File Structure

```
node_finder/
├── Cargo.toml
├── .env
├── .gitignore
├── SPEC.md
├── config.json          # Created at runtime
└── src/
    ├── main.rs
    ├── bot/
    │   ├── mod.rs
    │   ├── commands.rs   # /start, /help handlers
    │   ├── callbacks.rs  # Button callback handlers
    │   └── keyboards.rs  # Inline keyboard builders
    ├── shodan/
    │   ├── mod.rs
    │   └── client.rs     # Shodan API client
    ├── validator/
    │   ├── mod.rs
    │   ├── http.rs       # HTTP RPC validation
    │   ├── ws.rs         # WebSocket RPC validation
    │   └── archive.rs    # Archive node detection
    ├── config/
    │   ├── mod.rs
    │   └── storage.rs    # JSON config persistence
    └── chains/
        ├── mod.rs
        └── genesis.rs    # Hardcoded genesis hashes
```

---

## RPC Methods Used

| Method | Purpose |
|--------|---------|
| `eth_chainId` | Verify correct chain |
| `eth_blockNumber` | Check sync status |
| `eth_getBlockByNumber` | Archive detection (blocks 1, 100, 1000000) |

---

## Security Considerations

### Honeypot Detection
- Verify `eth_chainId` matches expected value
- Verify genesis block (block 0) hash matches hardcoded known hash
- Do NOT verify account balances or latest block hashes (overkill)

### Input Validation
- Sanitize user-provided chain IDs (must be valid integer)
- Validate user-provided RPC URLs (must be valid URL format)
- No shell command execution from user input

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 1.0.0 | 2026-01-20 | Initial specification |
