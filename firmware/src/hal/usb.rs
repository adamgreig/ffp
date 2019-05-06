use core::mem::size_of;
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

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
struct SetupPID {
    bmRequestType: u8,
    bRequest: u8,
    wValue: u16,
    wIndex: u16,
    wLength: u16,
}

#[repr(u8)]
enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
}

#[allow(non_camel_case_types)]
#[repr(u8)]
enum StandardRequest {
    CLEAR_FEATURE = 1,
    GET_CONFIGURATION = 8,
    GET_DESCRIPTOR = 6,
    GET_INTERFACE = 10,
    GET_STATUS = 0,
    SET_ADDRESS = 5,
    SET_CONFIGURATION = 9,
    SET_DESCRIPTOR = 7,
    SET_FEATURE = 3,
    SET_INTERFACE = 11,
    SYNCH_FRAME = 12,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
struct DeviceDescriptor {
    bLength: u8,
    bDescriptorType: u8,
    bcdUSB: u16,
    bDeviceClass: u8,
    bDeviceSubClass: u8,
    bDeviceProtocol: u8,
    bMaxPacketSize0: u8,
    idVendor: u16,
    idProduct: u16,
    bcdDevice: u16,
    iManufacturer: u8,
    iProduct: u8,
    iSerialNumber: u8,
    bNumConfigurations: u8,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
struct ConfigurationDescriptor {
    bLength: u8,
    bDescriptorType: u8,
    wTotalLength: u16,
    bNumInterfaces: u8,
    bConfigurationValue: u8,
    iConfiguration: u8,
    bmAttributes: u8,
    bMaxPower: u8,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
struct InterfaceDescriptor {
    bLength: u8,
    bDescriptorType: u8,
    bInterfaceNumber: u8,
    bAlternateSetting: u8,
    bNumEndpoints: u8,
    bInterfaceClass: u8,
    bInterfaceSubClass: u8,
    bInterfaceProtocol: u8,
    iInterface: u8,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
struct EndpointDescriptor {
    bLength: u8,
    bDescriptorType: u8,
    bEndpointAddress: u8,
    bmAttributes: u8,
    wMaxPacketSize: u16,
    bInterval: u8,
}

#[allow(non_snake_case)]
#[repr(C)]
#[repr(packed)]
struct StringDescriptor {
    bLength: u8,
    bDescriptorType: u8,
    bString: [u8; 16],
}

const USB_SRAM: u32 = 0x4000_6000;
const EP1BUF: *mut EPBuf256 = USB_SRAM as *mut _;
const EP0BUF: *mut EPBuf64 = (USB_SRAM + 512) as *mut _;
const BTABLE: *const [BTable; 8] = (USB_SRAM + 512 + 128) as *const _;

// Test use only! From http://pid.codes/1209/0001/
const VENDOR_ID: u16 = 0x1209;
const PRODUCT_ID: u16 = 0x0001;
const DEVICE_ID: u16 = 0x0000;

const HID_CLASS: u8 = 3;

static DEVICE_DESCRIPTOR: DeviceDescriptor = DeviceDescriptor {
    bLength: size_of::<DeviceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Device as u8,
    bcdUSB: 0x0200,
    bDeviceClass: HID_CLASS,
    bDeviceSubClass: 0,
    bDeviceProtocol: 0,
    bMaxPacketSize0: 64,
    idVendor: VENDOR_ID,
    idProduct: PRODUCT_ID,
    bcdDevice: DEVICE_ID,
    iManufacturer: 1,
    iProduct: 2,
    iSerialNumber: 3,
    bNumConfigurations: 1,
};

static STRING_DESCRIPTORS: [StringDescriptor; 4] = [
    StringDescriptor {
        bLength: 4,
        bDescriptorType: DescriptorType::String as u8,
        bString: [0x09, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    },
    StringDescriptor {
        bLength: 6,
        bDescriptorType: DescriptorType::String as u8,
        bString: [0x00, 0x41, 0x00, 0x47, 0x00, 0x00, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    },
    StringDescriptor {
        bLength: 8,
        bDescriptorType: DescriptorType::String as u8,
        bString: [0x00, 0x46, 0x00, 0x50, 0x00, 0x50, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    },
    StringDescriptor {
        bLength: 8,
        bDescriptorType: DescriptorType::String as u8,
        bString: [0x00, 0x30, 0x00, 0x30, 0x00, 0x31, 0x00, 0x00,
                  0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    },
];

static CONFIGURATION_DESCRIPTOR: ConfigurationDescriptor = ConfigurationDescriptor {
    bLength: size_of::<ConfigurationDescriptor>() as u8,
    bDescriptorType: DescriptorType::Configuration as u8,
    wTotalLength: (size_of::<ConfigurationDescriptor>() + size_of::<InterfaceDescriptor>() +
                   size_of::<EndpointDescriptor>() * 3) as u16,
    bNumInterfaces: 1,
    bConfigurationValue: 0,
    iConfiguration: 0,
    bmAttributes: 0b1000_0000,
    bMaxPower: 50,
};

static INTERFACE_DESCRIPTOR: InterfaceDescriptor = InterfaceDescriptor {
    bLength: size_of::<InterfaceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Interface as u8,
    bInterfaceNumber: 0,
    bAlternateSetting: 0,
    bNumEndpoints: 2,
    bInterfaceClass: HID_CLASS,
    bInterfaceSubClass: 0,
    bInterfaceProtocol: 0,
    iInterface: 0,
};

static ENDPOINT_DESCRIPTORS: [EndpointDescriptor; 3] = [
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b0_000_0000,
        bmAttributes: 0b00_00_00_00,
        wMaxPacketSize: 64,
        bInterval: 0,
    },
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b1_000_0001,
        bmAttributes: 0b11_00_00_00,
        wMaxPacketSize: 256,
        bInterval: 10,
    },
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b0_000_0001,
        bmAttributes: 0b11_00_00_00,
        wMaxPacketSize: 256,
        bInterval: 10,
    },
];

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

    /// Call when the endpoint specified by `ep_id` has received or transmitted a packet
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
        modify_reg!(usb, self.usb, EP0R, STAT_RX: Self::stat_valid(stat_rx));
    }

    fn process_data_tx(&self) {
    }

    fn process_data_rx(&self) {
        // Indicate we're ready to receive again
        let stat_rx = read_reg!(usb, self.usb, EP1R, STAT_RX);
        modify_reg!(usb, self.usb, EP1R, STAT_RX: Self::stat_valid(stat_rx));
    }

    fn stat_disabled(stat: u32) -> u32 {
        (!stat & 0b10) | (!stat & 0b01)
    }

    fn stat_stall(stat: u32) -> u32 {
        (stat & 0b10) | (!stat & 0b01)
    }

    fn stat_nak(stat: u32) -> u32 {
        (!stat & 0b10) | (stat & 0b01)
    }

    fn stat_valid(stat: u32) -> u32 {
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
                   STAT_TX: Self::stat_nak(stat_tx), STAT_RX: Self::stat_valid(stat_rx));

        // Set up EP1R to be a bidirectional interrupt endpoint,
        // with STAT_TX to NAK=10 and STAT_RX to Valid=11
        let (stat_tx, stat_rx) = read_reg!(usb, self.usb, EP1R, STAT_TX, STAT_RX);
        write_reg!(usb, self.usb, EP1R,
                   CTR_RX: 0, EP_TYPE: Interrupt, EP_KIND: 0, CTR_TX: 0, EA: 0,
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

        // Set EF=1 with address 0 to enable processing incoming packets
        modify_reg!(usb, self.usb, DADDR, ADD: address as u32, EF: Enabled);
    }
}
