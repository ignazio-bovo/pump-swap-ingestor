CREATE TABLE IF NOT EXISTS trades (
                                      amount_lamport UInt64,
                                      amount_usd Float64,
                                      is_sell UInt8,
                                      user String,
                                      timestamp_unix UInt64,
                                      timestamp DateTime MATERIALIZED toDateTime(timestamp_unix),
                                      tx_hash String,
                                      log_index UInt64,
                                      pool String,
                                      fees UInt64,
                                      fees_usd Float64
) ENGINE = ReplacingMergeTree()
      ORDER BY (tx_hash, log_index)  -- This enforces uniqueness on the combination
      PARTITION BY toYYYYMM(timestamp)
      SETTINGS index_granularity = 8192;