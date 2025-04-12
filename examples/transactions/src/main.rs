
use std::collections::HashMap;

use chrono::Local;
use dotenvy::dotenv;
use futures_util::SinkExt;
use grpc_client::{AppError, TransactionFormat, YellowstoneGrpc};
use log::{error, info};
use tokio_stream::StreamExt;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeRequestPing,
    subscribe_update::UpdateOneof,
};

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
                        let transaction_pretty: TransactionFormat = sut.into();
                        info!("transaction: {:#?}", transaction_pretty);
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
