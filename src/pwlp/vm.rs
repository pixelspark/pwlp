use super::program::{Program};
use super::instructions::{Prefix, Unary, Binary};

impl Program {
	pub fn run(&self) {
		let mut pc = 0;
		let mut stack: Vec<u32> = vec![];

		let get_length_result = 123;
		let get_precise_time_result = 1337;
		let get_wall_time_result = 1338;

		while pc < self.code.len() {
			let ins = Prefix::from(self.code[pc]);
			if let Some(i) = ins {
				let postfix = self.code[pc] & 0x0F;
				println!("{:04}.\t{:02x}\t{}", pc, self.code[pc], i);

				match i {
					Prefix::PUSHI => {
						for _ in 0..postfix {
							let value = self.code[pc + 1] as u32 |
								(self.code[pc + 2] as u32) << 8 |
								(self.code[pc + 3] as u32) << 16 |
								(self.code[pc + 4] as u32) << 24;
							stack.push(value);
							println!("\tPUSH {}", value);
							pc += 4;
						}
					},
					Prefix::PUSHB => {
						if postfix == 0 {
							stack.push(0);
						}
						else {
							for _ in 0..postfix {
								pc += 1;
								println!("\tPUSH {}", self.code[pc]);
								stack.push(self.code[pc] as u32);
							}
						}
					},
					Prefix::POP => {
						for _ in 0..postfix {
							let _ = stack.pop();
						}
					},
					Prefix::JMP | Prefix::JZ | Prefix::JNZ => {
						let target = ((self.code[pc + 1] as u32) | (self.code[pc + 2] as u32) << 8) as usize;

						pc = match i {
							Prefix::JMP => target,
							Prefix::JZ => {
								let head = stack.pop().unwrap();
								if head == 0 {
									target
								}
								else {
									pc + 3
								}
							},
							Prefix::JNZ => {
								let head = stack.pop().unwrap();
								if head != 0 {
									target
								}
								else {
									pc
								}
							},
							_ => unreachable!()
						};
						continue;
					},
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
								Binary::GT => if lhs > rhs { 1 } else { 0 },
								Binary::GTE => if lhs >= rhs { 1 } else { 0 },
								Binary::LT => if lhs < rhs { 1 } else { 0 },
								Binary::LTE => if lhs <= rhs { 1 } else { 0 }
							})
						}
						else {
							println!("invalid binary postfix: {}", postfix);
							break;
						}
					},
					Prefix::UNARY => {
						if let Some(op) = Unary::from(postfix) {
							let lhs = stack.pop().unwrap();
							stack.push(match op {
								Unary::DEC => lhs - 1,
								Unary::INC => lhs + 1,
								Unary::NEG => unimplemented!(),
								Unary::NOT => !lhs,
								Unary::SHL8 => lhs << 8,
								Unary::SHR8 => lhs >> 8
							});
						}
						else {
							println!("invalid binary postfix: {}", postfix);
							break;
						}
					},
					Prefix::USER => {
						match postfix {
							0 => stack.push(get_length_result),
							1 => stack.push(get_wall_time_result),
							2=>  stack.push(get_precise_time_result),
							3 => {
								println!("set_pixel {}", stack.last().unwrap());
							},
							4 => {
								println!("blit")
							},
							_ => {
								println!("(unknown user function)");
								break;
							}
						}
					},
					Prefix::SPECIAL => {
						let name = match postfix {
							12 => "swap",
							13 => "dump",
							14 => "yield",
							15 => "twobyte",
							_ => unimplemented!()
						};
						println!("\t{}", name);
					},
					_ => {
						println!("{:04}.\t{:02x}\tUnknown prefix\n", pc, self.code[pc]);
					}
				}
			}
			else {
				println!("{:04}.\t{:02x}\tUnknown instruction\n", pc, self.code[pc]);
				break;
			}

			println!("\t\tstack: {:?}", stack);
			pc += 1;
		}
	}
}