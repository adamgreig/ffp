# FFP Platform-Specific Drivers

## Linux

Copy the `99-ffp.rules` file to your udev configuration directory and restart
udev:

```
sudo cp 99-ffp.rules /etc/udev/rules.d
sudo systemctl restart udev
```

Confirm you are in the `plugdev` group, or modify `99-ffp.rules` to use another
group or specify a specific user.
