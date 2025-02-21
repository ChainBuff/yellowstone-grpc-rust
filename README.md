# yellowstone-grpc-rust

Yellowstone gRPC 是获取 Solana 链上数据最快的方式。数据以流的方式推送，客户端需要配置订阅来获取和解析数据。

本教程旨在提供一些简单的订阅配置例子，帮助你快速熟悉此工具。

---

## subscribe-tx 订阅账户交易

``` bash
cargo test --package yellowstone-grpc-rust --bin yellowstone-grpc-rust -- subscribe_tx::subscribe_tx_tests::test_subscribe_tx --exact --show-output
```

## subscribe-logs 订阅 token 交易，解析Logs会包含池子的最新数据

``` bash
cargo test --package yellowstone-grpc-rust --bin yellowstone-grpc-rust -- subscribe_logs::subscribe_logs_tests --show-output
```

## subscribe-instructions 订阅交易指令，解析 instructions 中包含的 swap 交易

``` bash
cargo test --package yellowstone-grpc-rust --bin yellowstone-grpc-rust -- subscribe_logs::subscribe_instructions_tests --show-output
```

## data-latency 测试数据延迟

```bash
cargo test --package yellowstone-grpc-rust --bin yellowstone-grpc-rust -- data_latency_tests::test_data_latency --show-output
```


### PUMP 池子

``` rust
PumpEvent Some(
    TradeEvent {
        mint: Aa4QWNkS3RLUv7DA9BM1a2Hzm4HDQo5PyRefqDJnpump,
        sol_amount: 114631249,
        token_amount: 606908476440,
        is_buy: false,
        user: 2i1S6JReBVBtHRS3WnZjzrf6EPqgMWrtWkUC4T68Y1Yq,
        timestamp: 1735823228,
        virtual_sol_reserves: 77916811328,
        virtual_token_reserves: 413132930150753,
        real_sol_reserves: 47916811328,
        real_token_reserves: 133232930150753,
    },
)
```

### RAYDIUM

``` rust
RaydiumEvent Some(
    SwapBaseInLog {
        log_type: 3,
        amount_in: 30000000,
        minimum_out: 258187550602,
        direction: 2,
        user_source: 30000000,
        pool_coin: 43626306718,
        pool_pc: 391838738896727,
        out_amount: 268593366858,
    },
)
```
