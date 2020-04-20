# FFP Control Software

The control software for FFP runs on your computer and uses the FFP hardware to
program an FPGA or SPI flash. It is written in Rust.

## Pre-built Binaries

Pre-built binaries are available for Windows and Linux on the [Releases] page.
You must have [libusb] installed or available on your system.

[Releases]: https://github.com/adamgreig/ffp/releases
[libusb]: https://libusb.info

## Build Requirements

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

## Cross-Compiling for Windows from Linux

From a stock Ubuntu 18.04 image, the following commands generate an `ffp.exe`
suitable for 64-bit Windows:

```sh
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# (accept defaults)
$ source $HOME/.cargo/env
$ cd /tmp
$ rustup target add x86_64-pc-windows-gnu
$ apt install -y p7zip-full build-essential gcc-mingw-w64-x86-64 libusb-1.0 pkg-config
$ wget https://github.com/libusb/libusb/releases/download/v1.0.23/libusb-1.0.23.7z
$ 7z x libusb-1.0.23.7z
$ git clone https://github.com/adamgreig/ffp
$ cd ffp/software
$ mkdir .cargo
$ echo -e '[target.x86_64-pc-windows-gnu]\nlinker = "x86_64-w64-mingw32-gcc"\nrustflags = [ "-L", "/tmp/MinGW64/dll/"]' > .cargo/config
$ env PKG_CONFIG_ALLOW_CROSS=1 cargo build --release --target x86_64-pc-windows-gnu
```

The resulting binary is `target/x86_64-pc-windows-gnu/release/ffp.exe`. It
needs the `libusb-1.0.dll` file from `/tmp/MinGW64/dll/` available on the
Windows system, either in the same directory as `ffp.exe` or installed
system-wide.
