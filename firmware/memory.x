MEMORY
{
    FLASH  : ORIGIN = 0x08000000, LENGTH = 32K
    RAM    : ORIGIN = 0x20000000, LENGTH = 6K
    USBRAM : ORIGIN = 0x40006000, LENGTH = 1K
}

SECTIONS
{
    .usbram : ALIGN(4)
    {
        *(.usbram .usbram.*);
        . = ALIGN(4);
    } > USBRAM
}
