use std::time::Duration;
use failure::ResultExt;
use crate::{FFPError, Result};

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
enum Command {
    SetCS = 1,
    SetFPGAReset = 2,
    SetMode = 3,
    SetTPwr = 4,
    SetLED = 6,
    Bootload = 7,
}

#[derive(Copy, Clone, Debug)]
#[repr(u16)]
enum Mode {
    HighZ = 0,
    Flash = 1,
    FPGA = 2,
}

pub struct Programmer<'a> {
    handle: libusb::DeviceHandle<'a>,
}

impl <'a> Programmer<'a> {
    const ID_VENDOR: u16        = 0x1209;
    const ID_PRODUCT: u16       = 0xff50;
    const REQUEST_TYPE_SET: u8  = 2 << 5;
    const TX_EP: u8             = 0x01;
    const RX_EP: u8             = 0x81;
    const CHUNK_SIZE: usize     = 64;

    pub fn from_handle(mut handle: libusb::DeviceHandle<'a>) -> Result<Self> {
        handle.claim_interface(0).context("Error claiming interface")?;
        let programmer = Self { handle };
        programmer.led_on()?;
        Ok(programmer)
    }

    pub fn find(context: &'a libusb::Context) -> Result<Self> {
        for device in context.devices().context("Error getting devices")?.iter() {
            let dd = device.device_descriptor().context("Error reading descriptor")?;
            if dd.vendor_id() == Self::ID_VENDOR && dd.product_id() == Self::ID_PRODUCT {
                let handle = device.open().context("Error opening device")?;
                return Self::from_handle(handle);
            }
        }
        Err(FFPError::NoDeviceFound)?
    }

    pub fn led_on(&self) -> Result<()> {
        self.set(Command::SetLED, 1)
    }

    pub fn led_off(&self) -> Result<()> {
        self.set(Command::SetLED, 0)
    }

    pub fn reset(&self) -> Result<()> {
        self.set(Command::SetFPGAReset, 0)
    }

    pub fn unreset(&self) -> Result<()> {
        self.set(Command::SetFPGAReset, 1)
    }

    pub fn select(&self) -> Result<()> {
        self.set(Command::SetCS, 0)
    }

    pub fn unselect(&self) -> Result<()> {
        self.set(Command::SetCS, 1)
    }

    pub fn high_z_mode(&self) -> Result<()> {
        self.set(Command::SetMode, Mode::HighZ as u16)
    }

    pub fn flash_mode(&self) -> Result<()> {
        self.set(Command::SetMode, Mode::Flash as u16)
    }

    pub fn fpga_mode(&self) -> Result<()> {
        self.set(Command::SetMode, Mode::FPGA as u16)
    }

    pub fn power_on(&self) -> Result<()> {
        self.set(Command::SetTPwr, 1)
    }

    pub fn power_off(&self) -> Result<()> {
        self.set(Command::SetTPwr, 0)
    }

    pub fn bootload(&self) -> Result<()> {
        self.set(Command::Bootload, 0)
    }

    pub fn write(&self, data: &[u8]) -> Result<Vec<u8>> {
        let timeout = Duration::from_millis(100);
        let mut rx = Vec::new();
        for chunk in data.chunks(Self::CHUNK_SIZE) {
            let mut rx_chunk = vec![0u8; chunk.len()];
            self.handle.write_bulk(Self::TX_EP, chunk, timeout)
                       .context("Error writing data")?;
            match self.handle.read_bulk(Self::RX_EP, &mut rx_chunk, timeout) {
                Ok(n) if n == chunk.len() => rx.extend(rx_chunk),
                Ok(n) => Err(FFPError::NotEnoughData {
                    expected: chunk.len(), read: n
                })?,
                Err(e) => Err(FFPError::USBError(e))
                          .context("Error reading data")?,
            }
        }
        Ok(rx)
    }

    fn set(&self, request: Command, value: u16) -> Result<()> {
        let timeout = Duration::from_millis(100);
        match self.handle.write_control(
            Self::REQUEST_TYPE_SET, request as u8, value, 0, &[], timeout)
        {
            Ok(_) => Ok(()),
            Err(e) => Err(FFPError::USBError(e))
                        .context(format!("Error sending request {:?} {}", request, value))?,
        }
    }
}

impl<'a> Drop for Programmer<'a> {
    fn drop(&mut self) {
        self.led_off().ok();
    }
}
