use std::thread::sleep;
use std::time::Duration;
use failure::ResultExt;
use crate::{Programmer, Flash, Result};

/// FPGA manager
pub struct FPGA<'a> {
    programmer: &'a Programmer<'a>,
}

impl<'a> FPGA<'a> {
    /// Create a new `FPGA` using the given `Programmer`
    pub fn new(programmer: &'a Programmer) -> Self {
        Self { programmer }
    }

    /// Reset the attached FPGA
    pub fn reset(&self) -> Result<()> {
        self.programmer.reset()?;
        sleep(Duration::from_millis(10));
        self.programmer.unreset()
    }

    /// Enable target power
    pub fn power_on(&self) -> Result<()> {
        self.programmer.power_on()
    }

    /// Disable target power
    pub fn power_off(&self) -> Result<()> {
        self.programmer.power_off()
    }

    /// Program the attached FPGA with the provided bitstream
    ///
    /// The FPGA will be reset and start executing after programming completion.
    pub fn program(&self, data: &[u8]) -> Result<()> {
        // Hold FPGA in reset while we power down the flash
        self.programmer.reset()?;
        let flash = Flash::new(self.programmer);
        flash.power_down()?;

        // Release FPGA from reset while in slave SPI mode
        self.programmer.fpga_mode()?;
        self.programmer.select()?;
        self.programmer.unreset()?;

        // Wait for FPGA to come out of reset
        sleep(Duration::from_millis(10));

        // Send 8 dummy clocks with CS high then re-assert CS
        self.programmer.unselect()?;
        self.programmer.write(&[0x00; 1])?;
        self.programmer.select()?;

        // Send configuration data
        self.programmer.write(data).context("Error writing configuration data")?;

        // Release CS and wait for configuration to be complete
        self.programmer.unselect()?;
        self.programmer.write(&[0x00; 40])?;

        Ok(())
    }
}
