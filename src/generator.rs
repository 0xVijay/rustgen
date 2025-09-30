use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug, Deserialize)]
struct Config {
    positions: Vec<Vec<String>>,
    output_dir: String,
    max_file_size_gb: u64,
    checkpoint_interval: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Checkpoint {
    current_combination: Vec<u16>,
    file_count: u32,
    total_processed: u64,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <config.json>", args[0]);
        std::process::exit(1);
    }

    let config_path = &args[1];
    let config: Config = serde_json::from_str(&fs::read_to_string(config_path)?)?;
    
    // Load BIP39 wordlist
    let wordlist = load_bip39_wordlist()?;
    
    // Validate all words in config
    validate_words(&config.positions, &wordlist)?;
    
    // Create output directory
    fs::create_dir_all(&config.output_dir)?;
    
    // Load or create checkpoint
    let checkpoint_path = format!("{}/checkpoint.json", config.output_dir);
    let mut checkpoint = load_checkpoint(&checkpoint_path)?;
    
    // Calculate total combinations
    let total_combinations = calculate_total_combinations(&config.positions);
    println!("Total combinations to generate: {}", total_combinations);
    
    // Create progress bar
    let pb = ProgressBar::new(total_combinations);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>10}/{len:10} ({percent:>3}%) {msg}")
        .unwrap()
        .progress_chars("#>-"));
    
    // Generate seeds
    generate_seeds(&config, &wordlist, &mut checkpoint, &pb)?;
    
    pb.finish_with_message("Generation complete!");
    Ok(())
}

fn load_bip39_wordlist() -> Result<Vec<String>> {
    // Try to load from data directory first, then fallback to embedded
    let wordlist_path = "data/bip39-english.txt";
    if Path::new(wordlist_path).exists() {
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

fn validate_words(positions: &[Vec<String>], wordlist: &[String]) -> Result<()> {
    for (i, position) in positions.iter().enumerate() {
        for word in position {
            if !wordlist.contains(word) {
                return Err(anyhow::anyhow!("Invalid word '{}' at position {}", word, i));
            }
        }
    }
    Ok(())
}

fn calculate_total_combinations(positions: &[Vec<String>]) -> u64 {
    positions.iter().map(|pos| pos.len() as u64).product()
}

fn load_checkpoint(checkpoint_path: &str) -> Result<Checkpoint> {
    if Path::new(checkpoint_path).exists() {
        let content = fs::read_to_string(checkpoint_path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(Checkpoint {
            current_combination: vec![0; 12],
            file_count: 0,
            total_processed: 0,
        })
    }
}

fn save_checkpoint(checkpoint: &Checkpoint, checkpoint_path: &str) -> Result<()> {
    let content = serde_json::to_string_pretty(checkpoint)?;
    fs::write(checkpoint_path, content)?;
    Ok(())
}

fn generate_seeds(
    config: &Config,
    wordlist: &[String],
    checkpoint: &mut Checkpoint,
    pb: &ProgressBar,
) -> Result<()> {
    let max_file_size_bytes = config.max_file_size_gb * 1024 * 1024 * 1024;
    let seeds_per_file = max_file_size_bytes / 17; // 17 bytes per seed
    
    let mut current_file = Vec::new();
    let mut file_count = checkpoint.file_count;
    let mut total_processed = checkpoint.total_processed;
    
    // Convert word indices to combination indices
    let mut combination = checkpoint.current_combination.clone();
    let mut indices = vec![0; 12];
    
    // Calculate starting position
    for i in 0..12 {
        indices[i] = combination[i] as usize;
    }
    
    loop {
        // Generate current combination
        let words: Vec<String> = config.positions
            .iter()
            .enumerate()
            .map(|(i, pos)| pos[indices[i]].clone())
            .collect();
        
        // Validate BIP39 checksum
        if is_valid_bip39(&words, wordlist) {
            // Encode to 17-byte binary format
            let seed_bytes = encode_seed(&words, wordlist);
            current_file.extend_from_slice(&seed_bytes);
        }
        
        total_processed += 1;
        pb.set_position(total_processed);
        pb.set_message(format!("{} seeds/sec", total_processed / (pb.elapsed().as_secs() + 1)));
        
        // Save checkpoint periodically
        if total_processed % config.checkpoint_interval == 0 {
            checkpoint.current_combination = combination.clone();
            checkpoint.file_count = file_count;
            checkpoint.total_processed = total_processed;
            save_checkpoint(checkpoint, &format!("{}/checkpoint.json", config.output_dir))?;
        }
        
        // Write file when full
        if current_file.len() >= seeds_per_file as usize * 17 {
            let filename = format!("{}/batch_{}.bin", config.output_dir, file_count);
            fs::write(&filename, &current_file)?;
            current_file.clear();
            file_count += 1;
        }
        
        // Move to next combination
        if !increment_combination(&mut indices, &config.positions) {
            break;
        }
        
        // Update combination for checkpoint
        for i in 0..12 {
            combination[i] = indices[i] as u16;
        }
    }
    
    // Write remaining seeds
    if !current_file.is_empty() {
        let filename = format!("{}/batch_{}.bin", config.output_dir, file_count);
        fs::write(&filename, &current_file)?;
    }
    
    Ok(())
}

fn is_valid_bip39(words: &[String], _wordlist: &[String]) -> bool {
    if words.len() != 12 {
        return false;
    }
    
    // Join words into mnemonic phrase
    let phrase = words.join(" ");
    
    // Use bip39 crate to validate
    use bip39::{Mnemonic, Language};
    Mnemonic::parse_in(Language::English, &phrase).is_ok()
}


fn encode_seed(words: &[String], wordlist: &[String]) -> [u8; 17] {
    let mut indices = Vec::new();
    for word in words {
        let idx = wordlist.iter().position(|w| w == word).unwrap() as u16;
        indices.push(idx);
    }
    
    let mut result = [0u8; 17];
    let mut bit_pos = 0;
    
    for &idx in &indices {
        for bit in 0..11 {
            let byte_pos = bit_pos / 8;
            let bit_offset = 7 - (bit_pos % 8);
            if (idx >> (10 - bit)) & 1 == 1 {
                result[byte_pos] |= 1 << bit_offset;
            }
            bit_pos += 1;
        }
    }
    
    result
}

fn increment_combination(indices: &mut [usize], positions: &[Vec<String>]) -> bool {
    for i in (0..indices.len()).rev() {
        indices[i] += 1;
        if indices[i] < positions[i].len() {
            return true;
        }
        indices[i] = 0;
    }
    false
}
