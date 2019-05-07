use stm32ral::RWRegister;
use super::packets::*;

#[allow(non_snake_case)]
#[repr(C)]
pub struct BTable {
    pub ADDR_TX: RWRegister<u16>,
    pub COUNT_TX: RWRegister<u16>,
    pub ADDR_RX: RWRegister<u16>,
    pub COUNT_RX: RWRegister<u16>,
}

#[repr(C)]
pub struct ControlEPBuf {
    pub tx: [u8; 64],
    pub rx: [u8; 64],
}

#[repr(C)]
pub struct BulkEPBuf {
    pub tx: [u8; 256],
    pub rx: [u8; 256],
}

pub const USB_SRAM: u32 = 0x4000_6000;
pub const EP1BUF: *mut BulkEPBuf = USB_SRAM as *mut _;
pub const EP0BUF: *mut ControlEPBuf = (USB_SRAM + 512) as *mut _;
pub const BTABLE: *const [BTable; 8] = (USB_SRAM + 512 + 128) as *const _;

impl ControlEPBuf {
    /// Copy `data` into the tx buffer
    pub fn write_tx(&mut self, data: &[u8]) {
        self.tx[..data.len()].copy_from_slice(data);
    }
}

impl BulkEPBuf {
    /// Copy `data` into the tx buffer
    pub fn write_tx(&mut self, data: &[u8]) {
        self.tx[..data.len()].copy_from_slice(data);
    }
}

impl BTable {
    /// Set the COUNT_TX field to `n`
    pub fn tx_count(&self, n: usize) {
        self.COUNT_TX.write(n as u16);
    }

    /// Get the current COUNT_RX value
    pub fn rx_count(&self) -> usize {
        (self.COUNT_RX.read() & 0x3FF) as usize
    }
}
