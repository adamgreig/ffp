#[allow(non_snake_case)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BTableRow {
    pub ADDR_TX: u16,
    pub COUNT_TX: u16,
    pub ADDR_RX: u16,
    pub COUNT_RX: u16,
}

#[repr(C)]
pub struct EPBuf {
    pub tx: [u16; 32],
    pub rx: [u16; 32],
}

pub const USB_SRAM: u32 = 0x4000_6000;

#[link_section=".usbram"]
pub static mut EP0BUF: EPBuf = EPBuf::new();

#[link_section=".usbram"]
pub static mut EP1BUF: EPBuf = EPBuf::new();

#[link_section=".usbram"]
pub static mut BTABLE: [BTableRow; 8] = [BTableRow::new(); 8];

impl EPBuf {
    pub const fn new() -> Self {
        EPBuf {
            tx: [0u16; 32], rx: [0u16; 32]
        }
    }

    /// Copy `data` into the tx buffer
    pub fn write_tx(&mut self, data: &[u8]) {
        let data_u16 = unsafe {
            core::slice::from_raw_parts(&data[0] as *const _ as *const u16, data.len() / 2)
        };
        for idx in 0..data_u16.len() {
            unsafe { core::ptr::write_volatile(&mut self.tx[idx], data_u16[idx]) };
        }
    }
}

impl BTableRow {
    pub const fn new() -> Self {
        BTableRow { ADDR_TX: 0, COUNT_TX: 0, ADDR_RX: 0, COUNT_RX: 0 }
    }

    /// Set the COUNT_TX field to `n`
    pub fn tx_count(&mut self, n: usize) {
        self.COUNT_TX = n as u16;
    }

    /// Get the current COUNT_RX value
    #[allow(unused)]
    pub fn rx_count(&self) -> usize {
        (self.COUNT_RX & 0x3FF) as usize
    }
}
