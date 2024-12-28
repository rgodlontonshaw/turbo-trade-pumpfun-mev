use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, compute_budget::ComputeBudgetInstruction,
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, transaction::Transaction,
    hash::Hash,
};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

#[derive(Debug)]
struct TokenInfo {
    pump_progress: u8,
    max_holders: u8,
    market_cap: u64,
    dev_hold: f64,
    graduated: bool,
}

/// Filters tokens based on given criteria
fn filter_token(token: &TokenInfo) -> bool {
    token.pump_progress >= 99 &&
    token.max_holders <= 35 &&
    token.market_cap >= 4000 &&
    token.dev_hold <= 1.0 &&
    token.graduated
}

/// Fetch token information (stub implementation for demonstration purposes)
async fn fetch_token_info() -> Vec<TokenInfo> {
    vec![
        TokenInfo {
            pump_progress: 99,
            max_holders: 30,
            market_cap: 5000,
            dev_hold: 0.5,
            graduated: true,
        },
        TokenInfo {
            pump_progress: 90,
            max_holders: 40,
            market_cap: 3000,
            dev_hold: 1.2,
            graduated: false,
        },
    ]
}

async fn fetch_blockhash_with_retry(client: &RpcClient, retries: u32) -> Result<Hash, String> {
    let mut attempts = 0;
    let mut delay = Duration::from_millis(100);

    while attempts < retries {
        match client.get_latest_blockhash_with_commitment(CommitmentConfig::processed()).await {
            Ok(blockhash) => return Ok(blockhash.0),
            Err(_) if attempts < retries - 1 => {
                attempts += 1;
                eprintln!("Retrying to fetch blockhash... Attempt {}/{}", attempts, retries);
                sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
            Err(e) => return Err(format!("Failed to fetch blockhash after {} retries: {:?}", retries, e)),
        }
    }

    Err("Exceeded maximum retries".to_string())
}

pub async fn spammer(
    prices_4_spam: Vec<Instruction>,
    client: &Arc<RpcClient>,
    PAYER: &Arc<Keypair>,
    m_pk: &Pubkey,
    instructions_vec: &Vec<Instruction>,
) {
    let max_retries = 3; // Reduced for free RPC testing
    let mut in_trade = false;
    let base_delay = Duration::from_millis(1000); // Configurable base delay

    // Fetch and filter tokens
    let tokens = fetch_token_info().await;
    let valid_tokens: Vec<_> = tokens.into_iter().filter(filter_token).collect();

    if valid_tokens.is_empty() {
        println!("No tokens passed the filters.");
        return;
    }

    println!("Filtered tokens: {:?}", valid_tokens);

    for (i, price_ix) in prices_4_spam.into_iter().enumerate() {
        if in_trade {
            println!("Already in a trade, stopping further monitoring.");
            break;
        }

        // Fetch blockhash just before transaction creation
        let recent_blockhash = match fetch_blockhash_with_retry(client, max_retries).await {
            Ok(blockhash) => blockhash,
            Err(e) => {
                eprintln!("Failed to fetch blockhash: {}", e);
                continue; // Skip this transaction
            }
        };

        // Prepare transaction
        let mut ix_vec = instructions_vec.clone();
        ix_vec.push(price_ix);

        let tx = Transaction::new_signed_with_payer(
            &ix_vec,
            Some(m_pk),
            &[PAYER],
            recent_blockhash,
        );

        match client.send_transaction(&tx).await {
            Ok(signature) => {
                println!("Transaction succeeded with signature: {}", signature);
                in_trade = true; // Stop monitoring once a trade is initiated
                break;
            }
            Err(e) => {
                match &e.kind {
                    solana_client::client_error::ClientErrorKind::Reqwest(reqwest_err) => {
                        eprintln!("Rate-limited: {:?}", reqwest_err);
                        let delay = base_delay * (i as u32 + 1); // Exponential backoff
                        eprintln!("Retrying after {:?} delay...", delay);
                        sleep(delay).await;
                    }
                    solana_client::client_error::ClientErrorKind::RpcError(rpc_error) => {
                        eprintln!("RPC Error: {:?}", rpc_error);
                    }
                    _ => eprintln!("Transaction failed: {:?}", e),
                }
            }
        }

        // Add a delay to avoid rate-limiting
        sleep(base_delay).await;
    }

    println!("Spammer function exiting. Monitoring halted due to active trade or completion.");
}


/// Generate instructions for fees
pub async fn array_of_fees(spam_amount: u64, spam_price: u64) -> Vec<Instruction> {
    let mut instructions = Vec::new();

    for i in 0..spam_amount {
        let unit_price_ix = ComputeBudgetInstruction::set_compute_unit_price(spam_price + i);
        instructions.push(unit_price_ix);
    }

    instructions
}