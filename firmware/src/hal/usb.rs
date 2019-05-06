use stm32ral::usb;
use stm32ral::{RWRegister, read_reg, write_reg, modify_reg};

#[allow(non_snake_case)]
#[repr(C)]
struct BTable {
    ADDR_TX: RWRegister<u16>,
    COUNT_TX: RWRegister<u16>,
    ADDR_RX: RWRegister<u16>,
    COUNT_RX: RWRegister<u16>,
}

#[repr(C)]
struct EPBuf64 {
    tx: [u8; 64],
    rx: [u8; 64],
}

#[repr(C)]
struct EPBuf256 {
    tx: [u8; 256],
    rx: [u8; 256],
}

const USB_SRAM: u32 = 0x4000_6000;
const EP1BUF: *mut EPBuf256 = USB_SRAM as *mut _;
const EP0BUF: *mut EPBuf64 = (USB_SRAM + 512) as *mut _;
const BTABLE: *const [BTable; 8] = (USB_SRAM + 512 + 128) as *const _;

pub struct USB {
    usb: usb::Instance,
}

impl USB {
    /// Create a new USB object from the peripheral instance
    pub fn new(usb: usb::Instance) -> Self {
        USB { usb }
    }

    /// Unsafely force creation of a new USB object
    pub unsafe fn steal() -> Self {
        USB { usb: usb::USB::steal() }
    }

    /// Initialise the USB peripheral ready to start processing packets
    pub fn setup(&self) {
        self.power_on_reset();
        self.write_btable();
        self.usb_reset();
        self.configure_endpoints(0);
    }

    /// Call this function when a USB interrupt occurs.
    ///
    /// Note that the caller is responsible for managing the NVIC including clearing
    /// the pending bit.
    pub fn interrupt(&self) {
        let (ctr, reset, ep_id) = read_reg!(usb, self.usb, ISTR, CTR, RESET, EP_ID);
        if reset == 1 {
            self.usb_reset();
        } else if ctr == 1 {
            self.ctr(ep_id as u8);
        }
    }

    fn ctr(&self, ep_id: u8) {
        match ep_id {
            0 => {
                let (ctr_tx, ctr_rx) = read_reg!(usb, self.usb, EP0R, CTR_TX, CTR_RX);
                if ctr_tx == 1 {
                    modify_reg!(usb, self.usb, EP0R, CTR_TX: 0);
                    self.process_control_tx();
                }
                if ctr_rx == 1 {
                    modify_reg!(usb, self.usb, EP0R, CTR_RX: 0);
                    self.process_control_rx();
                }
            },
            1 => {
                let (ctr_tx, ctr_rx) = read_reg!(usb, self.usb, EP0R, CTR_TX, CTR_RX);
                if ctr_tx == 1 {
                    modify_reg!(usb, self.usb, EP0R, CTR_TX: 0);
                    self.process_data_tx();
                }
                if ctr_rx == 1 {
                    modify_reg!(usb, self.usb, EP0R, CTR_RX: 0);
                    self.process_data_rx();
                }
            },
            _ => (),
        }
    }

    fn process_control_tx(&self) {
    }

    fn process_control_rx(&self) {
        // Indicate we're ready to receive again
        let stat_rx = read_reg!(usb, self.usb, EP0R, STAT_RX);
        modify_reg!(usb, self.usb, EP0R, STAT_RX: Self::toggle_valid(stat_rx));
    }

    fn process_data_tx(&self) {
    }

    fn process_data_rx(&self) {
        // Indicate we're ready to receive again
        let stat_rx = read_reg!(usb, self.usb, EP1R, STAT_RX);
        modify_reg!(usb, self.usb, EP1R, STAT_RX: Self::toggle_valid(stat_rx));
    }

    fn toggle_disabled(stat: u32) -> u32 {
        (!stat & 0b10) | (!stat & 0b01)
    }

    fn toggle_stall(stat: u32) -> u32 {
        (stat & 0b10) | (!stat & 0b01)
    }

    fn toggle_nak(stat: u32) -> u32 {
        (!stat & 0b10) | (stat & 0b01)
    }

    fn toggle_valid(stat: u32) -> u32 {
        (stat & 0b10) | (stat & 0b01)
    }

    pub fn power_on_reset(&self) {
        // Activate analog power supply while transceiver is in reset
        modify_reg!(usb, self.usb, CNTR, PDWN: Disabled, FRES: Reset);
        // Wait t_STARTUP (1µs)
        cortex_m::asm::delay(48);
        // Bring USB transceiver out of reset
        modify_reg!(usb, self.usb, CNTR, PDWN: Disabled, FRES: NoReset);
        // Set buffer table to start at BTABLE.
        // We write the entire register to avoid dealing with the shifted-by-3 field.
        write_reg!(usb, self.usb, BTABLE, (BTABLE as u32) - USB_SRAM);
        // Clear ISTR
        write_reg!(usb, self.usb, ISTR, 0);
        // Enable reset masks
        modify_reg!(usb, self.usb, CNTR, CTRM: Enabled, RESETM: Enabled);
    }

    fn write_btable(&self) {
        unsafe {
            (*BTABLE)[0].ADDR_TX.write((&(*EP0BUF).tx as *const _ as u32 - USB_SRAM) as u16);
            (*BTABLE)[0].ADDR_RX.write((&(*EP0BUF).rx as *const _ as u32 - USB_SRAM) as u16);
            (*BTABLE)[0].COUNT_RX.write((1<<15) | (64 / 32) << 10);
            (*BTABLE)[1].ADDR_TX.write((&(*EP1BUF).tx as *const _ as u32 - USB_SRAM) as u16);
            (*BTABLE)[1].ADDR_RX.write((&(*EP1BUF).rx as *const _ as u32 - USB_SRAM) as u16);
            (*BTABLE)[0].COUNT_RX.write((1<<15) | (256 / 32) << 10);
        }
    }

    pub fn usb_reset(&self) {
        // Ensure peripheral will not respond while we set up endpoints
        modify_reg!(usb, self.usb, DADDR, EF: Disabled);

        // Clear ISTR
        write_reg!(usb, self.usb, ISTR, 0);

        // Set up EP0R to handle default control endpoint.
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP0R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP0R,
                   CTR_RX: 0, EP_TYPE: Control, EP_KIND: 0, CTR_TX: 0, EA: 0,
                   STAT_TX: Self::toggle_nak(stat_tx), STAT_RX: Self::toggle_valid(stat_rx));

        // Ensure all other endpoints are disabled by writing their current
        // values of STAT_TX/STAT_RX, setting them to 00 (disabled)
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP1R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP1R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP2R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP2R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP3R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP3R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP4R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP4R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP5R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP5R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP6R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP6R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP7R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP7R, STAT_TX: stat_tx, STAT_RX: stat_rx);

        // Set EF=1 with address 0 to enable processing incoming packets
        modify_reg!(usb, self.usb, DADDR, ADD: 0, EF: Enabled);
    }

    pub fn configure_endpoints(&self, address: u8) {
        // Ensure peripheral will not respond while we set up endpoints
        modify_reg!(usb, self.usb, DADDR, EF: Disabled);

        // Set up EP0R to handle default control endpoint.
        // Note STAT_TX/STAT_RX bits are write-1-to-toggle, write-0-to-leave-unchanged,
        // we want to set STAT_RX to Valid=11 and STAT_TX to Nak=10.
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP0R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP0R,
                   CTR_RX: 0, EP_TYPE: Control, EP_KIND: 0, CTR_TX: 0, EA: 0,
                   STAT_TX: Self::toggle_nak(stat_tx), STAT_RX: Self::toggle_valid(stat_rx));

        // Set up EP1R to be a bidirectional interrupt endpoint,
        // with STAT_TX to NAK=10 and STAT_RX to Valid=11
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP1R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP1R,
                   CTR_RX: 0, EP_TYPE: Interrupt, EP_KIND: 0, CTR_TX: 0, EA: 0,
                   STAT_TX: Self::toggle_nak(stat_tx), STAT_RX: Self::toggle_valid(stat_rx));

        // Ensure all other endpoints are disabled by writing their current
        // values of STAT_TX/STAT_RX, setting them to 00 (disabled)
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP1R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP1R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP2R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP2R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP3R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP3R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP4R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP4R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP5R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP5R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP6R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP6R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP7R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP7R, STAT_TX: stat_tx, STAT_RX: stat_rx);

        // Set EF=1 with address 0 to enable processing incoming packets
        modify_reg!(usb, self.usb, DADDR, ADD: address as u32, EF: Enabled);
    }
}
