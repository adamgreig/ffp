#!/usr/bin/env python3
"""
Set FFP TPwr
"""

import time
import usb.core


ID_VENDOR = 0x1209
ID_PRODUCT = 0x0001


def main():
    with open("/dev/urandom", "rb") as f:
        data = f.read(128*1024)

    dev = usb.core.find(idVendor=ID_VENDOR, idProduct=ID_PRODUCT)
    t0 = time.time()
    print("Writing 128kB...")
    for chunk in range(0, len(data), 64):
        txdata = data[chunk:chunk+64]
        ntx = dev.write(0x01, txdata)
        rx = dev.read(0x81, 64)
        if ntx != len(rx):
            print(f"Chunk {chunk}: TX {ntx}, RX {len(rx)}!")
        if list(txdata) != list(rx):
            print(f"Chunk {chunk}: TX != RX")
            print("    ", txdata)
            print("    ", list(rx))
    tT = time.time()
    tD = tT - t0
    rate = (len(data) / tD) / 1024
    rate_mbps = rate * 8 / 1024
    print(f"Done in {tD:.03f}s, {rate:.0f}kB/s, {rate_mbps:.02f}Mbps")


if __name__ == "__main__":
    main()
