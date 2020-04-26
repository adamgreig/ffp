// Copyright 2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::usb;
use stm32ral::{read_reg, write_reg};

use super::{USBStackRequest, Endpoint, stat_disabled, stat_nak, stat_valid};
use super::buffers::*;

/// USB handling code for SWO streaming endpoint
pub(super) struct SWOEndpoint {
    epbuf: &'static mut EPBuf,
    btable: &'static mut BTableRow,
    tx_busy: bool,
}

impl SWOEndpoint {
    /// Indicate a packet has been loaded into the buffer and is ready for transmission
    fn tx_valid(&self, usb: &usb::Instance) {
        let (stat_tx, ep_type, ea) = read_reg!(usb, usb, EP4R, STAT_TX, EP_TYPE, EA);
        write_reg!(usb, usb, EP4R, CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 1, EA: ea,
                   STAT_TX: stat_valid(stat_tx));
    }

    /// Returns true if this endpoint is still busy with a transmission.
    pub fn is_busy(&self) -> bool {
        self.tx_busy
    }
}

impl Endpoint for SWOEndpoint {
    fn new(epbuf: &'static mut EPBuf, btable: &'static mut BTableRow) -> Self {
        SWOEndpoint { epbuf, btable, tx_busy: false }
    }

    fn write_btable(&mut self) {
        self.btable.write(&self.epbuf);
    }

    fn reset_endpoint(&self, usb: &usb::Instance) {
        let (stat_tx, stat_rx) = read_reg!(usb, usb, EP4R, STAT_TX, STAT_RX);
        write_reg!(usb, usb, EP4R,
                   STAT_TX: stat_disabled(stat_tx), STAT_RX: stat_disabled(stat_rx));
    }

    fn configure_endpoint(&self, usb: &usb::Instance) {
        // Set up EP4R to be a unidirectional bulk IN endpoint,
        // with STAT_TX to nak and STAT_RX to disabled,
        // and DTOG_TX and DTOG_RX both set to 0.
        let (stat_tx, stat_rx, dtog_rx, dtog_tx) =
            read_reg!(usb, usb, EP4R, STAT_TX, STAT_RX, DTOG_RX, DTOG_TX);
        write_reg!(usb, usb, EP4R,
                   CTR_RX: 1, EP_TYPE: Bulk, EP_KIND: 0, CTR_TX: 1, EA: 4,
                   DTOG_RX: dtog_rx, DTOG_TX: dtog_tx,
                   STAT_TX: stat_nak(stat_tx), STAT_RX: stat_disabled(stat_rx));
    }

    fn process_transfer(&mut self, usb: &usb::Instance) -> Option<USBStackRequest> {
        let (ctr_tx, ctr_rx, ep_type, ea) =
            read_reg!(usb, usb, EP4R, CTR_TX, CTR_RX, EP_TYPE, EA);
        if ctr_tx == 1 {
            self.tx_busy = false;
            // Clear CTR_TX
            write_reg!(usb, usb, EP4R,
                       CTR_RX: 1, EP_TYPE: ep_type, CTR_TX: 0, EA: ea);
        }
        if ctr_rx == 1 {
            // Clear CTR_RX
            write_reg!(usb, usb, EP4R,
                       CTR_RX: 0, EP_TYPE: ep_type, CTR_TX: 1, EA: ea);
        }
        None
    }

    fn transmit_slice(&mut self, usb: &usb::Instance, data: &[u8]) {
        assert!(data.len() <= 64);
        self.epbuf.write_tx(data);
        self.btable.tx_count(data.len());
        self.tx_valid(usb);
        self.tx_busy = true;
    }

    /// We never receive data, so this method does nothing.
    fn rx_valid(&mut self, _usb: &usb::Instance) {
    }

    /// We never receive data, so this method does nothing.
    fn rx_stall(&mut self, _usb: &usb::Instance) {
    }
}
