# FFP Firmware

## Building

You need to have Rust installed (visit [rustup.rs](https://rustup.rs)) and
the thumbv6m-none-eabi target available:

```
rustup target add thumbv6m-none-eabi
```

Once installed, use Cargo to build:

```
cargo build --release
```

The resulting binary is an ELF file in
`target/thumbv6m-none-eabi/release/ffp_firmware` which can be programmed via
your usual programmer, or see below for bootloading.

## Bootloading

You can reprogram the FFP using its built-in USB bootloader. You'll need
dfu-utils installed.

```
cargo build --release
arm-none-eabi-objcopy -O binary -S target/thumbv6m-none-eabi/release/ffp_firmware ffp.bin
dfu-suffix -a ffp.bin -v 0483 -p df11
ffp bootload
dfu-util -a 0 -s 0x08000000 -D ffp.bin
```

Reconnect the device after programming to load new firmware.

When programming a newly manufactured device, the onboard bootloader starts
automatically, so you can skip the `ffp bootload` step.

## Licence

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
