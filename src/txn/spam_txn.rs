use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, compute_budget::ComputeBudgetInstruction,
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, transaction::Transaction,
};
use std::sync::Arc;
use std::time::Instant;
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

pub async fn spammer(
    prices_4_spam: Vec<Instruction>,
    client: &Arc<RpcClient>,
    PAYER: &Arc<Keypair>,
    m_pk: &Pubkey,
    instructions_vec: &Vec<Instruction>,
) {
    let mut handles: Vec<JoinHandle<Option<String>>> = Vec::new();

    // Fetch token info and filter
    let tokens = fetch_token_info().await;
    let valid_tokens: Vec<_> = tokens.into_iter().filter(filter_token).collect();

    if valid_tokens.is_empty() {
        println!("No tokens passed the filters.");
        return;
    }

    println!("Filtered tokens: {:?}", valid_tokens);

    for (i, price_ix) in prices_4_spam.into_iter().enumerate() {
        // Refresh blockhash for each transaction
        let recent_blockhash = match client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await
        {
            Ok(blockhash) => blockhash.0,
            Err(e) => {
                eprintln!("Failed to fetch blockhash: {:?}", e);
                continue; // Skip this transaction
            }
        };

        // Clone resources for the async block
        let mut ix_vec = instructions_vec.clone();
        let client_clone = client.clone();
        let payer_clone = PAYER.clone();

        // Add the price-specific instruction
        ix_vec.push(price_ix);

        // Create transaction
        let tx = Transaction::new_signed_with_payer(
            &ix_vec,
            Some(&m_pk),
            &[&payer_clone],
            recent_blockhash,
        );

        // Spawn a task to send the transaction
        let handle = tokio::spawn(async move {
            match client_clone.send_transaction(&tx).await {
                Ok(signature) => {
                    println!("Transaction succeeded with signature: {}", signature);
                    Some(signature.to_string())
                }
                Err(e) => {
                    eprintln!("Transaction failed: {:?}", e); // Log full error
                    None
                }
            }
        });

        handles.push(handle);

        // Add a small delay to respect rate limits
        sleep(Duration::from_millis(100)).await;

        // Optional: Log progress
        println!("Transaction {} queued", i + 1);
    }

    // Wait for all transactions to finish and collect results
    let mut signatures = Vec::new();
    for handle in handles {
        if let Ok(Some(sig)) = handle.await {
            signatures.push(sig);
        }
    }

    println!("Successful Transactions: {}", signatures.len());
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
