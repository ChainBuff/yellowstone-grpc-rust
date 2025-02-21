#[cfg(test)]
mod subscribe_instructions_tests {
    use crate::common::{
        // event::{RayCPMMSwapBaseInLog},
        event::{EventTrait, RayCPMMSwapBaseInLog}, myerror::AppError, yellowstone_grpc::{TransactionPretty, YellowstoneGrpc}
    };
    use chrono::Local;
    use dotenvy::dotenv;
    use futures::{channel::mpsc, sink::SinkExt, stream::StreamExt};
    use log::{error, info};
    use tokio::test;
    use solana_sdk::pubkey;
    use solana_sdk::pubkey::Pubkey;
    use yellowstone_grpc_proto::geyser::{
        subscribe_update::UpdateOneof, SubscribeRequest, SubscribeRequestPing,
    };
    use std::env;

    const CPMM_PROGRAM_ID: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");

    const SWAP_BASE_INPUT_PARTNER_DISCRIMINATOR: u64 =
        u64::from_le_bytes([143, 190, 90, 218, 196, 30, 51, 222]);

    #[test]
    async fn test_subscribe_instructions() -> Result<(), AppError> {
        dotenv().ok();
        pretty_env_logger::init_custom_env("RUST_LOG");
        let yellowstone_url = env::var("YELLOWSTONE_URL")?;
        let yellowstone_grpc = YellowstoneGrpc::new(yellowstone_url);

        let transactions = yellowstone_grpc.subscribe_transaction(vec![], vec![], vec![]);

        let (mut subscribe_tx, mut stream) = yellowstone_grpc.connect(transactions).await??;

        let (mut tx, mut rx) = mpsc::channel::<TransactionPretty>(1000);

        tokio::spawn(async move {
            while let Some(message) = stream.next().await {
                match message {
                    Ok(msg) => {
                        match msg.update_oneof {
                            Some(UpdateOneof::Transaction(sut)) => {
                                let transaction_pretty: TransactionPretty = sut.into();
                                let _ = tx.try_send(transaction_pretty);
                            }
                            Some(UpdateOneof::Ping(_)) => {
                                let _ = subscribe_tx
                                    .send(SubscribeRequest {
                                        ping: Some(SubscribeRequestPing { id: 1 }),
                                        ..Default::default()
                                    })
                                    .await;
                                info!("service is ping: {}", Local::now());
                            }
                            Some(UpdateOneof::Pong(_)) => {
                                info!("service is pong: {}", Local::now());
                            }
                            _ => {}
                        }
                        continue;
                    }
                    Err(error) => {
                        error!("error: {error:?}");
                        break;
                    }
                }
            }
        });

        while let Some(transaction_pretty) = rx.next().await {
            let trade_raw = transaction_pretty.tx.clone();
            let meta = &trade_raw.meta.clone().unwrap();
            if meta.err.is_some() {
                continue;
            }

            let instructions = transaction_pretty.instructions.clone();
            for i in instructions.iter() {
                let program_id = transaction_pretty.account_keys[i.program_id_index as usize].clone();
                if program_id != CPMM_PROGRAM_ID.to_string() {
                    continue;
                }

                info!("raydium cpmm instruction {:?}", i);
                let (disc_bytes, rest) = i.data.split_at(8);
                let discriminator = u64::from_le_bytes(disc_bytes.try_into().unwrap());
                if discriminator == SWAP_BASE_INPUT_PARTNER_DISCRIMINATOR {
                    info!("raydium cpmm swap input");
                    info!("transaction hash: {:?}", transaction_pretty.signature);
                    info!("amm: {:?}", transaction_pretty.account_keys[i.accounts[3] as usize].clone());
                    info!("vault_a: {:?}", transaction_pretty.account_keys[i.accounts[6] as usize].clone());
                    info!("vault_b: {:?}", transaction_pretty.account_keys[i.accounts[7] as usize].clone());

                    let log = RayCPMMSwapBaseInLog::from_bytes(rest).unwrap();
                    info!("log: {:?}", log);
                }
            }

        }

        Ok(())
    }
}
