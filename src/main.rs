use dotenv::dotenv;
use futures_util::{StreamExt, SinkExt}; // Import SinkExt for `send`
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair};
use std::env;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message::Text};

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    // Load environment variables
    let payer_key = env::var("PAYER").expect("PAYER must be set in .env file");
    let payer = Arc::new(Keypair::from_base58_string(&payer_key));

    let rpc_https_url = env::var("RPC_HTTPS_URL").expect("RPC_HTTPS_URL must be set in .env file");
    let wss_https_url = env::var("WSS_HTTPS_URL").expect("WSS_HTTPS_URL must be set in .env file");
    let tracked_wallet = env::var("TRACKED_WALLET").expect("TRACKED_WALLET must be set in .env file");

    let client = Arc::new(RpcClient::new(rpc_https_url));

    // WebSocket listener for wallet transactions
    loop {
        match connect_async(wss_https_url.clone()).await {
            Ok((mut stream, _)) => {
                println!("WebSocket connection established.");

                // Send the subscription request for the tracked wallet
                if let Err(e) = send_request(&mut stream, &tracked_wallet).await {
                    eprintln!("Failed to send subscription request: {}", e);
                    continue;
                }

                while let Some(message) = stream.next().await {
                    match message {
                        Ok(Text(text)) => {
                            match serde_json::from_str::<Value>(&text) {
                                Ok(parsed) => {
                                    if let Some(logs) = parsed["params"]["result"]["value"]["logs"].as_array() {
                                        for log in logs {
                                            if let Some(log_str) = log.as_str() {
                                                println!("Detected transaction log: {}", log_str);

                                                // Trigger purchase
                                                if let Err(err) = purchase_and_sell(client.clone(), payer.clone()).await {
                                                    eprintln!("Error during purchase/sell: {}", err);
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => eprintln!("Failed to parse WebSocket message: {}", e),
                            }
                        }
                        Ok(_) => println!("Received non-text WebSocket message."),
                        Err(e) => eprintln!("WebSocket error: {}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("WebSocket connection error: {}", e);
                println!("Reconnecting in 5 seconds...");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn send_request(
    stream: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    wallet: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let subscription_request = format!(
        r#"{{
            "jsonrpc":"2.0",
            "id":1,
            "method":"logsSubscribe",
            "params":[
                {{"mentions":["{}"]}}
            ]
        }}"#,
        wallet
    );

    stream.send(tokio_tungstenite::tungstenite::Message::Text(subscription_request)).await?;
    println!("Subscribed to wallet: {}", wallet);
    Ok(())
}

async fn purchase_and_sell(client: Arc<RpcClient>, payer: Arc<Keypair>) -> Result<(), Box<dyn std::error::Error>> {
    // Simulated purchase transaction
    println!("Executing purchase transaction...");
    let purchase_result = execute_transaction(client.clone(), payer.clone()).await;
    match purchase_result {
        Ok(signature) => println!("Purchase successful: {}", signature),
        Err(err) => {
            eprintln!("Purchase failed: {}", err);
            return Err(err.into());
        }
    }

    // Wait 4-5 seconds
    sleep(Duration::from_secs(4)).await;

    // Simulated sell transaction
    println!("Executing sell transaction...");
    let sell_result = execute_transaction(client, payer).await;
    match sell_result {
        Ok(signature) => println!("Sell successful: {}", signature),
        Err(err) => {
            eprintln!("Sell failed: {}", err);
            return Err(err.into());
        }
    }

    Ok(())
}

async fn execute_transaction(_client: Arc<RpcClient>, _payer: Arc<Keypair>) -> Result<String, Box<dyn std::error::Error>> {
    println!("Transaction executed (mock). Returning mock signature.");
    Ok("mock_signature".to_string())
}
