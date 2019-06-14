use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;
use clap::{Arg, App, AppSettings, SubCommand};
use clap::{value_t, crate_authors, crate_description, crate_version};
use ffp::{Programmer, Flash, FPGA};

fn main() -> ffp::Result<()> {
    let matches = App::new("ffp fpga/flash programmer")
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .about(crate_description!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .global_setting(AppSettings::ColoredHelp)
        .global_setting(AppSettings::DeriveDisplayOrder)
        .global_setting(AppSettings::GlobalVersion)
        .global_setting(AppSettings::InferSubcommands)
        .global_setting(AppSettings::VersionlessSubcommands)
        .arg(Arg::with_name("quiet")
             .help("Suppress informative output")
             .long("quiet")
             .short("q")
             .global(true))
        .arg(Arg::with_name("serial")
             .help("Serial number of FFP device to use")
             .long("serial")
             .short("s")
             .takes_value(true)
             .global(true))
        .arg(Arg::with_name("index")
             .help("Index of FFP device to use")
             .long("index")
             .short("i")
             .conflicts_with("serial")
             .takes_value(true)
             .global(true))
        .subcommand(SubCommand::with_name("fpga")
            .about("Reset, power, and program the FPGA")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("reset")
                        .about("Reset the FPGA"))
            .subcommand(SubCommand::with_name("power")
                        .about("Control FPGA power from FFP board")
                        .arg(Arg::with_name("power")
                             .possible_values(&["on", "off"])
                             .required(true)))
            .subcommand(SubCommand::with_name("program")
                        .about("Program FPGA with bitstream")
                        .arg(Arg::with_name("file")
                             .help("File to program to FPGA")
                             .required(true))))
        .subcommand(SubCommand::with_name("flash")
            .about("Read/write flash memory")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(SubCommand::with_name("id")
                        .about("Read flash ID"))
            .subcommand(SubCommand::with_name("erase")
                        .about("Completely erase flash"))
            .subcommand(SubCommand::with_name("program")
                        .about("Program flash chip with binary data from file")
                        .arg(Arg::with_name("file")
                             .help("File to write to flash")
                             .required(true))
                        .arg(Arg::with_name("offset")
                             .help("Start address (in bytes) to read from")
                             .long("offset")
                             .default_value("0"))
                        .arg(Arg::with_name("no-verify")
                             .help("Disable automatic readback verification")
                             .short("n")
                             .long("no-verify")))
            .subcommand(SubCommand::with_name("read")
                        .about("Read contents of flash chip to file")
                        .arg(Arg::with_name("file")
                             .help("File to write with contents of flash")
                             .required(true))
                        .arg(Arg::with_name("length")
                             .help("Length (in bytes) to read from flash")
                             .long("length")
                             .default_value("135183"))
                        .arg(Arg::with_name("offset")
                             .help("Start address (in bytes) to read from")
                             .long("offset")
                             .default_value("0"))))
        .subcommand(SubCommand::with_name("bootload")
            .about("Reset FFP hardware into USB bootloader"))
        .subcommand(SubCommand::with_name("devices")
            .about("List available FFP devices"))
        .get_matches();

    let t0 = Instant::now();
    let context = libusb::Context::new().expect("Error getting libusb context");
    let quiet = matches.is_present("quiet");

    // Special-case devices which does not need a programmer
    if matches.subcommand_name().unwrap() == "devices" {
        let devices = Programmer::get_serials(&context)?;
        match devices.len() {
            0 => println!("No FFP devices found."),
            _ => {
                match devices.len() {
                    1 => println!("1 device found:"),
                    _ => println!("{} devices found:", devices.len()),
                }
                for (idx, serial) in devices.iter().enumerate() {
                    println!("    {}: {}", idx, serial);
                }
            },
        }
        return Ok(());
    }

    let programmer = if matches.is_present("serial") {
        Programmer::by_serial(&context, matches.value_of("serial").unwrap())
    } else if matches.is_present("index") {
        Programmer::by_index(&context, value_t!(matches.value_of("index"), usize).unwrap())
    } else {
        Programmer::find(&context)
    }?;

    match matches.subcommand_name() {
        Some("fpga") => {
            let fpga = FPGA::new(&programmer);
            let matches = matches.subcommand_matches("fpga").unwrap();
            match matches.subcommand_name() {
                Some("reset") => {
                    if !quiet { println!("Resetting FPGA") };
                    fpga.reset()?;
                },
                Some("power") => {
                    let matches = matches.subcommand_matches("power").unwrap();
                    let arg = matches.value_of("power").unwrap();
                    if arg == "on" {
                        if !quiet { println!("Turning on target power") };
                        fpga.power_on()?;
                    } else if arg == "off" {
                        if !quiet { println!("Turning off target power") };
                        fpga.power_off()?;
                    }
                },
                Some("program") => {
                    if !quiet { println!("Programming FPGA") };
                    let matches = matches.subcommand_matches("program").unwrap();
                    let path = matches.value_of("file").unwrap();
                    let mut file = File::open(path)?;
                    let mut data = Vec::new();
                    file.read_to_end(&mut data)?;
                    fpga.program(&data)?;
                },
                _ => panic!(),
            }
        },
        Some("flash") => {
            let flash = Flash::new(&programmer);
            let id = flash.read_id().expect("Error reading flash ID");
            if !quiet { println!("Flash ID: {}", id) };
            let matches = matches.subcommand_matches("flash").unwrap();
            match matches.subcommand_name() {
                Some("id") => {
                    if quiet { println!("Flash ID: {}", id) };
                },
                Some("erase") => {
                    if !quiet { println!("Erasing flash") };
                    flash.erase()?;
                },
                Some("program") => {
                    if !quiet { println!("Programming flash") };
                    let matches = matches.subcommand_matches("program").unwrap();
                    let path = matches.value_of("file").unwrap();
                    let offset = value_t!(matches.value_of("offset"), u32).unwrap();
                    let verify = !matches.is_present("no-verify");
                    let mut file = File::open(path)?;
                    let mut data = Vec::new();
                    file.read_to_end(&mut data)?;
                    flash.program(offset, &data, verify)?;
                    programmer.unreset()?;
                },
                Some("read") => {
                    if !quiet { println!("Reading flash to file") };
                    let matches = matches.subcommand_matches("read").unwrap();
                    let path = matches.value_of("file").unwrap();
                    let offset = value_t!(matches.value_of("offset"), u32).unwrap();
                    let length = value_t!(matches.value_of("length"), usize).unwrap();
                    let mut file = File::create(path)?;
                    let data = flash.read(offset, length)?;
                    file.write_all(&data)?;
                },
                _ => panic!(),
            }
        },
        Some("bootload") => {
            if !quiet { println!("Resetting FFP into bootloader") };
            programmer.bootload()?;
        },
        _ => panic!(),
    };

    let t1 = t0.elapsed();
    if !quiet { println!("Finished in {}.{:02}s", t1.as_secs(), t1.subsec_millis()/10) };

    Ok(())
}
