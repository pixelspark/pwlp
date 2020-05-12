use super::program::Program;
use super::protocol::{Message, MessageType};
use eui48::MacAddress;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::{UdpSocket, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceConfig {
	program: Option<String>,
	secret: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DeviceStatus {
	address: SocketAddr,

	#[serde(skip)]
	last_seen: Instant
}

pub struct Server {
	devices: HashMap<String, DeviceConfig>,
	status: Arc<Mutex<HashMap<String, DeviceStatus>>>,
	default_secret: String,
	default_program: Program,
}

impl Server {
	pub fn new(
		devices: HashMap<String, DeviceConfig>,
		default_secret: &str,
		default_program: Program,
	) -> Server {
		Server {
			devices,
			status: Arc::new(Mutex::new(HashMap::new())),
			default_secret: default_secret.to_string(),
			default_program,
		}
	}

	pub fn status(&mut self) -> Arc<Mutex<HashMap<String, DeviceStatus>>> {
		self.status.clone()
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
					let canonical_mac = mac.to_canonical();
					let device_config: Option<&DeviceConfig> = if self.devices.contains_key(&canonical_mac) {
						Some(&self.devices[&canonical_mac])
					} else {
						None
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
							let mac_identifier = mac.to_canonical();
							println!(
								"{} @ {}: {:?} t={}",
								&mac_identifier,
								&source_address,
								m.message_type,
								m.unix_time
							);

							// Update or create device status
							{
								let mut m = self.status.lock().unwrap();
								let mut new_status = match m.get(&mac_identifier) {
									Some(status) => {
										(*status).clone()
									},
									None => {
										DeviceStatus {
											address: source_address.clone(),
											last_seen: Instant::now()
										}
									}
								};
								new_status.last_seen = Instant::now();
								println!("{} status: {:?}", &mac_identifier, &new_status);
								m.insert(mac_identifier, new_status);
							}

							match m.message_type {
								MessageType::Ping => {
									let pong = Message {
										message_type: MessageType::Pong,
										unix_time: m.unix_time,
										mac_address: MacAddress::nil(),
										payload: None,
									};

									// Check deserialize
									assert!(Message::from_buffer(&pong.signed(secret), secret).is_ok(), "deserialize own message");

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
