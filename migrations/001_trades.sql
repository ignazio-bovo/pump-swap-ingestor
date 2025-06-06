CREATE TABLE IF NOT EXISTS trades (
                                      amount_sol UInt64,
                                      is_sell UInt8,           -- ClickHouse doesn't have native boolean, use UInt8 (0/1)
                                      user String,
                                      timestamp UInt64,        -- Unix timestamp as UInt64
                                      tx_hash String,
                                      log_index UInt64,        -- Changed from usize to UInt64 for consistency
                                      pool String
) ENGINE = MergeTree()
    ORDER BY (timestamp, tx_hash, log_index)  -- Good primary key for time-series data
    PARTITION BY toYYYYMM(toDateTime(timestamp))  -- Partition by month for better performance
    SETTINGS index_granularity = 8192;