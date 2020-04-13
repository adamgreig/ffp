# FFP Control Software

The control software for FFP runs on your computer and uses the FFP hardware to
program an FPGA or SPI flash. It is written in Rust.

## Requirements

* You must have a working Rust compiler installed. Visit
[rustup.rs](https://rustup.rs) to install Rust.

* You'll need to set up drivers or permissions to access the USB device, see
  the [drivers page](/driver/) for more details.


## Building

```
cargo build --release
```

You can either run the ffp executable directly from `target/release/ffp`, or
install it for your user using `cargo install --path .`.

## Installing

FFP software can be installed directly using Cargo:

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

## Python Alternative

The prototype for this software was written as a Python script which is also
available ([prog.py](/scripts/prog.py)).
