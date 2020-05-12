use super::program::Program;
use super::protocol::{Message, MessageType};
use super::strip::Strip;
use super::vm::{Outcome, VM};
use eui48::MacAddress;
use mac_address::get_mac_address;
use std::convert::TryInto;
use std::error::Error;
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

pub struct Client {
	vm: VM,
	secret: Vec<u8>,
	fps_limit: Option<usize>,
}

impl dyn Strip {
	fn set_all_pixels_to(&mut self, r: u8, g: u8, b: u8) {
		for i in 0..self.length() {
			self.set_pixel(i, r, g, b);
		}
		self.blit();
	}
}

impl Client {
	pub fn new(vm: VM, secret: &[u8], fps_limit: Option<usize>) -> Client {
		Client {
			vm,
			secret: secret.to_vec(),
			fps_limit,
		}
	}

	pub fn run(&mut self, bind_address: &str, server_address: &str) -> Result<(), Box<dyn Error>> {
		// Set everything to the same color
		self.vm.strip().set_all_pixels_to(0, 0, 0);

		let mac = get_mac_address()?.expect("could not obtain own MAC address");
		let mac_address =
			MacAddress::from_bytes(&mac.bytes()).expect("reading MAC address from bytes failed");

		// Start networking thread
		let secret = self.secret.to_owned();
		let bind_address = bind_address.to_owned();
		let server_address = server_address.to_owned();
		println!(
			"Running as client with MAC {} at {} with server {}",
			mac_address, bind_address, server_address
		);
		let (tx, rx) = mpsc::channel();

		thread::spawn(move || {
			println!("Client binding to address {}", bind_address);
			let socket = UdpSocket::bind(bind_address).expect("could not bind to address");

			socket
				.set_read_timeout(Some(Duration::from_secs(1)))
				.unwrap();

			let mut last_ping_time = SystemTime::now();
			let ping_interval = Duration::from_secs(30);

			loop {
				// Send a welcome message
				let welcome = Message::new(MessageType::Ping, mac_address, None)
					.expect("message construction failed");
				let signed = welcome.signed(&secret);
				println!("Sending welcome to server {}", server_address);
				socket
					.send_to(&signed, &server_address)
					.expect("send_to failed");

				while SystemTime::now().duration_since(last_ping_time).unwrap() < ping_interval {
					let mut buf = [0; 1500];
					match socket.recv_from(&mut buf) {
						Ok((amt, source_address)) => {
							println!("Received {} from {}", amt, source_address);

							// Decode message (from_buffer verifies HMAC)
							match Message::from_buffer(&buf[0..amt], &secret) {
								Err(t) => println!(
									"{} error {:?} (size={}b secret={:?})",
									source_address, t, amt, secret
								),
								Ok(m) => {
									println!(
										"{}: {:?} t={}",
										source_address, m.message_type, m.unix_time
									);

									// TODO check message time
									match m.message_type {
										MessageType::Run => {
											if let Some(payload) = m.payload {
												tx.send(Program::from_binary(payload)).unwrap();
											} else {
												// Run empty program
												tx.send(Program::new()).unwrap();
											}
										}
										MessageType::Pong
										| MessageType::Ping
										| MessageType::Set
										| MessageType::Unknown => {
											// Ignore
											println!("Ignoring message");
										}
									}
								}
							}
						}
						Err(e) => {
							if e.kind() != std::io::ErrorKind::WouldBlock {
								panic!(e)
							} else {
								// Time-out, which is expected
							}
						}
					}
				}
				last_ping_time = SystemTime::now();
			}
		});

		// Strip thread
		let mut program = Some(rx.recv()?);
		loop {
			let p = program;
			program = None;

			if let Some(p) = &p {
				println!("Starting program:\n{:?}", p);
			}
			let mut state = self.vm.start(p.unwrap(), None);
			let mut last_yield_time = SystemTime::now();
			let frame_time = if let Some(fps) = self.fps_limit {
				Some(Duration::from_millis((1000 / fps).try_into().unwrap()))
			} else {
				None
			};
			let mut running = true;

			let instruction_limit_per_cycle = 1000;

			while running {
				let outcome = state.run(Some(instruction_limit_per_cycle));

				// See if there is a new program waiting
				if let Ok(p) = rx.try_recv() {
					println!("set new program {:?}", p);
					program = Some(p);
					running = false;
				// Go into next iteration and start new program
				} else {
					match outcome {
						Outcome::LocalInstructionLimitReached => {
							// Just continue on a new cycle
						}
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
						Outcome::GlobalInstructionLimitReached | Outcome::Ended => {
							// Await a new program
							program = Some(rx.recv().unwrap());
							running = false;
						}
						Outcome::Error(e) => {
							println!(
								"Error in VM at pc={}: {:?}, awaiting next program",
								state.pc(),
								e
							);
							program = Some(rx.recv().unwrap());
							running = false;
						}
					}
				}
			}
		}
	}
}
