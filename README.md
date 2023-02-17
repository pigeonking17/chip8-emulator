## CHIP-8 Emulator

[![forthebadge](http://forthebadge.com/images/badges/made-with-rust.svg)](http://forthebadge.com)

A CHIP-8 emulator that has been written in Rust. This is still a work in progress and as such is missing many features.

## Compilation

```bash
# Clone this repository.
$ git clone https://github.com/pigeonking17/chip8-emulator

# Go into the repository.
$ cd chip8-emulator

# Compile with cargo.
$ cargo build --release
```

## Usage

You can either run it through cargo or directly with the binary.

```bash
# With cargo:
$ cargo run --release -- --program program.ch8

# With the binary:
$ ./target/release/cpu-emulator --program program.ch8
```

## License
GPL3
