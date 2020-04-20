# FFP Firmware

## Pre-built Binaries

Pre-built binaries are available for each release on the [Releases] page. See
below for instructions on using the files to program an FFP device. The easiest
method is to use `dfu-util` with the `ffp_firmware.dfu` pre-built file, which
uses the onboard USB bootloader.

[Releases]: https://github.com/adamgreig/ffp/releases

## Build Requirements

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

To generate a binary file suitable for bootloading with dfu-util:
```
cargo build --release
arm-none-eabi-objcopy -O binary -S target/thumbv6m-none-eabi/release/ffp_firmware ffp.bin
dfu-suffix -a ffp.bin -v 0483 -p df11
```

STM32CubeProg can load data directly from the ELF file, but only if it has a
`.elf` extension. Simply copy the elf file:

```
cargo build --release
cp target/thumbv6m-none-eabi/release/ffp_firmware ffp_firmware.elf
```

To put an already-programmed FFP device into bootload mode:
```
ffp bootload
```
Devices which have a totally erased flash will come up in bootloader mode
automatically.

To bootload using dfu-util:
```
dfu-util -a 0 -s 0x08000000 -D ffp.bin
```

To bootload using STM32CubeProg, open the `ffp_firmware.elf` file made above,
and download it to the FFP.

Reconnect the device after programming to load new firmware.

### Preparing a DfuSe file

Using `dfu-tool`, a DfuSe file can be created for more convenient bootloading:

```
arm-none-eabi-objcopy -O binary -S target/thumbv6m-none-eabi/release/ffp_firmware ffp.bin
dfu-tool convert dfuse ffp.bin ffp.dfu
dfu-tool set-vendor ffp.dfu 0483
dfu-tool set-product ffp.dfu df11
dfu-tool set-address ffp.dfu 0x08000000
```

Unfortunately dfu-tool seemingly cannot set the alt setting to 0, only to >=1,
so that must still be specified by dfu-util:

```
dfu-util -a 0 -D ffp.dfu
```

## Licence

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE)
  or http://www.apache.org/licenses/LICENSE-2.0 )
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or
  http://opensource.org/licenses/MIT )

at your option.
