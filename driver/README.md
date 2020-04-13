# FFP Platform-Specific Drivers

## Linux

Copy the `99-ffp.rules` file to your udev configuration directory and restart
udev:

```
sudo cp 99-ffp.rules /etc/udev/rules.d
sudo udevadm control --reload
```

Confirm you are in the `plugdev` group, or modify `99-ffp.rules` to use another
group or specify a specific user. If you have just added yourself to the
`plugdev` group (e.g., `sudo usermod -aG plugdev $USER`), you will need to
log out and log back in again for the group change to take effect.

## Windows

For use as an CMSIS-DAP v1 programmer, no additional driver installation should
be necessary, as the default HID drivers should work. However, this has not
been tested -- please report back if you've got the CMSIS-DAP functionality
working under Windows.

On Windows 8 and above, WinUSB drivers should automatically be installed for
FFP's SPI and CMSIS-DAP v2 interfaces. If these do not work, we recommend using
[Zadig](https://zadig.akeo.ie/) to set up a driver.  Download and run the Zadig
executable, select `FFP SPI Interface (Interface 0)` in the dropdown, and click
`Install Driver`, and click the `Replace Driver` button.  This will install a
default WinUSB driver for the FFP VID:PID combination on Interface 0, allowing
the FFP host software to use the device.

## MacOS

No information for MacOS is known at present. If you've got FFP working on
MacOS, please consider updating this file.
