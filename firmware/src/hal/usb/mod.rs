// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::usb;
use stm32ral::{read_reg, write_reg, modify_reg};

use crate::app::{PinState, Request};

mod packets;
mod buffers;
mod descriptors;

mod control_endpoint;
mod spi_endpoint;
mod dap1_endpoint;
mod dap2_endpoint;
mod swo_endpoint;

use control_endpoint::ControlEndpoint;
use spi_endpoint::SPIEndpoint;
use dap1_endpoint::DAP1Endpoint;
use dap2_endpoint::DAP2Endpoint;
use swo_endpoint::SWOEndpoint;

use buffers::*;

/// USB stack interface
pub struct USB {
    usb: usb::Instance,
    ctl_endpoint: ControlEndpoint,
    spi_endpoint: SPIEndpoint,
    dap1_endpoint: DAP1Endpoint,
    dap2_endpoint: DAP2Endpoint,
    swo_endpoint: SWOEndpoint,
}

trait Endpoint {
    fn new(epbuf: &'static mut EPBuf, btable: &'static mut BTableRow) -> Self;
    fn write_btable(&mut self);
    fn reset_endpoint(&self, usb: &usb::Instance);
    fn configure_endpoint(&self, usb: &usb::Instance);
    fn process_transfer(&mut self, usb: &usb::Instance) -> Option<USBStackRequest>;
    fn transmit_slice(&mut self, usb: &usb::Instance, data: &[u8]);
    fn rx_valid(&mut self, usb: &usb::Instance);
    fn rx_stall(&mut self, usb: &usb::Instance);
}

/// Enum of requests an Endpoint may make of the stack
#[derive(Copy, Clone)]
enum USBStackRequest {
    Reset,
    SetAddress(u16),
    SetConfiguration,
    AppRequest(Request),
    AppRequestAndDetach(Request),
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
                ctl_endpoint: ControlEndpoint::new(&mut EP0BUF, &mut BTABLE[0]),
                spi_endpoint: SPIEndpoint::new(&mut EP1BUF, &mut BTABLE[1]),
                dap1_endpoint: DAP1Endpoint::new(&mut EP2BUF, &mut BTABLE[2]),
                dap2_endpoint: DAP2Endpoint::new(&mut EP3BUF, &mut BTABLE[3]),
                swo_endpoint: SWOEndpoint::new(&mut EP4BUF, &mut BTABLE[4]),
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
    ///
    /// This function will clear the interrupt bits of all interrupts
    /// it processes; if any are unprocessed the USB interrupt keeps
    /// triggering until all are processed.
    pub fn interrupt(&mut self) -> Option<Request> {
        let (ctr, susp, wkup, reset, ep_id) =
            read_reg!(usb, self.usb, ISTR, CTR, SUSP, WKUP, RESET, EP_ID);

        // Handle receiving a USB RESET signal
        if reset == 1 {
            // Trigger a peripheral reset, clearing configuration
            self.usb_reset();

            // Clear RESET flag
            write_reg!(usb, self.usb, ISTR, CTR: 1, SUSP: 1, WKUP: 1, RESET: 0);
        }

        // Handle wakeup detection
        if wkup == 1 {
            // Bring USB peripheral out of suspend
            modify_reg!(usb, self.usb, CNTR, FSUSP: 0);
            // Clear WKUP flag
            write_reg!(usb, self.usb, ISTR, CTR: 1, SUSP: 1, WKUP: 0, RESET: 1);
        }

        // Handle suspend mode request
        if susp == 1 {
            // Put USB peripheral into suspend and low-power mode
            modify_reg!(usb, self.usb, CNTR, FSUSP: Suspend, LPMODE: Enabled);
            // Clear SUSP flag
            write_reg!(usb, self.usb, ISTR, CTR: 1, SUSP: 0, WKUP: 1, RESET: 1);

            // Let the application know we've entered SUSPEND so it
            // can take appropriate action to reduce power consumption
            return Some(Request::Suspend);
        }

        // Handle transfer complete
        if ctr == 1 {
            // Delegate handling transfer completion to endpoints,
            // which may respond with a request for the stack.
            let ep_req = match ep_id {
                0 => self.ctl_endpoint.process_transfer(&self.usb),
                1 => self.spi_endpoint.process_transfer(&self.usb),
                2 => self.dap1_endpoint.process_transfer(&self.usb),
                3 => self.dap2_endpoint.process_transfer(&self.usb),
                4 => self.swo_endpoint.process_transfer(&self.usb),
                _ => None,
            };

            return match ep_req {
                Some(USBStackRequest::Reset) => {
                    self.usb_reset();
                    None
                },
                Some(USBStackRequest::SetAddress(addr)) => {
                    self.set_address(addr);
                    None
                },
                Some(USBStackRequest::SetConfiguration) => {
                    self.set_configuration();
                    None
                },
                Some(USBStackRequest::AppRequest(req)) => {
                    Some(req)
                }
                Some(USBStackRequest::AppRequestAndDetach(req)) => {
                    self.detach();
                    Some(req)
                },
                None => None,
            };

            // CTR flag is read-only and cleared by clearing the CTR_RX/CTR_TX
            // bits in the corresponding EPnR registers. If either bit is not
            // cleared by the endpoint in process_transfer, the CTR flag stays
            // set and process_transfer will be called again.
        }

        None
    }

    /// Transmit the current tpwr state in response to a recent GetTPwr request
    pub fn tpwr_reply(&mut self, tpwr: PinState) {
        let data = [tpwr as u8, 0];
        self.ctl_endpoint.transmit_slice(&self.usb, &data[..]);
    }

    /// Transmit a given slice of data out the bulk endpoint
    pub fn spi_data_reply(&mut self, data: &[u8]) {
        self.spi_endpoint.transmit_slice(&self.usb, data);
    }

    /// Indicate we can currently receive data
    pub fn spi_data_enable(&mut self) {
        self.spi_endpoint.rx_valid(&self.usb);
    }

    /// Indicate we cannot currently receive data
    pub fn spi_data_disable(&mut self) {
        self.spi_endpoint.rx_stall(&self.usb);
    }

    /// Transmit a DAP report back over the DAPv1 HID interface
    pub fn dap1_reply(&mut self, data: &[u8]) {
        self.dap1_endpoint.transmit_slice(&self.usb, data);
    }

    /// Transmit a DAP report back over the DAPv2 bulk interface
    pub fn dap2_reply(&mut self, data: &[u8]) {
        self.dap2_endpoint.transmit_slice(&self.usb, data);
    }

    /// Check if SWO endpoint is currently busy transmitting data
    pub fn dap2_swo_is_busy(&self) -> bool {
        self.swo_endpoint.is_busy()
    }

    /// Transmit SWO streaming data back over the DAPv2 bulk interface
    pub fn dap2_stream_swo(&mut self, data: &[u8]) {
        self.swo_endpoint.transmit_slice(&self.usb, data);
    }

    /// Indicate we can currently receive DAP requests
    pub fn dap_enable(&mut self) {
        self.dap1_endpoint.rx_valid(&self.usb);
        self.dap2_endpoint.rx_valid(&self.usb);
    }

    /// Indicate we cannot currently receive DAP requests
    pub fn dap_disable(&mut self) {
        self.dap1_endpoint.rx_stall(&self.usb);
        self.dap2_endpoint.rx_stall(&self.usb);
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
        // Write the buffer table descriptors
        self.ctl_endpoint.write_btable();
        self.spi_endpoint.write_btable();
        self.dap1_endpoint.write_btable();
        self.dap2_endpoint.write_btable();
        self.swo_endpoint.write_btable();
        // Set buffer table to start at BTABLE.
        // We write the entire register to avoid dealing with the shifted-by-3 field.
        write_reg!(usb, self.usb, BTABLE,
                   unsafe { (&BTABLE as *const _ as u32) - USB_SRAM });
        // Clear ISTR
        write_reg!(usb, self.usb, ISTR, 0);
        // Enable reset masks
        modify_reg!(usb, self.usb, CNTR,
                    CTRM: Enabled, RESETM: Enabled, SUSPM: Enabled, WKUPM: Enabled);
    }

    /// Put device into USB_RESET state
    ///
    /// Respond to address 0 on EP0 only
    fn usb_reset(&self) {
        // Ensure peripheral will not respond while we set up endpoints
        write_reg!(usb, self.usb, DADDR, EF: Disabled);

        // Clear ISTR
        write_reg!(usb, self.usb, ISTR, 0);

        // Set endpoints to reset state
        self.ctl_endpoint.reset_endpoint(&self.usb);
        self.spi_endpoint.reset_endpoint(&self.usb);
        self.dap1_endpoint.reset_endpoint(&self.usb);
        self.dap2_endpoint.reset_endpoint(&self.usb);
        self.swo_endpoint.reset_endpoint(&self.usb);

        // Ensure all other endpoints are disabled
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP5R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP5R,
                   STAT_TX: stat_disabled(stat_tx), STAT_RX: stat_disabled(stat_rx));
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP6R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP6R,
                   STAT_TX: stat_disabled(stat_tx), STAT_RX: stat_disabled(stat_rx));
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP7R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP7R,
                   STAT_TX: stat_disabled(stat_tx), STAT_RX: stat_disabled(stat_rx));

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

    /// Apply received address to device
    fn set_address(&self, address: u16) {
        modify_reg!(usb, self.usb, DADDR, ADD: address as u32);
    }

    /// Set our operational configuration:
    ///
    /// EP0: Bidirectional control (default, left unchanged)
    /// EP1: Bidirectional bulk (SPI transactions)
    /// EP2: Bidirectional interrupt (CMSIS-DAP HID)
    fn set_configuration(&self) {
        // Configure our known endpoints
        self.ctl_endpoint.configure_endpoint(&self.usb);
        self.spi_endpoint.configure_endpoint(&self.usb);
        self.dap1_endpoint.configure_endpoint(&self.usb);
        self.dap2_endpoint.configure_endpoint(&self.usb);
        self.swo_endpoint.configure_endpoint(&self.usb);

        // Ensure all other endpoints are disabled by writing their current
        // values of STAT_TX/STAT_RX, setting them to 00 (disabled)
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP5R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP5R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP6R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP6R, STAT_TX: stat_tx, STAT_RX: stat_rx);
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP7R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP7R, STAT_TX: stat_tx, STAT_RX: stat_rx);
    }
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
