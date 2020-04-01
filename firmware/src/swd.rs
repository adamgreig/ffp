use crate::hal::{spi::SPI, gpio::Pins};

#[derive(Copy, Clone, Debug)]
pub enum Error {
    BadParity,
    AckWait,
    AckFault,
    AckProtocol,
    AckUnknown(u8),
}

pub type Result<T> = core::result::Result<T, Error>;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum DPRegister {
    DPIDR       = 0,
    CTRLSTAT    = 1,
    SELECT      = 2,
    RDBUFF      = 3,
}

pub struct SWD<'a> {
    spi: &'a SPI,
    pins: &'a Pins<'a>,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum APnDP {
    DP = 0,
    AP = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum RnW {
    W = 0,
    R = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum ACK {
    OK          = 0b001,
    WAIT        = 0b010,
    FAULT       = 0b100,
    PROTOCOL    = 0b111,
}

impl ACK {
    pub fn check_ok(ack: u8) -> Result<()> {
        match ack {
            v if v == (ACK::OK as u8) => Ok(()),
            v if v == (ACK::WAIT as u8) => Err(Error::AckWait),
            v if v == (ACK::FAULT as u8) => Err(Error::AckFault),
            v if v == (ACK::PROTOCOL as u8) => Err(Error::AckProtocol),
            _ => Err(Error::AckUnknown(ack)),
        }
    }
}

impl<'a> SWD<'a> {
    pub fn new(spi: &'a SPI, pins: &'a Pins) -> Self {
        spi.setup_dap();
        SWD { spi, pins }
    }

    fn line_reset(&self) {
        for _ in 0..7 {
            self.spi.tx8(0xFF);
        }
        self.spi.wait_busy();
    }

    fn jtag_to_swd(&self) {
        self.spi.tx8(0x9E);
        self.spi.tx8(0xE7);
    }

    pub fn idle_high(&self) {
        self.spi.tx4(0xF);
    }

    pub fn idle_low(&self) {
        self.spi.tx4(0x0);
    }

    pub fn start(&self) {
        self.pins.swd_tx();
        self.line_reset();
        self.jtag_to_swd();
        self.line_reset();
        self.idle_low();
    }

    pub fn read_dp(&self, a: DPRegister) -> Result<u32> {
        self.read(APnDP::DP, a as u8)
    }

    pub fn write_dp(&self, a: DPRegister, data: u32) -> Result<()> {
        self.write(APnDP::DP, a as u8, data)
    }

    pub fn read_ap(&self, a: u8) -> Result<u32> {
        self.read(APnDP::AP, a)
    }

    pub fn write_ap(&self, a: u8, data: u32) -> Result<()> {
        self.write(APnDP::AP, a, data)
    }

    fn read(&self, apndp: APnDP, a: u8) -> Result<u32> {
        let req = Self::make_request(apndp, RnW::R, a);
        self.spi.tx8(req);
        self.pins.swd_rx();
        self.spi.drain();

        // 1 clock for turnaround and 3 for ACK
        let ack = self.spi.rx4() >> 1;
        ACK::check_ok(ack as u8)?;

        // 32 clocks for data
        let mut data = self.spi.rx8_chain_first() as u32;
        data |= (self.spi.rx8_chain() as u32) << 8;
        data |= (self.spi.rx8_chain() as u32) << 16;
        data |= (self.spi.rx8_chain() as u32) << 24;

        // 8 clocks for parity + turnaround, rest are trailing
        // It's quicker to do 8 than drain FIFO, swap to 4bit, and do 4
        let parity = (self.spi.rx8_chain_last() & 1) as u32;

        // Back to driving SWDIO to ensure it doesn't float high
        self.pins.swd_tx();

        //let data = w1 | (w2 << 8) | (w3 << 16) | (w4 << 24);
        match parity == (data.count_ones() & 1) {
            true => return Ok(data),
            false => return Err(Error::BadParity),
        }
    }

    fn write(&self, apndp: APnDP, a: u8, data: u32) -> Result<()> {
        let req = Self::make_request(apndp, RnW::W, a);
        let parity = data.count_ones() & 1;

        self.spi.tx8(req);
        self.spi.wait_busy();
        self.pins.swd_rx();
        self.spi.drain();

        // 1 clock for turnaround and 3 for ACK and 1 for turnaround
        let ack = self.spi.rx5() >> 1;
        ACK::check_ok(ack as u8)?;

        self.pins.swd_tx();

        // 32 clocks for data
        // Doing 4x8bit plus 8bit parity turns out to be
        // quicker than 2x16 bit plus 4bit parity, mainly
        // because with 8bit writes we can keep the FIFO hot.
        self.spi.tx8(((data >> 0) & 0xFF) as u8);
        self.spi.tx8(((data >> 8) & 0xFF) as u8);
        self.spi.tx8(((data >> 16) & 0xFF) as u8);
        self.spi.tx8(((data >> 24) & 0xFF) as u8);

        // 8 clocks for parity and trailing idle
        // It's quicker to run 8 clocks than wait for SPI buffer
        // to empty, change data size to 4 bits, then run 4 clocks.
        self.spi.tx8(parity as u8);
        self.spi.wait_busy();

        Ok(())
    }

    fn make_request(apndp: APnDP, rnw: RnW, a: u8) -> u8 {
        let req = (1 << 0) | ((apndp as u8) << 1) | ((rnw as u8) << 2) | (a << 3) | (1 << 7);
        let parity = (req.count_ones() & 1) as u8;
        req | (parity << 5)
    }
}
