# Simplified Design - 3 Files

Let me break down the cleanest architecture:

---

## File Structure

```
seed-recovery/
├── Cargo.toml
├── generator.rs          # File 1: Generate valid seeds
├── finder_cpu.rs         # File 2: CPU-based matching
└── finder_gpu.rs         # File 3: GPU-based matching (OpenCL)
```

---

## Architecture

### **File 1: `generator.rs`**
```
Input: config.json
    ↓
Load constraints (12 positions × word options)
    ↓
Generate combinations (first 11 words)
    ↓
For each combination:
    - Calculate valid 12th words (BIP39 checksum)
    - Encode to 17-byte hex
    - Write to binary file
    ↓
Output: seeds/batch_0.bin, seeds/batch_1.bin, ...
        checkpoint.json
```

### **File 2: `finder_cpu.rs`**
```
Input: seeds/ folder + target_address + derivation_path
    ↓
Read binary files (mmap for speed)
    ↓
Multi-threaded workers:
    - Decode hex → word indices
    - Indices → mnemonic string
    - Mnemonic → PBKDF2 → seed (64 bytes)
    - Seed → HD derive path → private key
    - Private key → Ethereum address
    - Compare with target
    ↓
Output: FOUND.txt (if match) or "Not found"
```

### **File 3: `finder_gpu.rs`**
```
Input: seeds/ folder + target_address + derivation_path
    ↓
Load ALL seeds into GPU memory
    ↓
GPU kernels (parallel):
    - Decode hex → indices
    - PBKDF2 on GPU (massive parallelism)
    - HD derivation on GPU
    - Address generation
    - Compare in parallel
    ↓
Output: FOUND.txt (if match)
```

---

## Config Format

### `config.json` (for generator)
```json
{
  "positions": [
    ["abandon", "ability", "able"],
    ["abandon"],
    ["abandon", "ability"],
    ["abandon"],
    ["abandon"],
    ["abandon"],
    ["abandon"],
    ["abandon"],
    ["abandon"],
    ["abandon"],
    ["abandon", "ability"],
    ["abandon", "ability", "able", "about"]
  ],
  "output_dir": "./seeds",
  "max_file_size_gb": 5,
  "checkpoint_interval": 1000000
}
```

### `finder_config.json` (for finder)
```json
{
  "target_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "derivation_path": "m/44'/60'/0'/0/2",
  "seeds_dir": "./seeds",
  "mode": "cpu",
  "threads": 0
}
```

---

## CLI Usage

```bash
# Build
cargo build --release

# Step 1: Generate valid seeds
./target/release/generator config.json

# Step 2a: Find with CPU
./target/release/finder_cpu finder_config.json

# Step 2b: Find with GPU
./target/release/finder_gpu finder_config.json
```

Or unified:
```bash
# Alternative: Single binary with subcommands
cargo build --release

./seed-recovery generate config.json
./seed-recovery find --cpu finder_config.json
./seed-recovery find --gpu finder_config.json
```

---

## Key Implementation Details

### 1. **Hex Encoding (17 bytes)**
```
12 words × 11 bits = 132 bits = 16.5 bytes
Padded to 17 bytes for alignment

Word indices: [0, 1, 2, ..., 11]  (each 0-2047, needs 11 bits)
Packed binary format
```

### 2. **BIP39 Checksum Validation**
```
First 11 words → 121 bits
Calculate SHA-256 of entropy
First 4 bits = checksum
Find which 12th words match this checksum
```

### 3. **Ethereum Address Derivation**
```
Mnemonic 
  → PBKDF2-HMAC-SHA512 (2048 iterations) 
  → 64-byte seed
  → BIP32 master key 
  → Derive m/44'/60'/0'/0/X 
  → Private key
  → secp256k1 public key
  → Keccak256(pubkey)[12:] 
  → Ethereum address (0x...)
```

### 4. **GPU Optimization**
```
PBKDF2 is the bottleneck (2048 iterations)
GPU can process 1000s in parallel
Expected: 100-500x speedup for large datasets
```

---

## Dependencies

### Cargo.toml
```toml
[package]
name = "seed-recovery"
version = "1.0.0"
edition = "2021"

[[bin]]
name = "generator"
path = "generator.rs"

[[bin]]
name = "finder_cpu"
path = "finder_cpu.rs"

[[bin]]
name = "finder_gpu"
path = "finder_gpu.rs"

[dependencies]
# Shared
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"

# Generator
sha2 = "0.10"

# CPU Finder
bip39 = "2.0"
tiny-hderive = "0.3"
secp256k1 = "0.28"
tiny-keccak = { version = "2.0", features = ["keccak"] }
rayon = "1.8"
memmap2 = "0.9"

# GPU Finder
ocl = "0.19"  # OpenCL for GPU

# CLI
clap = { version = "4.4", features = ["derive"] }
indicatif = "0.17"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

---

## Data Flow

```
User Input:
  config.json → [generator] → seeds/batch_*.bin
                                      ↓
                           finder_config.json
                                      ↓
                    ┌─────────────────┴─────────────────┐
                    ↓                                   ↓
             [finder_cpu]                        [finder_gpu]
            (multi-threaded)                    (OpenCL parallel)
                    ↓                                   ↓
                FOUND.txt or "Not found"
```

---

## Folder Structure After Running

```
seed-recovery/
├── config.json                    # Your input
├── finder_config.json             # Finder settings
├── seeds/                         # Generated by generator
│   ├── batch_0.bin               # Binary seeds
│   ├── batch_1.bin
│   └── checkpoint.json           # Resume state
├── FOUND.txt                     # Output if seed found
└── target/release/
    ├── generator
    ├── finder_cpu
    └── finder_gpu
```

---

## Questions Before I Code

### 1. **Do you want 3 separate binaries OR 1 binary with subcommands?**

**Option A: 3 binaries** (what I described)
```bash
./generator config.json
./finder_cpu finder_config.json
./finder_gpu finder_config.json
```

**Option B: 1 binary with subcommands** (cleaner)
```bash
./seed-recovery generate config.json
./seed-recovery find --mode cpu finder_config.json
./seed-recovery find --mode gpu finder_config.json
```

### 2. **GPU Support: Do you have NVIDIA or AMD GPU?**
- **NVIDIA**: Use CUDA (faster but NVIDIA-only)
- **AMD/NVIDIA/Intel**: Use OpenCL (works everywhere but slightly slower)
- **Both**: I'll use OpenCL (more compatible)

### 3. **Checkpoint behavior:**
- Auto-resume on restart? (Yes/No)
- Save checkpoint every X seeds? (I suggest 100K-1M)

### 4. **Output detail level:**
- Just progress bar? 
- Or detailed stats (seeds/sec, ETA, memory usage)?

---

## My Recommendation

**Go with Option B (1 binary, subcommands)** because:
- Cleaner user experience
- Shared code between CPU/GPU (less duplication)
- Easier to add future features

Let me know your preferences and I'll code all 3 files!