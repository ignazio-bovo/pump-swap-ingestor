CREATE TABLE trades (
                        amount_sol UInt64,
                        is_sell UInt8,  -- ClickHouse uses UInt8 for boolean (0/1)
                        user String,
                        timestamp DateTime64(3),  -- Assuming millisecond precision
                        tx_hash String,
                        log_index UInt32,
                        pool String
) ENGINE = MergeTree()
ORDER BY (timestamp, pool, user)
PARTITION BY toYYYYMM(timestamp);