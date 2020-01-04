use std::net::UdpSocket;
use super::vm::VM;
use super::protocol::{Message, MessageType};
use mac_address::get_mac_address;
use std::error::Error;
use eui48::MacAddress;

pub struct Client {
	vm: VM,
	secret: Vec<u8>
}

impl Client {
	pub fn new(vm: VM, secret: &[u8]) -> Client {
		Client {
			vm,
			secret: secret.to_vec()
		}
	}

	fn set_all_pixels_to(&mut self, r: u8, g: u8, b: u8) {
		let strip = self.vm.strip();
		for i in 0..strip.length() {
			strip.set_pixel(i, r, g, b);
		}
		strip.blit();
	}

	pub fn run(&mut self, bind_address: &str, server_address: &str) -> Result<(), Box<dyn Error>> {
		// Set everything to the same color
		self.set_all_pixels_to(255, 255, 255);

		println!("Client will bind to address {}", bind_address);
		let socket = UdpSocket::bind(bind_address)?;
		

		// Send a welcome message
		let mac = get_mac_address()?.expect("could not obtain own MAC address");
		let mac_address = MacAddress::from_bytes(&mac.bytes()).expect("reading MAC address from bytes failed");

		let welcome = Message::new(MessageType::Ping, &mac_address, None)?;
		let signed = welcome.signed(&self.secret);
		println!("Sending welcome to server {} from MAC {}", server_address, mac_address);
		socket.send_to(&signed, server_address)?;
		Ok(())
	}
}
