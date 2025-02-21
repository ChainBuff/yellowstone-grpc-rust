#[cfg(test)]
mod data_latency_tests {
    use crate::common::{
        myerror::AppError,
        yellowstone_grpc::YellowstoneGrpc,
    };
    use tokio::test;
    use dotenvy::dotenv;
    use std::env;
    use log::{error, info};
    use futures::{channel::mpsc, sink::SinkExt, stream::StreamExt};
    use yellowstone_grpc_proto::geyser::{
        SubscribeRequest, SubscribeRequestPing, subscribe_update::UpdateOneof, SubscribeUpdateBlockMeta,
    };
    use chrono::{DateTime, TimeZone, Utc, Local};
    #[test]
    async fn test_data_latency() -> Result<(), AppError> {
        dotenv().ok();
        pretty_env_logger::init_custom_env("RUST_LOG");
        let yellowstone_url = env::var("YELLOWSTONE_URL")?;
        let yellowstone_grpc = YellowstoneGrpc::new(yellowstone_url);

        let (mut subscribe_tx, mut stream) = yellowstone_grpc.subscribe_confirmed_block().await??;

        let (mut tx, mut rx) = mpsc::channel::<SubscribeUpdateBlockMeta>(1000);

        tokio::spawn(async move {
            while let Some(message) = stream.next().await {
                match message {
                    Ok(msg) => {
                        match msg.update_oneof {
                            Some(UpdateOneof::BlockMeta(block_meta)) => {
                                let _ = tx.try_send(block_meta);
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

        while let Some(event) = rx.next().await {
            let timestamp = event.block_time.unwrap().timestamp;
            let datetime: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0).unwrap();
            info!("block timestamp: {}, now: {}, latency: {}ms", datetime, Local::now(), Local::now().signed_duration_since(datetime).num_milliseconds());
        }
        Ok(())
    }
}
