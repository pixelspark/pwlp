use std::fmt;

#[allow(dead_code)]
pub enum Prefix {
	POP = 0x0,
	PUSHB = 0x10,
	PEEK = 0x20,
	PUSHI = 0x30,
	JMP = 0x40,
	JZ = 0x50,
	JNZ = 0x60,
	UNARY = 0x70,
	BINARY = 0x80,
	USER = 0xE0,
	SPECIAL = 0xF0,
}

impl Prefix {
	pub fn from(code: u8) -> Option<Prefix> {
		match code & 0xF0 {
			0x0 => Some(Prefix::POP),
			0x10 => Some(Prefix::PUSHB),
			0x20 => Some(Prefix::PEEK),
			0x30 => Some(Prefix::PUSHI),
			0x40 => Some(Prefix::JMP),
			0x50 => Some(Prefix::JZ),
			0x60 => Some(Prefix::JNZ),
			0x70 => Some(Prefix::UNARY),
			0x80 => Some(Prefix::BINARY),
			0xE0 => Some(Prefix::USER),
			0xF0 => Some(Prefix::SPECIAL),
			_ => None,
		}
	}
}

impl std::fmt::Display for Prefix {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Prefix::POP => "POP",
				Prefix::PUSHB => "PUSHB",
				Prefix::PEEK => "PEEKB",
				Prefix::PUSHI => "PUSHI",
				Prefix::JMP => "JMP",
				Prefix::JZ => "JZ",
				Prefix::JNZ => "JNZ",
				Prefix::UNARY => "UNARY",
				Prefix::BINARY => "BINARY",
				Prefix::USER => "USER",
				Prefix::SPECIAL => "SPECIAL",
			}
		)
	}
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Special {
	SWAP = 12,
	DUMP = 13,
	YIELD = 14,
	TWOBYTE = 15,
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Unary {
	INC = 0,
	DEC = 1,
	NOT = 2,
	NEG = 3,
	SHL8 = 4,
	SHR8 = 5,
}

impl Unary {
	pub fn from(code: u8) -> Option<Unary> {
		match code {
			0 => Some(Unary::INC),
			1 => Some(Unary::DEC),
			2 => Some(Unary::NOT),
			3 => Some(Unary::NEG),
			4 => Some(Unary::SHL8),
			5 => Some(Unary::SHR8),
			_ => None,
		}
	}
}

impl std::fmt::Display for Unary {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Unary::INC => "INC",
				Unary::DEC => "DEC",
				Unary::NOT => "NOT",
				Unary::NEG => "NEG",
				Unary::SHL8 => "SHL8",
				Unary::SHR8 => "SHR8",
			}
		)
	}
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Binary {
	ADD = 0,
	SUB = 1,
	DIV = 2,
	MUL = 3,
	MOD = 4,
	AND = 5,
	OR = 6,
	XOR = 7,
	GT = 8,
	GTE = 9,
	LT = 10,
	LTE = 11,
	EQ = 12,
	NEQ = 13,
	SHL = 14,
	SHR = 15
}

impl Binary {
	pub fn from(code: u8) -> Option<Binary> {
		match code {
			0 => Some(Binary::ADD),
			1 => Some(Binary::SUB),
			2 => Some(Binary::DIV),
			3 => Some(Binary::MUL),
			4 => Some(Binary::MOD),
			5 => Some(Binary::AND),
			6 => Some(Binary::OR),
			7 => Some(Binary::XOR),
			8 => Some(Binary::GT),
			9 => Some(Binary::GTE),
			10 => Some(Binary::LT),
			11 => Some(Binary::LTE),
			12 => Some(Binary::EQ),
			13 => Some(Binary::NEQ),
			14 => Some(Binary::SHL),
			15 => Some(Binary::SHR),
			_ => None,
		}
	}
}

impl std::fmt::Display for Binary {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Binary::ADD => "ADD",
				Binary::AND => "AND",
				Binary::DIV => "DIV",
				Binary::EQ => "EQ",
				Binary::GT => "GT",
				Binary::GTE => "GTE",
				Binary::LT => "LT",
				Binary::LTE => "LTE",
				Binary::MOD => "MOD",
				Binary::MUL => "MUL",
				Binary::NEQ => "NEQ",
				Binary::OR => "OR",
				Binary::SUB => "SUB",
				Binary::XOR => "XOR",
				Binary::SHL => "SHL",
				Binary::SHR => "SHR"
			}
		)
	}
}

#[allow(dead_code, non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UserCommand {
	GET_LENGTH = 0,
	GET_WALL_TIME = 1,
	GET_PRECISE_TIME = 2,
	SET_PIXEL = 3,
	BLIT = 4,
	RANDOM_INT = 5,
	GET_PIXEL = 6
}
