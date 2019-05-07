use core::mem::size_of;
use super::packets::*;

// Test use only! From http://pid.codes/1209/0001/
const VENDOR_ID: u16 = 0x1209;
const PRODUCT_ID: u16 = 0x0001;
const DEVICE_ID: u16 = 0x0000;

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

pub static STRING_DESCRIPTORS: [StringDescriptor; 4] = [
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

pub static CONFIGURATION_DESCRIPTOR: ConfigurationDescriptor = ConfigurationDescriptor {
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

pub static ENDPOINT_DESCRIPTORS: [EndpointDescriptor; 3] = [
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
        bmAttributes: 0b10_00_00_00,
        wMaxPacketSize: 64,
        bInterval: 10,
    },
    EndpointDescriptor {
        bLength: size_of::<EndpointDescriptor>() as u8,
        bDescriptorType: DescriptorType::Endpoint as u8,
        bEndpointAddress: 0b0_000_0001,
        bmAttributes: 0b10_00_00_00,
        wMaxPacketSize: 64,
        bInterval: 10,
    },
];

