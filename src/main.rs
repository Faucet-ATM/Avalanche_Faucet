use axum::{
    routing::post,
    http::StatusCode,
    response::IntoResponse,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use web3::transports::Http;
use web3::types::{Address, U256, TransactionRequest};
use web3::Web3;
use tokio::main;

///
/// Avalanche Fuji测试网: https://api.avax-test.network/ext/bc/C/rpc
#[main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 构建路由
    let app = Router::new()
        .route("/claim", post(claim));

    // 初始化监听器
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn claim(Json(param): Json<Claim>) -> impl IntoResponse {
    let transport = Http::new(&param.rpc).unwrap();
    let web3 = Web3::new(transport);
    
    let address: Address = param.address.parse().unwrap();
    let amount = U256::from(param.amount);

    let tx_object = TransactionRequest {
        from: param.from.parse().unwrap(), // 从请求参数中获取发送地址
        to: Some(address),
        gas: Some(21000.into()), // 设置 gas limit
        gas_price: Some(web3.eth().gas_price().await.unwrap()), // 设置 gas price
        value: Some(amount),
        data: None,
        nonce: None,
        condition: None,
    };

    match web3.eth().send_transaction(tx_object).await {
        Ok(tx_hash) => {
            (StatusCode::OK, Json(ClaimRes { hash: format!("{:?}", tx_hash) }))
        }
        Err(err) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ClaimRes { hash: format!("Error: {:?}", err) }))
        }
    }
}

///
/// 参数结构体
#[derive(Deserialize)]
struct Claim {
    rpc: String,
    from: String, // 添加发送地址字段
    address: String,
    amount: u64
}

///
/// 响应结构体
#[derive(Serialize)]
struct ClaimRes {
    hash: String
}
