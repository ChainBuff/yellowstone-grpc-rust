# yellowstone-grpc-rust

Yellowstone gRPC 是获取 Solana 链上数据最快的方式。数据以流的方式推送，客户端需要配置订阅来获取和解析数据。

本教程旨在提供一些简单的订阅配置例子，帮助你快速熟悉此工具。

---

## subscribe-tx 订阅账户交易

``` bash
# cargo run --bin transactions
```

## subscribe-logs 订阅 token 交易，解析Logs会包含池子的最新数据

``` bash
# cargo run --bin parse-log
```

## subscribe-account 订阅账户变化

``` bash
# cargo run --bin accounts
```
