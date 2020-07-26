use super::instructions::{Binary, Prefix, Special, Unary, UserCommand};
use super::program::Program;
use super::strip::Strip;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct State<'a> {
	vm: &'a mut VM,
	program: Program,
	pc: usize,
	stack: Vec<u32>,
	start_time: SystemTime,
	instruction_count: usize,
	instruction_limit: Option<usize>,
	deterministic_rng: ChaCha20Rng,
}

pub struct VM {
	trace: bool,
	strip: Box<dyn Strip>,
	deterministic: bool,
}

#[derive(Debug)]
pub enum VMError {
	UnknownInstruction,
	StackUnderflow,
}

pub enum Outcome {
	Ended,
	GlobalInstructionLimitReached,
	LocalInstructionLimitReached,
	Yielded,
	Error(VMError),
}

impl<'a> State<'a> {
	fn new(vm: &'a mut VM, program: Program, instruction_limit: Option<usize>) -> State<'a> {
		State {
			vm,
			program,
			pc: 0,
			stack: vec![],
			start_time: SystemTime::now(),
			instruction_limit,
			instruction_count: 0,
			deterministic_rng: ChaCha20Rng::from_seed([0u8; 32]),
		}
	}
	pub fn pc(&self) -> usize {
		self.pc
	}

	fn pushi(&mut self, postfix: u8) {
		for _ in 0..postfix {
			let value = u32::from(self.program.code[self.pc + 1])
				| u32::from(self.program.code[self.pc + 2]) << 8
				| u32::from(self.program.code[self.pc + 3]) << 16
				| u32::from(self.program.code[self.pc + 4]) << 24;
			self.stack.push(value);

			if self.vm.trace {
				print!("\tv={}", value);
			}
			self.pc += 4;
		}
	}

	fn pushb(&mut self, postfix: u8) {
		if postfix == 0 {
			self.stack.push(0);
		} else {
			for _ in 0..postfix {
				self.pc += 1;
				if self.vm.trace {
					print!("\tv={}", self.program.code[self.pc]);
				}
				self.stack.push(u32::from(self.program.code[self.pc]));
			}
		}
	}

	fn user(&mut self, postfix: u8) -> Option<Outcome> {
		let user = UserCommand::from(postfix);

		match user {
			None => Some(Outcome::Error(VMError::UnknownInstruction)),
			Some(UserCommand::GET_LENGTH) => {
				self.stack.push(self.vm.strip.length() as u32);
				None
			}
			Some(UserCommand::GET_WALL_TIME) => {
				if self.vm.deterministic {
					self.stack.push((self.instruction_count / 10) as u32);
				} else {
					let time = SystemTime::now()
						.duration_since(UNIX_EPOCH)
						.unwrap()
						.as_secs();
					self.stack.push((time & std::u32::MAX as u64) as u32); // Wrap around when we exceed u32::MAX
				}
				None
			}
			Some(UserCommand::GET_PRECISE_TIME) => {
				if self.vm.deterministic {
					self.stack.push(self.instruction_count as u32);
				} else {
					let time = SystemTime::now()
						.duration_since(self.start_time)
						.unwrap()
						.as_millis();
					self.stack.push((time & std::u32::MAX as u128) as u32); // Wrap around when we exceed u32::MAX
				}
				None
			}
			Some(UserCommand::SET_PIXEL) => {
				if self.stack.is_empty() {
					return Some(Outcome::Error(VMError::StackUnderflow));
				}
				let v = self.stack.pop().unwrap();
				let r = (((v >> 0) as u32) & 0xFF) as u8;
				let g = (((v >> 8) as u32) & 0xFF) as u8;
				let b = (((v >> 16) as u32) & 0xFF) as u8;
				let idx = self.stack.last().unwrap();

				if self.vm.trace {
					print!("\tset_pixel {} idx={} r={} g={}, b={}", v, idx, r, g, b);
				}
				self.vm.strip.set_pixel(*idx, r, g, b);
				None
			}
			Some(UserCommand::BLIT) => {
				if self.vm.trace {
					print!("\tblit");
				}
				self.vm.strip.blit();
				None
			}
			Some(UserCommand::RANDOM_INT) => {
				if self.stack.is_empty() {
					return Some(Outcome::Error(VMError::StackUnderflow));
				}
				let v = self.stack.pop().unwrap();
				self.stack.push(self.deterministic_rng.gen_range(0, v));
				None
			}
			Some(UserCommand::GET_PIXEL) => {
				if self.stack.is_empty() {
					return Some(Outcome::Error(VMError::StackUnderflow));
				}
				let v = self.stack.pop().unwrap();
				let color = self.vm.strip.get_pixel(v);
				let color_value = (v & 0xFF)
					| (color.r as u32) << 8
					| (color.g as u32) << 16
					| (color.b as u32) << 24;
				self.stack.push(color_value);
				None
			}
		}
	}

	fn special(&mut self, postfix: u8) -> Option<Outcome> {
		let special = Special::from(postfix);

		match special {
			None => Some(Outcome::Error(VMError::UnknownInstruction)),
			Some(Special::SWAP) => {
				if self.stack.len() < 2 {
					return Some(Outcome::Error(VMError::StackUnderflow));
				}
				let lhs = self.stack.pop().unwrap();
				let rhs = self.stack.pop().unwrap();
				self.stack.push(lhs);
				self.stack.push(rhs);
				None
			}
			Some(Special::DUMP) => {
				// DUMP
				println!("DUMP: {:?}", self.stack);
				None
			}
			Some(Special::YIELD) => {
				self.pc += 1;
				Some(Outcome::Yielded)
			}
			Some(Special::TWOBYTE) => Some(Outcome::Error(VMError::UnknownInstruction)),
		}
	}

	pub fn run(&mut self, local_instruction_limit: Option<usize>) -> Outcome {
		let mut local_instruction_count = 0;
		while self.pc < self.program.code.len() {
			// Enforce global instruction count limit
			if let Some(limit) = self.instruction_limit {
				if self.instruction_count >= limit {
					return Outcome::GlobalInstructionLimitReached;
				}
			}

			// Enforce local instruction count limit
			if let Some(limit) = local_instruction_limit {
				if local_instruction_count >= limit {
					return Outcome::LocalInstructionLimitReached;
				}
			}

			let ins = Prefix::from(self.program.code[self.pc]);
			if let Some(i) = ins {
				self.instruction_count += 1;
				local_instruction_count += 1;
				let postfix = self.program.code[self.pc] & 0x0F;

				if self.vm.trace {
					print!("{:04}.\t{:02x}\t{}", self.pc, self.program.code[self.pc], i);
				}

				match i {
					Prefix::PUSHI => {
						self.pushi(postfix);
					}
					Prefix::PUSHB => {
						self.pushb(postfix);
					}
					Prefix::POP => {
						assert!(
							(postfix as usize) <= self.stack.len(),
							"cannot pop beyond stack (pop {} elements > stack size {})!",
							postfix,
							self.stack.len()
						);

						for _ in 0..postfix {
							let _ = self.stack.pop();
						}
					}
					Prefix::PEEK => {
						assert!(
							(postfix as usize) < self.stack.len(),
							"cannot peek beyond stack (index {} > stack size {})!",
							postfix,
							self.stack.len()
						);
						let val = self.stack[self.stack.len() - (postfix as usize) - 1];
						if self.vm.trace {
							print!("\tindex={} v={}", postfix, val);
						}
						self.stack.push(val);
					}
					Prefix::JMP | Prefix::JZ | Prefix::JNZ => {
						let target = (u32::from(self.program.code[self.pc + 1])
							| (u32::from(self.program.code[self.pc + 2]) << 8)) as usize;

						self.pc = match i {
							Prefix::JMP => target,
							Prefix::JZ => {
								if self.stack.is_empty() {
									return Outcome::Error(VMError::StackUnderflow);
								}
								let head = self.stack.last().unwrap();
								if *head == 0 {
									target
								} else {
									self.pc + 3
								}
							}
							Prefix::JNZ => {
								if self.stack.is_empty() {
									return Outcome::Error(VMError::StackUnderflow);
								}
								let head = self.stack.last().unwrap();
								if *head != 0 {
									target
								} else {
									self.pc + 3
								}
							}
							_ => return Outcome::Error(VMError::UnknownInstruction),
						};

						if self.vm.trace {
							println!();
						}
						continue;
					}
					Prefix::BINARY => {
						if let Some(op) = Binary::from(postfix) {
							if self.stack.len() < 2 {
								return Outcome::Error(VMError::StackUnderflow);
							}
							let rhs = self.stack.pop().unwrap();
							let lhs = self.stack.pop().unwrap();
							self.stack.push(op.apply(lhs, rhs))
						} else {
							if self.vm.trace {
								println!("invalid binary postfix: {}", postfix);
							}
							return Outcome::Error(VMError::UnknownInstruction);
						}
					}
					Prefix::UNARY => {
						if let Some(op) = Unary::from(postfix) {
							if self.stack.is_empty() {
								return Outcome::Error(VMError::StackUnderflow);
							}
							let lhs = self.stack.pop().unwrap();
							self.stack.push(op.apply(lhs));
						} else {
							if self.vm.trace {
								println!("invalid binary postfix: {}", postfix);
							}
							return Outcome::Error(VMError::UnknownInstruction);
						}
					}
					Prefix::USER => {
						if let Some(outcome) = self.user(postfix) {
							return outcome;
						}
					}
					Prefix::SPECIAL => {
						if let Some(outcome) = self.special(postfix) {
							return outcome;
						}
					}
				}
			} else {
				if self.vm.trace {
					println!(
						"{:04}.\t{:02x}\tUnknown instruction\n",
						self.pc, self.program.code[self.pc]
					);
				}
				break;
			}

			if self.vm.trace {
				println!("\tstack: {:?}", self.stack);
			}
			self.pc += 1;
		}

		if self.vm.trace {
			println!("Ended; {} instructions executed", self.instruction_count);
		}

		Outcome::Ended
	}
}

impl<'a> VM {
	pub fn new(strip: Box<dyn Strip>) -> VM {
		VM {
			trace: false,
			strip,
			deterministic: false,
		}
	}

	#[allow(clippy::borrowed_box)]
	pub fn strip(&'a mut self) -> &'a mut Box<dyn Strip> {
		&mut self.strip
	}

	pub fn set_trace(&mut self, trace: bool) {
		self.trace = trace
	}

	pub fn set_deterministic(&mut self, d: bool) {
		self.deterministic = d
	}

	pub fn start(&mut self, program: Program, instruction_limit: Option<usize>) -> State {
		State::new(self, program, instruction_limit)
	}
}
