mod pwlp;

#[cfg(feature = "wasm")]
mod lib {
	use super::pwlp::program::Program;
	use super::pwlp::strip::DummyStrip;
	use super::pwlp::vm::{Outcome, VM};
	use wasm_bindgen::prelude::*;

	#[wasm_bindgen]
	pub fn compile(source: &str) -> Result<Vec<u8>, JsValue> {
		match Program::from_source(&source) {
			Ok(prg) => Ok(prg.code.to_vec()),
			Err(s) => Err(JsValue::from(s)),
		}
	}

	#[wasm_bindgen]
	pub fn assemble(source: &str) -> Result<String, JsValue> {
		match Program::from_source(&source) {
			Ok(prg) => Ok(format!("{:?}", prg)),
			Err(s) => Err(JsValue::from(s)),
		}
	}

	#[wasm_bindgen]
	pub fn run(
		binary: &[u8],
		length: u32,
		instruction_limit: Option<usize>,
	) -> Result<String, JsValue> {
		let program = Program::from_binary(binary.to_vec());
		// Run program
		let strip = DummyStrip::new(length, true);
		let mut vm = VM::new(Box::new(strip));
		vm.set_deterministic(true);
		vm.set_trace(false);

		let mut state = vm.start(program, instruction_limit);
		let mut running = true;
		let mut output = String::new();

		while running {
			match state.run(None) {
				Outcome::Yielded => {}
				Outcome::GlobalInstructionLimitReached
				| Outcome::LocalInstructionLimitReached
				| Outcome::Ended => running = false,
				Outcome::Error(e) => {
					return Err(JsValue::from(format!(
						"Error in VM at pc={}: {:?}",
						state.pc(),
						e
					)));
				}
			}
			output += &state.vm.strip().to_string();
			output += "\n";
		}

		Ok(output)
	}
}

#[cfg(feature = "wasm")]
pub use lib::*;
