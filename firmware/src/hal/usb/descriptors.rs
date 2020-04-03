// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use core::mem::size_of;
use super::packets::*;

pub static STRING_LANGS: [u16; 1] = [0x0409];
pub static STRING_MFN: &str = "AGG";
pub static STRING_PRD: &str = "FFP r1 with CMSIS-DAP Support";
pub static STRING_IF_SPI: &str = "FFP SPI Interface";
pub static STRING_IF_DAP: &str = "FFP CMSIS-DAP v1 SWD Interface";

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
                   DAP_INTERFACE_DESCRIPTOR.bLength as usize +
                   DAP_HID_DESCRIPTOR.bLength as usize +
                   size_of::<[EndpointDescriptor; DAP_NUM_ENDPOINTS]>()) as u16,
    bNumInterfaces: 2,
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
    bNumEndpoints: 2,
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

pub static DAP_INTERFACE_DESCRIPTOR: InterfaceDescriptor = InterfaceDescriptor {
    bLength: size_of::<InterfaceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Interface as u8,
    bInterfaceNumber: 1,
    bAlternateSetting: 0,
    bNumEndpoints: 2,
    bInterfaceClass: 0x03,
    bInterfaceSubClass: 0,
    bInterfaceProtocol: 0,
    iInterface: 5,
};

pub static DAP_HID_DESCRIPTOR: HIDDescriptor = HIDDescriptor {
    bLength: (size_of::<HIDDescriptor>() + DAP_HID_REPORT_LENGTH) as u8,
    bDescriptorType: DescriptorType::HID as u8,
    bcdHID: 0x0111,
    bCountryCode: 0x00,
    bNumDescriptors: 1,
    bSubDescriptorType: DescriptorType::HIDReport as u8,
    wSubDescriptorLength: DAP_HID_REPORT_LENGTH as u16,
};

pub const DAP_HID_REPORT_LENGTH: usize = 32;
pub static DAP_HID_REPORT: [u8; DAP_HID_REPORT_LENGTH] = [
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

const DAP_NUM_ENDPOINTS: usize = 2;
pub static DAP_ENDPOINT_DESCRIPTORS: [EndpointDescriptor; DAP_NUM_ENDPOINTS] = [
    // EP2 IN, INTERRUPT
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b1_000_0010,
        bmAttributes: 0b00_00_00_11,
        wMaxPacketSize: 64,
        bInterval: 10,
    },

    // EP2 OUT, INTERRUPT
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b0_000_0010,
        bmAttributes: 0b00_00_00_11,
        wMaxPacketSize: 64,
        bInterval: 10,
    },
];

