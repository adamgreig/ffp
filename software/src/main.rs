use std::fs::File;
use std::io::prelude::*;
use std::env;
use ffp::{Programmer, Flash, FPGA};

fn main() -> ffp::Result<()> {
    let context = libusb::Context::new().expect("Error getting libusb context");
    let programmer = Programmer::find(&context)?;
    let flash = Flash::new(&programmer);
    let id = flash.read_id().expect("Error getting Flash ID");
    println!("{}", id);
    let path = env::args().nth(1).expect("Expected file path as first argument");
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    //let fpga = FPGA::new(&programmer);
    //fpga.program(&data)?;
    flash.program(0, &data)?;
    programmer.unreset()?;
    Ok(())
}
