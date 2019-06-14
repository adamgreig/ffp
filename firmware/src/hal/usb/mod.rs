// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::usb;
use stm32ral::{read_reg, write_reg, modify_reg};

mod packets;
mod buffers;
mod descriptors;

use packets::{*, ToBytes};
use buffers::*;
use descriptors::*;

use crate::app::{PinState, Mode, Request};
use crate::hal::unique_id::get_hex_id;

/// USB stack interface
pub struct USB {
    usb: usb::Instance,
    btable: &'static mut [BTableRow; 8],
    ep0buf: &'static mut EPBuf,
    ep1buf: &'static mut EPBuf,
    pending_address: Option<u16>,
    pending_request: Option<Request>,
    pending_request_ready: bool,
}

impl USB {
    /// Create a new USB object from the peripheral instance
    pub fn new(usb: usb::Instance) -> Self {
        // UNSAFE: We can only be given a usb::Instance _once_ safely,
        // so if we've been given one, we can also safely take ownership
        // of the static buffers.
        unsafe {
            USB {
                usb,
                btable: &mut BTABLE,
                ep0buf: &mut EP0BUF,
                ep1buf: &mut EP1BUF,
                pending_address: None,
                pending_request: None,
                pending_request_ready: false,
            }
        }
    }

    /// Initialise the USB peripheral ready to start processing packets
    pub fn setup(&mut self) {
        self.power_on_reset();
        self.usb_reset();
        self.attach();
    }

    /// Process a pending USB interrupt.
    ///
    /// Call this function when a USB interrupt occurs.
    ///
    /// Returns Some(Request) if a new request has been received
    /// from the host.
    pub fn interrupt(&mut self) -> Option<Request> {
        let (ctr, susp, wkup, reset, ep_id) =
            read_reg!(usb, self.usb, ISTR, CTR, SUSP, WKUP, RESET, EP_ID);

        if reset == 1 {
            self.usb_reset();
            write_reg!(usb, self.usb, ISTR, CTR: 1, SUSP: 1, WKUP: 1, RESET: 0);
        }

        if ctr == 1 {
            match ep_id {
                0 => self.process_control_ctr(),
                1 => self.process_data_ctr(),
                _ => {},
            }
            write_reg!(usb, self.usb, ISTR, CTR: 0, SUSP: 1, WKUP: 1, RESET: 1);
        }

        if susp == 1 {
            // Put USB peripheral into suspend and low-power mode
            modify_reg!(usb, self.usb, CNTR, FSUSP: Suspend, LPMODE: Enabled);
            write_reg!(usb, self.usb, ISTR, CTR: 1, SUSP: 0, WKUP: 1, RESET: 1);

            // Let the application know we've entered SUSPEND so it
            // can take appropriate action to reduce power consumption
            self.pending_request = Some(Request::Suspend);
            self.pending_request_ready = true;
        }

        if wkup == 1 {
            // Bring USB peripheral out of suspend
            modify_reg!(usb, self.usb, CNTR, FSUSP: 0);
            write_reg!(usb, self.usb, ISTR, CTR: 1, SUSP: 1, WKUP: 0, RESET: 1);
        }

        self.get_request()
    }

    /// Transmit the current tpwr state in response to a recent GetTPwr request
    pub fn reply_tpwr(&mut self, tpwr: PinState) {
        let data = [tpwr as u8, 0];
        self.control_tx_slice(&data[..]);
    }

    /// Transmit a given slice of data out the bulk endpoint
    pub fn reply_data(&mut self, data: &[u8]) {
        self.data_tx_slice(data);
    }

    /// Indicate we can currently receive data
    pub fn enable_data_rx(&mut self) {
        self.data_rx_valid();
    }

    /// Indicate we cannot currently receive data
    pub fn disable_data_rx(&mut self) {
        self.data_rx_stall();
    }

    /// Get any pending request, updating pending_request_ready as appropriate
    fn get_request(&mut self) -> Option<Request> {
        if let Some(req) = self.pending_request {
            if self.pending_request_ready {
                self.pending_request_ready = false;
                self.pending_request = None;
                Some(req)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Process a communication-completed event on EP0
    fn process_control_ctr(&mut self) {
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
    }

    /// Process a communication-completed event on EP1
    fn process_data_ctr(&mut self) {
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
    }

    /// Process transmission complete on EP0
    fn process_control_tx(&mut self) {
        // If we had a pending address change, we must have just sent the STATUS ACK,
        // so apply the new address now and clear the pending change.
        if let Some(addr) = self.pending_address {
            self.set_address(addr);
            self.pending_address = None;
        }

        // If we had a pending bootload, we've now sent the ACK, so
        // it's safe to detach from the bus. The application will
        // reset the device when it next calls `get_request()`.
        if let Some(Request::Bootload) = self.pending_request {
            self.detach();
        }

        // Once transmission is complete we can release any pending requests
        // to the application.
        if self.pending_request.is_some() {
            self.pending_request_ready = true;
        }
    }

    /// Process reception complete on EP0
    fn process_control_rx(&mut self) {
        // Check if we received a SETUP packet
        if read_reg!(usb, self.usb, EP0R, SETUP) == 1 {
            self.process_setup_rx();
        }
        // Resume reception on EP0
        self.control_rx_valid();
    }

    /// Resume reception of new control packets
    fn control_rx_valid(&self) {
        // Indicate we're ready to receive again by setting STAT_RX to VALID
        let (stat_rx, ep_type, ea) = read_reg!(usb, self.usb, EP0R, STAT_RX, EP_TYPE, EA);
        write_reg!(usb, self.usb, EP0R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_RX: Self::stat_valid(stat_rx));
    }

    /// Return the given slice as data
    fn control_tx_slice(&mut self, data: &[u8]) {
        assert!(data.len() <= 64);
        self.ep0buf.write_tx(data);
        self.btable[0].tx_count(data.len());
        self.control_tx_valid();
    }

    /// Indicate a packet has been loaded into the buffer and is ready for transmission
    fn control_tx_valid(&self) {
        let (stat_tx, ep_type, ea) = read_reg!(usb, self.usb, EP0R, STAT_TX, EP_TYPE, EA);
        write_reg!(usb, self.usb, EP0R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_TX: Self::stat_valid(stat_tx));
    }

    /// Set the control endpoint to STALL in both directions
    ///
    /// This indicates an error processing the request to the host,
    /// and will be reset by hardware to NAK on both directions upon
    /// the next SETUP reception.
    fn control_stall(&self) {
        let (stat_tx, stat_rx, ep_type, ea) =
            read_reg!(usb, self.usb, EP0R, STAT_TX, STAT_RX, EP_TYPE, EA);
        write_reg!(usb, self.usb, EP0R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_TX: Self::stat_stall(stat_tx), STAT_RX: Self::stat_stall(stat_rx));
    }

    /// Send a 0-length ACK STATUS response to the next IN transfer
    fn control_tx_ack(&mut self) {
        self.btable[0].tx_count(0);
        self.control_tx_valid();
    }

    /// Process receiving a SETUP packet
    fn process_setup_rx(&mut self) {
        let setup = SetupPID::from_buf(&self.ep0buf);
        match setup.setup_type() {
            // Process standard requests
            SetupType::Standard => match StandardRequest::from_u8(setup.bRequest) {
                Some(StandardRequest::GetDescriptor) => {
                    let [descriptor_index, descriptor_type] = setup.wValue.to_le_bytes();
                    self.process_get_descriptor(
                        setup.wLength, descriptor_type as u8, descriptor_index as u8);
                },
                Some(StandardRequest::GetStatus) => {
                    // Reply with dummy status 0x0000
                    let data = [0u8, 0u8];
                    self.control_tx_slice(&data[..]);
                },
                Some(StandardRequest::SetAddress) => {
                    // Store new address for application after sending STATUS back
                    self.pending_address = Some(setup.wValue);
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
    fn process_get_descriptor(
        &mut self, w_length: u16, descriptor_type: u8, descriptor_index: u8
    ) {
        match DescriptorType::from_u8(descriptor_type) {
            Some(DescriptorType::Device) =>
                self.process_get_device_descriptor(w_length),
            Some(DescriptorType::Configuration) =>
                self.process_get_configuration_descriptor(w_length),

            Some(DescriptorType::String) =>
                self.process_get_string_descriptor(w_length, descriptor_index),

            // Ignore other descriptor types
            _ => self.control_stall(),
        }
    }

    /// Transmit DEVICE descriptor
    fn process_get_device_descriptor(&mut self, w_length: u16) {
        let n = u16::min(DEVICE_DESCRIPTOR.bLength as u16, w_length) as usize;
        let data = DEVICE_DESCRIPTOR.to_bytes();
        self.ep0buf.write_tx(data);
        self.btable[0].tx_count(n);
        self.control_tx_valid();
    }

    /// Transmit CONFIGURATION, INTERFACE, and all ENDPOINT descriptors
    fn process_get_configuration_descriptor(&mut self, w_length: u16) {
        // We need to first copy all the descriptors into a single buffer,
        // as they are not u16-aligned.
        let mut buf = [0u8; 64];

        // Copy CONFIGURATION_DESCRIPTOR into buf
        let n1 = CONFIGURATION_DESCRIPTOR.bLength as usize;
        let data1 = CONFIGURATION_DESCRIPTOR.to_bytes();
        buf[0..n1].copy_from_slice(data1);

        // Copy INTERFACE_DESCRIPTOR into buf
        let n2 = INTERFACE_DESCRIPTOR.bLength as usize;
        let data2 = INTERFACE_DESCRIPTOR.to_bytes();
        buf[n1..n1+n2].copy_from_slice(data2);

        // Copy all ENDPOINT_DESCRIPTORS into buf
        let mut n = n1+n2;
        for ep in ENDPOINT_DESCRIPTORS.iter() {
            let len = ep.bLength as usize;
            let data = ep.to_bytes();
            buf[n..n+len].copy_from_slice(data);
            n += len;
        }

        // Only send as much data as was requested
        let n = usize::min(n, w_length as usize);

        // Copy buf into the actual endpoint buffer
        self.ep0buf.write_tx(&buf[..n]);
        self.btable[0].tx_count(n);

        // Set up transfer
        self.control_tx_valid();
    }

    /// Transmit STRING descriptor
    fn process_get_string_descriptor(&mut self, w_length: u16, idx: u8) {
        // Send a STRING descriptor
        // First construct the descriptor dynamically; we do this so the
        // UTF-8 encoded strings can be stored as statics instead of
        // manually typing out the bytes for UTF-16.
        let desc = match idx {
            // Special case string 0 which is a list of language IDs
            0 => {
                let mut desc = StringDescriptor {
                    bLength: 2 + 2 * STRING_LANGS.len() as u8,
                    bDescriptorType: DescriptorType::String as u8,
                    bString: [0u8; 48],
                };
                // Pack the u16 language codes into the u8 array
                for (idx, lang) in STRING_LANGS.iter().enumerate() {
                    let [u1, u2] = lang.to_le_bytes();
                    desc.bString[idx*2  ] = u1;
                    desc.bString[idx*2+1] = u2;
                }
                desc
            },

            // Handle manufacturer, product, and serial number strings
            1 | 2 | 3 => {
                let id;
                let string = match idx {
                    1 => Ok(STRING_MFN),
                    2 => Ok(STRING_PRD),
                    3 => { id = get_hex_id(); core::str::from_utf8(&id) },
                    _ => unreachable!(),
                };
                let string = match string {
                    Ok(s) => s,
                    Err(_) => {
                        self.control_stall();
                        return;
                    }
                };
                let mut desc = StringDescriptor {
                    bLength: 2 + 2 * string.len() as u8,
                    bDescriptorType: DescriptorType::String as u8,
                    bString: [0u8; 48],
                };
                // Encode the &str to an iter of u16 and pack them
                for (idx, cp) in string.encode_utf16().enumerate() {
                    let [u1, u2] = cp.to_le_bytes();
                    desc.bString[idx*2  ] = u1;
                    desc.bString[idx*2+1] = u2;
                }
                desc
            },

            // Reject any unknown indicies
            _ => {
                self.control_stall();
                return;
            }
        };

        let n = u16::min(desc.bLength as u16, w_length) as usize;

        let data = desc.to_bytes();
        self.ep0buf.write_tx(data);
        self.btable[0].tx_count(n);
        self.control_tx_valid();
    }

    /// Handle a vendor-specific request
    fn process_vendor_request(&mut self, setup: &SetupPID) {
        match VendorRequest::from_u8(setup.bRequest) {
            Some(VendorRequest::SetCS) => {
                match setup.wValue {
                    0 => self.pending_request = Some(Request::SetCS(PinState::Low)),
                    1 => self.pending_request = Some(Request::SetCS(PinState::High)),
                    _ => return self.control_stall(),
                };
                self.control_tx_ack();
            },

            Some(VendorRequest::SetFPGA) => {
                match setup.wValue {
                    0 => self.pending_request = Some(Request::SetFPGA(PinState::Low)),
                    1 => self.pending_request = Some(Request::SetFPGA(PinState::High)),
                    _ => return self.control_stall(),
                };
                self.control_tx_ack();
            },

            Some(VendorRequest::SetMode) => {
                match setup.wValue {
                    0 => self.pending_request = Some(Request::SetMode(Mode::HighImpedance)),
                    1 => self.pending_request = Some(Request::SetMode(Mode::Flash)),
                    2 => self.pending_request = Some(Request::SetMode(Mode::FPGA)),
                    _ => return self.control_stall(),
                };
                self.control_tx_ack();
            },

            Some(VendorRequest::SetTPwr) => {
                match setup.wValue {
                    0 => self.pending_request = Some(Request::SetTPwr(PinState::Low)),
                    1 => self.pending_request = Some(Request::SetTPwr(PinState::High)),
                    _ => return self.control_stall(),
                };
                self.control_tx_ack();
            },

            Some(VendorRequest::SetLED) => {
                match setup.wValue {
                    0 => self.pending_request = Some(Request::SetLED(PinState::Low)),
                    1 => self.pending_request = Some(Request::SetLED(PinState::High)),
                    _ => return self.control_stall(),
                };
                self.control_tx_ack();
            }

            Some(VendorRequest::GetTPwr) => {
                self.pending_request = Some(Request::GetTPwr);
                // We don't ACK this, instead we immediately release the
                // pending request to the application which will call
                // `reply_tpwr()` with the TPwr state, and we transmit that.
                self.pending_request_ready = true;
            },

            Some(VendorRequest::Bootload) => {
                self.pending_request = Some(Request::Bootload);
                self.control_tx_ack();
            },

            // Ignore unknown requests
            _ => {
                self.control_stall();
            },
        }
    }

    /// Process transmission complete on EP1
    fn process_data_tx(&mut self) {
        // If we've got a pending request, we must have just sent an ACK,
        // so release the pending request to the application.
        if self.pending_request.is_some() {
            self.pending_request_ready = true;
        }
    }

    /// Process reception complete on EP1
    fn process_data_rx(&mut self) {
        // Copy the received data
        let mut data = [0u8; 64];
        let n = self.ep1buf.read_rx(&self.btable[1], &mut data);
        self.pending_request = Some(Request::Transmit((data, n)));
        self.pending_request_ready = true;

        // Indicate we're ready to receive again
        self.data_rx_valid();
    }

    /// Resume reception of new data packets
    fn data_rx_valid(&self) {
        // Indicate we're ready to receive again by setting STAT_RX to VALID
        let (stat_rx, ep_type, ea) = read_reg!(usb, self.usb, EP1R, STAT_RX, EP_TYPE, EA);
        write_reg!(usb, self.usb, EP1R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_RX: Self::stat_valid(stat_rx));
    }

    /// Mark data reception as invalid
    fn data_rx_stall(&self) {
        let (stat_rx, ep_type, ea) = read_reg!(usb, self.usb, EP1R, STAT_RX, EP_TYPE, EA);
        write_reg!(usb, self.usb, EP1R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_RX: Self::stat_stall(stat_rx));
    }

    fn data_tx_slice(&mut self, data: &[u8]) {
        assert!(data.len() <= 64);
        self.ep1buf.write_tx(data);
        self.btable[1].tx_count(data.len());
        self.data_tx_valid();
    }

    /// Indicate a packet has been loaded into the buffer and is ready for transmission
    fn data_tx_valid(&self) {
        let (stat_tx, ep_type, ea) = read_reg!(usb, self.usb, EP1R, STAT_TX, EP_TYPE, EA);
        write_reg!(usb, self.usb, EP1R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_TX: Self::stat_valid(stat_tx));

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
    fn power_on_reset(&mut self) {
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
        write_reg!(usb, self.usb, BTABLE, (self.btable as *const _ as u32) - USB_SRAM);
        // Clear ISTR
        write_reg!(usb, self.usb, ISTR, 0);
        // Enable reset masks
        modify_reg!(usb, self.usb, CNTR,
                    CTRM: Enabled, RESETM: Enabled, SUSPM: Enabled, WKUPM: Enabled);
    }

    /// Write the BTABLE descriptors with the addresses and sizes of the available buffers
    fn write_btable(&mut self) {
        self.btable[0].write(&self.ep0buf);
        self.btable[1].write(&self.ep1buf);
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

    /// Enable the D+ pullup to attach to the host
    fn attach(&self) {
        // Enable the DP pull-up to signal attachment to the host
        modify_reg!(usb, self.usb, BCDR, DPPU: Enabled);
    }

    /// Disable the D+ pullup to detach from the host
    fn detach(&self) {
        // Enable the DP pull-up to signal attachment to the host
        modify_reg!(usb, self.usb, BCDR, DPPU: Disabled);
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
        // with STAT_TX to NAK=10 and STAT_RX to Stall=11,
        // and DTOG_TX and DTOG_RX both set to 0.
        let (stat_tx, stat_rx, dtog_rx, dtog_tx) =
            read_reg!(usb, self.usb, EP1R, STAT_TX, STAT_RX, DTOG_RX, DTOG_TX);
        write_reg!(usb, self.usb, EP1R,
                   CTR_RX: 1, EP_TYPE: Bulk, EP_KIND: 0, CTR_TX: 1, EA: 1,
                   DTOG_RX: dtog_rx, DTOG_TX: dtog_tx,
                   STAT_TX: Self::stat_nak(stat_tx), STAT_RX: Self::stat_stall(stat_rx));

        // Ensure all other endpoints are disabled by writing their current
        // values of STAT_TX/STAT_RX, setting them to 00 (disabled)
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
