use axum::{
    extract::Json, http::StatusCode, response::IntoResponse, routing::get, routing::post, Router,
};
use dotenv::dotenv;
use ethers::prelude::*;
use rust_decimal::prelude::{FromStr, ToPrimitive};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::result::Result as StdResult;

use thiserror::Error;
#[derive(Debug, Serialize, Deserialize)]
struct TransferRes {
    success: bool,
    tx_id: String,
    explorer_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferPost {
    address: String,
    network: String,
    amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferErrorRes {
    success: bool,
    message: String,
}

#[derive(Debug, Error)]
enum TransferError {
    #[error("Network connection error: {0}")]
    NetworkError(String),
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),
    #[error("Failed to get asset balance: {0}")]
    GetBalanceError(String),
    #[error("Invalid amount format: {0}")]
    InvalidAmountFormat(String),
    #[error("Invalid receiver address: {0}")]
    InvalidReceiverAddress(String),
    #[error("Transaction failed: {0}")]
    TransactionError(String),
}

impl IntoResponse for TransferError {
    fn into_response(self) -> axum::response::Response {
        let message = self.to_string();
        let error_res = TransferErrorRes {
            success: false,
            message,
        };
        let json = axum::Json(error_res);
        (StatusCode::INTERNAL_SERVER_ERROR, json).into_response()
    }
}

async fn transfer(data: Json<TransferPost>) -> StdResult<Json<TransferRes>, TransferError> {
    // Load the private key from the environment
    let key = env::var("KEY").expect("KEY 未设置");

    // Setup the provider
    let provider = Provider::<Http>::try_from(&data.network)
        .map_err(|err| TransferError::NetworkError(err.to_string()))?;

    // Setup the wallet
    let wallet = LocalWallet::from_str(&key)
        .map_err(|err| TransferError::InvalidPrivateKey(err.to_string()))?;
    let wallet = wallet.with_chain_id(1u64); // You might want to adjust the chain ID

    // Get the balance
    let _balance = provider
        .get_balance(wallet.address(), None)
        .await
        .map_err(|err| TransferError::GetBalanceError(err.to_string()))?;

    // Parse the amount as Decimal
    let amount_str = &data.amount;
    let precision: u32 = 18; // or any other precision value based on the token

    let amount = Decimal::from_str(amount_str)
        .map_err(|_| TransferError::InvalidAmountFormat("Failed to parse amount".to_string()))?;

    let multiplier = Decimal::new(10u64.pow(precision).try_into().unwrap(), 0);
    let result = amount * multiplier;

    let smallest_unit_int = result.to_u64().ok_or_else(|| {
        TransferError::InvalidAmountFormat("Failed to convert to integer".to_string())
    })?;
    println!("Amount in smallest unit: {}", smallest_unit_int);

    // Parse the receiver address
    let receiver = Address::from_str(&data.address)
        .map_err(|err| TransferError::InvalidReceiverAddress(err.to_string()))?;

    // Create and sign the transaction
    let tx = TransactionRequest::new()
        .to(receiver)
        .value(U256::from(smallest_unit_int))
        .from(wallet.address());

    let pending_tx = provider
        .send_transaction(tx, None)
        .await
        .map_err(|err| TransferError::TransactionError(err.to_string()))?;

    let tx_hash = pending_tx.tx_hash();

    // Wait for the transaction to be mined
    let _receipt = pending_tx
        .await
        .map_err(|err| TransferError::TransactionError(err.to_string()))?;

    // Respond with the transaction ID
    Ok(Json(TransferRes {
        success: true,
        tx_id: format!("{:?}", tx_hash),
        explorer_url: explorer_url(&format!("{:?}", tx_hash)),
    }))
}

fn explorer_url(tx_id: &str) -> String {
    let base_url = "https://snowtrace.io/tx/"; // Adjust to the appropriate Avalanche explorer URL
    format!("{}{}", base_url, tx_id)
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/avalanche/request", post(transfer));

    let port = env::var("PORT").unwrap_or_else(|_| "6007".to_string());
    let port: u16 = port.parse().expect("Invalid port number");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
