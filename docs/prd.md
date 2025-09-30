# Seed Recovery Tool - Product Requirements Document

## Business Context & Problem Statement

**Problem**: Users who have lost access to their cryptocurrency wallets often remember partial information about their seed phrase - specifically, they know possible word combinations for each of the 12 positions but not the exact sequence. Without the complete seed phrase, they cannot recover their funds.

**Solution**: A high-performance seed recovery tool that generates all valid combinations from known word constraints and validates them against a target Ethereum address using the specified derivation path.

**Value Proposition**: Enables users to recover lost wallets when they have partial seed phrase knowledge, potentially saving thousands of dollars in lost cryptocurrency.

## Success Metrics & KPIs

- **Primary KPI**: Seeds processed per second (target: >10,000 seeds/sec on CPU, >100,000 seeds/sec on GPU)
- **Secondary KPIs**:
  - Time to first valid seed found
  - Memory efficiency (GB per million seeds)
  - Success rate for valid partial seeds
  - User satisfaction (ease of use, clear progress indication)

## User Stories & Acceptance Criteria

### Epic 1: Seed Generation
**As a user with partial seed knowledge, I want to generate all valid seed combinations so that I can recover my wallet.**

**User Story 1.1**: Generate valid seeds from constraints
- **Given** I have a config file with word options for each position
- **When** I run the generator
- **Then** it should create binary files with all valid BIP39 combinations
- **And** it should show progress in a single-line terminal output
- **And** it should create checkpoint files for resumption

**Acceptance Criteria**:
- [ ] Reads JSON config with 12 position arrays of word options
- [ ] Validates BIP39 checksum for each combination
- [ ] Outputs binary files in 5GB chunks
- [ ] Shows real-time progress: `Generating: 1,234,567/50,000,000 seeds (2.5%) - 15,432 seeds/sec`
- [ ] Creates checkpoint.json for resumption
- [ ] Handles interruption gracefully

### Epic 2: Seed Validation (CPU)
**As a user, I want to validate generated seeds against my target address using CPU processing.**

**User Story 2.1**: CPU-based seed validation
- **Given** I have generated seed files and target address
- **When** I run the CPU finder
- **Then** it should validate seeds against the target address
- **And** it should show progress and performance metrics
- **And** it should output results when found

**Acceptance Criteria**:
- [ ] Reads binary seed files using memory mapping
- [ ] Multi-threaded processing (auto-detect CPU cores)
- [ ] Derives Ethereum addresses using specified path
- [ ] Shows progress: `Scanning: 5,432,100/50,000,000 seeds (10.9%) - 12,345 seeds/sec - ETA: 1h 23m`
- [ ] Outputs FOUND.txt with matching seed when discovered
- [ ] Handles "Not found" case gracefully

### Epic 3: Seed Validation (GPU)
**As a user with GPU hardware, I want maximum performance validation using GPU acceleration.**

**User Story 3.1**: GPU-accelerated seed validation
- **Given** I have generated seed files and GPU hardware
- **When** I run the GPU finder
- **Then** it should achieve 10x+ performance over CPU
- **And** it should handle large datasets efficiently

**Acceptance Criteria**:
- [ ] Uses OpenCL for cross-platform GPU support
- [ ] Loads all seeds into GPU memory
- [ ] Parallel PBKDF2 and address derivation
- [ ] Shows progress: `GPU Scanning: 25,432,100/50,000,000 seeds (50.9%) - 156,789 seeds/sec - ETA: 4m 12s`
- [ ] Handles GPU memory limitations gracefully
- [ ] Falls back to CPU if GPU unavailable

## Technical Architecture

### File Structure
```
seed-recovery/
├── Cargo.toml
├── generator.rs          # File 1: Generate valid seeds
├── finder_cpu.rs         # File 2: CPU-based matching
├── finder_gpu.rs         # File 3: GPU-based matching (OpenCL)
├── config.json           # Generator configuration
└── finder_config.json    # Finder configuration
```

### Data Flow
```
User Input (config.json) → Generator → Binary Seeds → Finder (CPU/GPU) → Results
```

### Configuration Format

**config.json** (Generator):
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

**finder_config.json** (Finder):
```json
{
  "target_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "derivation_path": "m/44'/60'/0'/0/2",
  "seeds_dir": "./seeds",
  "mode": "cpu",
  "threads": 0
}
```

## Error Handling & Edge Cases

### Input Validation
- [ ] Validate JSON config format
- [ ] Check word lists against BIP39 dictionary
- [ ] Validate Ethereum address format
- [ ] Verify derivation path format

### Runtime Errors
- [ ] Handle file I/O errors gracefully
- [ ] Manage memory allocation failures
- [ ] GPU initialization failures with fallback
- [ ] Interrupt handling (Ctrl+C) with cleanup

### Output Handling
- [ ] Create output directory if missing
- [ ] Handle disk space exhaustion
- [ ] Atomic file writes to prevent corruption
- [ ] Clear error messages for common issues

## Performance Requirements

### CPU Performance
- Target: 10,000+ seeds/sec on modern CPU
- Memory usage: <2GB for 10M seeds
- Multi-threading: Auto-detect and utilize all cores

### GPU Performance
- Target: 100,000+ seeds/sec on modern GPU
- Memory: Load all seeds into GPU memory
- Fallback: CPU mode if GPU unavailable

### Storage
- Binary format: 17 bytes per seed (132 bits + padding)
- Chunking: 5GB files maximum
- Checkpointing: Every 1M seeds

## Timeline & Milestones

### Phase 1: Core Implementation (Week 1)
- [ ] Day 1-2: Generator implementation
- [ ] Day 3-4: CPU finder implementation  
- [ ] Day 5: GPU finder implementation
- [ ] Day 6-7: Integration testing and optimization

### Phase 2: Polish & Testing (Week 2)
- [ ] Error handling and validation
- [ ] Performance optimization
- [ ] Documentation and examples
- [ ] Final testing and bug fixes

## Dependencies

### Rust Dependencies
```toml
[dependencies]
# Core
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
clap = { version = "4.4", features = ["derive"] }

# Crypto
sha2 = "0.10"
bip39 = "2.0"
tiny-hderive = "0.3"
secp256k1 = "0.28"
tiny-keccak = { version = "2.0", features = ["keccak"] }

# Performance
rayon = "1.8"
memmap2 = "0.9"
ocl = "0.19"  # OpenCL for GPU
indicatif = "0.17"  # Progress bars
```

## CLI Usage

```bash
# Build
cargo build --release

# Generate seeds
./target/release/generator config.json

# Find with CPU
./target/release/finder_cpu finder_config.json

# Find with GPU  
./target/release/finder_gpu finder_config.json
```

## Success Criteria

- [ ] Successfully generates valid seeds from partial constraints
- [ ] Achieves target performance metrics (10K+ CPU, 100K+ GPU seeds/sec)
- [ ] Finds target address when valid seed exists
- [ ] Handles errors gracefully with clear messages
- [ ] Provides clear progress indication in single-line terminal output
- [ ] Works on Windows, macOS, and Linux
- [ ] Memory efficient for large seed sets

## Future Enhancements

- [ ] Support for different derivation paths
- [ ] Multiple address validation
- [ ] Web interface for non-technical users
- [ ] Distributed processing across multiple machines
- [ ] Support for other cryptocurrencies (Bitcoin, etc.)
