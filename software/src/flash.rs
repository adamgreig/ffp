use std::convert::TryInto;
use crate::{Programmer, FFPError, Result};

#[derive(Copy, Clone, Debug)]
#[allow(unused)]
#[repr(u8)]
enum Command {
    WriteEnable = 0x06,
    WriteDisable = 0x04,
    ReadStatusRegister1 = 0x05,
    ReadStatusRegister2 = 0x35,
    WriteStatusRegister = 0x01,
    PageProgram = 0x02,
    SectorErase = 0x20,
    BlockErase32KB = 0x52,
    BlockErase64KB = 0xD8,
    ChipErase = 0xC7,
    ProgramSuspend = 0x75,
    ProgramResume = 0x7A,
    PowerDown = 0xB9,
    ReadData = 0x03,
    FastRead = 0x0B,
    ReleasePowerdown = 0xAB,
    ReadDeviceID = 0x90,
    ReadJEDECID = 0x9F,
    ReadUniqueID = 0x4B,
    ReadSFDPRegister = 0x5A,
    EnableReset = 0x66,
    Reset = 0x99,
}

#[derive(Copy, Clone, Debug)]
pub struct FlashID {
    manufacturer_id: u8,
    device_id: u8,
    unique_id: u64,
}

impl std::fmt::Display for FlashID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Manufacturer {:02X}, Device {:02X}, Unique ID {:016X}",
               self.manufacturer_id, self.device_id, self.unique_id)
    }
}

pub trait FlashAccess {
    fn enable(&self) -> Result<()>;
    fn select(&self) -> Result<()>;
    fn unselect(&self) -> Result<()>;
    fn write(&self, data: &[u8]) -> Result<Vec<u8>>;
}

impl FlashAccess for &Programmer {
    fn enable(&self) -> Result<()> {
        (self as &Programmer).flash_mode()
    }

    fn select(&self) -> Result<()> {
        (self as &Programmer).select()
    }

    fn unselect(&self) -> Result<()> {
        (self as &Programmer).unselect()
    }

    fn write(&self, data: &[u8]) -> Result<Vec<u8>> {
        (self as &Programmer).write(data)
    }
}

/// Directly attached SPI flash manager.
pub struct SPIFlash<'a> {
    programmer: &'a Programmer,
    flash: Flash<&'a Programmer>,
}

impl<'a> SPIFlash<'a> {
    /// Create a new `Flash` using the given `Programmer`
    pub fn new(programmer: &'a Programmer) -> Self {
        Self { programmer, flash: Flash::new(programmer) }
    }

    /// Read the attached flash device, manufacturer, and unique IDs
    pub fn read_id(&self) -> Result<FlashID> {
        self.programmer.reset()?;
        self.flash.read_id()
    }

    /// Read `length` bytes of data from the attached flash, starting at `address`
    pub fn read(&self, address: u32, length: usize) -> Result<Vec<u8>> {
        self.flash.read(address, length)
    }

    /// Program the attached flash with `data` starting at `address`.
    ///
    /// If `verify` is true, also read-back the programmed data and
    /// return FFPError::ReadbackError if it did not match what was written.
    pub fn program(&self, address: u32, data: &[u8], verify: bool) -> Result<()> {
        self.flash.program(address, data, verify)
    }

    /// Erase entire flash chip
    pub fn erase(&self) -> Result<()> {
        self.flash.erase()
    }

    /// Reset the attached flash
    pub fn reset(&self) -> Result<()> {
        self.flash.reset()
    }

    /// Power down the attached flash
    pub fn power_down(&self) -> Result<()> {
        self.flash.power_down()
    }

    /// Power up the attached flash
    pub fn power_up(&self) -> Result<()> {
        self.flash.power_up()
    }
}

/// Abstract SPI flash manager.
pub struct Flash<A: FlashAccess> {
    access: A,
}

impl<A: FlashAccess> Flash<A> {
    pub fn new(access: A) -> Self {
        Self { access }
    }

    /// Read the attached flash device, manufacturer, and unique IDs
    pub fn read_id(&self) -> Result<FlashID> {
        self.power_up()?;
        self.reset()?;
        let (manufacturer_id, device_id) = self.read_device_id()?;
        let unique_id = self.read_unique_id()?;
        Ok(FlashID { manufacturer_id, device_id, unique_id })
    }

    /// Read `length` bytes of data from the attached flash, starting at `address`
    pub fn read(&self, address: u32, length: usize) -> Result<Vec<u8>> {
        self.fast_read(address, length)
    }

    /// Program the attached flash with `data` starting at `address`.
    ///
    /// If `verify` is true, also read-back the programmed data and
    /// return FFPError::ReadbackError if it did not match what was written.
    pub fn program(&self, address: u32, data: &[u8], verify: bool) -> Result<()> {
        self.erase_for_data(address, data.len())?;
        self.program_data(address, data)?;
        if verify {
            let programmed = self.read(address, data.len())?;
            if programmed == data {
                Ok(())
            } else {
                Err(FFPError::ReadbackError)?
            }
        } else {
            Ok(())
        }
    }

    /// Erase entire flash chip
    pub fn erase(&self) -> Result<()> {
        self.write_enable()?;
        self.chip_erase()?;
        self.wait_while_busy()?;
        Ok(())
    }

    /// Reset the attached flash
    pub fn reset(&self) -> Result<()> {
        self.command(Command::EnableReset)?;
        self.command(Command::Reset)
    }

    /// Power down the attached flash
    pub fn power_down(&self) -> Result<()> {
        self.command(Command::PowerDown)
    }

    /// Power up the attached flash
    pub fn power_up(&self) -> Result<()> {
        self.command(Command::ReleasePowerdown)
    }

    fn erase_for_data(&self, address: u32, length: usize) -> Result<()> {
        // Adjust length and address to be 64K aligned
        const BLOCK_SIZE: usize = 64 * 1024;
        let length = length + (address as usize % BLOCK_SIZE) as usize;
        let address = address & 0xFF0000;
        let mut n_blocks = length / BLOCK_SIZE;
        if length % BLOCK_SIZE != 0 { n_blocks += 1 };
        for block in 0..n_blocks {
            self.write_enable()?;
            self.block_erase_64k(address + (block * BLOCK_SIZE) as u32)?;
            self.wait_while_busy()?;
        }
        Ok(())
    }

    fn program_data(&self, address: u32, data: &[u8]) -> Result<()> {
        // Pad to obtain page alignment
        const PAGE_SIZE: usize = 256;
        let pad_length = address as usize % PAGE_SIZE;
        let tx = if pad_length != 0 {
            let mut tx = vec![0xFF; pad_length];
            tx.extend(data);
            tx
        } else {
            data.to_vec()
        };
        let address = address & 0xFFFF00;

        // Write pages
        for (idx, page_data) in tx.chunks(PAGE_SIZE).enumerate() {
            self.write_enable()?;
            self.page_program(address + (idx*PAGE_SIZE) as u32, page_data)?;
            self.wait_while_busy()?;
        }
        Ok(())
    }

    fn write_enable(&self) -> Result<()> {
        self.command(Command::WriteEnable)
    }

    #[allow(dead_code)]
    fn write_disable(&self) -> Result<()> {
        self.command(Command::WriteDisable)
    }

    fn page_program(&self, address: u32, data: &[u8]) -> Result<()> {
        assert!(data.len() >= 1, "Cannot program 0 bytes of data");
        assert!(data.len() <= 256, "Cannot program more than 256 bytes per page");
        let mut tx = address.to_be_bytes()[1..].to_vec();
        tx.extend(data);
        self.exchange(Command::PageProgram, &tx, 0)?;
        Ok(())
    }

    fn fast_read(&self, address: u32, length: usize) -> Result<Vec<u8>> {
        let length = length + 1;
        let address = &address.to_be_bytes()[1..];
        self.exchange(Command::FastRead, address, length).map(|data| data[1..].to_vec())
    }

    fn chip_erase(&self) -> Result<()> {
        self.command(Command::ChipErase)
    }

    fn block_erase_64k(&self, address: u32) -> Result<()> {
        self.exchange(Command::BlockErase64KB, &address.to_be_bytes()[1..], 0)?;
        Ok(())
    }

    #[allow(dead_code)]
    fn block_erase_32k(&self, address: u32) -> Result<()> {
        self.exchange(Command::BlockErase32KB, &address.to_be_bytes()[1..], 0)?;
        Ok(())
    }

    #[allow(dead_code)]
    fn sector_erase(&self, address: u32) -> Result<()> {
        self.exchange(Command::SectorErase, &address.to_be_bytes()[1..], 0)?;
        Ok(())
    }

    fn read_device_id(&self) -> Result<(u8, u8)> {
        self.exchange(Command::ReadDeviceID, &[], 3+2)
            .map(|data| (data[3], data[4]))
    }

    fn read_unique_id(&self) -> Result<u64> {
        self.exchange(Command::ReadUniqueID, &[], 4+8)
            .and_then(|data| Ok(u64::from_be_bytes((&data[4..]).try_into()?)))
    }

    fn read_status1(&self) -> Result<u8> {
        self.exchange(Command::ReadStatusRegister1, &[], 1).map(|data| data[0])
    }

    #[allow(dead_code)]
    fn read_status2(&self) -> Result<u8> {
        self.exchange(Command::ReadStatusRegister2, &[], 1).map(|data| data[0])
    }

    fn is_busy(&self) -> Result<bool> {
        self.read_status1().map(|status| status & 1 == 1)
    }

    fn wait_while_busy(&self) -> Result<()> {
        while self.is_busy()? {}
        Ok(())
    }

    /// Writes `command` and `data` to the flash memory, then returns `nbytes` of response.
    fn exchange(&self, command: Command, data: &[u8], nbytes: usize) -> Result<Vec<u8>> {
        let mut tx = vec![command as u8];
        tx.extend(data);
        tx.extend(vec![0u8; nbytes]);
        self.access.enable()?;
        self.access.select()?;
        let rx = self.access.write(&tx)?;
        self.access.unselect()?;
        Ok(rx[1+data.len()..].to_vec())
    }

    /// Convenience method for issuing a single command and not caring about the returned data
    fn command(&self, command: Command) -> Result<()> {
        self.exchange(command, &[], 0)?;
        Ok(())
    }
}
