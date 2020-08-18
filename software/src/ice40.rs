use std::thread::sleep;
use std::time::Duration;
use failure::ResultExt;
use crate::{Programmer, Flash, Result};

/// iCE40 FPGA manager
pub struct ICE40<'a> {
    programmer: &'a Programmer,
}

impl<'a> ICE40<'a> {
    /// Create a new `ICE40` using the given `Programmer`
    pub fn new(programmer: &'a Programmer) -> Self {
        Self { programmer }
    }

    /// Reset the attached iCE40
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

    /// Program the attached iCE40 with the provided bitstream
    ///
    /// The iCE40 will be reset and start executing after programming completion.
    pub fn program(&self, data: &[u8]) -> Result<()> {
        // Hold iCE40 in reset while we power down the flash
        self.programmer.reset()?;
        let flash = Flash::new(self.programmer);
        flash.power_down()?;

        // Release iCE40 from reset while in slave SPI mode
        self.programmer.fpga_mode()?;
        self.programmer.select()?;
        self.programmer.unreset()?;

        // Wait for iCE40 to come out of reset
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
