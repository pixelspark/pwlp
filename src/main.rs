mod pwlp;
mod test;
extern crate clap;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use pwlp::parser::parse;
use pwlp::program::Program;
use pwlp::server::{DeviceConfig, Server};
use pwlp::strip;
use pwlp::vm::VM;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdin, Read, Write};

#[cfg(feature = "raspberrypi")]
extern crate rppal;

#[cfg(feature = "raspberrypi")]
use rppal::spi;

#[derive(Deserialize, Debug, Clone)]
struct Config {
	bind_address: Option<String>,
	secret: Option<String>,
	program: Option<String>,
	devices: Option<HashMap<String, DeviceConfig>>,
}

fn vm_from_options(options: &ArgMatches) -> VM {
	let length = options
		.value_of("length")
		.unwrap_or("10")
		.parse::<u8>()
		.unwrap();

	let strip = strip::DummyStrip::new(length, true);
	let mut vm = VM::new(Box::new(strip));

	#[cfg(feature = "raspberrypi")]
	{
		if options.is_present("hardware") {
			let spi_bus = match options.value_of("bus") {
				Some(bus_str) => match bus_str {
					"0" => spi::Bus::Spi0,
					"1" => spi::Bus::Spi1,
					"2" => spi::Bus::Spi2,
					_ => panic!("invalid SPI bus number (should be 0, 1 or 2)"),
				},
				None => spi::Bus::Spi0,
			};

			let ss = match options.value_of("ss") {
				Some(ss_str) => match ss_str {
					"0" => spi::SlaveSelect::Ss0,
					"1" => spi::SlaveSelect::Ss1,
					"2" => spi::SlaveSelect::Ss2,
					_ => panic!("invalid SS number (should be 0, 1 or 2)"),
				},
				None => spi::SlaveSelect::Ss0,
			};

			let spi = spi::Spi::new(spi_bus, ss, 2_000_000, spi::Mode::Mode0)
				.expect("spi bus could not be created");
			let strip = strip::spi_strip::SPIStrip::new(spi, length);
			vm = VM::new(Box::new(strip));
		}
	}

	vm.set_trace(options.is_present("trace"));
	vm.set_deterministic(options.is_present("deterministic"));
	vm
}

fn main() -> std::io::Result<()> {
	let matches = App::new("pwlp-server")
		.version("1.0")
		.about("Pixelspark wireless LED protocol server")
		.author("Pixelspark")
		.subcommand(
			SubCommand::with_name("compile")
				.about("compiles a script to binary")
				.arg(
					Arg::with_name("file")
						.index(1)
						.takes_value(true)
						.help("the file to compile"),
				)
				.arg(
					Arg::with_name("output")
						.index(2)
						.takes_value(true)
						.help("the file to write binary output to"),
				),
		)
		.subcommand(
			SubCommand::with_name("disassemble")
				.about("disassemble binary file to instructions")
				.arg(
					Arg::with_name("file")
						.takes_value(true)
						.help("the binary to disassemble"),
				),
		)
		.subcommand(
			SubCommand::with_name("run")
				.about("run a script")
				.arg(Arg::with_name("file")
					.index(1)
					.takes_value(true)
					.help("the file to run")
				)
				.arg(Arg::with_name("binary")
						.short("b")
						.long("binary")
						.takes_value(false)
						.help("interpret source as binary"))
				.arg(Arg::with_name("hardware")
						.short("h")
						.long("hardware")
						.takes_value(false)
						.help("output to actual hardware (if supported)"))
				.arg(Arg::with_name("length")
						.long("length")
						.short("l")
						.takes_value(true)
						.value_name("10")
						.help("length of the LED strip"))
				.arg(Arg::with_name("bus")
						.long("bus")
						.takes_value(true)
						.value_name("0")
						.help("number of SPI bus to use"))
				.arg(Arg::with_name("ss")
						.long("ss")
						.takes_value(true)
						.value_name("0")
						.help("the slave-select port to use for the SPI bus"))
				.arg(Arg::with_name("instruction-limit")
						.long("instruction-limit")
						.takes_value(true)
						.value_name("0")
						.help("the maximum number of instructions to execute (default: 0 = no limit)"))
				.arg(Arg::with_name("deterministic")
						.long("deterministic")
						.takes_value(false)
						.help("make output of non-deterministic functions (time, randomness) deterministic (For testing purposes)"))
				.arg(Arg::with_name("trace")
						.short("t")
						.long("trace")
						.takes_value(false)
						.help("show instructions as they are executed")
				),
		)
		.subcommand(
			SubCommand::with_name("client")
				.about("run as client")
				.arg(Arg::with_name("hardware")
						.short("h")
						.long("hardware")
						.takes_value(false)
						.help("output to actual hardware (if supported)"))
				.arg(Arg::with_name("length")
						.long("length")
						.short("l")
						.takes_value(true)
						.value_name("10")
						.help("length of the LED strip"))
				.arg(Arg::with_name("bus")
						.long("bus")
						.takes_value(true)
						.value_name("0")
						.help("number of SPI bus to use"))
				.arg(Arg::with_name("ss")
						.long("ss")
						.takes_value(true)
						.value_name("0")
						.help("the slave-select port to use for the SPI bus"))
				.arg(Arg::with_name("trace")
						.short("t")
						.long("trace")
						.takes_value(false)
						.help("show instructions as they are executed")
				),
		)
		.subcommand(
			SubCommand::with_name("serve")
				.about("start server")
				.arg(
					Arg::with_name("bind")
						.short("b")
						.long("bind")
						.value_name("0.0.0.0:33333")
						.help("Address the server should listen at (overrides default key set in config)")
						.takes_value(true),
				)
				.arg(
					Arg::with_name("secret")
						.short("s")
						.long("secret")
						.value_name("secret")
						.help("Default HMAC-SHA1 key to use for signing messages when no device-specific key is configured (overrides default key set in config)")
						.takes_value(true)
				)
				.arg(
					Arg::with_name("program")
						.short("p")
						.long("program")
						.value_name("program.bin")
						.help("Default program to serve when no device-specific program has been set (overrides default program file name set in config)")
						.takes_value(true)
				)
				.arg(
					Arg::with_name("config")
						.short("c")
						.long("config")
						.value_name("config.toml")
						.help("Config file to read")
						.takes_value(true),
				),
		)
		.setting(AppSettings::ArgRequiredElseHelp)
		.get_matches();

	// Read configuration file
	let config_file = matches.value_of("config").unwrap_or("config.toml");
	let mut config_string = String::new();
	File::open(config_file)?.read_to_string(&mut config_string)?;
	let config: Config = toml::from_str(&config_string)?;

	// Find out which subcommand to perform
	if let Some(_client_matches) = matches.subcommand_matches("client") {
		//// let vm = vm_from_options(&client_matches);
		unimplemented!();
	} else if let Some(run_matches) = matches.subcommand_matches("run") {
		let interpret_as_binary = run_matches.is_present("binary");

		let program = if interpret_as_binary {
			let mut source = Vec::<u8>::new();
			if let Some(source_file) = run_matches.value_of("file") {
				File::open(source_file)?.read_to_end(&mut source)?;
			} else {
				stdin().read_to_end(&mut source)?;
			}
			Program::from_binary(source)
		} else {
			let mut source = String::new();
			if let Some(source_file) = run_matches.value_of("file") {
				File::open(source_file)?.read_to_string(&mut source)?;
			} else {
				stdin().read_to_string(&mut source)?;
			}
			match parse(&source) {
				Ok(prg) => prg,
				Err(s) => panic!("Parsing failed: {}", s),
			}
		};

		let instruction_limit: Option<usize> = if run_matches.is_present("instruction-limit") {
			Some(
				run_matches
					.value_of("instruction-limit")
					.unwrap()
					.parse::<usize>()
					.expect("invalid limit number"),
			)
		} else {
			None
		};

		let mut vm = vm_from_options(&run_matches);
		let mut state = vm.start(program);
		state.run(instruction_limit);
	} else if let Some(matches) = matches.subcommand_matches("compile") {
		let mut source = String::new();
		if let Some(source_file) = matches.value_of("file") {
			File::open(source_file)?.read_to_string(&mut source)?;
		} else {
			stdin().read_to_string(&mut source)?;
		}

		match parse(&source) {
			Ok(prg) => {
				if !matches.is_present("output") {
					println!("Program:\n{:?}", &prg);
				}
				if let Some(out_file) = matches.value_of("output") {
					File::create(out_file)?.write_all(&prg.code)?;
				}
			}
			Err(s) => println!("Error: {}", s),
		};
	} else if let Some(matches) = matches.subcommand_matches("disassemble") {
		let mut source = Vec::<u8>::new();
		if let Some(source_file) = matches.value_of("file") {
			File::open(source_file)?.read_to_end(&mut source)?;
		} else {
			stdin().read_to_end(&mut source)?;
		}

		let program = Program::from_binary(source);
		println!("{:?}", program);
	} else if let Some(matches) = matches.subcommand_matches("serve") {
		let global_secret = config
			.secret
			.unwrap_or_else(|| String::from(matches.value_of("bind").unwrap_or("secret")));

		let default_program = match matches.value_of("program") {
			Some(path) => Program::from_file(&path).expect("error reading specified program file"),
			None => match config.program {
				Some(path) => {
					Program::from_file(&path).expect("program specified in config not found")
				}
				None => default_serve_program(),
			},
		};

		// Start server
		// Figure out bind address and open socket
		let config_bind_address = config
			.bind_address
			.unwrap_or_else(|| String::from("0.0.0.0:33333"));
		let bind_address = matches.value_of("bind").unwrap_or(&config_bind_address);

		let mut server = Server::new(config.devices, &global_secret, default_program);
		println!("PWLP server listening at {}", bind_address);
		server.run(bind_address)?;
	};
	Ok(())
}

fn default_serve_program() -> Program {
	// Use a hardcoded default program
	let mut program = Program::new();
	program
		.push(0) // let counter = 0
		.repeat_forever(|p| {
			// while(true)
			p.inc() // counter++
				.get_length() // let length = get_length()
				.r#mod() // counter % length
				.get_length() // let led_counter = get_length()
				.repeat(|q| {
					// while(--led_counter)
					q.dup()
						.peek(2)
						.lte() // led_counter <= length
						.if_zero(|r| {
							r.peek(1)
								.push(0xFF_00_00_00)
								.or() // led_value = 0xFF000000 | led_counter
								.set_pixel() // set_pixel(led_value)
								.pop(1);
						})
						.if_not_zero(|r| {
							r.peek(1).push(0x00_FF_00_00).or().set_pixel().pop(1);
						})
						.pop(1);
				})
				.blit()
				.pop(1)
				.r#yield();
		});
	program
}
