mod pwlp;
mod test;
extern crate clap;

use clap::{App, AppSettings, Arg, SubCommand};
use eui48::MacAddress;
use pwlp::parser::parse;
use pwlp::program::Program;
use pwlp::vm::VM;
use pwlp::{Message, MessageType};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdin, Read, Write};
use std::net::UdpSocket;

#[derive(Deserialize, Debug, Clone)]
struct Config {
	bind_address: Option<String>,
	secret: Option<String>,
	program: Option<String>,
	devices: Option<HashMap<String, DeviceConfig>>,
}

#[derive(Deserialize, Debug, Clone)]
struct DeviceConfig {
	program: Option<String>,
	secret: Option<String>,
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

	// Find out which subcommand to perform
	if let Some(run_matches) = matches.subcommand_matches("run") {
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

		let mut vm = VM::new(run_matches.is_present("trace"));
		vm.run(&program);
	}
	if let Some(matches) = matches.subcommand_matches("compile") {
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
		if let Some(source_file) = matches.value_of("binary") {
			File::open(source_file)?.read_to_end(&mut source)?;
		} else {
			stdin().read_to_end(&mut source)?;
		}

		let program = Program::from_binary(source);
		println!("{:?}", program);
	} else if let Some(matches) = matches.subcommand_matches("serve") {
		// Read configuration file
		let config_file = matches.value_of("config").unwrap_or("config.toml");
		let mut config_string = String::new();
		File::open(config_file)?.read_to_string(&mut config_string)?;
		let config: Config = toml::from_str(&config_string)?;

		// Start server
		// Figure out bind address and open socket
		let config_bind_address = config
			.bind_address
			.unwrap_or_else(|| String::from("0.0.0.0:33333"));
		let bind_address = matches.value_of("bind").unwrap_or(&config_bind_address);
		let socket = UdpSocket::bind(bind_address).expect("could not bind to socket");

		let global_secret = config
			.secret
			.unwrap_or_else(|| String::from(matches.value_of("bind").unwrap_or("secret")))
			.as_bytes()
			.to_owned();

		let default_program = match matches.value_of("program") {
			Some(path) => Program::from_file(&path).expect("error reading specified program file"),
			None => {
				match config.program {
					Some(path) => {
						Program::from_file(&path).expect("program specified in config not found")
					}
					None => {
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
												r.peek(1)
													.push(0x00_FF_00_00)
													.or()
													.set_pixel()
													.pop(1);
											})
											.pop(1);
									})
									.blit()
									.pop(1)
									.r#yield();
							});
						program
					}
				}
			}
		};
		println!("PWLP server listening at {}", bind_address);

		loop {
			let mut buf = [0; 1500];
			let (amt, source_address) = socket.recv_from(&mut buf)?;

			match Message::peek_mac_address(&buf[0..amt]) {
				Err(t) => println!("\tError reading MAC address: {:?}", t),
				Ok(mac) => {
					// Do we have a config for this mac?
					let device_config: Option<&DeviceConfig> = match &config.devices {
						Some(devices) => Some(&devices[&mac.to_canonical()]),
						None => None,
					};

					// Find the secret to use to verify the message signature
					let secret = match &device_config {
						Some(d) => match &d.secret {
							Some(s) => s.as_bytes(),
							None => &global_secret,
						},
						None => &global_secret,
					};

					// Decode message
					match Message::from_buffer(&buf[0..amt], secret) {
						Err(t) => println!(
							"{} error {:?} (size={}b source={} secret={:?})",
							source_address, t, amt, mac, secret
						),
						Ok(m) => {
							println!(
								"{} @ {}: {:?} t={}",
								mac.to_canonical(),
								source_address,
								m.message_type,
								m.unix_time
							);

							match m.message_type {
								MessageType::Ping => {
									let pong = Message {
										message_type: MessageType::Pong,
										unix_time: m.unix_time,
										mac_address: MacAddress::nil(),
										payload: None,
									};

									// Check deserialize
									Message::from_buffer(&pong.signed(secret), secret)
										.expect("deserialize own message");

									if let Err(t) =
										socket.send_to(&pong.signed(secret), source_address)
									{
										println!("Send pong failed: {:?}", t);
									}

									let device_program = if let Some(config) = &device_config {
										if let Some(path) = &config.program {
											Program::from_file(&path)
												.expect("error loading device-specific program")
										} else {
											default_program.clone()
										}
									} else {
										default_program.clone()
									};

									let run = Message {
										message_type: MessageType::Run,
										unix_time: m.unix_time,
										mac_address: MacAddress::nil(),
										payload: Some(device_program.code),
									};

									if let Err(t) =
										socket.send_to(&run.signed(secret), source_address)
									{
										println!("Send pong failed: {:?}", t);
									}
								}
								MessageType::Pong => {
									// Ignore
								}
								_ => {}
							}
						}
					}
				}
			}
		}
	};
	Ok(())
}
