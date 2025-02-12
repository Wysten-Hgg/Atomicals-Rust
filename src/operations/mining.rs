use crate::types::{AtomicalsTx, mint::BitworkInfo};
use crate::errors::{Error, Result};
use bitcoin::{Transaction, TxIn, TxOut, Script};
use bitcoin::locktime::absolute::LockTime;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_TRANSACTION_SIZE: usize = 100_000; // 100KB
const NONCE_RANGE_PER_THREAD: u32 = 1_000_000;

#[derive(Debug, Clone)]
pub struct MiningOptions {
    pub max_tries: u64,
    pub threads: usize,
    pub timeout_secs: u64,
    pub target_tx_size: usize,
}

impl Default for MiningOptions {
    fn default() -> Self {
        Self {
            max_tries: 1_000_000,
            threads: num_cpus::get(),
            timeout_secs: 3600, // 1 hour
            target_tx_size: 1000, // Target transaction size in bytes
        }
    }
}

#[derive(Debug)]
pub struct MiningResult {
    pub transaction: Transaction,
    pub nonce: u32,
    pub attempts: u64,
    pub duration: Duration,
    pub hash_rate: f64, // Hashes per second
}

pub fn mine_transaction(
    mut tx: Transaction,
    bitwork: BitworkInfo,
    options: MiningOptions,
) -> Result<MiningResult> {
    // Validate transaction size
    let tx_size = bitcoin::consensus::encode::serialize(&tx).len();
    if tx_size > MAX_TRANSACTION_SIZE {
        return Err(Error::MiningError(
            format!("Transaction size {} exceeds maximum {}", tx_size, MAX_TRANSACTION_SIZE)
        ));
    }

    let start_time = Instant::now();
    let found = Arc::new(AtomicBool::new(false));
    let attempts = Arc::new(AtomicU64::new(0));

    // Create thread pool with specified number of threads
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(options.threads)
        .build()
        .map_err(|e| Error::MiningError(format!("Failed to create thread pool: {}", e)))?;

    // Split the nonce range into chunks for each thread
    let chunks: Vec<_> = (0..options.threads)
        .map(|i| {
            let start = (i as u32) * NONCE_RANGE_PER_THREAD;
            let end = start + NONCE_RANGE_PER_THREAD;
            (start, end)
        })
        .collect();

    // Mining result channel
    let (tx_sender, rx_receiver) = std::sync::mpsc::channel();

    // Spawn mining threads
    pool.scope(|s| {
        for (start, end) in chunks {
            let tx_clone = tx.clone();
            let found = found.clone();
            let attempts = attempts.clone();
            let tx_sender = tx_sender.clone();
            let bitwork = bitwork.clone();

            s.spawn(move |_| {
                let mut rng = thread_rng();
                let mut nonce_counter = start;

                while !found.load(Ordering::Relaxed) 
                    && nonce_counter < end 
                    && attempts.load(Ordering::Relaxed) < options.max_tries {
                    
                    // Check timeout
                    if start_time.elapsed().as_secs() > options.timeout_secs {
                        found.store(true, Ordering::Relaxed);
                        break;
                    }

                    // Generate random nonce within our range
                    let nonce = rng.gen_range(nonce_counter..end);
                    nonce_counter = nonce + 1;
                    
                    // Update nonce in transaction
                    let mut tx_attempt = tx_clone.clone();
                    if let Some(input) = tx_attempt.input.get_mut(0) {
                        input.sequence = nonce;
                    }

                    // Calculate txid
                    let txid = tx_attempt.txid().to_string();
                    attempts.fetch_add(1, Ordering::Relaxed);

                    // Check if txid matches bitwork requirements
                    if bitwork.matches(&txid) {
                        found.store(true, Ordering::Relaxed);
                        let duration = start_time.elapsed();
                        let total_attempts = attempts.load(Ordering::Relaxed);
                        let hash_rate = total_attempts as f64 / duration.as_secs_f64();

                        let result = MiningResult {
                            transaction: tx_attempt,
                            nonce,
                            attempts: total_attempts,
                            duration,
                            hash_rate,
                        };

                        tx_sender.send(Some(result)).ok();
                        break;
                    }
                }
            });
        }
    });

    // Drop the original sender to close the channel
    drop(tx_sender);

    // Get the result
    match rx_receiver.recv() {
        Ok(Some(result)) => Ok(result),
        Ok(None) => Err(Error::MiningError("Mining failed: no result found".into())),
        Err(_) => Err(Error::MiningError("Mining failed: channel closed".into())),
    }
}

// Helper function to create a mining transaction template
pub fn create_mining_tx(
    inputs: Vec<TxIn>,
    outputs: Vec<TxOut>,
    data: Vec<u8>,
) -> Result<Transaction> {
    // Validate inputs and outputs
    if inputs.is_empty() {
        return Err(Error::MiningError("No inputs provided".into()));
    }
    if outputs.is_empty() {
        return Err(Error::MiningError("No outputs provided".into()));
    }

    let mut tx = Transaction {
        version: 2,
        lock_time: 0,
        input: inputs,
        output: outputs,
    };

    // Add OP_RETURN output with mining data if provided
    if !data.is_empty() {
        let script = bitcoin::Script::new_op_return(&data);
        tx.output.push(TxOut {
            value: 0,
            script_pubkey: script,
        });
    }

    // Validate final transaction size
    let tx_size = bitcoin::consensus::encode::serialize(&tx).len();
    if tx_size > MAX_TRANSACTION_SIZE {
        return Err(Error::MiningError(
            format!("Transaction size {} exceeds maximum {}", tx_size, MAX_TRANSACTION_SIZE)
        ));
    }

    Ok(tx)
}

// Helper function to estimate mining time based on bitwork difficulty
pub fn estimate_mining_time(bitwork: &BitworkInfo) -> Duration {
    let difficulty = bitwork.difficulty as f64;
    let hashes_needed = 16f64.powi(difficulty as i32);
    let hashes_per_second = 100_000f64; // Estimated hashes per second
    let seconds = hashes_needed / hashes_per_second;
    Duration::from_secs_f64(seconds)
}
