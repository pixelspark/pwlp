mod pwlp;
mod test;
extern crate clap;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use pwlp::client::Client;
use pwlp::parser::parse;
use pwlp::program::Program;
use pwlp::server::{DeviceConfig, Server};
use pwlp::strip;
use pwlp::vm::{Outcome, VM};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdin, Read, Write};
use std::time::{Duration, SystemTime};

#[cfg(feature = "raspberrypi")]
extern crate rppal;

#[cfg(feature = "raspberrypi")]
use rppal::spi;

#[derive(Deserialize, Debug, Clone)]
struct Config {
	client: Option<ClientConfig>,
	server: Option<ServerConfig>,
	#[cfg(feature = "api")]
	api: Option<pwlp::api::APIConfig>,
}

#[derive(Deserialize, Debug, Clone)]
struct ClientConfig {
	bind_address: Option<String>,
	server_address: Option<String>,
	secret: Option<String>,
	fps_limit: Option<usize>,
}

#[derive(Deserialize, Debug, Clone)]
struct ServerConfig {
	bind_address: Option<String>,
	server_address: Option<String>,
	secret: Option<String>,
	program: Option<String>,
	devices: Option<HashMap<String, DeviceConfig>>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let mut serve_subcommand = SubCommand::with_name("serve")
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
		);

	#[cfg(feature = "api")]
	{
		serve_subcommand = serve_subcommand.arg(
			Arg::with_name("no-api")
				.long("no-api")
				.help("Disables the HTTP API")
				.takes_value(false),
		);

		serve_subcommand = serve_subcommand.arg(
			Arg::with_name("bind-api")
				.long("bind-api")
				.value_name("127.0.0.1:33334")
				.help("Address the HTTP API server should listen at (overrides default key set in config)")
				.takes_value(true)
		);
	}

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
				.arg(Arg::with_name("fps-limit")
						.long("fps-limit")
						.takes_value(true)
						.value_name("0")
						.help("the maximum number of frames per second to execute (default = no limit)"))
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
				.arg(
					Arg::with_name("bind")
						.short("b")
						.long("bind")
						.value_name("0.0.0.0:33332")
						.help("Address the client should listen at (overrides default key set in config)")
						.takes_value(true),
				)
				.arg(Arg::with_name("secret")
						.long("secret")
						.takes_value(true)
						.value_name("secret")
						.help("secret key used to sign communications with the server"))
				.arg(Arg::with_name("server")
						.long("server")
						.takes_value(true)
						.value_name("0.0.0.0:33333")
						.help("address of the server"))
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
						.help("show instructions as they are executed"))
				.arg(Arg::with_name("fps-limit")
						.long("fps-limit")
						.takes_value(true)
						.value_name("60")
						.help("the maximum number of frames per second to execute (default = 60, 0 indicates no limit)"))
				.arg(Arg::with_name("initial")
						.long("initial")
						.takes_value(true)
						.help("path to the initial program to run on start-up")
					)
					.arg(Arg::with_name("binary")
						.long("binary")
						.takes_value(false)
						.help("interpret initial program file as binary"))
		)
		.subcommand(serve_subcommand)
		.setting(AppSettings::ArgRequiredElseHelp)
		.get_matches();

	// Read configuration file
	let config_file = matches.value_of("config").unwrap_or("config.toml");
	let mut config_string = String::new();
	match File::open(config_file) {
		Ok(mut config_opened) => {
			config_opened.read_to_string(&mut config_string)?;
		}
		Err(e) => {
			println!("failed to open configuration file: {:?}", e);
		}
	}
	let config: Config = toml::from_str(&config_string)?;

	// Find out which subcommand to perform
	if let Some(client_matches) = matches.subcommand_matches("client") {
		return client(config, client_matches);
	} else if let Some(run_matches) = matches.subcommand_matches("run") {
		return run(run_matches);
	} else if let Some(matches) = matches.subcommand_matches("compile") {
		return compile(matches);
	} else if let Some(matches) = matches.subcommand_matches("disassemble") {
		return disassemble(matches);
	} else if let Some(matches) = matches.subcommand_matches("serve") {
		return serve(config, matches).await;
	};
	Ok(())
}

fn client(config: Config, client_matches: &ArgMatches) -> std::io::Result<()> {
	let mut bind_address: String = String::from("0.0.0.0:33332");
	let mut secret: String = String::from("secret");
	let mut server_address: String = String::from("224.0.0.1:33333");
	let mut fps_limit = Some(60);

	// Read configured values
	if let Some(client_config) = config.client {
		if let Some(v) = client_config.bind_address {
			bind_address = v;
		}
		if let Some(v) = client_config.server_address {
			server_address = v;
		}
		if let Some(v) = client_config.secret {
			secret = v;
		}
		if let Some(v) = client_config.fps_limit {
			fps_limit = Some(v);
		}
	}

	// Read arguments
	if let Some(v) = client_matches.value_of("bind") {
		bind_address = v.to_string();
	}
	if let Some(v) = client_matches.value_of("server") {
		server_address = v.to_string();
	}
	if let Some(v) = client_matches.value_of("secret") {
		secret = v.to_string();
	}
	if let Some(v) = client_matches.value_of("fps-limit") {
		fps_limit = Some(v.parse().unwrap());
	}

	let initial_program = match client_matches.value_of("initial") {
		Some(path) => {
			// Interpret as binary?
			let interpret_as_binary = client_matches.is_present("binary");

			if interpret_as_binary {
				let mut source = Vec::<u8>::new();
				File::open(path)?.read_to_end(&mut source)?;
				Some(Program::from_binary(source))
			} else {
				let mut source = String::new();
				File::open(path)?.read_to_string(&mut source)?;
				match parse(&source) {
					Ok(prg) => Some(prg),
					Err(s) => panic!("Parsing default program failed: {}", s),
				}
			}
		}
		None => None,
	};

	if fps_limit == Some(0) {
		fps_limit = None;
	}

	let vm = vm_from_options(&client_matches);
	let mut client = Client::new(vm, &secret.as_bytes(), fps_limit);
	client
		.run(&bind_address, &server_address, initial_program)
		.expect("running the client failed");
	Ok(())
}

fn run(run_matches: &ArgMatches) -> std::io::Result<()> {
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

	let fps: Option<u64> = if run_matches.is_present("fps-limit") {
		Some(
			run_matches
				.value_of("fps-limit")
				.unwrap()
				.parse::<u64>()
				.expect("invalid FPS limit number"),
		)
	} else {
		None
	};

	let mut vm = vm_from_options(&run_matches);
	let mut state = vm.start(program, instruction_limit);
	let mut last_yield_time = SystemTime::now();
	let frame_time = if let Some(fps) = fps {
		Some(Duration::from_millis(1000 / fps))
	} else {
		None
	};
	let mut running = true;

	while running {
		match state.run(None) {
			Outcome::Yielded => {
				if let Some(frame_time) = frame_time {
					let now = SystemTime::now();
					let passed = now.duration_since(last_yield_time).unwrap();
					if passed < frame_time {
						// We have some time left in this frame, sit it out
						std::thread::sleep(frame_time - passed);
					}
					last_yield_time = now;
				}
			}
			Outcome::GlobalInstructionLimitReached
			| Outcome::LocalInstructionLimitReached
			| Outcome::Ended => running = false,
			Outcome::Error(e) => {
				println!("Error in VM at pc={}: {:?}", state.pc(), e);
			}
		}
	}
	Ok(())
}

fn compile(matches: &ArgMatches) -> std::io::Result<()> {
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
	Ok(())
}

fn disassemble(matches: &ArgMatches) -> std::io::Result<()> {
	let mut source = Vec::<u8>::new();
	if let Some(source_file) = matches.value_of("file") {
		File::open(source_file)?.read_to_end(&mut source)?;
	} else {
		stdin().read_to_end(&mut source)?;
	}

	let program = Program::from_binary(source);
	println!("{:?}", program);
	Ok(())
}

async fn serve(config: Config, serve_matches: &ArgMatches<'_>) -> std::io::Result<()> {
	let mut server = build_server(&config, serve_matches);
	let state = server.state();

	// Get bind address
	let mut bind_address = String::from("0.0.0.0:33333");
	if let Some(server_config) = &config.server {
		if let Some(v) = server_config.bind_address.clone() {
			bind_address = v;
		}
	}

	if let Some(v) = serve_matches.value_of("bind") {
		bind_address = v.to_string();
	}

	println!("PWLP server listening at {}", bind_address);
	let server_task = tokio::task::spawn_blocking(move || match server.run(&bind_address) {
		Ok(()) => (),
		Err(t) => println!("PWLP server ended with error: {:?}", t),
	});

	#[cfg(feature = "api")]
	{
		let mut api_config = config.api.clone().unwrap_or_else(pwlp::api::APIConfig::new);

		if let Some(v) = serve_matches.value_of("bind-api") {
			api_config.bind_address = Some(v.to_string());
		}

		if serve_matches.is_present("no-api") {
			api_config.enabled = false;
		}

		let (_, _) = tokio::join!(pwlp::api::serve_http(&api_config, state), server_task);
	}

	#[cfg(not(feature = "api"))]
	tokio::join!(server_task);
	Ok(())
}

fn build_server(config: &Config, serve_matches: &ArgMatches<'_>) -> Server {
	let mut global_secret = String::from("secret");
	let mut default_program_path: Option<String> = None;
	let mut devices: HashMap<String, DeviceConfig> = HashMap::new();

	// Read configured values
	if let Some(server_config) = &config.server {
		if let Some(v) = &server_config.secret {
			global_secret = v.clone();
		}

		if let Some(v) = &server_config.program {
			default_program_path = Some(v.clone());
		}

		if let Some(d) = &server_config.devices {
			devices = d.clone();
		}
	}

	// Read arguments
	if let Some(v) = serve_matches.value_of("program") {
		default_program_path = Some(v.to_string());
	}
	if let Some(v) = serve_matches.value_of("secret") {
		global_secret = v.to_string();
	}

	let default_program = match default_program_path {
		Some(path) => Program::from_file(&path).expect("error reading specified program file"),
		None => default_serve_program(),
	};

	Server::new(devices, &global_secret, default_program)
}

fn vm_from_options(options: &ArgMatches) -> VM {
	let length = options
		.value_of("length")
		.unwrap_or("10")
		.parse::<u32>()
		.expect("length must be >0");

	if length == 0 {
		panic!("length cannot be zero");
	}

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

			let spi = spi::Spi::new(spi_bus, ss, 1_000_000, spi::Mode::Mode0)
				.expect("spi bus could not be created");
			let strip = strip::spi_strip::SPIStrip::new(spi, length);
			vm = VM::new(Box::new(strip));
		}
	}

	vm.set_trace(options.is_present("trace"));
	vm.set_deterministic(options.is_present("deterministic"));
	vm
}

fn default_serve_program() -> Program {
	parse(include_str!("../test/blink.txt")).unwrap()
}
