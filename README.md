# annoyance2

Digital musical tesla coil interrupter

Digital version of https://github.com/erikdesjardins/annoyance. Intended to run on a "blue pill" [STM32F103C8](https://www.st.com/resource/en/datasheet/stm32f103c8.pdf) board.

## Setup

Add the target corresponding to Cortex-M3:

```sh
rustup target add thumbv7m-none-eabi
```

Install tools for linking/flashing:

```sh
cargo install flip-link probe-run
```

Follow `probe-rs` docs to install drivers for flashing:

https://probe.rs/docs/getting-started/probe-setup/

## Development

### Run on device

```sh
# use `debug` or `trace` for more info
set DEFMT_LOG=info

cargo run --manifest-path firmware/Cargo.toml --target thumbv7m-none-eabi
```

### Run with visualizer

```sh
cargo run --manifest-path firmware/Cargo.toml --target thumbv7m-none-eabi --release | cargo run --manifest-path visualizer/Cargo.toml --release
```

### Misc

```
cargo objdump --release --bin main -- --disassemble --no-show-raw-insn --print-imm-hex
```
