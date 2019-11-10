use super::instructions::{Binary, Prefix, Unary};
use super::program::Program;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

impl Program {
	/** Run a program. Note, this is not deterministic (e.g. contains calls to current time, random number generation)
	 * so not suitable to use in tests. */
	pub fn run(&self) {
		let mut rng = rand::thread_rng();
		let mut pc = 0;
		let mut stack: Vec<u32> = vec![];
		let start_time = SystemTime::now();

		let get_length_result = 123;
		let mut instruction_count = 0;

		while pc < self.code.len() {
			let ins = Prefix::from(self.code[pc]);
			if let Some(i) = ins {
				instruction_count += 1;
				let postfix = self.code[pc] & 0x0F;
				print!("{:04}.\t{:02x}\t{}", pc, self.code[pc], i);

				match i {
					Prefix::PUSHI => {
						for _ in 0..postfix {
							let value = u32::from(self.code[pc + 1])
								| u32::from(self.code[pc + 2]) << 8
								| u32::from(self.code[pc + 3]) << 16
								| u32::from(self.code[pc + 4]) << 24;
							stack.push(value);
							print!("\tv={}", value);
							pc += 4;
						}
					}
					Prefix::PUSHB => {
						if postfix == 0 {
							stack.push(0);
						} else {
							for _ in 0..postfix {
								pc += 1;
								print!("\tv={}", self.code[pc]);
								stack.push(u32::from(self.code[pc]));
							}
						}
					}
					Prefix::POP => {
						for _ in 0..postfix {
							let _ = stack.pop();
						}
					}
					Prefix::PEEK => {
						let val = stack[stack.len() - (postfix as usize) - 1];
						print!("\tindex={} v={}", postfix, val);
						stack.push(val);
					}
					Prefix::JMP | Prefix::JZ | Prefix::JNZ => {
						let target = (u32::from(self.code[pc + 1])
							| (u32::from(self.code[pc + 2]) << 8)) as usize;

						pc = match i {
							Prefix::JMP => target,
							Prefix::JZ => {
								let head = stack.last().unwrap();
								if *head == 0 {
									target
								} else {
									pc + 3
								}
							}
							Prefix::JNZ => {
								let head = stack.last().unwrap();
								if *head != 0 {
									target
								} else {
									pc + 3
								}
							}
							_ => unreachable!(),
						};
						println!();
						continue;
					}
					Prefix::BINARY => {
						if let Some(op) = Binary::from(postfix) {
							let rhs = stack.pop().unwrap();
							let lhs = stack.pop().unwrap();
							stack.push(match op {
								Binary::ADD => lhs + rhs,
								Binary::SUB => lhs - rhs,
								Binary::MUL => lhs * rhs,
								Binary::DIV => lhs / rhs,
								Binary::MOD => lhs % rhs,
								Binary::AND => lhs & rhs,
								Binary::OR => lhs | rhs,
								Binary::XOR => lhs ^ rhs,
								Binary::EQ => {
									if lhs == rhs {
										1
									} else {
										0
									}
								}
								Binary::NEQ => {
									if lhs != rhs {
										1
									} else {
										0
									}
								}
								Binary::GT => {
									if lhs > rhs {
										1
									} else {
										0
									}
								}
								Binary::GTE => {
									if lhs >= rhs {
										1
									} else {
										0
									}
								}
								Binary::LT => {
									if lhs < rhs {
										1
									} else {
										0
									}
								}
								Binary::LTE => {
									if lhs <= rhs {
										1
									} else {
										0
									}
								}
							})
						} else {
							println!("invalid binary postfix: {}", postfix);
							break;
						}
					}
					Prefix::UNARY => {
						if let Some(op) = Unary::from(postfix) {
							let lhs = stack.pop().unwrap();
							stack.push(match op {
								Unary::DEC => lhs - 1,
								Unary::INC => lhs + 1,
								Unary::NEG => unimplemented!(),
								Unary::NOT => !lhs,
								Unary::SHL8 => lhs << 8,
								Unary::SHR8 => lhs >> 8,
							});
						} else {
							println!("invalid binary postfix: {}", postfix);
							break;
						}
					}
					Prefix::USER => match postfix {
						0 => stack.push(get_length_result),
						1 => {
							// GET_WALL_TIME
							let time = SystemTime::now()
								.duration_since(UNIX_EPOCH)
								.unwrap()
								.as_secs();
							stack.push((time & std::u32::MAX as u64) as u32); // Wrap around when we exceed u32::MAX
						}
						2 => {
							// GET_PRECISE_TIME
							let time = SystemTime::now()
								.duration_since(start_time)
								.unwrap()
								.as_millis();
							stack.push((time & std::u32::MAX as u128) as u32); // Wrap around when we exceed u32::MAX
						}
						3 => {
							let v = stack.last().unwrap();
							let idx = v & 0xFF;
							let r = ((v >> 8) as u32) & 0xFF;
							let g = ((v >> 16) as u32) & 0xFF;
							let b = ((v >> 24) as u32) & 0xFF;
							print!("\tset_pixel {} idx={} r={} g={}, b={}", v, idx, r, g, b);
						}
						4 => print!("\tblit"),
						5 => {
							let v = stack.pop().unwrap();
							stack.push(rng.gen_range(0, v));
						}
						_ => {
							print!("\t(unknown user function)");
							break;
						}
					},
					Prefix::SPECIAL => {
						let name = match postfix {
							12 => "swap",
							13 => "dump",
							14 => "yield",
							15 => "twobyte",
							_ => unimplemented!(),
						};
						print!("\t{}", name);
					}
				}
			} else {
				println!("{:04}.\t{:02x}\tUnknown instruction\n", pc, self.code[pc]);
				break;
			}

			println!("\tstack: {:?}", stack);
			pc += 1;
		}
		println!("Ended; {} instructions executed", instruction_count);
	}
}
