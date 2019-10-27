extern crate hmacsha1;
use hmacsha1::hmac_sha1;

use std::convert::TryInto;
use eui48::{MacAddress};

#[derive(Debug)]
#[repr(u8)]
pub enum MessageType {
	Ping = 0x01,
	Pong = 0x02,
	Set = 0x03,
	Run = 0x04,
	Unknown = 0xFF
}

impl MessageType {
	pub fn from(t: u8) -> MessageType {
		match t {
			0x01 => MessageType::Ping,
			0x02 => MessageType::Pong,
			0x03 => MessageType::Set,
			0x04 => MessageType::Run,
			_ => MessageType::Unknown
		}
	}
}

#[derive(Debug)]
pub enum MessageError {
	SignatureInvalid,
	MessageTooShort,
	MacAddressInvalid
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Message {
	mac_address: MacAddress,
	unix_time: u32,
	message_type: MessageType,
}

const SHA1_SIZE: usize = 20;

impl Message {
	pub fn peek_mac_address(buffer: &[u8]) -> Result<MacAddress, MessageError> {
		if buffer.len() < (SHA1_SIZE + 6) {
			return Err(MessageError::MessageTooShort);
		}

		match MacAddress::from_bytes(&buffer[0..6]) {
			Ok(m) => return Ok(m),
			Err(()) => return Err(MessageError::MacAddressInvalid)
		}
	}

	pub fn from_buffer(buffer: &[u8], key: &[u8]) -> Result<Message, MessageError> {
		let data_size = buffer.len() - SHA1_SIZE;
		if data_size < 6 {
			return Err(MessageError::MessageTooShort);
		}

		// Verify message signature
		let calculated_hmac = hmac_sha1(key, &buffer[0..data_size]);
		let provided_hmac = &buffer[data_size..(data_size + SHA1_SIZE)];

		// Verify HMAC
		if calculated_hmac != provided_hmac {
			return Err(MessageError::SignatureInvalid);
		}

		// MAC address
		let mac_address = Message::peek_mac_address(buffer)?;
		let type_number = buffer[(6 + 4)];

		Ok(Message {
			mac_address: mac_address,
			unix_time: u32::from_le_bytes(buffer[6..10].try_into().unwrap()),
			message_type: MessageType::from(type_number)
		})
	}
}