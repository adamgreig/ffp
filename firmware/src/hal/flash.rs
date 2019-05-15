// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::{write_reg, flash};

pub struct Flash {
    flash: flash::Instance,
}

impl Flash {
    pub fn new(flash: flash::Instance) -> Self {
        Flash { flash }
    }

    pub fn setup(&self) {
        write_reg!(flash, self.flash, ACR, PRFTBE: Enabled, LATENCY: WS1);
    }
}
