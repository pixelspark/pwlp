use std::fmt;
use std::fs::File;
use std::io::{Read, Write};

use super::instructions::{Binary, Prefix, Special, Unary, UserCommand};

#[derive(Clone)]
pub struct Program {
	pub(crate) code: Vec<u8>,
	pub(crate) stack_size: i32,
	pub(crate) offset: usize,
}

#[allow(dead_code)]
impl Program {
	fn write(&mut self, buffer: &[u8]) -> &mut Program {
		self.code.write_all(buffer).unwrap();
		self
	}

	pub fn from_binary(data: Vec<u8>) -> Program {
		Program {
			code: data,
			stack_size: 0,
			offset: 0,
		}
	}

	pub fn from_file(path: &str) -> std::io::Result<Program> {
		let mut stored_bin = Vec::<u8>::new();
		File::open(path)?.read_to_end(&mut stored_bin)?;
		Ok(Program {
			code: stored_bin,
			stack_size: 0,
			offset: 0,
		})
	}

	pub fn new() -> Program {
		Program {
			code: Vec::<u8>::new(),
			stack_size: 0,
			offset: 0,
		}
	}

	pub fn nop(&mut self) -> &mut Program {
		self.write(&[Prefix::POP as u8]) // POP 0
	}

	pub fn pop(&mut self, n: u8) -> &mut Program {
		assert!(n <= 15, "cannot pop more than 15 stack items");
		self.stack_size -= i32::from(n);
		self.write(&[Prefix::POP as u8 | n]) // POP n
	}

	pub fn peek(&mut self, n: u8) -> &mut Program {
		assert!(n <= 15, "cannot peek more than 15 stack items");
		self.stack_size += 1;
		self.write(&[Prefix::PEEK as u8 | n]) // PEEK n
	}

	pub fn unary(&mut self, u: Unary) -> &mut Program {
		self.write(&[Prefix::UNARY as u8 | u as u8]) // UNARY u
	}

	pub(crate) fn binary(&mut self, u: Binary) -> &mut Program {
		self.stack_size -= 1;
		self.write(&[Prefix::BINARY as u8 | u as u8]) // BINARY u
	}

	pub fn special(&mut self, u: Special) -> &mut Program {
		self.stack_size += match u {
			Special::DUMP => 0,
			Special::SWAP => 0,
			Special::YIELD => 0,
			Special::TWOBYTE => 0,
		};
		self.write(&[Prefix::SPECIAL as u8 | u as u8]) // SPECIAL u
	}

	pub fn user(&mut self, u: UserCommand) -> &mut Program {
		self.stack_size += match u {
			UserCommand::GET_LENGTH => 1,
			UserCommand::GET_PRECISE_TIME => 1,
			UserCommand::GET_WALL_TIME => 1,
			UserCommand::BLIT => 0,
			UserCommand::SET_PIXEL => 0,
			UserCommand::RANDOM_INT => 0,
		};
		self.write(&[Prefix::USER as u8 | u as u8]) // SPECIAL u
	}

	fn skip<F>(&mut self, prefix: Prefix, mut builder: F) -> &mut Program
	where
		F: FnMut(&mut Program) -> (),
	{
		let mut fragment = Program {
			code: Vec::<u8>::new(),
			stack_size: 0,
			offset: self.current_pc(),
		};
		builder(&mut fragment);
		assert!(
			fragment.stack_size == 0,
			"fragment in branch cannot modify stack size"
		);

		// Always write three-byte jumps for now
		let address = self.current_pc() + 3 + fragment.code.len();
		self.write(&[
			prefix as u8,
			(address & 0xFF) as u8,
			((address >> 8) & 0xFF) as u8,
		]);
		self.write(&fragment.code)
	}

	pub fn if_zero<F>(&mut self, builder: F) -> &mut Program
	where
		F: FnMut(&mut Program) -> (),
	{
		self.skip(Prefix::JNZ, builder)
	}

	pub fn if_not_zero<F>(&mut self, builder: F) -> &mut Program
	where
		F: FnMut(&mut Program) -> (),
	{
		self.skip(Prefix::JZ, builder)
	}

	pub fn repeat_forever<F>(&mut self, mut builder: F) -> &mut Program
	where
		F: FnMut(&mut Program) -> (),
	{
		let mut fragment = Program {
			code: Vec::<u8>::new(),
			stack_size: 0,
			offset: self.current_pc(),
		};
		builder(&mut fragment);
		assert!(
			fragment.stack_size == 0,
			"fragment in loop cannot modify stack size"
		);

		let start = self.current_pc();
		self.write(&fragment.code);
		self.write(&[
			Prefix::JMP as u8,
			(start & 0xFF) as u8,
			((start >> 8) & 0xFF) as u8,
		]);
		self
	}

	fn current_pc(&self) -> usize {
		self.offset + self.code.len()
	}

	pub fn repeat<F>(&mut self, mut builder: F) -> &mut Program
	where
		F: FnMut(&mut Program) -> (),
	{
		let mut fragment = Program {
			code: Vec::<u8>::new(),
			stack_size: 0,
			offset: self.current_pc(),
		};
		builder(&mut fragment);
		assert!(
			fragment.stack_size == 0,
			"fragment in loop cannot modify stack size"
		);

		let start = self.current_pc();
		self.write(&fragment.code);
		self.write(&[Prefix::UNARY as u8 | Unary::DEC as u8]);
		self.write(&[
			Prefix::JNZ as u8,
			(start & 0xFF) as u8,
			((start >> 8) & 0xFF) as u8,
		]);
		self
	}

	pub fn repeat_times<F>(&mut self, times: u32, builder: F) -> &mut Program
	where
		F: FnMut(&mut Program) -> (),
	{
		self.push(times);
		self.repeat(builder);
		self.pop(1)
	}

	pub fn inc(&mut self) -> &mut Program {
		self.unary(Unary::INC)
	}

	pub fn dec(&mut self) -> &mut Program {
		self.unary(Unary::DEC)
	}

	pub fn not(&mut self) -> &mut Program {
		self.unary(Unary::NOT)
	}

	pub fn neg(&mut self) -> &mut Program {
		self.unary(Unary::NEG)
	}

	pub fn add(&mut self) -> &mut Program {
		self.binary(Binary::ADD)
	}

	pub fn and(&mut self) -> &mut Program {
		self.binary(Binary::AND)
	}

	pub fn div(&mut self) -> &mut Program {
		self.binary(Binary::DIV)
	}

	pub fn gt(&mut self) -> &mut Program {
		self.binary(Binary::GT)
	}

	pub fn gte(&mut self) -> &mut Program {
		self.binary(Binary::GTE)
	}

	pub fn lt(&mut self) -> &mut Program {
		self.binary(Binary::LT)
	}

	pub fn lte(&mut self) -> &mut Program {
		self.binary(Binary::LTE)
	}

	pub fn r#mod(&mut self) -> &mut Program {
		self.binary(Binary::MOD)
	}

	pub fn mul(&mut self) -> &mut Program {
		self.binary(Binary::MUL)
	}

	pub fn or(&mut self) -> &mut Program {
		self.binary(Binary::OR)
	}

	pub fn sub(&mut self) -> &mut Program {
		self.binary(Binary::SUB)
	}

	pub fn xor(&mut self) -> &mut Program {
		self.binary(Binary::XOR)
	}

	pub fn dump(&mut self) -> &mut Program {
		self.special(Special::DUMP)
	}

	pub fn dup(&mut self) -> &mut Program {
		self.peek(0)
	}

	pub fn swap(&mut self) -> &mut Program {
		self.special(Special::SWAP)
	}

	pub fn r#yield(&mut self) -> &mut Program {
		self.special(Special::YIELD)
	}

	pub fn set_pixel(&mut self) -> &mut Program {
		self.user(UserCommand::SET_PIXEL)
	}

	pub fn blit(&mut self) -> &mut Program {
		self.user(UserCommand::BLIT)
	}

	pub fn get_length(&mut self) -> &mut Program {
		self.user(UserCommand::GET_LENGTH)
	}

	pub fn get_precise_time(&mut self) -> &mut Program {
		self.user(UserCommand::GET_PRECISE_TIME)
	}

	pub fn get_wall_time(&mut self) -> &mut Program {
		self.user(UserCommand::GET_WALL_TIME)
	}

	pub fn push(&mut self, b: u32) -> &mut Program {
		self.stack_size += 1;
		match b {
			0 => self.code.write(&[Prefix::PUSHB as u8]).unwrap(),
			_ if b <= 0xFF => self
				.code
				.write(&[Prefix::PUSHB as u8 | 0x01, b as u8])
				.unwrap(),
			_ => self
				.code
				.write(&[
					Prefix::PUSHI as u8 | 0x01,
					(b & 0xFF) as u8,
					((b >> 8) & 0xFF) as u8,
					((b >> 16) & 0xFF) as u8,
					((b >> 24) & 0xFF) as u8,
				])
				.unwrap(),
		};
		self
	}
}

impl fmt::Debug for Program {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut pc = 0;
		while pc < self.code.len() {
			let ins = Prefix::from(self.code[pc]);
			if let Some(i) = ins {
				let postfix = self.code[pc] & 0x0F;
				write!(f, "{:04}.\t{:02x}\t{}", pc, self.code[pc], i)?;
				match i {
					Prefix::PUSHI => {
						write!(
							f,
							"\t{:02x?}",
							&self.code[(pc + 1)..(pc + 1 + (postfix as usize) * 4)]
						)?;
						pc += (postfix as usize) * 4;
					}
					Prefix::PUSHB => {
						if postfix == 0 {
							write!(f, "\t0")?;
						} else {
							write!(
								f,
								"\t{:02x?}",
								&self.code[(pc + 1)..(pc + 1 + (postfix as usize))]
							)?;
							pc += postfix as usize;
						}
					}
					Prefix::JMP | Prefix::JZ | Prefix::JNZ => {
						let target =
							u32::from(self.code[pc + 1]) | u32::from(self.code[pc + 2]) << 8;
						write!(f, "\tto {}", target)?;
						pc += 2
					}
					Prefix::BINARY => {
						if let Some(op) = Binary::from(postfix) {
							write!(f, "\t{}", op)?;
						} else {
							write!(f, "\tunknown {}", postfix)?;
						}
					}
					Prefix::UNARY => {
						if let Some(op) = Unary::from(postfix) {
							write!(f, "\t{}", op)?;
						} else {
							write!(f, "\tunknown {}", postfix)?;
						}
					}
					Prefix::USER => {
						let name = match postfix {
							0 => "get_length",
							1 => "get_wall_time",
							2 => "get_precise_time",
							3 => "set_pixel",
							4 => "blit",
							5 => "random_int",
							_ => "(unknown user function)",
						};
						write!(f, "\t{}", name)?;
					}
					Prefix::SPECIAL => {
						let name = match postfix {
							12 => "swap",
							13 => "dump",
							14 => "yield",
							15 => "two-byte instruction",
							_ => "(unknown special function)",
						};
						write!(f, "\t{}", name)?;
					}
					_ => {
						write!(f, "\t{}", postfix)?;
					}
				}
				writeln!(f)?;
			} else {
				writeln!(f, "{:04}.\t{:02x}\tUnknown instruction", pc, self.code[pc])?;
				break;
			}

			pc += 1;
		}
		Ok(())
	}
}
