use super::program::Program;
use super::protocol::{Message, MessageType};
use eui48::MacAddress;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::UdpSocket;

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceConfig {
	program: Option<String>,
	secret: Option<String>,
}

pub struct Server {
	devices: Option<HashMap<String, DeviceConfig>>,
	default_secret: String,
	default_program: Program,
}

impl Server {
	pub fn new(
		devices: Option<HashMap<String, DeviceConfig>>,
		default_secret: &str,
		default_program: Program,
	) -> Server {
		Server {
			devices,
			default_secret: default_secret.to_string(),
			default_program,
		}
	}

	pub fn run(&mut self, bind_address: &str) -> std::io::Result<()> {
		let socket = UdpSocket::bind(bind_address)?;

		loop {
			let mut buf = [0; 1500];
			let (amt, source_address) = socket.recv_from(&mut buf)?;

			match Message::peek_mac_address(&buf[0..amt]) {
				Err(t) => println!("\tError reading MAC address: {:?}", t),
				Ok(mac) => {
					// Do we have a config for this mac?
					let device_config: Option<&DeviceConfig> = match &self.devices {
						Some(devices) => Some(&devices[&mac.to_canonical()]),
						None => None,
					};

					// Find the secret to use to verify the message signature
					let secret = match &device_config {
						Some(d) => match &d.secret {
							Some(s) => s.as_bytes(),
							None => &self.default_secret.as_bytes(),
						},
						None => &self.default_secret.as_bytes(),
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
											self.default_program.clone()
										}
									} else {
										self.default_program.clone()
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
	}
}
