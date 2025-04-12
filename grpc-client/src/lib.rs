use std::{sync::Arc, time::Duration};

use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::VersionedTransaction};
use solana_transaction_status::TransactionStatusMeta;
use tokio::sync::Mutex;
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::{
    convert_from::create_tx_with_meta, geyser::SubscribeUpdateTransaction,
};

use anyhow::Error;
#[derive(Debug)]
#[allow(dead_code)]
pub struct AppError(Error);

impl<E> From<E> for AppError
where
    E: Into<Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}


#[derive(Debug)]
pub struct TransactionFormat {
    pub slot: u64,
    pub signature: Signature,
    #[allow(dead_code)]
    pub index: u64,
    pub meta: Option<TransactionStatusMeta>,
    #[allow(dead_code)]
    pub transation: VersionedTransaction,
    pub account_keys: Vec<Pubkey>,
}

impl From<SubscribeUpdateTransaction> for TransactionFormat {
    fn from(SubscribeUpdateTransaction { transaction, slot }: SubscribeUpdateTransaction) -> Self {
        let raw = transaction.expect("should be defined");
        let index = raw.index;
        let tx = create_tx_with_meta(raw).expect("valid tx with meta");
        Self {
            slot,
            index,
            signature: *tx.transaction_signature(),
            meta: tx.get_status_meta(),
            transation: tx.get_transaction(),
            account_keys: tx.account_keys().iter().copied().collect(),
        }
    }
}

pub struct YellowstoneGrpc {
    endpoint: String,
    x_token: Option<String>,
}

impl YellowstoneGrpc {
    pub fn new(endpoint: String, x_token: Option<String>) -> Self {
        Self { endpoint, x_token }
    }

    pub async fn build_client(
        self,
    ) -> Result<Arc<Mutex<GeyserGrpcClient<impl Interceptor>>>, AppError> {
        let client = GeyserGrpcClient::build_from_shared(self.endpoint)?
            .x_token(self.x_token)?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect_timeout(Duration::from_secs(10))
            .keep_alive_while_idle(true)
            .timeout(Duration::from_secs(60))
            .connect()
            .await?;
        Ok(Arc::new(Mutex::new(client)))
    }
}
