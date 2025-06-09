# Demo for Data Ingestion from Pump-Swap DEX

This system captures and stores real-time trading data from the pump-swap decentralized exchange using three main components:

## Instructions
[Docker](https://docs.docker.com/get-started/get-docker/) and Docker compose required

Start the ingestor
```shell
docker compose up -d
```
Stop the ingestor
```shell
docker compose down -v
```
## Exports
`exports` folder contains exported csv with sample data

# Overview

##  Source
Extracts trade data from pump-swap program logs, ensuring only successfully executed trades are captured.
Usd amount for trade can be computed by considering only the SOL amount and then fetching the actual price

## Ingestor
Connects to Solana's WebSocket endpoint for real-time transaction streaming:
- **WebSocket Connection**: Subscribes to `logsSubscribe` for raw transaction data
- **Non-blocking Architecture**: Uses unbounded channels for component synchronization
- **Real-time Processing**: Continuous data streaming without blocking operations

## Processor
Handles data transformation and preparation:
- **IDL-based Deserialization**: Parses raw transaction data using pump-swap IDL
- **Data Model Preparation**: Converts blockchain data to structured database format

### SOL Price Feed Subservice
- **Price Updates**: Fetches SOL/USD rates every 20 seconds
- **Thread-safe Sync**: Uses `RwLock` for concurrent access
- **Real-time Valuation**: Enables USD calculations for all trades

### Pool Cache 
Fetches pool information (`quote_mint`, `base_mint`) via RPC on cache miss to determine base and quote mint information

## ClickHouse Backend
- **Docker Integration**: Containerized ClickHouse instance
- **Web Interface**: Query playground at `http://localhost:8123/play`
- **Channel-based Input**: Receives data via synchronization channels

### Data Model

| Field | Type | Description |
|-------|------|-------------|
| `amount_lamport` | UInt64 | Trade amount in lamports (raw SOL units) |
| `amount_usd` | Float64 | Trade value in USD |
| `is_sell` | UInt8 | Trade direction (0=buy, 1=sell) |
| `user` | String | Trader's wallet address |
| `timestamp_unix` | UInt64 | Unix timestamp of transaction |
| `timestamp` | DateTime | Human-readable timestamp (materialized) |
| `tx_hash` | String | Unique transaction identifier |
| `log_index` | UInt64 | Position within transaction logs |
| `pool` | String | Trading pool identifier |
| `fees` | UInt64 | Transaction fees in lamports |
| `fees_usd` | Float64 | Transaction fees in USD |
| `quote_mint` | String | Quote token mint address |
| `base_mint` | String | Base token mint address |
| `quote_amount` | UInt64 | Raw quote token amount traded |
| `base_amount` | UInt64 | Raw base token amount traded |

## Analytics Queries

### Realized PnL by User
```clickhouse
SELECT 
   user,
   SUM(CASE WHEN is_sell = 1 THEN amount_usd ELSE -amount_usd END) - SUM(fees_usd) AS realized_pnl,
   COUNT(*) as total_trades,
   countIf(is_sell = 1) as sells,
   countIf(is_sell = 0) as buys
FROM "pump_swap_data"."trades"
GROUP BY user
ORDER BY realized_pnl DESC;
```
### Cumulative Realized PNL time series given account
```clickhouse
WITH realized_pnl AS ( -- as before
    SELECT 
        timestamp_unix,
        timestamp,
        CASE 
            WHEN is_sell = 1 THEN amount_usd
            ELSE -amount_usd
        END as trade_pnl
    FROM "pump_swap_data"."trades"
    WHERE user = '<SPECIFIC_WALLET_ADDRESS>'
    ORDER BY timestamp_unix
)
SELECT 
    timestamp,
    trade_pnl,
    SUM(trade_pnl) OVER (ORDER BY timestamp_unix) as cumulative_pnl
FROM realized_pnl
ORDER BY timestamp_unix;
```
## Improvements
### Immediate
- Implement an internal in memory retry queue for failed trade upsertion on db (for example in case Clickhouse is temporarely down)
- Wss should also have a retry logic in case of failed pushes, keeping track of the latest block processed is necessary in order to avoid gaps
- In case of high load parallel event processing can be implemented by spawning multiple tokio tasks to process trades concurrently while maintaining order
    using channels or async queues