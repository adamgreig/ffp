#!/usr/bin/env python3
"""
Set FFP TPwr
"""

import time
import usb.core


ID_VENDOR = 0x1209
ID_PRODUCT = 0x0001

SET_MODE = 3
SET_MODE_TYPE = (2 << 5)


def main():
    with open("/dev/urandom", "rb") as f:
        data = f.read(128*1024)

    dev = usb.core.find(idVendor=ID_VENDOR, idProduct=ID_PRODUCT)

    print("Setting to Flash mode")
    dev.ctrl_transfer(
        bmRequestType=SET_MODE_TYPE, bRequest=SET_MODE, wValue=int(1))

    t0 = time.time()
    print(f"Writing {len(data)/1024}kB...")
    chunk_size = 64
    for chunk in range(0, len(data), chunk_size):
        txdata = data[chunk:chunk+chunk_size]
        ntx = dev.write(0x01, txdata)
        rx = dev.read(0x81, chunk_size)
        if ntx != len(rx):
            print(f"Offset {chunk}: TX {ntx}, RX {len(rx)}!")
        if list(txdata) != list(rx):
            print(f"Offset {chunk}: TX != RX")
            print("    ", list(txdata))
            print("    ", list(rx))
    tT = time.time()
    tD = tT - t0
    rate = (len(data) / tD) / 1024
    rate_mbps = rate * 8 / 1024
    print(f"Done in {tD:.03f}s, {rate:.0f}kB/s, {rate_mbps:.02f}Mbps")

    print("Setting to hi-z mode")
    dev.ctrl_transfer(
        bmRequestType=SET_MODE_TYPE, bRequest=SET_MODE, wValue=int(0))


if __name__ == "__main__":
    main()
