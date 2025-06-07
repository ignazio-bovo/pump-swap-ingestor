# Demo for Data Ingestion from Pump-Swap DEX
This system consists of three main components that work together to capture and store real-time trading data from the pump-swap decentralized exchange:

## Source
Data is derived by the logs from the pump swap program, this ensures that the actual trade on pump and swap has been executed successfully

## Ingestor

The ingestor connects to Solana's public WebSocket endpoint to receive raw transaction data in real-time. Key features:

- **WebSocket Connection**: Subscribes to Solana's `logsSubscribe` method to receive raw byte payloads
- **Non-blocking Architecture**: Uses unbounded channels for synchronization between components
- **Real-time Processing**: Streams transaction data continuously without blocking operations

## Deserializer / Processor

The processor handles data transformation and preparation for storage:

- **IDL-based Deserialization**: Uses the pump-swap IDL (Interface Description Language) to parse raw transaction data
- **Data Model Preparation**: Converts raw blockchain data into structured format suitable for database storage

### SOL Price Feed Subcomponent

A dedicated task manages USD pricing data:

- **Price Updates**: Fetches current SOL/USD exchange rates every 20 seconds
- **Thread-safe Synchronization**: Uses `RwLock` for safe concurrent access between processor and price feed tasks
- **Real-time Valuation**: Enables USD value calculation for all trades

## ClickHouse Backend

Handles data persistence and querying:

- **Docker Integration**: Connects to containerized ClickHouse instance
- **Web Interface**: Query playground available at `http://localhost:8123/play`
- **Channel-based Input**: Receives structured data from the ingestor via synchronization channels

### Data Model

The system uses a simplified schema designed for trading analytics:

| Field | Type | Description |
|-------|------|-------------|
| `amount_lamport` | UInt64 | Trade amount in lamports (raw SOL units) |
| `amount_usd` | Float64 | Trade value in USD |
| `is_sell` | UInt8 | Trade direction (0=buy, 1=sell) |
| `user` | String | Trader's wallet address |
| `timestamp` | DateTime | Transaction timestamp |
| `tx_hash` | String | Unique transaction identifier |
| `log_index` | UInt64 | Position within transaction logs |
| `pool` | String | Trading pool identifier |
| `fees` | UInt64 | Transaction fees in lamports |
| `fees_usd` | Float64 | Transaction fees in USD |

This architecture enables real-time capture, processing, and analysis of pump-swap trading activity with minimal latency and high reliability.

## Queries

### Basic PNL
```clickhouse
SELECT 
    user,
    SUM(CASE 
        WHEN is_sell = 1 THEN amount_usd 
        ELSE -amount_usd 
    END) - SUM(fees_usd) AS realized_pnl,
    COUNT(*) as total_trades,
    SUM(CASE WHEN is_sell = 1 THEN 1 ELSE 0 END) as sells,
    SUM(CASE WHEN is_sell = 0 THEN 1 ELSE 0 END) as buys
FROM "pump_swap_data"."trades" 
GROUP BY user
ORDER BY realized_pnl DESC;
```
### Cumulative Time series for pnl for a specific account
```clickhouse
WITH trade_pnl AS (
    SELECT 
        timestamp_unix,
        timestamp,
        CASE 
            WHEN is_sell = 1 THEN amount_usd - fees_usd
            ELSE -(amount_usd + fees_usd)
        END as trade_pnl
    FROM trades 
    WHERE user = '<SPECIFIC_WALLET_ADDRESS>'
    ORDER BY timestamp_unix
)
SELECT 
    timestamp_unix,
    timestamp,
    trade_pnl,
    SUM(trade_pnl) OVER (ORDER BY timestamp_unix) as cumulative_pnl
FROM trade_pnl
ORDER BY timestamp_unix;
```

## Further Improvements
- The pool information is not directly available from the events so it must be fetched separately
- In case it's necessary the logs can be processed in parallel for further speed