# Compose Sidecar Integration

This module provides an **HTTP client** for op-rbuilder to poll the external **Compose Sidecar** service for cross-chain
transactions.

## Overview

The sidecar integration enables op-rbuilder to receive and execute cross-chain transactions (XTs) that are coordinated
by the Compose Sidecar. These transactions are part of atomic cross-chain operations that must be included in both
participating rollups.

```
┌─────────────────────┐         ┌─────────────────────┐
│  Shared Publisher   │         │   Compose Sidecar   │
│  (2PC Coordinator)  │◄───────►│   (Go HTTP server)  │
└─────────────────────┘  TCP    └──────────┬──────────┘
                                           │
                                           │ POST /transactions
                                           │ (HTTP pull model)
                                           ▼
                                ┌─────────────────────┐
                                │    op-rbuilder      │
                                │  (Rust HTTP client) │
                                └──────────┬──────────┘
                                           │
                                           │ WebSocket (outbound)
                                           ▼
                                ┌─────────────────────┐
                                │    rollup-boost     │
                                │  (flashblocks sub)  │
                                └─────────────────────┘
```

## Architecture

### Pull Model (Flashblocks)

The builder uses a **pull model** where it polls the sidecar at the start of each flashblock:

1. Builder starts building a flashblock
2. Builder calls `POST /transactions` to the sidecar
3. Sidecar responds with either:
    - `hold: true` - Builder should wait and retry
    - `hold: false` + transactions - Builder executes these transactions
    - `hold: false` + empty - No cross-chain transactions for this flashblock

### Transaction Execution Order

Within each flashblock, transactions are executed in this order:

```
1. Sequencer transactions (from FCU attributes)
2. Builder transactions (end-of-block builder payments)
3. Sidecar transactions (cross-chain XTs)  ◄── This module
4. Pool transactions (user mempool)
```

## API

### Endpoint

```
POST {sidecar_endpoint}/transactions
```

### Request

```json
{
    "chain_id": 11155420,
    "block_number": 12345,
    "flashblock_index": 0,
    "state_root": "0x...",
    "timestamp": 1705123456,
    "gas_limit": 30000000
}
```

| Field              | Type   | Description                                      |
|--------------------|--------|--------------------------------------------------|
| `chain_id`         | `u64`  | Chain ID of this rollup                          |
| `block_number`     | `u64`  | Current block number being built                 |
| `flashblock_index` | `u64`  | Index of flashblock within the block (0-indexed) |
| `state_root`       | `B256` | State root of the parent block                   |
| `timestamp`        | `u64`  | Timestamp of the block being built               |
| `gas_limit`        | `u64`  | Gas limit available for this flashblock batch    |

### Response (Hold)

When the sidecar is waiting for cross-chain coordination to complete:

```json
{
    "hold": true,
    "poll_after_ms": 50
}
```

The builder will sleep for `poll_after_ms` milliseconds and retry, up to `max_retries` times.

### Response (Transactions Ready)

When cross-chain transactions are ready for inclusion:

```json
{
    "hold": false,
    "transactions": [
        {
            "raw": "0x02f8...",
            "required": true,
            "instance_id": "abc123"
        }
    ]
}
```

| Field         | Type      | Description                                                                    |
|---------------|-----------|--------------------------------------------------------------------------------|
| `raw`         | `Bytes`   | EIP-2718 RLP-encoded transaction bytes                                         |
| `required`    | `bool`    | If `true`, block build fails if tx fails. If `false`, tx is skipped on failure |
| `instance_id` | `string?` | Optional ID for tracking the cross-chain coordination instance                 |

### Response (No Transactions)

When there are no cross-chain transactions:

```json
{
    "hold": false,
    "transactions": []
}
```

## Configuration

### CLI Arguments

```bash
op-rbuilder \
  --sidecar.endpoint http://localhost:8082 \
  --sidecar.poll-timeout-ms 200 \
  --sidecar.max-retries 5
```

### Environment Variables

```bash
SIDECAR_ENDPOINT=http://localhost:8082
SIDECAR_POLL_TIMEOUT_MS=200
SIDECAR_MAX_RETRIES=5
```

### Configuration Options

| Option         | Default | Description                                   |
|----------------|---------|-----------------------------------------------|
| `endpoint`     | (empty) | Sidecar HTTP endpoint. Empty = disabled       |
| `poll_timeout` | 200ms   | HTTP request timeout for each poll            |
| `max_retries`  | 5       | Max retries when sidecar returns `hold: true` |

## Transaction Execution

### Required Transactions

Required transactions (`required: true`) **must** succeed for the block to be valid:

- Decode failure → Block build fails
- Signature recovery failure → Block build fails
- Exceeds gas/DA limits → Block build fails
- EVM execution error → Block build fails
- Transaction reverts → Block build fails

### Optional Transactions

Optional transactions (`required: false`) are best-effort:

- Decode failure → Skip transaction, continue
- Signature recovery failure → Skip transaction, continue
- Exceeds gas/DA limits → Skip transaction, continue
- EVM execution error → Skip transaction, continue
- Transaction reverts → Skip transaction, continue

### Validation Checks

Before execution, each transaction is validated:

1. **Decode**: Transaction must be valid EIP-2718 RLP
2. **Signature**: ECDSA signature must recover valid sender
3. **Type**: Must not be deposit or blob transaction
4. **Gas limit**: Must fit within flashblock gas budget
5. **DA limit**: Must fit within data availability budget (Fjord)
6. **DA footprint**: Must fit within DA footprint limit (Jovian)

## Error Handling

### Sidecar Errors

| Error            | Behavior                                  |
|------------------|-------------------------------------------|
| HTTP error       | Log warning, continue without sidecar txs |
| Hold timeout     | Log warning, continue without sidecar txs |
| Sidecar disabled | Return `None`, no sidecar polling         |

### Execution Errors (Required)

| Error                     | Behavior         |
|---------------------------|------------------|
| `DecodeError`             | Fail block build |
| `SignatureRecoveryFailed` | Fail block build |
| `InvalidTransactionType`  | Fail block build |
| `LimitsExceeded`          | Fail block build |
| `ExecutionFailed`         | Fail block build |
| `TransactionReverted`     | Fail block build |

### Execution Errors (Optional)

All errors for optional transactions result in the transaction being skipped with debug logging.

## Module Structure

```
sidecar/
├── mod.rs      # Module exports
├── client.rs   # HTTP client with hold/retry loop
├── config.rs   # SidecarConfig
├── types.rs    # PollRequest, PollResponse, ExternalTransaction, SidecarError
└── README.md   # This documentation
```
