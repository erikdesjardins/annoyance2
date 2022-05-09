# annoyance2

Digital musical tesla coil interrupter

Digital version of https://github.com/erikdesjardins/annoyance. Intended to run on a "blue pill" [STM32F103C8](https://www.st.com/resource/en/datasheet/stm32f103c8.pdf) board.

## Setup

```sh
cargo install flip-link probe-run
rustup target add thumbv7m-none-eabi
```

## Development

```sh
cargo watch --clear --delay 1 --exec check
```

## Testing

See https://crates.io/crates/defmt-test.

```sh
cargo test -p testsuite
```
