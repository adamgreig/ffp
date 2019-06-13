# FFP Control Software

The control software for FFP runs on your computer and uses the FFP hardware to
program an FPGA or SPI flash. It is written in Rust.

## Building

```
cargo build --release
```

## Installing

FFP software can be installed using Cargo:

```
cargo install ffp
```

## Usage

Run `ffp help` for detailed usage. Commonly used commands:

* `ffp fpga program bitstream.bin`
* `ffp fpga reset`
* `ffp fpga power on`
* `ffp flash id`
* `ffp flash program bitstream.bin`
