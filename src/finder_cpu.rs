use anyhow::Result;
use serde::Deserialize;
use std::fs;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use memmap2::Mmap;

#[derive(Debug, Deserialize)]
struct FinderConfig {
    target_address: String,
    derivation_path: String,
    seeds_dir: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <finder_config.json>", args[0]);
        std::process::exit(1);
    }

    let config_path = &args[1];
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
    
    // Scan seeds
    let result = scan_seeds(&config, &wordlist, &seed_files, &pb)?;
    
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
    
    for file in seed_files {
        println!("Scanning file: {}", file);
        
        let file = fs::File::open(file)?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        // Process seeds in chunks
        let chunk_size = 17 * 1000; // 1000 seeds per chunk
        let mut offset = 0;
        
        while offset < mmap.len() {
            let end = std::cmp::min(offset + chunk_size, mmap.len());
            let chunk = &mmap[offset..end];
            
            // Process chunk in parallel
            let chunk_result: Option<String> = chunk
                .chunks(17)
                .par_bridge()
                .find_map_any(|seed_bytes| {
                    if seed_bytes.len() == 17 {
                        match decode_and_derive(seed_bytes, wordlist, &config.derivation_path) {
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
                });
            
            if let Some(found_seed) = chunk_result {
                return Ok(Some(found_seed));
            }
            
            // Update progress
            let seeds_processed = (end - offset) / 17;
            pb.inc(seeds_processed as u64);
            
            offset = end;
        }
    }
    
    Ok(None)
}

fn decode_and_derive(seed_bytes: &[u8], wordlist: &[String], derivation_path: &str) -> Result<String> {
    // Decode 17-byte binary to word indices
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
    
    // Convert indices to mnemonic
    let words: Vec<String> = indices.iter()
        .map(|&idx| wordlist[idx].clone())
        .collect();
    
    let mnemonic_phrase = words.join(" ");
    
    // Derive Ethereum address
    derive_ethereum_address(&mnemonic_phrase, derivation_path)
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
    
    let words: Vec<String> = indices.iter()
        .map(|&idx| wordlist[idx].clone())
        .collect();
    
    words.join(" ")
}

fn derive_ethereum_address(mnemonic_phrase: &str, derivation_path: &str) -> Result<String> {
    use bip39::{Mnemonic, Language};
    use tiny_keccak::{Hasher, Keccak};
    use bitcoin::bip32::{ExtendedPrivKey, DerivationPath};
    use bitcoin::secp256k1::{Secp256k1, PublicKey};
    use std::str::FromStr;
    
    // Parse mnemonic
    let mnemonic = Mnemonic::parse_in(Language::English, mnemonic_phrase)?;
    let seed = mnemonic.to_seed("");
    
    // Parse derivation path
    let derivation_path = DerivationPath::from_str(derivation_path)?;
    
    // Create extended private key from seed
    let master_key = ExtendedPrivKey::new_master(bitcoin::Network::Bitcoin, &seed)?;
    
    // Derive the key
    let secp = Secp256k1::new();
    let derived_key = master_key.derive_priv(&secp, &derivation_path)?;
    
    // Get private key
    let private_key = derived_key.private_key;
    
    // Get public key
    let public_key = PublicKey::from_secret_key(&secp, &private_key);
    let public_key_bytes = public_key.serialize_uncompressed();
    
    // Calculate Ethereum address
    let mut hasher = Keccak::v256();
    hasher.update(&public_key_bytes[1..]); // Skip the 0x04 prefix
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    
    let address = format!("0x{}", hex::encode(&hash[12..]));
    Ok(address)
}
