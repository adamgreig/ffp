// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

#![allow(clippy::inconsistent_digit_grouping)]

use core::mem::size_of;
use super::packets::*;

pub static STRING_LANGS: [u16; 1] = [0x0409];
pub static STRING_MFN: &str = "AGG";
pub static STRING_PRD: &str = "FFP r1 with CMSIS-DAP Support";
pub static STRING_IF_SPI: &str = "FFP SPI Interface";
pub static STRING_IF_DAP1: &str = "FFP CMSIS-DAP v1 Interface";
pub static STRING_IF_DAP2: &str = "FFP CMSIS-DAP v2 Interface";
pub static STRING_MOS: &str = "MSFT100A";

// Assigned by http://pid.codes/1209/FF50/
const VENDOR_ID: u16 = 0x1209;
const PRODUCT_ID: u16 = 0xFF50;
const DEVICE_ID: u16 = 0x0001;

pub static DEVICE_DESCRIPTOR: DeviceDescriptor = DeviceDescriptor {
    bLength: size_of::<DeviceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Device as u8,
    bcdUSB: 0x0200,
    bDeviceClass: 0,
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

pub static CONFIGURATION_DESCRIPTOR: ConfigurationDescriptor = ConfigurationDescriptor {
    bLength: size_of::<ConfigurationDescriptor>() as u8,
    bDescriptorType: DescriptorType::Configuration as u8,
    wTotalLength: (size_of::<ConfigurationDescriptor>() +
                   SPI_INTERFACE_DESCRIPTOR.bLength as usize +
                   size_of::<[EndpointDescriptor; SPI_NUM_ENDPOINTS]>() +
                   DAP1_INTERFACE_DESCRIPTOR.bLength as usize +
                   DAP1_HID_DESCRIPTOR.bLength as usize +
                   size_of::<[EndpointDescriptor; DAP1_NUM_ENDPOINTS]>() +
                   DAP2_INTERFACE_DESCRIPTOR.bLength as usize +
                   size_of::<[EndpointDescriptor; DAP2_NUM_ENDPOINTS]>()) as u16,
    bNumInterfaces: 3,
    bConfigurationValue: 1,
    iConfiguration: 0,
    bmAttributes: 0b1000_0000,
    bMaxPower: 50,
};

pub static SPI_INTERFACE_DESCRIPTOR: InterfaceDescriptor = InterfaceDescriptor {
    bLength: size_of::<InterfaceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Interface as u8,
    bInterfaceNumber: 0,
    bAlternateSetting: 0,
    bNumEndpoints: SPI_NUM_ENDPOINTS as u8,
    bInterfaceClass: 0xFF,
    bInterfaceSubClass: 0,
    bInterfaceProtocol: 0,
    iInterface: 4,
};

const SPI_NUM_ENDPOINTS: usize = 2;
pub static SPI_ENDPOINT_DESCRIPTORS: [EndpointDescriptor; SPI_NUM_ENDPOINTS] = [
    // EP1 IN, BULK
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b1_000_0001,
        bmAttributes: 0b00_00_00_10,
        wMaxPacketSize: 64,
        bInterval: 10,
    },

    // EP1 OUT, BULK
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b0_000_0001,
        bmAttributes: 0b00_00_00_10,
        wMaxPacketSize: 64,
        bInterval: 10,
    },
];

pub static DAP1_INTERFACE_DESCRIPTOR: InterfaceDescriptor = InterfaceDescriptor {
    bLength: size_of::<InterfaceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Interface as u8,
    bInterfaceNumber: 1,
    bAlternateSetting: 0,
    bNumEndpoints: DAP1_NUM_ENDPOINTS as u8,
    bInterfaceClass: 0x03,
    bInterfaceSubClass: 0,
    bInterfaceProtocol: 0,
    iInterface: 5,
};

pub static DAP1_HID_DESCRIPTOR: HIDDescriptor = HIDDescriptor {
    bLength: size_of::<HIDDescriptor>() as u8,
    bDescriptorType: DescriptorType::HID as u8,
    bcdHID: 0x0111,
    bCountryCode: 0x00,
    bNumDescriptors: 1,
    bSubDescriptorType: DescriptorType::HIDReport as u8,
    wSubDescriptorLength: DAP1_HID_REPORT_LENGTH as u16,
};

pub const DAP1_HID_REPORT_LENGTH: usize = 32;
pub static DAP1_HID_REPORT: [u8; DAP1_HID_REPORT_LENGTH] = [
    0x06, 0x00, 0xFF,           // GLOBAL Usage Page    0xFF00: Vendor Defined
    0x09, 0x01,                 // LOCAL  Usage         0x01: Vendor Usage 1
    0xA1, 0x01,                 // MAIN   Collection    0x01: Application
    0x15, 0x00,                 // GLOBAL Logical Min   0x00: 0
    0x25, 0xFF,                 // GLOBAL Logical Max   0xFF: 255
    0x75, 0x08,                 // GLOBAL Report Size   0x08: 8 bits

    0x95, 0x40,                 // GLOBAL Report Count  0x40: 64 bytes
    0x09, 0x01,                 // LOCAL  Usage         0x01: Vendor Usage 1
    0x81, 0x02,                 // MAIN   Input         0x02: Data, Variable

    0x95, 0x40,                 // GLOBAL Report Count  0x40: 64 bytes
    0x09, 0x01,                 // LOCAL  Usage         0x01: Vendor Usage 1
    0x91, 0x02,                 // MAIN   Output        0x02: Data, Variable

    0x95, 0x01,                 // GLOBAL Report Count  0x01: 1 byte
    0x09, 0x01,                 // LOCAL  Usage         0x01: Vendor Usage 1
    0xB1, 0x02,                 // MAIN   Feature       0x02: Data, Variable
    0xC0,                       // MAIN   End Collection
];

const DAP1_NUM_ENDPOINTS: usize = 2;
pub static DAP1_ENDPOINT_DESCRIPTORS: [EndpointDescriptor; DAP1_NUM_ENDPOINTS] = [
    // EP2 IN, INTERRUPT
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b1_000_0010,
        bmAttributes: 0b00_00_00_11,
        wMaxPacketSize: 64,
        bInterval: 1,
    },

    // EP2 OUT, INTERRUPT
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b0_000_0010,
        bmAttributes: 0b00_00_00_11,
        wMaxPacketSize: 64,
        bInterval: 1,
    },
];

pub static DAP2_INTERFACE_DESCRIPTOR: InterfaceDescriptor = InterfaceDescriptor {
    bLength: size_of::<InterfaceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Interface as u8,
    bInterfaceNumber: 2,
    bAlternateSetting: 0,
    bNumEndpoints: DAP2_NUM_ENDPOINTS as u8,
    bInterfaceClass: 0xFF,
    bInterfaceSubClass: 0,
    bInterfaceProtocol: 0,
    iInterface: 6,
};

const DAP2_NUM_ENDPOINTS: usize = 3;
pub static DAP2_ENDPOINT_DESCRIPTORS: [EndpointDescriptor; DAP2_NUM_ENDPOINTS] = [
    // EP3 OUT, BULK
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b0_000_0011,
        bmAttributes: 0b00_00_00_10,
        wMaxPacketSize: 64,
        bInterval: 10,
    },

    // EP3 IN, BULK
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b1_000_0011,
        bmAttributes: 0b00_00_00_10,
        wMaxPacketSize: 64,
        bInterval: 10,
    },

    // EP4 IN, BULK
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b1_000_0100,
        bmAttributes: 0b00_00_00_10,
        wMaxPacketSize: 64,
        bInterval: 10,
    },
];

const MS_COMPATIBLE_ID_WINUSB: [u8; 8] = [b'W', b'I', b'N', b'U', b'S', b'B', 0, 0];

pub static MS_COMPATIBLE_ID_DESCRIPTOR: MSCompatibleIDDescriptor = MSCompatibleIDDescriptor {
    dwLength: size_of::<MSCompatibleIDDescriptor>() as u32,
    bcdVersion: 0x0100,
    wIndex: OSFeatureDescriptorType::CompatibleID as u16,
    bNumSections: 2,
    _rsvd0: [0; 7],
    features: [
        MSCompatibleIDDescriptorFunction {
            bInterfaceNumber: 0,
            _rsvd0: 0,
            sCompatibleID: MS_COMPATIBLE_ID_WINUSB,
            sSubCompatibleID: [0u8; 8],
            _rsvd1: [0u8; 6],
        },
        MSCompatibleIDDescriptorFunction {
            bInterfaceNumber: 2,
            _rsvd0: 0,
            sCompatibleID: MS_COMPATIBLE_ID_WINUSB,
            sSubCompatibleID: [0u8; 8],
            _rsvd1: [0u8; 6],
        },
    ],
};

pub static IF2_MS_PROPERTIES_OS_DESCRIPTOR: MSPropertiesOSDescriptor = MSPropertiesOSDescriptor {
    bcdVersion: 0x0100,
    wIndex: OSFeatureDescriptorType::Properties as u16,
    wCount: 1,
    features: [
        MSPropertiesOSDescriptorFeature {
            dwPropertyDataType: MSPropertyDataType::REG_SZ as u32,
            bPropertyName: "DeviceInterfaceGUID\x00",
            bPropertyData: "{CDB3B5AD-293B-4663-AA36-1AAE46463776}\x00",
        }
    ],
};
