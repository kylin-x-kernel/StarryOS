# axwatchdog

A Non-Maskable Interrupt (NMI) based hard lockup detection watchdog for system monitoring.

## Overview

axwatchdog is a hard lockup detection implementation that uses NMI mechanisms to periodically trigger interrupts and monitor system state. When a hard lockup occurs, the watchdog can trigger appropriate handling mechanisms.

## Usage

### Initialization

```rust
use axwatchdog::nmi::{HARD_LOCKUP_THRESHOLD, init_primary, init_secondary};
// Initialize on primary core
init_primary(HARD_LOCKUP_THRESHOLD)?;
// Initialize on secondary cores (call when each secondary core boots)
init_secondary(HARD_LOCKUP_THRESHOLD)?;
```

## Hardware Requirements

### PMU NMI Source
- ARMv8-A architecture (AArch64)
- PMUv3-compatible processor
- Performance Monitoring Unit support

### SDEI NMI Source (Planned)
- ARM SDEI compatible firmware/hypervisor

## Notes

- Currently supports only AArch64 architecture
- PMU support requires `pmu` feature enabled
- Interrupt priority set to highest (0)
- Requires initialization on each core in multi-core systems

## Development Status

- ✅ PMU NMI Source: Implemented
- 🔄 SDEI NMI Source: In Development
- ✅ Multi-core Support: Implemented