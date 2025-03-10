use std::{collections::HashMap, fmt, time::Duration};

use futures::{channel::mpsc, sink::Sink, Stream};
use rustls::crypto::{ring::default_provider, CryptoProvider};
use tonic::{transport::channel::ClientTlsConfig, Status};
use yellowstone_grpc_client::{GeyserGrpcClient, GeyserGrpcClientResult};
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeUpdate,
    SubscribeUpdateTransaction,
};

use crate::common::myerror::AppError;

type TransactionsFilterMap = HashMap<String, SubscribeRequestFilterTransactions>;

use solana_sdk::signature::Signature;
use solana_transaction_status::{
    EncodedTransactionWithStatusMeta, TransactionWithStatusMeta, UiTransactionEncoding,
};

#[allow(dead_code)]
pub struct TransactionPretty {
    pub slot: u64,
    pub signature: Signature,
    pub is_vote: bool,
    pub tx: TransactionWithStatusMeta,
}
impl fmt::Debug for TransactionPretty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct TxWrap<'a>(&'a EncodedTransactionWithStatusMeta);
        impl<'a> fmt::Debug for TxWrap<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let serialized = serde_json::to_string(self.0).expect("failed to serialize");
                fmt::Display::fmt(&serialized, f)
            }
        }

        f.debug_struct("TransactionPretty")
            .field("slot", &self.slot)
            .field("signature", &self.signature)
            .field("is_vote", &self.is_vote)
            .field(
                "tx",
                &TxWrap(
                    &self
                        .tx
                        .clone()
                        .encode(UiTransactionEncoding::Base64, Some(u8::MAX), true)
                        .expect("failed to encode"),
                ),
            )
            .finish()
    }
}

impl From<SubscribeUpdateTransaction> for TransactionPretty {
    fn from(SubscribeUpdateTransaction { transaction, slot }: SubscribeUpdateTransaction) -> Self {
        let tx = transaction.expect("should be defined");
        Self {
            slot,
            signature: Signature::try_from(tx.signature.as_slice()).expect("valid signature"),
            is_vote: tx.is_vote,
            tx: yellowstone_grpc_proto::convert_from::create_tx_with_meta(tx)
                .expect("valid tx with meta"),
        }
    }
}

pub struct YellowstoneGrpc {
    endpoint: String,
}

impl YellowstoneGrpc {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    pub fn subscribe_transaction(
        &self,
        account_include: Vec<String>,
        account_exclude: Vec<String>,
        account_required: Vec<String>,
    ) -> TransactionsFilterMap {
        let mut transactions: TransactionsFilterMap = HashMap::new();

        transactions.insert(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                signature: None,
                account_include,
                account_exclude,
                account_required,
            },
        );

        transactions
    }

    pub async fn connect(
        &self,
        transactions: TransactionsFilterMap,
    ) -> Result<
        GeyserGrpcClientResult<(
            impl Sink<SubscribeRequest, Error = mpsc::SendError>,
            impl Stream<Item = Result<SubscribeUpdate, Status>>,
        )>,
        AppError,
    > {
        if CryptoProvider::get_default().is_none() {
            default_provider()
                .install_default()
                .map_err(|e| anyhow::anyhow!("Failed to install crypto provider: {:?}", e))?;
        }

        let mut client = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .connect()
            .await?;

        let subscribe_request = SubscribeRequest {
            transactions,
            commitment: Some(CommitmentLevel::Processed.into()),
            ..Default::default()
        };

        Ok(client.subscribe_with_request(Some(subscribe_request)).await)
    }
}
