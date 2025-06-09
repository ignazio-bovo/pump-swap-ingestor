CREATE TABLE IF NOT EXISTS trades
(
    amount_lamport UInt64,
    amount_usd     Float64,
    is_sell        UInt8,
    user           String,
    timestamp_unix UInt64,
    timestamp      DateTime MATERIALIZED toDateTime(timestamp_unix),
    tx_hash        String,
    log_index      UInt64,
    pool           String,
    fees           UInt64,
    fees_usd       Float64,
    quote_mint    String,
    base_mint     String,
    quote_amount   UInt64,
    base_amount    UInt64
) ENGINE = ReplacingMergeTree()
      ORDER BY (user, tx_hash, log_index)
      PARTITION BY (ascii(substring(user, -1, 1)) % 10)
      SETTINGS index_granularity = 8192;