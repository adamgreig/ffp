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

    /// Reset the attached target
    pub fn reset(&self) -> Result<()> {
        self.programmer.reset_mcu()?;
        sleep(Duration::from_millis(10));
        self.programmer.unreset_mcu()
    }

    /// Enable target power
    pub fn power_on(&self) -> Result<()> {
        self.programmer.power_on()
    }

    /// Disable target power
    pub fn power_off(&self) -> Result<()> {
        self.programmer.power_off()
    }

    /// Read all IDCODEs on the JTAG scan chain.
    pub fn idcodes(&self) -> Result<Vec<u32>> {
        self.programmer.jtag_mode()?;

        // Set all devices up for a read of IDCODE
        let request = SequenceBuilder::new()
            // Write TMS=1 for 5 clocks to ensure we are in test-logic-reset.
            .mode(5, 1)
            // Enter run-test/idle
            .mode(1, 0)
            // Enter select-dr-scan
            .mode(1, 1)
            // Enter capture-dr, shift-dr
            .mode(2, 0)
            // Read first IDCODE
            .read(32, 0);

        let mut idcodes = Vec::new();
        let mut data = request.execute(self.programmer)?;
        let mut idcode = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

        // Read subsequent IDCODEs
        let request = SequenceBuilder::new().read(32, 0);

        // TODO: How do we handle devices without IDCODE which enter BYPASS?

        // Loop over all the incoming IDCODEs
        while idcode != 0xFFFF_FFFF && idcode != 0x0000_0000 {
            idcodes.push(idcode);
            data = request.clone().execute(self.programmer)?;
            idcode = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        }

        Ok(idcodes)
    }

    /// Scan through and print all IDCODEs on the scan chain
    pub fn print_idcodes(&self) -> Result<()> {
        let idcodes = self.idcodes()?;
        for idcode in idcodes.iter() {
            println!("Read IDCODE: 0x{:08X}", idcode);
        }
        println!("Read {} IDCODEs in total.", idcodes.len());

        Ok(())
    }
}

#[allow(unused)]
pub enum TAPState {
    TestLogicReset,
    RunTestIdle,
    SelectDRScan,
    CaptureDR,
    ShiftDR,
    Exit1DR,
    PauseDR,
    Exit2DR,
    UpdateDR,
    SelectIRScan,
    CaptureIR,
    ShiftIR,
    Exit1IR,
    PauseIR,
    Exit2IR,
    UpdateIR,
}

pub struct TAP<'a> {
    programmer: &'a Programmer,
    state: TAPState,
    idx: usize,
}

impl<'a> TAP<'a> {
    pub fn new(programmer: &'a Programmer, idx: usize) -> Result<Self> {
        programmer.jtag_mode()?;
        SequenceBuilder::new().mode(5, 1).mode(1, 0).execute(programmer)?;
        Ok(Self { programmer, state: TAPState::RunTestIdle, idx })
    }

    pub fn write_ir(&self, data: &[u8], nbits: usize) -> Result<()> {
        assert!(data.len() * 8 >= nbits);
        SequenceBuilder::new()
            .mode(2, 1)     // Select-DR-Scan, Select-IR-Scan
            .mode(2, 0)     // Capture-IR, Shift-IR
            .write(nbits - 1, 0, data)
            .write(1, 1, &[data.last().unwrap() >> 7])
            .mode(1, 1)     // Update-IR
            .mode(1, 0)     // Run-Test/Idle
            .execute(self.programmer)?;
        Ok(())
    }

    pub fn read_dr(&self, nbits: usize) -> Result<Vec<u8>> {
        SequenceBuilder::new()
            .mode(1, 1)     // Select-DR-Scan
            .mode(2, 0)     // Capture-DR, Shift-DR
            .read(nbits, 0)
            .mode(2, 1)     // Exit1-DR, Update-DR
            .mode(1, 0)     // Run-Test/Idle
            .execute(self.programmer)
    }

    pub fn write_dr(&self, data: &[u8], nbits: usize) -> Result<()> {
        assert!(data.len() * 8 >= nbits);
        SequenceBuilder::new()
            .mode(1, 1)     // Select-DR-Scan
            .mode(2, 0)     // Capture-DR, Shift-DR
            .write(nbits - 1, 0, data)
            .write(1, 1, &[data.last().unwrap() >> 7])
            .mode(1, 1)     // Update-DR
            .mode(1, 0)     // Run-Test/Idle
            .execute(self.programmer)?;
        Ok(())
    }

    pub fn run_test_idle(&self, n: usize) -> Result<()> {
        SequenceBuilder::new()
            .mode(n, 0)     // Select-DR-Scan
            .execute(self.programmer)?;
        Ok(())
    }
}

#[derive(Clone)]
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

    pub fn request(mut self, len: usize, tms: u8, tdi: Option<&[u8]>, capture: bool)
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

    pub fn write(self, len: usize, tms: u8, tdi: &[u8]) -> Self {
        self.request(len, tms, Some(tdi), false)
    }

    pub fn mode(self, len: usize, tms: u8) -> Self {
        self.request(len, tms, None, false)
    }

    pub fn read(self, len: usize, tms: u8) -> Self {
        self.request(len, tms, None, true)
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
