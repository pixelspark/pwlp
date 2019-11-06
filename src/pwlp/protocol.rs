use byteorder::{LittleEndian, WriteBytesExt};
use hmacsha1::hmac_sha1;

use eui48::MacAddress;
use std::convert::TryInto;

#[derive(Debug)]
#[repr(u8)]
pub enum MessageType {
	Ping,
	Pong,
	Set,
	Run,
	Unknown,
}

impl MessageType {
	pub fn from(t: u8) -> MessageType {
		match t {
			0x01 => MessageType::Ping,
			0x02 => MessageType::Pong,
			0x03 => MessageType::Set,
			0x04 => MessageType::Run,
			_ => MessageType::Unknown,
		}
	}
}

impl From<&MessageType> for u8 {
	fn from(v: &MessageType) -> u8 {
		match v {
			MessageType::Ping => 0x01,
			MessageType::Pong => 0x02,
			MessageType::Set => 0x03,
			MessageType::Run => 0x04,
			_ => panic!("invalid message type"),
		}
	}
}

#[derive(Debug)]
pub enum MessageError {
	SignatureInvalid,
	MessageTooShort,
	MacAddressInvalid,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Message {
	pub mac_address: MacAddress,
	pub unix_time: u32,
	pub message_type: MessageType,
	pub payload: Option<Vec<u8>>,
}

const SHA1_SIZE: usize = 20;
const MAC_SIZE: usize = 6;
const MESSAGE_TYPE_SIZE: usize = 1;
const TIME_SIZE: usize = 4;

impl Message {
	// Wire format is [MAC: 6] [TIME: 4] [TYPE: 1] .... [SHA1: 20]
	pub fn peek_mac_address(buffer: &[u8]) -> Result<MacAddress, MessageError> {
		if buffer.len() < (SHA1_SIZE + MAC_SIZE) {
			return Err(MessageError::MessageTooShort);
		}

		match MacAddress::from_bytes(&buffer[0..6]) {
			Ok(m) => Ok(m),
			Err(()) => Err(MessageError::MacAddressInvalid),
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
		let type_number = buffer[(MAC_SIZE + TIME_SIZE)];

		let payload_size = data_size - MAC_SIZE - TIME_SIZE;

		Ok(Message {
			mac_address,
			unix_time: u32::from_le_bytes(
				buffer[MAC_SIZE..(MAC_SIZE + TIME_SIZE)].try_into().unwrap(),
			),
			message_type: MessageType::from(type_number),
			payload: match payload_size {
				0 => None,
				_ => Some(buffer[(buffer.len() - payload_size)..buffer.len()].to_vec()),
			},
		})
	}

	pub fn signed(&self, key: &[u8]) -> Vec<u8> {
		let data_size = MAC_SIZE
			+ TIME_SIZE + MESSAGE_TYPE_SIZE
			+ match &self.message_type {
				MessageType::Ping => 0,
				MessageType::Pong => 0,
				_ => 0,
			} + match &self.payload {
			None => 0,
			Some(p) => p.len(),
		};
		let mut buf = Vec::with_capacity(data_size + SHA1_SIZE);

		// Fill zero MAC
		buf.extend_from_slice(self.mac_address.as_bytes());

		buf.write_u32::<LittleEndian>(self.unix_time).unwrap();
		buf.push(u8::from(&self.message_type));
		if let Some(p) = &self.payload {
			buf.extend(p)
		}

		let signature = hmac_sha1(key, &buf[0..data_size]);
		buf.extend_from_slice(&signature);
		buf
	}
}
