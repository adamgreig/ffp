# FFP Firmware

## Building

```
cargo build --release
```

## Setting Option Bytes

Note that the built in bootloader will just keep jumping to the user
application if the `BOOT_SEL` option bit is set (the default). You have to
clear this to 0 to force always booting from main flash, at which point the
built in bootloader can be jumped to from the user application. Wild.

```
(gdb) mon option 0x1FFFF802 0x807F
0x1FFFF800: 0x55AA
0x1FFFF802: 0x807F
0x1FFFF804: 0x00FF
0x1FFFF806: 0x00FF
0x1FFFF808: 0x00FF
0x1FFFF80A: 0x00FF
0x1FFFF80C: 0x00FF
0x1FFFF80E: 0x00FF
```
