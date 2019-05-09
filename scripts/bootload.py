#!/usr/bin/env python3
"""
Bootloader script for FFP.

Sends a USB request to the FFP firmware to reboot into the
system bootloader image, suitable for use with DFU.
"""

import usb.core


ID_VENDOR = 0x1209
ID_PRODUCT = 0x0001
BOOTLOAD_REQUEST_TYPE = (0 << 7) | (2 << 5) | (0 << 0)
BOOTLOAD_REQUEST = 7


def main():
    dev = usb.core.find(idVendor=ID_VENDOR, idProduct=ID_PRODUCT)
    dev.ctrl_transfer(
        bmRequestType=BOOTLOAD_REQUEST_TYPE, bRequest=BOOTLOAD_REQUEST)


if __name__ == "__main__":
    main()
