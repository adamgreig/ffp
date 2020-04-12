# FFP Firmware

## Requirements

* You must have a working Rust compiler installed. Visit
[rustup.rs](https://rustup.rs) to install Rust.

* Your Rust toolchain needs the `thumbv6m-none-eabi` target installed:

```
rustup target add thumbv6m-none-eabi
```

## Building

Once Rust is installed, use Cargo to build:

```
cargo build --release
```

The resulting binary is an ELF file in
`target/thumbv6m-none-eabi/release/ffp_firmware` which can be programmed via
your usual programmer, or see below for generating a raw binary file and
bootloading over USB.

## Bootloading

You can reprogram the FFP using its built-in USB bootloader. You can use
[dfu-util](http://dfu-util.sourceforge.net/) or
[STM32CubeProg](https://www.st.com/en/development-tools/stm32cubeprog.html)
to perform the bootloading, both of which are available for Linux, MacOS, and
Windows.

To generate a binary file suitable for bootloading:
```
cargo build --release
arm-none-eabi-objcopy -O binary -S target/thumbv6m-none-eabi/release/ffp_firmware ffp.bin
```

To put the FFP device into bootload mode:
```
ffp bootload
```

To bootload using dfu-util:
```
dfu-suffix -a ffp.bin -v 0483 -p df11
dfu-util -a 0 -s 0x08000000 -D ffp.bin
```

Reconnect the device after programming to load new firmware.

When programming a newly manufactured device, the onboard bootloader starts
automatically, so you can skip the `ffp bootload` step.

## Licence

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE)
  or http://www.apache.org/licenses/LICENSE-2.0 )
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  http://opensource.org/licenses/MIT )

at your option.
