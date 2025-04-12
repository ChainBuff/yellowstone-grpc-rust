use std::collections::HashMap;

use chrono::Local;
use dotenvy::dotenv;
use futures_util::SinkExt;
use grpc_client::{AppError, YellowstoneGrpc};
use log::{error, info};
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signature::Signature};
use tokio_stream::StreamExt;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeRequestPing,
    subscribe_update::UpdateOneof,
};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();
    pretty_env_logger::init();

    let url = std::env::var("YELLOWSTONE_GRPC_URL").expect("YELLOWSTONE_GRPC_URL must be set");
    let grpc = YellowstoneGrpc::new(url, None);
    let client = grpc.build_client().await?;
    let addrs = vec!["53gas6nwoz3GjbdbmiZ5ywLdfnhqUQNEKvF5ErpXUc3S".to_string()];

    let subscribe_request = SubscribeRequest {
        accounts: HashMap::from([(
            "client".to_string(),
            SubscribeRequestFilterAccounts {
                account: addrs,
                ..Default::default()
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
                    Some(UpdateOneof::Account(subscribe_account)) => {
                        if let Some(account) = subscribe_account.account {
                            info!("account: {:#?}", account);
                            let account_pubkey = Pubkey::try_from(account.pubkey.as_slice())?;

                            info!("account_pubkey: {:#?}", account_pubkey);

                            let owner = Pubkey::try_from(account.owner.as_slice())?;
                            info!("owner: {:#?}", owner);

                            let account_signture =
                                match Signature::try_from(account.txn_signature()) {
                                    Ok(sig) => sig,
                                    Err(e) => {
                                        error!("account_signture error: {:#?}", e);
                                        continue;
                                    }
                                };

                            let account_info =
                                match spl_token::state::Account::unpack(&account.data) {
                                    Ok(info) => info,
                                    Err(e) => {
                                        error!("account_info error: {:#?}", e);
                                        continue;
                                    }
                                };

                            info!("account_signture: {:#?}", account_signture);
                            info!("account_info: {:#?}", account_info);
                        }
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
