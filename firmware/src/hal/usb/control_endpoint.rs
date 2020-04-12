// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use core::convert::TryFrom;

use stm32ral::usb;
use stm32ral::{read_reg, write_reg};

use super::{USBStackRequest, Endpoint, stat_stall, stat_nak, stat_valid};
use super::packets::{*, ToBytes};
use super::buffers::*;
use super::descriptors::*;

use crate::app::{PinState, Mode, Request};
use crate::hal::unique_id::get_hex_id;

/// USB handling code for control endpoint
pub(super) struct ControlEndpoint {
    epbuf: &'static mut EPBuf,
    btable: &'static mut BTableRow,
    pending_request: Option<USBStackRequest>,
    pending_tx: Option<(usize, usize)>,
    pending_tx_buf: [u8; 256],
}

impl ControlEndpoint {

    /// Handle transmission completion.
    ///
    /// Either we still have more data to transmit, in which case we prepare
    /// remaining data for transmission, or we have finished transmitting
    /// an acknowledgement and can now process an incoming request.
    fn process_tx_complete(&mut self, usb: &usb::Instance) -> Option<USBStackRequest> {
        // If we have more data to transmit, enqueue it
        if self.pending_tx.is_some() {
            self.transmit_next(usb);
        }

        // For many requests, especially including SET_ADDRESS, SET_CONFIGURATION,
        // or bootload, we have to ensure the response ACK has been transmitted
        // before the request is processed. The ControlEndpoint stores the request
        // and releases it to the stack after the ACK is sent.
        core::mem::take(&mut self.pending_request)
    }

    /// Handle reception completion.
    ///
    /// This typically fires after receiving a SETUP packet containing a request.
    fn process_rx_complete(&mut self, usb: &usb::Instance) -> Option<USBStackRequest> {
        let stack_request;
        if read_reg!(usb, usb, EP0R, SETUP) == 1 {
            stack_request = self.process_setup(usb);
        } else {
            stack_request = None;
        }

        // Resume reception on EP0
        self.rx_valid(usb);

        stack_request
    }

    /// Process receiving a SETUP packet.
    ///
    /// This may be a StandardRequest from the USB spec, or a vendor-specific
    /// request for our own application.
    fn process_setup(&mut self, usb: &usb::Instance) -> Option<USBStackRequest> {
        let setup = SetupPID::from_buf(&self.epbuf);
        match setup.setup_type() {
            // Process standard requests
            SetupType::Standard => match StandardRequest::try_from(setup.bRequest) {
                Ok(StandardRequest::GetDescriptor) => {
                    let [descriptor_index, descriptor_type] = setup.wValue.to_le_bytes();
                    self.process_get_descriptor(
                        usb, setup.wLength, descriptor_type as u8, descriptor_index as u8);
                    None
                },
                Ok(StandardRequest::GetStatus) => {
                    // Reply with dummy status 0x0000
                    let data = [0u8, 0u8];
                    self.transmit_slice(usb, &data[..]);
                    None
                },
                Ok(StandardRequest::SetAddress) => {
                    // Store new address to apply after ACK is sent
                    self.pending_request = Some(USBStackRequest::SetAddress(setup.wValue));
                    self.transmit_ack(usb);
                    None
                },
                Ok(StandardRequest::SetConfiguration) => {
                    // Apply requested configuration after ACK is sent
                    self.pending_request = match setup.wValue {
                        0 => Some(USBStackRequest::Reset),
                        1 => Some(USBStackRequest::SetConfiguration),
                        _ => None,
                    };
                    self.transmit_ack(usb);
                    None
                },
                _ => {
                    // Reject unknown requests
                    self.stall(usb);
                    None
                },
            },

            // Process vendor-specific requests
            SetupType::Vendor => self.process_vendor_request(usb, &setup),

            // Ignore unknown request types
            _ => { self.stall(usb); None },
        }
    }

    /// Send a 0-length ACK STATUS response to the next IN transfer
    fn transmit_ack(&mut self, usb: &usb::Instance) {
        self.btable.tx_count(0);
        self.tx_valid(usb);
    }

    fn transmit_next(&mut self, usb: &usb::Instance) {
        if let Some((idx, len)) = self.pending_tx {
            if len < 64 {
                // When there's less than the maximum packet size remaining,
                // immediately send all remaining data.
                // This also includes the case where len==0 and we send a ZLP.
                self.epbuf.write_tx(&self.pending_tx_buf[idx..idx+len]);
                self.btable.tx_count(len);
                self.pending_tx = None;
            } else {
                // For 64 or more bytes remaining, transmit next packet
                // and adjust pending_tx.
                self.epbuf.write_tx(&self.pending_tx_buf[idx..idx+64]);
                self.btable.tx_count(64);
                self.pending_tx = Some((idx+64, len-64));
            }
            self.tx_valid(usb);
        }
    }

    /// Indicate a packet has been loaded into the buffer and is ready for transmission
    fn tx_valid(&mut self, usb: &usb::Instance) {
        let (stat_tx, ep_type, ea) = read_reg!(usb, usb, EP0R, STAT_TX, EP_TYPE, EA);
        write_reg!(usb, usb, EP0R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_TX: stat_valid(stat_tx));
    }

    /// Set the control endpoint to STALL in both directions
    ///
    /// This indicates an error processing the request to the host,
    /// and will be reset by hardware to NAK on both directions upon
    /// the next SETUP reception.
    fn stall(&mut self, usb: &usb::Instance) {
        let (stat_tx, stat_rx, ep_type, ea) =
            read_reg!(usb, usb, EP0R, STAT_TX, STAT_RX, EP_TYPE, EA);
        write_reg!(usb, usb, EP0R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_TX: stat_stall(stat_tx), STAT_RX: stat_stall(stat_rx));
    }

    /// Handle a GET_DESCRIPTOR request
    fn process_get_descriptor(
        &mut self, usb: &usb::Instance,
        w_length: u16, descriptor_type: u8, descriptor_index: u8
    ) {
        match DescriptorType::try_from(descriptor_type) {
            Ok(DescriptorType::Device) =>
                self.process_get_device_descriptor(usb, w_length),
            Ok(DescriptorType::Configuration) =>
                self.process_get_configuration_descriptor(usb, w_length),
            Ok(DescriptorType::String) =>
                self.process_get_string_descriptor(usb, w_length, descriptor_index),
            Ok(DescriptorType::HIDReport) =>
                self.process_get_hid_report_descriptor(usb, w_length, descriptor_index),

            // Ignore other descriptor types
            _ => self.stall(usb),
        }
    }

    /// Transmit DEVICE descriptor
    fn process_get_device_descriptor(&mut self, usb: &usb::Instance, w_length: u16) {
        let n = u16::min(DEVICE_DESCRIPTOR.bLength as u16, w_length) as usize;
        let data = DEVICE_DESCRIPTOR.to_bytes();
        self.transmit_slice(usb, &data[..n]);
    }

    /// Transmit CONFIGURATION, INTERFACE, and all ENDPOINT descriptors
    fn process_get_configuration_descriptor(&mut self, usb: &usb::Instance, w_length: u16) {
        // We need to first copy all the descriptors into a single buffer,
        // as they are not u16-aligned. Helpfully our descriptors add up
        // to exactly 64 bytes, the maximum we can send in one transfer.
        // Previously this code implemented multiple transfers for larger
        // descriptors but it's no longer required.
        let mut buf = [0u8; 64];
        let mut n = 0;

        // Copy CONFIGURATION_DESCRIPTOR into buf
        let len = CONFIGURATION_DESCRIPTOR.bLength as usize;
        let data = CONFIGURATION_DESCRIPTOR.to_bytes();
        buf[n..n+len].copy_from_slice(data);
        n += len;

        // Copy SPI_INTERFACE_DESCRIPTOR into buf
        let len = SPI_INTERFACE_DESCRIPTOR.bLength as usize;
        let data = SPI_INTERFACE_DESCRIPTOR.to_bytes();
        buf[n..n+len].copy_from_slice(data);
        n += len;

        // Copy all SPI_ENDPOINT_DESCRIPTORS into buf
        for ep in SPI_ENDPOINT_DESCRIPTORS.iter() {
            let len = ep.bLength as usize;
            let data = ep.to_bytes();
            buf[n..n+len].copy_from_slice(data);
            n += len;
        }

        // Copy DAP_INTERFACE_DESCRIPTOR into buf
        let len = DAP_INTERFACE_DESCRIPTOR.bLength as usize;
        let data = DAP_INTERFACE_DESCRIPTOR.to_bytes();
        buf[n..n+len].copy_from_slice(data);
        n += len;

        // Copy DAP_HID_DESCRIPTOR into buf
        let len = DAP_HID_DESCRIPTOR.bLength as usize;
        let data = DAP_HID_DESCRIPTOR.to_bytes();
        buf[n..n+len].copy_from_slice(data);
        n += len;

        // Copy all DAP_ENDPOINT_DESCRIPTORS into buf
        for ep in DAP_ENDPOINT_DESCRIPTORS.iter() {
            let len = ep.bLength as usize;
            let data = ep.to_bytes();
            buf[n..n+len].copy_from_slice(data);
            n += len;
        }

        // Only send as much data as was requested
        let n = usize::min(n, w_length as usize);

        // Enqueue transmission
        self.transmit_slice(usb, &buf[..n]);
    }

    /// Transmit STRING descriptor
    fn process_get_string_descriptor(&mut self, usb: &usb::Instance, w_length: u16, idx: u8) {
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
                    bString: [0u8; 62],
                };
                // Pack the u16 language codes into the u8 array
                for (idx, lang) in STRING_LANGS.iter().enumerate() {
                    let [u1, u2] = lang.to_le_bytes();
                    desc.bString[idx*2  ] = u1;
                    desc.bString[idx*2+1] = u2;
                }
                desc
            },

            // Handle manufacturer, product, serial number, and interface strings
            1..=5 => {
                let id;
                let string = match idx {
                    1 => Ok(STRING_MFN),
                    2 => Ok(STRING_PRD),
                    3 => { id = get_hex_id(); core::str::from_utf8(&id) },
                    4 => Ok(STRING_IF_SPI),
                    5 => Ok(STRING_IF_DAP),
                    _ => unreachable!(),
                };
                let string = match string {
                    Ok(s) => s,
                    Err(_) => {
                        self.stall(usb);
                        return;
                    }
                };
                let mut desc = StringDescriptor {
                    bLength: 2 + 2 * string.len() as u8,
                    bDescriptorType: DescriptorType::String as u8,
                    bString: [0u8; 62],
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
                self.stall(usb);
                return;
            }
        };

        let n = u16::min(desc.bLength as u16, w_length) as usize;
        let data = desc.to_bytes();
        self.transmit_slice(usb, &data[..n]);
    }

    /// Transmit a HID REPORT descriptor
    fn process_get_hid_report_descriptor(&mut self, usb: &usb::Instance, w_length: u16, idx: u8) {
        let report = match idx {
            0 => &DAP_HID_REPORT[..],
            _ => {
                self.stall(usb);
                return;
            }
        };

        let n = usize::min(report.len(), w_length as usize);
        self.transmit_slice(usb, &report[..n]);
    }

    /// Handle a vendor-specific request
    fn process_vendor_request(
        &mut self, usb: &usb::Instance, setup: &SetupPID)
        -> Option<USBStackRequest>
    {
        match VendorRequest::try_from(setup.bRequest) {
            Ok(VendorRequest::SetCS) => {
                match PinState::try_from(setup.wValue) {
                    Ok(ps) => {
                        self.pending_request = Some(
                            USBStackRequest::AppRequest(Request::SetCS(ps)));
                        self.transmit_ack(usb);
                    },
                    _ => {
                        self.stall(usb);
                    },
                }
                None
            },

            Ok(VendorRequest::SetFPGA) => {
                match PinState::try_from(setup.wValue) {
                    Ok(ps) => {
                        self.pending_request = Some(
                            USBStackRequest::AppRequest(Request::SetFPGA(ps)));
                        self.transmit_ack(usb);
                    },
                    _ => {
                        self.stall(usb);
                    },
                }
                None
            },

            Ok(VendorRequest::SetMode) => {
                match Mode::try_from(setup.wValue) {
                    Ok(mode) => {
                        self.pending_request = Some(
                            USBStackRequest::AppRequest(Request::SetMode(mode)));
                        self.transmit_ack(usb);
                    },
                    _ => {
                        self.stall(usb);
                    },
                }
                None
            },

            Ok(VendorRequest::SetTPwr) => {
                match PinState::try_from(setup.wValue) {
                    Ok(ps) => {
                        self.pending_request = Some(
                            USBStackRequest::AppRequest(Request::SetTPwr(ps)));
                        self.transmit_ack(usb);
                    },
                    _ => {
                        self.stall(usb);
                    },
                }
                None
            },

            Ok(VendorRequest::GetTPwr) => {
                // We don't ACK this, instead we immediately return the
                // request to the application which will call `reply_tpwr()`
                // with the TPwr state, and we transmit that.
                Some(USBStackRequest::AppRequest(Request::GetTPwr))
            },

            Ok(VendorRequest::SetLED) => {
                match PinState::try_from(setup.wValue) {
                    Ok(ps) => {
                        self.pending_request = Some(
                            USBStackRequest::AppRequest(Request::SetLED(ps)));
                        self.transmit_ack(usb);
                    },
                    _ => {
                        self.stall(usb);
                    },
                }
                None
            },

            Ok(VendorRequest::Bootload) => {
                self.pending_request = Some(
                    USBStackRequest::AppRequestAndDetach(Request::Bootload));
                self.transmit_ack(usb);
                None
            },

            // Ignore unknown requests
            _ => {
                self.stall(usb);
                None
            },
        }
    }
}

impl Endpoint for ControlEndpoint {
    fn new(epbuf: &'static mut EPBuf, btable: &'static mut BTableRow) -> Self {
        ControlEndpoint {
            epbuf,
            btable,
            pending_request: None,
            pending_tx: None,
            pending_tx_buf: [0u8; 256],
        }
    }

    fn write_btable(&mut self) {
        self.btable.write(&self.epbuf);
    }

    fn reset_endpoint(&self, usb: &usb::Instance) {
        let (stat_tx, stat_rx) = read_reg!(usb, usb, EP0R, STAT_TX, STAT_RX);
        write_reg!(usb, usb, EP0R,
                   CTR_RX: 0, EP_TYPE: Control, EP_KIND: 0, CTR_TX: 0, EA: 0,
                   STAT_TX: stat_nak(stat_tx), STAT_RX: stat_valid(stat_rx));
    }

    fn configure_endpoint(&self, _usb: &usb::Instance) {
        // No operation required, as the control endpoint configuration
        // is identical to its reset state.
    }

    /// Handle a transfer completion
    ///
    /// Either CTR_TX or CTR_RX for this EPnR will be set to indicate the direction
    /// of the completed transfer. If a transfer has completed in both directions,
    /// both bits will be set. Until both bits are cleared, the USB interrupt will
    /// remain active and this method will continue to be called, so we don't need
    /// to handle both TX and RX cases together.
    fn process_transfer(&mut self, usb: &usb::Instance) -> Option<USBStackRequest> {
        let mut req = None;
        let (ctr_tx, ctr_rx, ep_type, ea) =
            read_reg!(usb, usb, EP0R, CTR_TX, CTR_RX, EP_TYPE, EA);
        if ctr_tx == 1 {
            req = self.process_tx_complete(usb);

            // Clear CTR_TX
            write_reg!(usb, usb, EP0R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 0, EA: ea);
        } else if ctr_rx == 1 {
            req = self.process_rx_complete(usb);

            // Clear CTR_RX
            write_reg!(usb, usb, EP0R, CTR_RX: 0, EP_TYPE: ep_type, CTR_TX: 1, EA: ea);
        }

        req
    }

    /// Enqueue a slice of up to 64 bytes of data for transmission.
    fn transmit_slice(&mut self, usb: &usb::Instance, data: &[u8]) {
        assert!(data.len() <= 320);
        if data.len() < 64 {
            // For packets less than the maximum packet size, we can immediately
            // send the entire packet.
            self.epbuf.write_tx(data);
            self.btable.tx_count(data.len());
        } else {
            // For packets equal to the maximum packet size, we need to send
            // a zero-length-packet afterwards. For packets greater, we store
            // the remaining data for later transmission.
            self.epbuf.write_tx(&data[..64]);
            self.btable.tx_count(64);
            let leftover = data.len() - 64;
            self.pending_tx_buf[..leftover].copy_from_slice(&data[64..]);
            self.pending_tx = Some((0, data.len() - 64));
        }
        self.tx_valid(usb);
    }

    fn rx_valid(&mut self, usb: &usb::Instance) {
        // Indicate we're ready to receive again by setting STAT_RX to VALID
        let (stat_rx, ep_type, ea) = read_reg!(usb, usb, EP0R, STAT_RX, EP_TYPE, EA);
        write_reg!(usb, usb, EP0R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_RX: stat_valid(stat_rx));
    }

    fn rx_stall(&mut self, _usb: &usb::Instance) {
        // We don't stall reception of the control endpoint.
    }
}
