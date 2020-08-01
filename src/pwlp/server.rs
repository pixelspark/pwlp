use super::program::Program;
use super::protocol::{Message, MessageType};
use eui48::MacAddress;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceConfig {
	program: Option<String>,
	secret: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DeviceStatus {
	pub address: SocketAddr,
	pub program: Option<Program>,
	
	#[serde(skip)]
	pub secret: String,

	#[serde(skip)]
	pub last_seen: Instant,
}

impl Serialize for Program {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_bytes(&self.code)
	}
}

pub struct ServerState {
	pub config: HashMap<String, DeviceConfig>,
	pub devices: HashMap<String, DeviceStatus>,
	pub socket: UdpSocket
}

pub struct Server {
	state: Arc<Mutex<ServerState>>,
	default_secret: String,
	default_program: Program,
}

impl Server {
	pub fn new(
		devices: HashMap<String, DeviceConfig>,
		default_secret: &str,
		default_program: Program,
		bind_address: &str
	) -> std::io::Result<Server> {
		Ok(Server {
			state: Arc::new(Mutex::new(ServerState {
				config: devices,
				devices: HashMap::new(),
				socket: UdpSocket::bind(bind_address)?
			})),
			default_secret: default_secret.to_string(),
			default_program,
		})
	}

	pub fn state(&mut self) -> Arc<Mutex<ServerState>> {
		self.state.clone()
	}

	pub fn run(&mut self) -> std::io::Result<()> {
		let socket = {
			let m = self.state.lock().unwrap();
			m.socket.try_clone()?
		};

		loop {
			let mut buf = [0; 1500];
			let (amt, source_address) = socket.recv_from(&mut buf)?;

			match Message::peek_mac_address(&buf[0..amt]) {
				Err(t) => log::error!("\tError reading MAC address: {:?}", t),
				Ok(mac) => {
					// Do we have a config for this mac?
					let canonical_mac = mac.to_canonical();
					let device_config: Option<DeviceConfig> = {
						let m = self.state.lock().unwrap();
						if m.config.contains_key(&canonical_mac) {
							Some(m.config[&canonical_mac].clone())
						} else {
							None
						}
					};

					// Find the secret to use to verify the message signature
					let secret = match &device_config {
						Some(d) => match &d.secret {
							Some(s) => s.clone(),
							None => self.default_secret.clone(),
						},
						None => self.default_secret.clone(),
					};

					// Decode message
					match Message::from_buffer(&buf[0..amt], secret.as_bytes()) {
						Err(t) => log::error!(
							"{} error {:?} (size={}b source={} secret={:?})",
							source_address, t, amt, mac, secret
						),
						Ok(msg) => {
							let mac_identifier = mac.to_canonical();
							log::info!(
								"{} @ {}: {:?} t={}",
								&mac_identifier, &source_address, msg.message_type, msg.unix_time
							);

							// Update or create device status
							{
								let mut m = self.state.lock().unwrap();
								let mut new_status = match m.devices.get(&mac_identifier) {
									Some(status) => (*status).clone(),
									None => DeviceStatus {
										address: source_address,
										program: None,
										secret: secret.clone(),
										last_seen: Instant::now(),
									},
								};
								new_status.last_seen = Instant::now();
								
								match msg.message_type {
									MessageType::Ping => {
										let pong = Message {
											message_type: MessageType::Pong,
											unix_time: msg.unix_time,
											mac_address: MacAddress::nil(),
											payload: None,
										};

										// Check deserialize
										let secret_bytes = secret.as_bytes();
										assert!(
											Message::from_buffer(&pong.signed(secret_bytes), secret_bytes).is_ok(),
											"deserialize own message"
										);

										if let Err(t) =
											socket.send_to(&pong.signed(secret.as_bytes()), source_address)
										{
											println!("Send pong failed: {:?}", t);
										}

										let device_program = if let Some(p) = new_status.program {
											p
										}
										else if let Some(config) = &device_config {
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
											unix_time: msg.unix_time,
											mac_address: MacAddress::nil(),
											payload: Some(device_program.clone().code),
										};

										new_status.program = Some(device_program);

										if let Err(t) =
											socket.send_to(&run.signed(secret.as_bytes()), source_address)
										{
											println!("Send pong failed: {:?}", t);
										}
									}
									MessageType::Pong => {
										// Ignore
									}
									_ => {}
								}

								m.devices.insert(mac_identifier, new_status);
							}
						}
					}
				}
			}
		}
	}
}
