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

pub fn run_generator(config_path: &str) -> Result<()> {
    let config: Config = serde_json::from_str(&fs::read_to_string(config_path)?)?;
    
    // Load BIP39 wordlist
    let wordlist = load_bip39_wordlist()?;
    
    // Validate all words in config
    validate_words(&config.positions, &wordlist)?;
    
    // Create output directory
    fs::create_dir_all(&config.output_dir)?;
    
    // Load or create checkpoint
    let checkpoint_path = format!("{}/checkpoint.json", config.output_dir);
    let mut checkpoint = load_checkpoint(&checkpoint_path, &config.positions)?;
    
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

fn load_checkpoint(checkpoint_path: &str, positions: &[Vec<String>]) -> Result<Checkpoint> {
    if Path::new(checkpoint_path).exists() {
        let content = fs::read_to_string(checkpoint_path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(Checkpoint {
            current_combination: vec![0; positions.len()],
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
    
    let max_file_size_bytes = config.max_file_size_gb * 1024 * 1024 * 1024;
    let seeds_per_file = max_file_size_bytes / 17; // 17 bytes per seed
    
    // Use larger buffer for better memory utilization
    let buffer_size = std::cmp::min(
        target_memory_usage / 4, // Use 1/4 of target memory for buffer
        seeds_per_file as usize * 17
    );
    
    let mut current_file = Vec::with_capacity(buffer_size);
    let mut file_count = checkpoint.file_count;
    let mut total_processed = checkpoint.total_processed;
    
    // Convert word indices to combination indices
    let mut combination = checkpoint.current_combination.clone();
    let mut indices = vec![0; config.positions.len()];
    
    // Calculate starting position
    for i in 0..config.positions.len() {
        indices[i] = (combination[i] as usize) % config.positions[i].len();
    }
    
    // Batch processing for better memory usage
    let batch_size = std::cmp::min(10000, seeds_per_file as usize / 10); // Process in batches
    let mut batch_buffer = Vec::with_capacity(batch_size * 17);
    
    loop {
        // Generate batch of combinations
        let mut batch_count = 0;
        while batch_count < batch_size {
            // Generate current combination
            let words: Vec<String> = config.positions
                .iter()
                .enumerate()
                .map(|(i, pos)| {
                    let idx = indices[i] % pos.len(); // Ensure index is within bounds
                    pos[idx].clone()
                })
                .collect();
            
            // Validate BIP39 checksum
            if is_valid_bip39(&words, wordlist) {
                // Encode to 17-byte binary format
                let seed_bytes = encode_seed(&words, wordlist);
                batch_buffer.extend_from_slice(&seed_bytes);
                batch_count += 1;
            }
            
            total_processed += 1;
            
            // Move to next combination
            if !increment_combination(&mut indices, &config.positions) {
                break;
            }
            
            // Update combination for checkpoint
            for i in 0..12 {
                combination[i] = indices[i] as u16;
            }
        }
        
        // Add batch to current file
        current_file.extend_from_slice(&batch_buffer);
        batch_buffer.clear();
        
        // Update progress
        pb.set_position(total_processed);
        let elapsed = pb.elapsed().as_secs();
        if elapsed > 0 {
            let seeds_per_sec = total_processed / elapsed;
            pb.set_message(format!("{} seeds/sec", seeds_per_sec));
        }
        
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
            println!("Written batch_{}.bin ({} bytes)", file_count, current_file.len());
            current_file.clear();
            file_count += 1;
        }
        
        // Check if we've processed all combinations
        if batch_count < batch_size {
            break;
        }
    }
    
    // Write remaining seeds
    if !current_file.is_empty() {
        let filename = format!("{}/batch_{}.bin", config.output_dir, file_count);
        fs::write(&filename, &current_file)?;
        println!("Written final batch_{}.bin ({} bytes)", file_count, current_file.len());
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
