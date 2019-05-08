use stm32ral::usb;
use stm32ral::{read_reg, write_reg, modify_reg};

mod packets;
mod buffers;
mod descriptors;

use packets::*;
use buffers::*;
use descriptors::*;

use super::gpio;

/// Store persistent state for USB stack
struct State {
    pending_address: Option<u16>,
}

impl State {
    const fn new() -> Self {
        State { pending_address: None }
    }
}

static mut STATE: State = State::new();

/// USB stack interface
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
        self.usb_reset();
        self.attach();
    }

    /// Call this function when a USB interrupt occurs.
    pub fn interrupt(&self) {
        let (ctr, susp, wkup, reset, ep_id) =
            read_reg!(usb, self.usb, ISTR, CTR, SUSP, WKUP, RESET, EP_ID);
        if reset == 1 {
            self.usb_reset();
        } else if ctr == 1 {
            self.ctr(ep_id as u8);
        } else if susp == 1 {
            // Turn off status LED
            let gpioa = unsafe { gpio::GPIO::new(stm32ral::gpio::GPIOA::steal()) };
            gpioa.clear(2);
            // Put USB peripheral into suspend and low-power mode
            modify_reg!(usb, self.usb, CNTR, FSUSP: Suspend, LPMODE: Enabled);
        } else if wkup == 1 {
            // Turn back on status LED
            let gpioa = unsafe { gpio::GPIO::new(stm32ral::gpio::GPIOA::steal()) };
            gpioa.set(2);
            // Bring USB peripheral out of suspend
            modify_reg!(usb, self.usb, CNTR, FSUSP: 0);
        }
        write_reg!(usb, self.usb, ISTR, 0);
    }

    /// Process a communication-completed event by EP `ep_id`
    fn ctr(&self, ep_id: u8) {
        match ep_id {
            // Handle events on control EP0
            0 => {
                let (ctr_tx, ctr_rx, ep_type, ea) =
                    read_reg!(usb, self.usb, EP0R, CTR_TX, CTR_RX, EP_TYPE, EA);
                if ctr_tx == 1 {
                    self.process_control_tx();
                    // Clear CTR_TX
                    write_reg!(usb, self.usb, EP0R,
                               CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 0, EA: ea);
                }
                if ctr_rx == 1 {
                    self.process_control_rx();
                    // Clear CTR_RX
                    write_reg!(usb, self.usb, EP0R,
                               CTR_RX: 0, EP_TYPE: ep_type, CTR_TX: 1, EA: ea);
                }
            },

            // Handle events on data EP1
            1 => {
                let (ctr_tx, ctr_rx, ep_type, ea) =
                    read_reg!(usb, self.usb, EP1R, CTR_TX, CTR_RX, EP_TYPE, EA);
                if ctr_tx == 1 {
                    self.process_data_tx();
                    // Clear CTR_TX
                    write_reg!(usb, self.usb, EP1R,
                               CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 0, EA: ea);
                }
                if ctr_rx == 1 {
                    self.process_data_rx();
                    // Clear CTR_RX
                    write_reg!(usb, self.usb, EP1R,
                               CTR_RX: 0, EP_TYPE: ep_type, CTR_TX: 1, EA: ea);
                }
            },

            // Ignore any other EPs
            _ => (),
        }
    }

    /// Process transmission complete on EP0
    fn process_control_tx(&self) {
        // If we had a pending address change, we must have just sent the STATUS ACK,
        // so apply the new address now and clear the pending change.
        match unsafe { STATE.pending_address } {
            Some(addr) => {
                self.set_address(addr);
                unsafe { STATE.pending_address = None };
            },
            None => (),
        }
    }

    /// Process reception complete on EP0
    fn process_control_rx(&self) {
        // Check if we received a SETUP packet
        if read_reg!(usb, self.usb, EP0R, SETUP) == 1 {
            self.process_setup_rx();
        }
        // Resume reception on EP0
        self.control_rx_ack();
    }

    /// Resume reception of new packets
    fn control_rx_ack(&self) {
        // Indicate we're ready to receive again by setting STAT_RX to VALID
        let stat_rx = read_reg!(usb, self.usb, EP0R, STAT_RX);
        write_reg!(usb, self.usb, EP0R, CTR_RX: 1, EP_TYPE: Control, CTR_TX: 1, EA: 0,
                   STAT_RX: Self::stat_valid(stat_rx));
    }

    /// Indicate a packet has been loaded into the buffer and is ready for transmission
    fn control_tx_valid(&self) {
        let stat_tx = read_reg!(usb, self.usb, EP0R, STAT_TX);
        write_reg!(usb, self.usb, EP0R, CTR_RX: 1, EP_TYPE: Control, CTR_TX: 1, EA: 0,
                   STAT_TX: Self::stat_valid(stat_tx));
    }

    /// Set the control endpoint to STALL in both directions
    ///
    /// This indicates an error processing the request to the host,
    /// and will be reset by hardware to NAK on both directions upon
    /// the next SETUP reception.
    fn control_stall(&self) {
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP0R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP0R, CTR_RX: 1, EP_TYPE: Control, CTR_TX: 1, EA: 0,
                   STAT_TX: Self::stat_stall(stat_tx), STAT_RX: Self::stat_stall(stat_rx));
    }

    /// Send a 0-length ACK STATUS response to the next IN transfer
    fn control_tx_ack(&self) {
        unsafe { BTABLE[0].tx_count(0) };
        self.control_tx_valid();
    }

    /// Process receiving a SETUP packet
    fn process_setup_rx(&self) {
        let setup = unsafe { SetupPID::from_buf(&EP0BUF) };
        match setup.setup_type() {
            // Process standard requests
            SetupType::Standard => match StandardRequest::from_u8(setup.bRequest) {
                Some(StandardRequest::GetDescriptor) => {
                    let descriptor_type = setup.wValue >> 8;
                    let descriptor_index = setup.wValue & 0xFF;
                    self.process_get_descriptor(
                        setup.wLength, descriptor_type as u8, descriptor_index as u8);
                },
                Some(StandardRequest::GetStatus) => {
                    // Reply with dummy status 0x0000
                    let data = [0u8, 0u8];
                    unsafe {
                        EP0BUF.write_tx(&data[..]);
                        BTABLE[0].tx_count(data.len());
                    }
                    self.control_tx_valid();
                },
                Some(StandardRequest::SetAddress) => {
                    // Store new address for application after sending STATUS back
                    unsafe { STATE.pending_address = Some(setup.wValue) };
                    self.control_tx_ack();
                },
                Some(StandardRequest::SetConfiguration) => {
                    // Apply requested configuration
                    match setup.wValue {
                        0 => self.usb_reset(),
                        1 => self.set_configuration(),
                        _ => {},
                    }
                    self.control_tx_ack();
                },
                _ => {
                    // Reject unknown requests
                    self.control_stall();
                },
            },

            // Process vendor-specific requests
            SetupType::Vendor => self.process_vendor_request(&setup),

            // Ignore unknown request types
            _ => {
                self.control_stall();
            },
        }
    }

    /// Handle a GET_DESCRIPTOR request
    fn process_get_descriptor(&self, w_length: u16, descriptor_type: u8, descriptor_index: u8) {
        match DescriptorType::from_u8(descriptor_type) {
            Some(DescriptorType::Device) => {
                // Send DEVICE_DESCRIPTOR
                let n = u16::min(DEVICE_DESCRIPTOR.bLength as u16, w_length) as usize;
                unsafe {
                    let data = core::slice::from_raw_parts(
                        &DEVICE_DESCRIPTOR as *const _ as u32 as *const u8, n);
                    EP0BUF.write_tx(data);
                    BTABLE[0].tx_count(n);
                }
                self.control_tx_valid();
            },

            Some(DescriptorType::Configuration) => {
                // Send CONFIGURATION_DESCRIPTOR, INTERFACE_DESCRIPTOR,
                // and all ENDPOINT_DESCRIPTORS

                // We need to first copy all the descriptors into a single buffer,
                // as they are not u16-aligned.
                let mut buf = [0u8; 64];

                // Copy CONFIGURATION_DESCRIPTOR into buf
                let n1 = CONFIGURATION_DESCRIPTOR.bLength as usize;
                let data1 = unsafe { core::slice::from_raw_parts(
                    &CONFIGURATION_DESCRIPTOR as *const _ as u32 as *const u8, n1)};
                buf[0..n1].copy_from_slice(data1);

                // Copy INTERFACE_DESCRIPTOR into buf
                let n2 = INTERFACE_DESCRIPTOR.bLength as usize;
                let data2 = unsafe { core::slice::from_raw_parts(
                    &INTERFACE_DESCRIPTOR as *const _ as u32 as *const u8, n2)};
                buf[n1..n1+n2].copy_from_slice(data2);

                // Copy all ENDPOINT_DESCRIPTORS into buf
                let mut n = n1+n2;
                for ep in ENDPOINT_DESCRIPTORS.iter() {
                    let len = ep.bLength as usize;
                    let data = unsafe { core::slice::from_raw_parts(
                        ep as *const _ as u32 as *const u8, len)};
                    buf[n..n+len].copy_from_slice(data);
                    n += len;
                }

                // Only send as much data as was requested
                let n = usize::min(n, w_length as usize);

                // Copy buf into the actual endpoint buffer
                unsafe {
                    EP0BUF.write_tx(&buf[..n]);
                    BTABLE[0].tx_count(n);
                }

                // Set up transfer
                self.control_tx_valid();
            },

            Some(DescriptorType::String) => {
                // Send a STRING descriptor
                let idx = descriptor_index as usize;
                let n = u16::min(STRING_DESCRIPTORS[idx].bLength as u16, w_length) as usize;
                unsafe {
                    let data = core::slice::from_raw_parts(
                        &STRING_DESCRIPTORS[idx] as *const _ as u32 as *const u8, n);
                    EP0BUF.write_tx(data);
                    BTABLE[0].tx_count(n);
                }
                self.control_tx_valid();
            }

            // Ignore other descriptor types
            _ => {
                self.control_stall();
            },
        }
    }

    /// Handle a vendor-specific request
    fn process_vendor_request(&self, setup: &SetupPID) {
        match VendorRequest::from_u8(setup.bRequest) {
            Some(VendorRequest::SetCS) => {
                let gpioa = unsafe { gpio::GPIO::new(stm32ral::gpio::GPIOA::steal()) };
                if setup.wValue == 1 {
                    gpioa.set(2);
                } else {
                    gpioa.clear(2);
                }
                self.control_tx_ack();
            },

            // Ignore unknown requests
            _ => {
                self.control_stall();
            },
        }
    }

    /// Process transmission complete on EP1
    fn process_data_tx(&self) {
    }

    /// Process reception complete on EP1
    fn process_data_rx(&self) {
        // Indicate we're ready to receive again
        let stat_rx = read_reg!(usb, self.usb, EP1R, STAT_RX);
        write_reg!(usb, self.usb, EP1R, CTR_RX: 1, EP_TYPE: Bulk, CTR_TX: 1, EA: 1,
                   STAT_RX: Self::stat_valid(stat_rx));
    }

    /// Return the bit pattern to write to a STAT field to update it to DISABLED
    fn stat_disabled(stat: u32) -> u32 {
        (stat & 0b10) | (stat & 0b01)
    }

    /// Return the bit pattern to write to a STAT field to update it to STALL
    fn stat_stall(stat: u32) -> u32 {
        (stat & 0b10) | (!stat & 0b01)
    }

    /// Return the bit pattern to write to a STAT field to update it to NAK
    fn stat_nak(stat: u32) -> u32 {
        (!stat & 0b10) | (stat & 0b01)
    }

    /// Return the bit pattern to write to a STAT field to update it to VALID
    fn stat_valid(stat: u32) -> u32 {
        (!stat & 0b10) | (!stat & 0b01)
    }

    /// Apply the power-on reset sequence
    ///
    /// Resets the USB peripheral and activates it.
    /// Does not enable any endpoints; call `usb_reset()` after `power_on_reset()`.
    fn power_on_reset(&self) {
        // Activate analog power supply while transceiver is in reset
        modify_reg!(usb, self.usb, CNTR, PDWN: Disabled, FRES: Reset);
        // Wait t_STARTUP (1µs)
        cortex_m::asm::delay(48);
        // Bring USB transceiver out of reset
        modify_reg!(usb, self.usb, CNTR, PDWN: Disabled, FRES: NoReset);
        // Ensure we remain nonresponsive to requests
        write_reg!(usb, self.usb, DADDR, EF: Disabled);
        // Write the buffer table descriptor
        self.write_btable();
        // Set buffer table to start at BTABLE.
        // We write the entire register to avoid dealing with the shifted-by-3 field.
        unsafe { write_reg!(usb, self.usb, BTABLE, (&BTABLE as *const _ as u32) - USB_SRAM) };
        // Clear ISTR
        write_reg!(usb, self.usb, ISTR, 0);
        // Enable reset masks
        modify_reg!(usb, self.usb, CNTR,
                    CTRM: Enabled, RESETM: Enabled, SUSPM: Enabled, WKUPM: Enabled);
    }

    /// Write the BTABLE descriptor with the addresses and sizes of the available buffers
    fn write_btable(&self) {
        unsafe {
            BTABLE[0].ADDR_TX = (&EP0BUF.tx as *const _ as u32 - USB_SRAM) as u16;
            BTABLE[0].ADDR_RX = (&EP0BUF.rx as *const _ as u32 - USB_SRAM) as u16;
            BTABLE[0].COUNT_TX = 0;
            BTABLE[0].COUNT_RX = (1<<15) | (64 / 32) << 10;
            BTABLE[1].ADDR_TX = (&EP1BUF.tx as *const _ as u32 - USB_SRAM) as u16;
            BTABLE[1].ADDR_RX = (&EP1BUF.rx as *const _ as u32 - USB_SRAM) as u16;
            BTABLE[1].COUNT_TX = 0;
            BTABLE[1].COUNT_RX = (1<<15) | (256 / 32) << 10;
        }
    }

    /// Put device into USB_RESET state
    ///
    /// Respond to address 0 on EP0 only
    fn usb_reset(&self) {
        // Ensure peripheral will not respond while we set up endpoints
        write_reg!(usb, self.usb, DADDR, EF: Disabled);

        // Clear ISTR
        write_reg!(usb, self.usb, ISTR, 0);

        // Set up EP0R to handle default control endpoint
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP0R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP0R,
                   CTR_RX: 0, EP_TYPE: Control, EP_KIND: 0, CTR_TX: 0, EA: 0,
                   STAT_TX: Self::stat_nak(stat_tx), STAT_RX: Self::stat_valid(stat_rx));

        // Ensure all other endpoints are disabled
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP1R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP1R,
                   STAT_TX: Self::stat_disabled(stat_tx), STAT_RX: Self::stat_disabled(stat_rx));
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP2R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP2R,
                   STAT_TX: Self::stat_disabled(stat_tx), STAT_RX: Self::stat_disabled(stat_rx));
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP3R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP3R,
                   STAT_TX: Self::stat_disabled(stat_tx), STAT_RX: Self::stat_disabled(stat_rx));
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP4R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP4R,
                   STAT_TX: Self::stat_disabled(stat_tx), STAT_RX: Self::stat_disabled(stat_rx));
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP5R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP5R,
                   STAT_TX: Self::stat_disabled(stat_tx), STAT_RX: Self::stat_disabled(stat_rx));
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP6R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP6R,
                   STAT_TX: Self::stat_disabled(stat_tx), STAT_RX: Self::stat_disabled(stat_rx));
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP7R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP7R,
                   STAT_TX: Self::stat_disabled(stat_tx), STAT_RX: Self::stat_disabled(stat_rx));

        // Set EF=1 with address 0 to enable processing incoming packets
        write_reg!(usb, self.usb, DADDR, ADD: 0, EF: Enabled);
    }

    /// Enable the D+ pullup to attach to host
    fn attach(&self) {
        // Enable the DP pull-up to signal attachment to the host
        modify_reg!(usb, self.usb, BCDR, DPPU: Enabled);
    }

    /// Apply specified address to device
    fn set_address(&self, address: u16) {
        modify_reg!(usb, self.usb, DADDR, ADD: address as u32);
    }

    /// Set default operational configuration
    ///
    /// Responds to control on EP0 and bidirectional bulk on EP1
    fn set_configuration(&self) {
        // Set up EP1R to be a bidirectional bulk endpoint,
        // with STAT_TX to NAK=10 and STAT_RX to Valid=11,
        // and DTOG_TX and DTOG_RX both set to 0.
        let (stat_tx, stat_rx, dtog_rx, dtog_tx) =
            read_reg!(usb, self.usb, EP1R, STAT_TX, STAT_RX, DTOG_RX, DTOG_TX);
        write_reg!(usb, self.usb, EP1R,
                   CTR_RX: 1, EP_TYPE: Bulk, EP_KIND: 0, CTR_TX: 1, EA: 1,
                   DTOG_RX: dtog_rx, DTOG_TX: dtog_tx,
                   STAT_TX: Self::stat_nak(stat_tx), STAT_RX: Self::stat_valid(stat_rx));

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
    }
}
