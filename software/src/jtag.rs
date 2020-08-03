use std::thread::sleep;
use std::time::Duration;
use crate::{Programmer, Result};

/// JTAG manager
pub struct JTAG<'a> {
    programmer: &'a Programmer,
}

impl<'a> JTAG<'a> {
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

    pub fn demo(&self) -> Result<()> {
        self.programmer.jtag_mode()?;
        self.programmer.reset_mcu()?;
        self.programmer.unreset_mcu()?;

        let request = SequenceBuilder::new()
            // Write TMS=1 for 5 clocks to ensure we are in test-logic-reset.
            .add_mode(5, 1)
            // Enter run-test/idle
            .add_mode(1, 0)
            // Enter select-dr-scan, select-ir-scan
            .add_mode(2, 1)
            // Enter capture-ir, shift-ir
            .add_mode(2, 0)
            // Write IR: both TAPs to IDCODE (first 8 bits, with TMS=0)
            .add_write(8, 0, &[0b0001_1110])
            // Write IR: both TAPs to IDCODE (final bit, with TMS=1),
            // and move to exit1-ir, update-ir, select-dr-scan
            .add_write(3, 1, &[0b0])
            // Enter capture-dr, shift-dr
            .add_mode(2, 0)
            // Read first DR
            .add_read(32, 0)
            // Read second DR
            .add_read(32, 0)
            // Enter exit1-dr, update-dr, select-dr-scan, select-ir-scan, test-logic-reset
            .add_mode(5, 1);

        let data = request.execute(self.programmer)?;

        let idcode1 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let idcode2 = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        println!("IDCODE1=0x{:08X} IDCODE2=0x{:08X}", idcode1, idcode2);

        Ok(())
    }
}

pub struct SequenceBuilder {
    num_sequences: usize,
    capture_length: usize,
    request: Vec<u8>,
}

impl SequenceBuilder {
    pub fn new() -> Self {
        SequenceBuilder {
            num_sequences: 0,
            capture_length: 0,
            request: Vec::new(),
        }
    }

    pub fn add_request(mut self, len: usize, tms: u8, tdi: Option<&[u8]>, capture: bool)
        -> Self
    {
        let nbytes = bytes_for_bits(len);
        let dummy = vec![0xff; nbytes];

        let tdi = match tdi {
            Some(tdi) => tdi,
            None => &dummy,
        };

        assert!(len <= 64);
        assert!(tdi.len() == nbytes);
        assert!(1 + self.request.len() + 1 + nbytes <= 64);

        let mut header = if len == 64 { 0 } else { len as u8 };
        if tms != 0 {
            header |= 1 << 6;
        }
        if capture {
            header |= 1 << 7;
            self.capture_length += nbytes;
        }

        self.request.extend_from_slice(&[header]);
        self.request.extend_from_slice(tdi);
        self.num_sequences += 1;
        self
    }

    pub fn add_write(self, len: usize, tms: u8, tdi: &[u8]) -> Self {
        self.add_request(len, tms, Some(tdi), false)
    }

    pub fn add_mode(self, len: usize, tms: u8) -> Self {
        self.add_request(len, tms, None, false)
    }

    pub fn add_read(self, len: usize, tms: u8) -> Self {
        self.add_request(len, tms, None, true)
    }

    fn to_bytes(self) -> Vec<u8> {
        let mut request = vec![self.num_sequences as u8];
        request.extend_from_slice(&self.request[..]);
        request
    }

    pub fn execute(self, programmer: &Programmer) -> Result<Vec<u8>> {
        let rxlen = self.capture_length;
        let request = self.to_bytes();
        programmer.jtag_sequence(&request[..], rxlen)
    }
}

/// Returns number of whole bytes required to hold `n` bits.
fn bytes_for_bits(n: usize) -> usize {
    (n + 7) / 8
}
