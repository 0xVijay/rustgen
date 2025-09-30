---
name: Seed Recovery CPU Finder Implementation
description: Implement CPU-based seed validation that checks generated seeds against target Ethereum address
priority: high
estimatedHours: 20
dependencies: [seed-recovery-generator]
elicit: false
---

## Task Overview
Create the `finder_cpu.rs` module that reads generated seed files and validates them against a target Ethereum address using multi-threaded CPU processing.

## Implementation Requirements

### Core Functionality
- [ ] Read binary seed files using memory mapping
- [ ] Multi-threaded processing (auto-detect CPU cores)
- [ ] Decode 17-byte binary format to word indices
- [ ] Convert indices to mnemonic string
- [ ] Derive Ethereum address using specified path
- [ ] Compare with target address

### Performance Requirements
- [ ] Target: 10,000+ seeds/sec on modern CPU
- [ ] Memory usage: <2GB for 10M seeds
- [ ] Multi-threading: Utilize all available cores
- [ ] Single-line progress: `Scanning: 5,432,100/50,000,000 seeds (10.9%) - 12,345 seeds/sec - ETA: 1h 23m`

### Error Handling
- [ ] Validate Ethereum address format
- [ ] Check derivation path format
- [ ] Handle file I/O errors gracefully
- [ ] Memory allocation error handling
- [ ] Interrupt handling (Ctrl+C) with cleanup

### Output Handling
- [ ] Create FOUND.txt with matching seed when discovered
- [ ] Handle "Not found" case gracefully
- [ ] Clear error messages for common issues

## Technical Implementation

### Dependencies
```toml
bip39 = "2.0"
tiny-hderive = "0.3"
secp256k1 = "0.28"
tiny-keccak = { version = "2.0", features = ["keccak"] }
rayon = "1.8"
memmap2 = "0.9"
indicatif = "0.17"
```

### Key Functions
- `load_config()` - Parse finder configuration
- `read_seed_files()` - Memory map binary files
- `decode_seed()` - Convert 17-byte binary to word indices
- `derive_address()` - Generate Ethereum address from seed
- `scan_seeds()` - Multi-threaded validation loop
- `save_result()` - Write FOUND.txt or "Not found"

### Address Derivation Process
1. Mnemonic → PBKDF2-HMAC-SHA512 (2048 iterations) → 64-byte seed
2. Seed → BIP32 master key
3. Derive path: m/44'/60'/0'/0/X
4. Private key → secp256k1 public key
5. Keccak256(pubkey)[12:] → Ethereum address

## Testing Strategy
- [ ] Unit tests for address derivation
- [ ] Integration test with known seed/address pair
- [ ] Performance test with large seed files
- [ ] Multi-threading test
- [ ] Error handling test

## Success Criteria
- [ ] Validates seeds against target address
- [ ] Achieves target performance (10K+ seeds/sec)
- [ ] Handles errors gracefully
- [ ] Provides clear progress indication
- [ ] Outputs results correctly
