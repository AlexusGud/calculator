# Rust port

This directory contains the first Rust application milestone alongside the unchanged Microsoft Calculator source tree.
The Rust app is an independent Slint crate; it does not yet replace or provide feature parity with the original UWP app.

## Current scope

- Standard, scientific, and programmer calculator modes
- Expression evaluation, history, and memory actions
- Date difference calculations with calendar-date validation
- Unit conversion across length, area, volume, mass, temperature, time, speed, energy, power, pressure, angle, and data
- Resizable Slint interface with grouped, scrollable navigation
- Drop-down unit selectors, a two-value converter display, and docked History/Memory panels
- Programmer-mode bit display grouped into nibbles and a separate settings screen

This is an independent Rust UI approximation, not a complete port of the original XAML interface. Currency conversion and
graphing are shown as unavailable; date addition/subtraction and the original app's platform integrations are not ported.
The conversion catalog currently focuses on common units.

## Build and test

From this directory, run:

```powershell
cargo check
cargo test
cargo run
```

The official application remains at the repository root and can be compared against this implementation during the port.
