use anyhow::Result;
use serde::Deserialize;
use std::fs;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use memmap2::Mmap;

#[derive(Debug, Deserialize)]
struct FinderConfig {
    target_address: String,
    seeds_dir: String,
}

pub fn run_finder(config_path: &str) -> Result<()> {
    let config: FinderConfig = serde_json::from_str(&fs::read_to_string(config_path)?)?;
    
    // Load BIP39 wordlist
    let wordlist = load_bip39_wordlist()?;
    
    // Find all seed files
    let seed_files = find_seed_files(&config.seeds_dir)?;
    if seed_files.is_empty() {
        eprintln!("No seed files found in {}", config.seeds_dir);
        std::process::exit(1);
    }
    
    println!("Found {} seed files", seed_files.len());
    
    // Calculate total seeds
    let total_seeds = calculate_total_seeds(&seed_files)?;
    println!("Total seeds to scan: {}", total_seeds);
    
    // Create progress bar
    let pb = ProgressBar::new(total_seeds);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>10}/{len:10} ({percent:>3}%) {msg}")
        .unwrap()
        .progress_chars("#>-"));
    
    // Set initial message
    pb.set_message("Starting scan...");
    
    // Start performance monitoring
    let start_time = std::time::Instant::now();
    
    // Scan seeds
    let result = scan_seeds(&config, &wordlist, &seed_files, &pb)?;
    
    // Show final performance stats
    let elapsed = start_time.elapsed();
    let total_seeds = pb.position();
    let seeds_per_sec = if elapsed.as_secs() > 0 {
        total_seeds / elapsed.as_secs()
    } else {
        total_seeds * 1000 / elapsed.as_millis() as u64
    };
    
    println!("Performance: {} seeds in {:.2}s ({}/sec)", 
             total_seeds, 
             elapsed.as_secs_f64(), 
             seeds_per_sec);
    
    pb.finish();
    
    if let Some(found_seed) = result {
        println!("FOUND! Seed: {}", found_seed);
        fs::write("FOUND.txt", &found_seed)?;
    } else {
        println!("Not found");
        fs::write("FOUND.txt", "Not found")?;
    }
    
    Ok(())
}

fn load_bip39_wordlist() -> Result<Vec<String>> {
    // Try to load from data directory first, then fallback to embedded
    let wordlist_path = "data/bip39-english.txt";
    if std::path::Path::new(wordlist_path).exists() {
        let content = fs::read_to_string(wordlist_path)?;
        Ok(content.lines().map(|s| s.to_string()).collect())
    } else {
        // Fallback to embedded wordlist
        load_embedded_wordlist()
    }
}

fn load_embedded_wordlist() -> Result<Vec<String>> {
    // This would contain the BIP39 wordlist as a static array
    // For now, return an error to encourage downloading the wordlist
    Err(anyhow::anyhow!("BIP39 wordlist not found. Please download it to data/bip39-english.txt"))
}

// Get available system memory in bytes (cross-platform)
fn get_available_memory() -> u64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("vm_stat").output() {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                for line in output_str.lines() {
                    if line.starts_with("Pages free:") {
                        if let Some(page_str) = line.split_whitespace().nth(2) {
                            if let Ok(pages) = page_str.parse::<u64>() {
                                return pages * 4096; // Convert pages to bytes
                            }
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("wmic").args(&["OS", "get", "TotalVisibleMemorySize", "/value"]).output() {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                for line in output_str.lines() {
                    if line.starts_with("TotalVisibleMemorySize=") {
                        if let Some(mb_str) = line.split('=').nth(1) {
                            if let Ok(mb) = mb_str.trim().parse::<u64>() {
                                return mb * 1024 * 1024; // Convert MB to bytes
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: assume 8GB if detection fails
    8 * 1024 * 1024 * 1024
}

fn find_seed_files(seeds_dir: &str) -> Result<Vec<String>> {
    let mut files = Vec::new();
    let entries = fs::read_dir(seeds_dir)?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("bin") {
            files.push(path.to_string_lossy().to_string());
        }
    }
    
    files.sort();
    Ok(files)
}

fn calculate_total_seeds(seed_files: &[String]) -> Result<u64> {
    let mut total = 0;
    for file in seed_files {
        let metadata = fs::metadata(file)?;
        total += metadata.len() / 17; // 17 bytes per seed
    }
    Ok(total)
}

fn scan_seeds(
    config: &FinderConfig,
    wordlist: &[String],
    seed_files: &[String],
    pb: &ProgressBar,
) -> Result<Option<String>> {
    let target_address = config.target_address.to_lowercase();
    
    // Get system memory and configure for maximum usage
    let available_memory = get_available_memory();
    let target_memory_usage = (available_memory as f64 * 0.8) as usize; // Use 80% of available memory
    let cpu_count = num_cpus::get();
    
    // Configure thread pool for maximum performance
    let stack_size = if cpu_count >= 16 {
        32 * 1024 * 1024 // 32MB for high-end systems
    } else if cpu_count >= 8 {
        16 * 1024 * 1024  // 16MB for mid-range systems
    } else {
        8 * 1024 * 1024   // 8MB for low-end systems
    };
    
    rayon::ThreadPoolBuilder::new()
        .num_threads(cpu_count)
        .stack_size(stack_size)
        .build_global()
        .unwrap();
    
    println!("Available memory: {:.2} GB", available_memory as f64 / (1024.0 * 1024.0 * 1024.0));
    println!("Target memory usage: {:.2} GB", target_memory_usage as f64 / (1024.0 * 1024.0 * 1024.0));
    println!("Using {} CPU cores", cpu_count);
    
    for file in seed_files {
        println!("Scanning file: {}", file);
        
        let file = fs::File::open(file)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let total_seeds = mmap.len() / 17;
        
        // Calculate optimal chunk size based on available memory
        let chunk_size = std::cmp::min(
            target_memory_usage / (17 * cpu_count), // Divide memory among threads
            total_seeds as usize / cpu_count // At least one chunk per thread
        );
        let chunk_size = std::cmp::max(chunk_size, 1000); // Minimum chunk size
        
        println!("Processing {} seeds in chunks of {} ({} chunks)", 
                total_seeds, chunk_size, (total_seeds as usize + chunk_size - 1) / chunk_size);
        
        // Use atomic counter for thread-safe progress tracking
        use std::sync::atomic::{AtomicUsize, Ordering};
        let processed_atomic = std::sync::Arc::new(AtomicUsize::new(0));
        let processed_atomic_clone = processed_atomic.clone();
        
        // Process file in memory-optimized chunks
        let result: Option<String> = mmap
            .chunks(chunk_size * 17)
            .par_bridge()
            .find_map_any(|chunk| {
                // Process each chunk with maximum parallelism
                chunk
                    .chunks(17)
                    .par_bridge()
                    .find_map_any(|seed_bytes| {
                        if seed_bytes.len() == 17 {
                            // Update progress with adaptive frequency
                            let current = processed_atomic_clone.fetch_add(1, Ordering::Relaxed);
                            let update_frequency = if cpu_count >= 16 {
                                5000 // Update every 5k seeds for high-end systems
                            } else if cpu_count >= 8 {
                                2000  // Update every 2k seeds for mid-range systems
                            } else {
                                1000  // Update every 1k seeds for low-end systems
                            };
                            
                            if current % update_frequency == 0 {
                                pb.set_position(current as u64);
                                let elapsed = pb.elapsed().as_secs_f64();
                                if elapsed > 0.0 {
                                    let seeds_per_sec = (current as f64) / elapsed;
                                    pb.set_message(format!("{:.0} seeds/sec", seeds_per_sec));
                                }
                                pb.tick();
                            }
                            
                            match derive_ethereum_address_optimized_bip32(seed_bytes) {
                                Ok(address) => {
                                    if address.to_lowercase() == target_address {
                                        Some(decode_to_mnemonic(seed_bytes, wordlist))
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => None,
                            }
                        } else {
                            None
                        }
                    })
            });
        
        // Final progress update
        pb.set_position(total_seeds as u64);
        let elapsed = pb.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            let seeds_per_sec = (total_seeds as f64) / elapsed;
            pb.set_message(format!("{:.0} seeds/sec", seeds_per_sec));
        }
        pb.tick();
        
        if let Some(found_seed) = result {
            return Ok(Some(found_seed));
        }
    }
    
    Ok(None)
}

// OPTIMIZED BIP32 with lookup tables for m/44'/60'/0'/0/2
fn derive_ethereum_address_optimized_bip32(seed_bytes: &[u8]) -> Result<String> {
    use bip39::{Mnemonic, Language};
    use tiny_keccak::{Hasher, Keccak};
    use bitcoin::bip32::{ExtendedPrivKey, DerivationPath};
    use bitcoin::secp256k1::{Secp256k1, PublicKey};
    use std::str::FromStr;
    
    // Pre-compute everything once
    static WORDLIST: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    static DERIVATION_PATH: std::sync::OnceLock<DerivationPath> = std::sync::OnceLock::new();
    static SECP: std::sync::OnceLock<Secp256k1<bitcoin::secp256k1::All>> = std::sync::OnceLock::new();
    
    let wordlist = WORDLIST.get_or_init(|| load_bip39_wordlist().unwrap());
    let derivation_path = DERIVATION_PATH.get_or_init(|| DerivationPath::from_str("m/44'/60'/0'/0/2").unwrap());
    let secp = SECP.get_or_init(|| Secp256k1::new());
    
    // Decode mnemonic indices with optimized bit operations
    let mut indices = [0usize; 12];
    let mut bit_pos = 0;
    
    // Unrolled loop for better performance
    for i in 0..12 {
        let mut word_idx = 0u16;
        // Unroll the inner loop for maximum speed
        let byte_pos_0 = bit_pos / 8;
        let bit_offset_0 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_0] >> bit_offset_0) & 1 == 1 { word_idx |= 1 << 10; }
        bit_pos += 1;
        
        let byte_pos_1 = bit_pos / 8;
        let bit_offset_1 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_1] >> bit_offset_1) & 1 == 1 { word_idx |= 1 << 9; }
        bit_pos += 1;
        
        let byte_pos_2 = bit_pos / 8;
        let bit_offset_2 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_2] >> bit_offset_2) & 1 == 1 { word_idx |= 1 << 8; }
        bit_pos += 1;
        
        let byte_pos_3 = bit_pos / 8;
        let bit_offset_3 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_3] >> bit_offset_3) & 1 == 1 { word_idx |= 1 << 7; }
        bit_pos += 1;
        
        let byte_pos_4 = bit_pos / 8;
        let bit_offset_4 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_4] >> bit_offset_4) & 1 == 1 { word_idx |= 1 << 6; }
        bit_pos += 1;
        
        let byte_pos_5 = bit_pos / 8;
        let bit_offset_5 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_5] >> bit_offset_5) & 1 == 1 { word_idx |= 1 << 5; }
        bit_pos += 1;
        
        let byte_pos_6 = bit_pos / 8;
        let bit_offset_6 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_6] >> bit_offset_6) & 1 == 1 { word_idx |= 1 << 4; }
        bit_pos += 1;
        
        let byte_pos_7 = bit_pos / 8;
        let bit_offset_7 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_7] >> bit_offset_7) & 1 == 1 { word_idx |= 1 << 3; }
        bit_pos += 1;
        
        let byte_pos_8 = bit_pos / 8;
        let bit_offset_8 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_8] >> bit_offset_8) & 1 == 1 { word_idx |= 1 << 2; }
        bit_pos += 1;
        
        let byte_pos_9 = bit_pos / 8;
        let bit_offset_9 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_9] >> bit_offset_9) & 1 == 1 { word_idx |= 1 << 1; }
        bit_pos += 1;
        
        let byte_pos_10 = bit_pos / 8;
        let bit_offset_10 = 7 - (bit_pos % 8);
        if (seed_bytes[byte_pos_10] >> bit_offset_10) & 1 == 1 { word_idx |= 1 << 0; }
        bit_pos += 1;
        
        indices[i] = word_idx as usize;
    }
    
    // Convert to mnemonic with minimal allocations
    let mut mnemonic_phrase = String::with_capacity(200);
    for (i, &idx) in indices.iter().enumerate() {
        if i > 0 {
            mnemonic_phrase.push(' ');
        }
        mnemonic_phrase.push_str(&wordlist[idx]);
    }
    
    // Parse mnemonic and get seed
    let mnemonic = Mnemonic::parse_in(Language::English, &mnemonic_phrase)?;
    let seed = mnemonic.to_seed("");
    
    // Use pre-computed derivation path
    let master_key = ExtendedPrivKey::new_master(bitcoin::Network::Bitcoin, &seed)?;
    let derived_key = master_key.derive_priv(secp, derivation_path)?;
    let private_key = derived_key.private_key;
    
    // Get public key
    let public_key = PublicKey::from_secret_key(secp, &private_key);
    let public_key_bytes = public_key.serialize_uncompressed();
    
    // Calculate Ethereum address with optimized hashing
    let mut hasher = Keccak::v256();
    hasher.update(&public_key_bytes[1..]); // Skip the 0x04 prefix
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    
    // Format address without additional allocations
    let address = format!("0x{}", hex::encode(&hash[12..]));
    Ok(address)
}

fn decode_to_mnemonic(seed_bytes: &[u8], wordlist: &[String]) -> String {
    let mut indices = Vec::new();
    let mut bit_pos = 0;
    
    for _ in 0..12 {
        let mut word_idx = 0u16;
        for bit in 0..11 {
            let byte_pos = bit_pos / 8;
            let bit_offset = 7 - (bit_pos % 8);
            if (seed_bytes[byte_pos] >> bit_offset) & 1 == 1 {
                word_idx |= 1 << (10 - bit);
            }
            bit_pos += 1;
        }
        indices.push(word_idx as usize);
    }
    
    let mut mnemonic = String::new();
    for (i, &idx) in indices.iter().enumerate() {
        if i > 0 {
            mnemonic.push(' ');
        }
        mnemonic.push_str(&wordlist[idx]);
    }
    
    mnemonic
}
