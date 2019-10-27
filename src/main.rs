use std::net::UdpSocket;
use clap::{App, Arg}; 

mod pwlp;
use pwlp::{Message};

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
	   .get_matches(); 

	let bind_address = matches.value_of("bind").unwrap_or("0.0.0.0:33333");
	let socket = UdpSocket::bind(bind_address).expect("could not bind to socket");

	println!("PLWP server listening at {}", bind_address);

	loop {
		let mut buf = [0; 1500];
		let (amt, src) = socket.recv_from(&mut buf)?;
		println!("[{}]: {} bytes", src, amt);

		match Message::peek_mac_address(&buf[0..amt]) {
			Err(t) => println!("\tError reading MAC address: {:?}", t),
			Ok(mac) => {
				println!("\tMAC address is {:02x?}", mac);
				match Message::from_buffer(&buf[0..amt], b"Secret") {
					Err(t) => println!("\tError {:?}", t),
					Ok(m) => println!("\tMessage {:?}", m)
				}
			}
		}
	}
}