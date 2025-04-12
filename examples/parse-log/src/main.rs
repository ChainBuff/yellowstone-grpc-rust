use std::collections::HashMap;

use anyhow::anyhow;
use base64::{Engine, engine::general_purpose};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::Local;
use dotenvy::dotenv;
use futures_util::SinkExt;
use grpc_client::{AppError, TransactionFormat, YellowstoneGrpc};
use log::{error, info};
use solana_sdk::pubkey::Pubkey;
use tokio_stream::StreamExt;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeRequestPing,
    subscribe_update::UpdateOneof,
};
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: u64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
}
const PROGRAM_DATA: &str = "Program data: ";

pub trait EventTrait: Sized + std::fmt::Debug {
    fn discriminator() -> [u8; 8];
    fn from_bytes(bytes: &[u8]) -> Result<Self, AppError>;
    fn valid_discrminator(head: &[u8]) -> bool;

    fn parse_logs<T: EventTrait + Clone>(logs: &[String]) -> Option<T> {
        logs.iter().rev().find_map(|log| {
            let payload = log.strip_prefix(PROGRAM_DATA)?;
            let bytes = general_purpose::STANDARD
                .decode(payload)
                .map_err(|e| AppError::from(anyhow!(e.to_string())))
                .ok()?;

            let (discr, rest) = bytes.split_at(8);
            if Self::valid_discrminator(discr) {
                T::from_bytes(rest).ok()
            } else {
                None
            }
        })
    }
}

impl EventTrait for TradeEvent {
    fn discriminator() -> [u8; 8] {
        [189, 219, 127, 211, 78, 230, 97, 238]
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, AppError> {
        Self::try_from_slice(bytes).map_err(|e| AppError::from(anyhow!(e.to_string())))
    }

    fn valid_discrminator(discr: &[u8]) -> bool {
        discr == Self::discriminator()
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();
    pretty_env_logger::init();

    let url = std::env::var("YELLOWSTONE_GRPC_URL").expect("YELLOWSTONE_GRPC_URL must be set");
    let grpc = YellowstoneGrpc::new(url, None);
    let client = grpc.build_client().await?;
    let addrs = vec!["6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string()];
    let subscribe_request = SubscribeRequest {
        transactions: HashMap::from([(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                signature: None,
                account_include: addrs,
                account_exclude: vec![],
                account_required: vec![],
            },
        )]),

        commitment: Some(CommitmentLevel::Processed.into()),
        ..Default::default()
    };

    let (mut subscribe_tx, mut stream) = client
        .lock()
        .await
        .subscribe_with_request(Some(subscribe_request))
        .await?;
    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => {
                match msg.update_oneof {
                    Some(UpdateOneof::Transaction(sut)) => {
                        let transaction: TransactionFormat = sut.into();

                        let meta = match transaction.meta {
                            Some(meta) => meta,
                            None => {
                                error!("meta not found");
                                continue;
                            }
                        };

                        let logs = &meta.log_messages.unwrap_or(vec![]);

                        if logs.is_empty() {
                            continue;
                        }

                        if let Some(trade_event) = TradeEvent::parse_logs::<TradeEvent>(logs) {
                            info!("trade_event: {:#?}", trade_event);
                        };
                    }
                    Some(UpdateOneof::Ping(_)) => {
                        // This is necessary to keep load balancers that expect client pings alive. If your load balancer doesn't
                        // require periodic client pings then this is unnecessary
                        let _ = subscribe_tx
                            .send(SubscribeRequest {
                                ping: Some(SubscribeRequestPing { id: 1 }),
                                ..Default::default()
                            })
                            .await;
                        info!("service is ping: {:#?}", Local::now());
                    }
                    Some(UpdateOneof::Pong(_)) => {
                        info!("service is pong: {:#?}", Local::now());
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
    Ok(())
}
