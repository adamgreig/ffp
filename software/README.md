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

You'll need to set up permissions to access the USB device, see the [drivers
file](/drivers/) for more details.

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
