# KRPSim

KRPSim explores hard resource-constrained scheduling problems (extended RCPSP) by combining a fast discrete-event simulator with a multi-island genetic algorithm. It ships with several puzzle-like scenarios inspired by manufacturing lines and management games.

## Highlights

-   Multi-island genetic algorithm with automatic stagnation resets and configurable mutation steps.
-   Deterministic runs by supplying a numeric seed; otherwise a fresh random seed is drawn.
-   Lightweight text format for defining stocks, processes, and optimization objectives.
-   Scenario simulator tuned for up to 10,000-cycle horizons with optional CSV stock logging for debugging.

## Prerequisites

-   Rust toolchain with edition 2024 support (install via [`rustup`](https://rustup.rs)).
-   `cargo` for building and running the binary.

## Build & Run

```bash
cargo build --release
./target/release/KRPSim input_files/pomme 1337
```

You can also use `cargo run --release -- <input_file> [seed]`. The optional seed must be a plain integer (the current parser does not accept the `--seed=<n>` form shown in the help message). When omitted the program samples a random seed and prints the best genome fitness to `stderr`.

## Input Format

Each scenario is a plain text file. Blank lines and lines that start with `#` are ignored. The grammar uses three kinds of statements:

-   Stock definition : Declares the initial quantity for a resource. Missing stocks default to 0. Example : `iron_plate:15`
-   Process definition: Describes consumption, production, and duration. Needs and results lists are semicolon-separated `name:qty` pairs. Either list may be empty. | `smelt:(ore:1;coal:1):(plate:1):3`
-   Optimize directive | Chooses the objective. Use `optimize:(stock)` to maximize a stock quantity, or `optimize:(time;stock)` to minimize the time to reach a stock threshold. Example :`optimize:(electronic_circuit)`

All identifiers are alphanumeric (underscores allowed for stocks) and quantities are signed integers. See the `input_files/` directory for complete examples such as `factorio`, `pomme`, and `recre`.

## Output

Execution runs the genetic search for the requested number of generations. Progress and diagnostics (including the best genome fitness and the final stock levels) are emitted to `stderr`. For experiments that need stock evolution traces, uncomment the logger in `src/ga.rs`—it writes `stock_evolution.csv` with per-cycle snapshots.

## Genetic Algorithm Overview

-   Eight islands evolve in parallel using Rayon-based parallel iterators.
-   Each genome encodes process priorities (random keys), a pending-stock divider, and a flag for disabling processes.
-   Selection keeps the top performers, while crossover/mutation refresh the rest of the population. Islands periodically import elites from neighbours.
-   Stagnating populations are reset with wider genetic diversity after configurable cooldowns.

The simulator itself evaluates genomes by running processes when their inputs are available, tracking deficits to avoid starving high-priority chains, and accumulating fitness based on the chosen optimize target.

## Sample Scenarios

-   `input_files/pomme`: bakery-style production chain with competing dessert goals.
-   `input_files/factorio`: automation bootstrap that starts from hand mining.
-   `input_files/recre`: amusement park management with sink processes.
-   `input_files/strigoi`, `input_files/ikea`, and more under `input_files/` for additional stress tests.

## Current 10,000-Cycle Upper Bounds

These are the best fitness values observed so far for the bundled scenarios (10,000-cycle horizon):

-   `pomme`: 308360
-   `recre`: 68
-   `year`: 25 (optimal)
-   `factorio`: 19555

## Development Notes

-   Format the code with `cargo fmt` and lint with `cargo clippy` before submitting substantial changes.
-   The `TODO.md` file tracks parser and GA improvements that are still outstanding.
