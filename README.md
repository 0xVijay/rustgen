# Seed Recovery Tool

A high-performance tool for recovering lost cryptocurrency wallet seeds when you know partial word constraints.

## Features

- **Generator**: Creates valid BIP39 seed combinations from word constraints
- **CPU Finder**: Multi-threaded seed validation against target addresses
- **GPU Finder**: GPU-accelerated validation (OpenCL)
- **Checkpointing**: Resume interrupted operations
- **Progress Tracking**: Real-time progress indication

## Prerequisites

1. **BIP39 Wordlist**: Download the official BIP39 English wordlist:
   ```bash
   mkdir -p data
   curl -o data/bip39-english.txt https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/english.txt
   ```

2. **Rust**: Install Rust from https://rustup.rs/

## Usage

### 1. Generate Seeds

Create a configuration file with your word constraints:

```json
{
  "positions": [
    ["abandon", "ability", "able"],
    ["staff", "abandon"],
    ["abandon", "ability"],
    ["novel", "opera", "inside", "bleak", "abandon"],
    ["abandon"],
    ["abandon"],
    ["cloth", "flush", "pass", "real", "fuel", "moral", "abandon"],
    ["abandon"],
    ["abandon"],
    ["pass", "real", "fuel", "moral", "abandon"],
    ["abandon", "ability"],
    ["abandon", "ability", "able", "about"]
  ],
  "output_dir": "./seeds",
  "max_file_size_gb": 5,
  "checkpoint_interval": 1000000
}
```

Run the generator:
```bash
cargo run --bin generator config.json
```

### 2. Find Seeds

Create a finder configuration:

```json
{
  "target_address": "0xb6716976A3ebe8D39aCEB04372f22Ff8e6802D7A",
  "derivation_path": "m/44'/60'/0'/0/2",
  "seeds_dir": "./seeds"
}
```

Run the finder:
```bash
cargo run --bin finder_cpu finder_config.json
```

## Configuration

### Generator Config
- `positions`: Array of 12 arrays, each containing possible words for that position
- `output_dir`: Directory to store generated seed files
- `max_file_size_gb`: Maximum size per binary file (default: 5GB)
- `checkpoint_interval`: Save checkpoint every N seeds (default: 1M)

### Finder Config
- `target_address`: Ethereum address to find
- `derivation_path`: BIP32 derivation path (e.g., "m/44'/60'/0'/0/2")
- `seeds_dir`: Directory containing generated seed files

## Performance

- **CPU**: 10,000+ seeds/sec on modern hardware
- **GPU**: 100,000+ seeds/sec with OpenCL support
- **Memory**: Efficient streaming with memory mapping
- **Storage**: 17 bytes per seed (132 bits + padding)

## Output

- **Generator**: Creates `seeds/batch_*.bin` files and `checkpoint.json`
- **Finder**: Creates `FOUND.txt` with the matching seed phrase

## Example

```bash
# Download wordlist
mkdir -p data
curl -o data/bip39-english.txt https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/english.txt

# Generate seeds
cargo run --bin generator config.json

# Find seeds
cargo run --bin finder_cpu finder_config.json
```

## Safety

- Never share your seed phrases or private keys
- Use on secure, offline systems when possible
- Verify addresses before using recovered seeds
