// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use core::mem::size_of;
use super::packets::*;

pub static STRING_LANGS: [u16; 1] = [0x0409];
pub static STRINGS: [&str; 3] = ["AGG", "FFP r1", "001"];

// Assigned by http://pid.codes/1209/FF50/
const VENDOR_ID: u16 = 0x1209;
const PRODUCT_ID: u16 = 0xFF50;
const DEVICE_ID: u16 = 0x0001;

pub static DEVICE_DESCRIPTOR: DeviceDescriptor = DeviceDescriptor {
    bLength: size_of::<DeviceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Device as u8,
    bcdUSB: 0x0200,
    bDeviceClass: 0xFF,
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
    wTotalLength: (size_of::<ConfigurationDescriptor>() + size_of::<InterfaceDescriptor>() +
                   size_of::<EndpointDescriptor>() * NUM_ENDPOINTS) as u16,
    bNumInterfaces: 1,
    bConfigurationValue: 1,
    iConfiguration: 0,
    bmAttributes: 0b1000_0000,
    bMaxPower: 50,
};

pub static INTERFACE_DESCRIPTOR: InterfaceDescriptor = InterfaceDescriptor {
    bLength: size_of::<InterfaceDescriptor>() as u8,
    bDescriptorType: DescriptorType::Interface as u8,
    bInterfaceNumber: 0,
    bAlternateSetting: 0,
    bNumEndpoints: 2,
    bInterfaceClass: 0xFF,
    bInterfaceSubClass: 0,
    bInterfaceProtocol: 0,
    iInterface: 0,
};

const NUM_ENDPOINTS: usize = 2;
pub static ENDPOINT_DESCRIPTORS: [EndpointDescriptor; NUM_ENDPOINTS] = [
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

