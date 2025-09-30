---
name: Seed Recovery GPU Finder Implementation
description: Implement GPU-accelerated seed validation using OpenCL for maximum performance
priority: high
estimatedHours: 24
dependencies: [seed-recovery-generator]
elicit: false
---

## Task Overview
Create the `finder_gpu.rs` module that uses GPU acceleration via OpenCL to validate seeds against target Ethereum address with 10x+ performance over CPU.

## Implementation Requirements

### Core Functionality
- [ ] Load all seeds into GPU memory
- [ ] OpenCL kernel for parallel PBKDF2 processing
- [ ] Parallel address derivation on GPU
- [ ] Parallel address comparison
- [ ] Fallback to CPU if GPU unavailable

### Performance Requirements
- [ ] Target: 100,000+ seeds/sec on modern GPU
- [ ] 10x+ speedup over CPU implementation
- [ ] Handle GPU memory limitations gracefully
- [ ] Single-line progress: `GPU Scanning: 25,432,100/50,000,000 seeds (50.9%) - 156,789 seeds/sec - ETA: 4m 12s`

### Error Handling
- [ ] GPU initialization failure with CPU fallback
- [ ] GPU memory allocation error handling
- [ ] OpenCL context creation errors
- [ ] Kernel compilation error handling
- [ ] Interrupt handling (Ctrl+C) with cleanup

### Output Handling
- [ ] Create FOUND.txt with matching seed when discovered
- [ ] Handle "Not found" case gracefully
- [ ] Clear error messages for GPU issues

## Technical Implementation

### Dependencies
```toml
ocl = "0.19"  # OpenCL for GPU
bip39 = "2.0"
tiny-hderive = "0.3"
secp256k1 = "0.28"
tiny-keccak = { version = "2.0", features = ["keccak"] }
memmap2 = "0.9"
indicatif = "0.17"
```

### Key Functions
- `init_gpu()` - Initialize OpenCL context and devices
- `load_seeds_to_gpu()` - Transfer seeds to GPU memory
- `compile_kernels()` - Compile OpenCL kernels
- `run_gpu_scan()` - Execute parallel validation
- `fallback_to_cpu()` - Switch to CPU if GPU fails
- `save_result()` - Write FOUND.txt or "Not found"

### OpenCL Kernels
- `pbkdf2_kernel.cl` - Parallel PBKDF2-HMAC-SHA512
- `derive_kernel.cl` - Parallel HD key derivation
- `address_kernel.cl` - Parallel Ethereum address generation
- `compare_kernel.cl` - Parallel address comparison

### GPU Memory Management
- Load all seeds into GPU memory at startup
- Process in batches to fit GPU memory
- Handle memory limitations with chunking

## Testing Strategy
- [ ] Unit tests for OpenCL initialization
- [ ] Integration test with known seed/address pair
- [ ] Performance test with large seed files
- [ ] GPU memory limitation test
- [ ] CPU fallback test
- [ ] Error handling test

## Success Criteria
- [ ] Achieves 10x+ performance over CPU
- [ ] Handles GPU errors gracefully
- [ ] Falls back to CPU when needed
- [ ] Provides clear progress indication
- [ ] Outputs results correctly
