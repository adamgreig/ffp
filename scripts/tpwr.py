#!/usr/bin/env python3
"""
Set FFP TPwr
"""

import sys
import time
import usb.core
import usb.control


ID_VENDOR = 0x1209
ID_PRODUCT = 0x0001
SET_TPWR_TYPE = (0 << 7) | (2 << 5) | (0 << 0)
GET_TPWR_TYPE = (1 << 7) | (2 << 5) | (0 << 0)
SET_TPWR = 4
GET_TPWR = 5


def main():
    dev = usb.core.find(idVendor=ID_VENDOR, idProduct=ID_PRODUCT)
    val = int(sys.argv[1])
    dev.ctrl_transfer(
        bmRequestType=SET_TPWR_TYPE, bRequest=SET_TPWR, wValue=val)
    time.sleep(0.1)
    r = dev.ctrl_transfer(
        bmRequestType=GET_TPWR_TYPE, bRequest=GET_TPWR, data_or_wLength=2)
    print("Detected TPwr:", r[0])


if __name__ == "__main__":
    main()
