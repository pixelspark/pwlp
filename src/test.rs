#[cfg(test)]
use std::fs;

#[cfg(test)]
use std::fs::File;

#[cfg(test)]
use super::pwlp::parse;

#[cfg(test)]
use std::io::Read;

#[test]
fn compare_output_of_compiler_to_stored_binaries() {
	// Read txt files in the 'tests' folder, compile them, then compare to the stored 'bin' file
	let paths = fs::read_dir("./test").unwrap();
	for path in paths {
		let name = path.unwrap();
		if let Some(os_ext) = name.path().extension() {
			if os_ext.to_str() == Some("txt") {
				let mut source = String::new();
				File::open(name.path())
					.unwrap()
					.read_to_string(&mut source)
					.unwrap();

				match parse(&source) {
					Ok(prg) => {
						// Compare with stored binary
						let bin_path = name.path().with_extension("bin");
						let mut stored_bin = Vec::<u8>::new();
						File::open(bin_path)
							.unwrap()
							.read_to_end(&mut stored_bin)
							.unwrap();

						if stored_bin.len() != prg.code.len() {
							panic!("Binary size is different for {}: {} compiled, {} stored\nCompiled: {:?}\nStored: {:?}", 
								name.path().display(),
								prg.code.len(),
								stored_bin.len(),
								prg.code,
								stored_bin)
						}

						for idx in 0..stored_bin.len() {
							if stored_bin[idx] != prg.code[idx] {
								panic!("Binary is different at index {}:\nCompiled: {:?}\nStored: {:?}", 
								idx,
								prg.code,
								stored_bin)
							}
						}

						// Verify disassembly is equal
						let dis_path = name.path().with_extension("dis");
						let mut stored_dis = String::new();
						File::open(dis_path)
							.unwrap()
							.read_to_string(&mut stored_dis)
							.unwrap();
						let my_dis = format!("{:?}\n", prg);
						assert_eq!(my_dis, stored_dis);
					}
					Err(s) => panic!("Parse error in {}: {}", name.path().display(), s),
				};
			}
		} else {
			println!("Not reading: {}", name.path().display())
		}
	}
}
