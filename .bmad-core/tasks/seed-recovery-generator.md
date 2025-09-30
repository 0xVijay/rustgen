---
name: Seed Recovery Generator Implementation
description: Implement the seed generation module that creates valid BIP39 combinations from partial constraints
priority: high
estimatedHours: 16
dependencies: []
elicit: false
---

## Task Overview
Create the `generator.rs` module that reads word constraints and generates all valid BIP39 seed combinations, outputting them to binary files.

## Implementation Requirements

### Core Functionality
- [ ] Read JSON config with 12 position arrays of word options
- [ ] Validate all words against BIP39 dictionary (2048 words)
- [ ] Generate all valid combinations respecting BIP39 checksum
- [ ] Encode seeds as 17-byte binary format (132 bits + padding)
- [ ] Write to binary files in 5GB chunks
- [ ] Create checkpoint.json for resumption

### Performance Requirements
- [ ] Process 1M+ seeds per second
- [ ] Memory efficient (stream processing)
- [ ] Single-line progress output: `Generating: 1,234,567/50,000,000 seeds (2.5%) - 15,432 seeds/sec`

### Error Handling
- [ ] Validate JSON config format
- [ ] Check word lists against BIP39 dictionary
- [ ] Handle file I/O errors gracefully
- [ ] Interrupt handling (Ctrl+C) with cleanup

### Output Format
- [ ] Binary files: `seeds/batch_0.bin`, `seeds/batch_1.bin`, etc.
- [ ] Checkpoint file: `seeds/checkpoint.json`
- [ ] Progress indication in terminal

## Technical Implementation

### Dependencies
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
sha2 = "0.10"
indicatif = "0.17"
```

### Key Functions
- `load_config()` - Parse JSON configuration
- `validate_words()` - Check against BIP39 dictionary
- `generate_combinations()` - Create all valid seed combinations
- `encode_seed()` - Convert to 17-byte binary format
- `write_batch()` - Write seeds to binary file
- `save_checkpoint()` - Save progress for resumption

### Binary Format
- 12 words × 11 bits = 132 bits
- Padded to 17 bytes (136 bits) for alignment
- Packed binary format for efficient storage

## Testing Strategy
- [ ] Unit tests for BIP39 validation
- [ ] Integration test with sample config
- [ ] Performance test with large word lists
- [ ] Error handling test with invalid inputs
- [ ] Checkpoint resumption test

## Success Criteria
- [ ] Generates valid BIP39 seeds from constraints
- [ ] Achieves target performance (1M+ seeds/sec)
- [ ] Handles errors gracefully
- [ ] Provides clear progress indication
- [ ] Creates resumable checkpoints
