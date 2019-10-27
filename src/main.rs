mod pwlp;
extern crate clap;

use std::net::UdpSocket;
use clap::{App, Arg};
use pwlp::{Message};
use serde::Deserialize;
use std::fs::{File};
use std::io::Read;

#[derive(Deserialize, Debug, Clone)]
struct Config {
	bind_address: Option<String>,
	secret: Option<String>,
	devices: Option<Vec<DeviceConfig>>,
}

#[derive(Deserialize, Debug, Clone)]
struct DeviceConfig {
	mac: String,
	secret: Option<String>
}

fn main() -> std::io::Result<()> {
	let matches = App::new("pwlp-server")
		.version("1.0")
		.about("Pixelspark wireless LED protocol server")
		.author("Pixelspark")
		.arg(Arg::with_name("bind")
			.short("b")
			.long("bind")
			.value_name("ADDRESS")
			.help("Address the server should listen at")
			.takes_value(true))
		.arg(Arg::with_name("config")
			.short("c")
			.long("config")
			.value_name("FILE")
			.help("Config file to read")
			.takes_value(true))
		.get_matches(); 

	// Read configuration file
	let config_file = matches.value_of("config").unwrap_or("config.toml");
	let mut config_string = String::new();
	File::open(config_file)?.read_to_string(&mut config_string)?;
	let config: Config = toml::from_str(&config_string)?;

	// Figure out bind address and open socket
	let config_bind_address = config.bind_address.unwrap_or(String::from("0.0.0.0:33333"));
	let bind_address = matches.value_of("bind").unwrap_or(&config_bind_address);
	let socket = UdpSocket::bind(bind_address).expect("could not bind to socket");

	let default_secret = String::from("secret");
	let global_secret = config.secret.unwrap_or(default_secret).as_bytes().to_owned();

	println!("PLWP server listening at {}", bind_address);

	loop {
		let mut buf = [0; 1500];
		let (amt, src) = socket.recv_from(&mut buf)?;
		println!("[{}]: {} bytes", src, amt);

		match Message::peek_mac_address(&buf[0..amt]) {
			Err(t) => println!("\tError reading MAC address: {:?}", t),
			Ok(mac) => {
				// Do we have a config for this mac?
				let device_config: Option<DeviceConfig> = match &config.devices {
					Some(devices) => {
						match devices.iter().find(|x| x.mac == mac.to_canonical()) {
							Some(d) => Some(d.clone()),
							None => None
						}
					},
					None => None
				};

				// Find the secret to use to verify the message signature
				let secret = match &device_config {
					Some(d) => match &d.secret {
						Some(s) => s.as_bytes(),
						None => &global_secret
					},
					None => &global_secret
				};

				// Decode message
				match Message::from_buffer(&buf[0..amt], secret) {
					Err(t) => println!("\tError {:?}", t),
					Ok(m) => println!("\tMessage {:?}", m)
				}
			}
		}
	}
}