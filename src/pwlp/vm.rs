use super::instructions::{Binary, Prefix, Unary};
use super::program::Program;
use super::strip::Strip;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct VM {
	trace: bool,
	strip: Box<dyn Strip>,
	instruction_limit: Option<u64>,
	deterministic: bool,
}

impl VM {
	pub fn new(strip: Box<dyn Strip>) -> VM {
		VM {
			trace: false,
			strip,
			instruction_limit: None,
			deterministic: false,
		}
	}

	pub fn set_trace(&mut self, trace: bool) {
		self.trace = trace
	}

	pub fn set_deterministic(&mut self, d: bool) {
		self.deterministic = d
	}

	pub fn set_instruction_limit(&mut self, limit: Option<u64>) {
		self.instruction_limit = limit
	}

	/** Run a program. Note, this is not deterministic (e.g. contains calls to current time, random number generation)
	 * so not suitable to use in tests. */
	pub fn run(&mut self, program: &Program) {
		let mut rng = rand::thread_rng();
		let mut deterministic_rng = ChaCha20Rng::from_seed([0u8; 32]);
		let mut pc = 0;
		let mut stack: Vec<u32> = vec![];
		let start_time = SystemTime::now();
		let fps = 60;
		let frame_time = Duration::from_millis(1000 / fps);
		let mut last_yield_time = SystemTime::now();

		let mut instruction_count = 0u64;

		while pc < program.code.len() {
			// Enforce instruction count limit
			if let Some(limit) = self.instruction_limit {
				if instruction_count >= limit {
					break;
				}
			}

			let ins = Prefix::from(program.code[pc]);
			if let Some(i) = ins {
				instruction_count += 1;
				let postfix = program.code[pc] & 0x0F;

				if self.trace {
					print!("{:04}.\t{:02x}\t{}", pc, program.code[pc], i);
				}

				match i {
					Prefix::PUSHI => {
						for _ in 0..postfix {
							let value = u32::from(program.code[pc + 1])
								| u32::from(program.code[pc + 2]) << 8
								| u32::from(program.code[pc + 3]) << 16
								| u32::from(program.code[pc + 4]) << 24;
							stack.push(value);

							if self.trace {
								print!("\tv={}", value);
							}
							pc += 4;
						}
					}
					Prefix::PUSHB => {
						if postfix == 0 {
							stack.push(0);
						} else {
							for _ in 0..postfix {
								pc += 1;
								if self.trace {
									print!("\tv={}", program.code[pc]);
								}
								stack.push(u32::from(program.code[pc]));
							}
						}
					}
					Prefix::POP => {
						for _ in 0..postfix {
							let _ = stack.pop();
						}
					}
					Prefix::PEEK => {
						assert!(
							(postfix as usize) < stack.len(),
							"cannot peek beyond stack (index {} > stack size {})!",
							postfix,
							stack.len()
						);
						let val = stack[stack.len() - (postfix as usize) - 1];
						if self.trace {
							print!("\tindex={} v={}", postfix, val);
						}
						stack.push(val);
					}
					Prefix::JMP | Prefix::JZ | Prefix::JNZ => {
						let target = (u32::from(program.code[pc + 1])
							| (u32::from(program.code[pc + 2]) << 8)) as usize;

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

						if self.trace {
							println!();
						}
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
								Binary::SHL => lhs << rhs,
								Binary::SHR => lhs >> rhs,
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
							if self.trace {
								println!("invalid binary postfix: {}", postfix);
							}
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
							if self.trace {
								println!("invalid binary postfix: {}", postfix);
							}
							break;
						}
					}
					Prefix::USER => match postfix {
						0 => stack.push(self.strip.length() as u32),
						1 => {
							// GET_WALL_TIME
							if self.deterministic {
								stack.push((instruction_count / 10) as u32);
							} else {
								let time = SystemTime::now()
									.duration_since(UNIX_EPOCH)
									.unwrap()
									.as_secs();
								stack.push((time & std::u32::MAX as u64) as u32); // Wrap around when we exceed u32::MAX
							}
						}
						2 => {
							// GET_PRECISE_TIME
							if self.deterministic {
								stack.push(instruction_count as u32);
							} else {
								let time = SystemTime::now()
									.duration_since(start_time)
									.unwrap()
									.as_millis();
								stack.push((time & std::u32::MAX as u128) as u32); // Wrap around when we exceed u32::MAX
							}
						}
						3 => {
							let v = stack.last().unwrap();
							let idx = (v & 0xFF) as u8;
							let r = (((v >> 8) as u32) & 0xFF) as u8;
							let g = (((v >> 16) as u32) & 0xFF) as u8;
							let b = (((v >> 24) as u32) & 0xFF) as u8;
							if self.trace {
								print!("\tset_pixel {} idx={} r={} g={}, b={}", v, idx, r, g, b);
							}
							self.strip.set_pixel(idx, r, g, b);
						}
						4 => {
							if self.trace {
								print!("\tblit");
							}
							self.strip.blit();
						}
						5 => {
							// RANDOM_INT
							let v = stack.pop().unwrap();
							if self.deterministic {
								stack.push(deterministic_rng.gen_range(0, v));
							} else {
								stack.push(rng.gen_range(0, v));
							}
						}
						6 => {
							// GET_PIXEL
							let v = stack.pop().unwrap();
							let color = self.strip.get_pixel((v & 0xFF) as u8);
							let color_value = (v & 0xFF)
								| (color.r as u32) << 8 | (color.g as u32) << 16
								| (color.b as u32) << 24;
							stack.push(color_value);
						}
						_ => {
							if self.trace {
								print!("\t(unknown user function)");
							}
							break;
						}
					},
					Prefix::SPECIAL => {
						match postfix {
							12 => {
								// SWAP
								let lhs = stack.pop().unwrap();
								let rhs = stack.pop().unwrap();
								stack.push(lhs);
								stack.push(rhs);
							}
							13 => {
								// DUMP
								println!("DUMP: {:?}", stack);
							}
							14 => {
								// YIELD
								let now = SystemTime::now();
								let passed = now.duration_since(last_yield_time).unwrap();
								if passed < frame_time {
									if self.trace {
										print!(
											"\t{}ms passed, {}ms frame time, {}ms left to wait",
											passed.as_millis(),
											frame_time.as_millis(),
											(frame_time - passed).as_millis()
										);
									}
									// We have some time left
									std::thread::sleep(frame_time - passed);
								} else if self.trace {
									print!(
										"{}ms passed, {}ms frame time, no time left to wait",
										passed.as_millis(),
										frame_time.as_millis()
									);
								}

								last_yield_time = now;
							}
							15 => {
								// TWOBYTE
								panic!("Two-byte instructions not implemented nor valid");
							}
							_ => unimplemented!(),
						}

						if self.trace {
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
				}
			} else {
				if self.trace {
					println!(
						"{:04}.\t{:02x}\tUnknown instruction\n",
						pc, program.code[pc]
					);
				}
				break;
			}

			if self.trace {
				println!("\tstack: {:?}", stack);
			}
			pc += 1;
		}

		if self.trace {
			println!("Ended; {} instructions executed", instruction_count);
		}
	}
}
