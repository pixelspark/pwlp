use std::fmt::Display;

pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

pub trait Strip {
	fn length(&self) -> u32;
	fn blit(&mut self);
	fn set_pixel(&mut self, idx: u32, r: u8, g: u8, b: u8);
	fn get_pixel(&self, idx: u32) -> Color;
}

impl Display for dyn Strip {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for idx in 0..self.length() {
			let color = self.get_pixel(idx);
			write!(f, "{:02x}{:02x}{:02x} ", color.r, color.g, color.b)?;
		}

		Ok(())
	}
}

pub struct DummyStrip {
	trace: bool,
	length: u32,
	data: Vec<u8>,
}

impl DummyStrip {
	pub fn new(length: u32, trace: bool) -> DummyStrip {
		DummyStrip {
			trace,
			length,
			data: vec![0u8; (length as usize) * 3],
		}
	}
}

impl Strip for DummyStrip {
	fn length(&self) -> u32 {
		self.length
	}

	fn set_pixel(&mut self, idx: u32, r: u8, g: u8, b: u8) {
		assert!(
			idx < self.length,
			"set_pixel: index {} exceeds strip length {}",
			idx,
			self.length
		);
		self.data[(idx as usize) * 3] = r;
		self.data[(idx as usize) * 3 + 1] = g;
		self.data[(idx as usize) * 3 + 2] = b;
	}

	fn get_pixel(&self, idx: u32) -> Color {
		assert!(
			idx < self.length,
			"get_pixel: index {} exceeds strip length {}",
			idx,
			self.length
		);
		Color {
			r: self.data[(idx as usize) * 3],
			g: self.data[(idx as usize) * 3 + 1],
			b: self.data[(idx as usize) * 3 + 2],
		}
	}

	fn blit(&mut self) {
		if self.trace {
			for idx in 0..self.length {
				print!(
					"{:02x}{:02x}{:02x} ",
					self.data[(idx as usize) * 3],
					self.data[(idx as usize) * 3 + 1],
					self.data[(idx as usize) * 3 + 2]
				);
			}
			println!();
		}
	}
}

#[cfg(feature = "raspberrypi")]
pub mod spi_strip {
	use super::Color;
	use rppal::spi::Spi;
	pub struct SPIStrip {
		spi: Spi,
		data: Vec<u8>,
		length: u32,
	}

	impl SPIStrip {
		pub fn new(spi: Spi, length: u32) -> SPIStrip {
			SPIStrip {
				spi,
				length,
				data: vec![0u8; (length as usize) * 3],
			}
		}
	}

	impl super::Strip for SPIStrip {
		fn length(&self) -> u32 {
			self.length
		}

		fn get_pixel(&self, idx: u32) -> Color {
			assert!(
				idx < self.length,
				"get_pixel: index {} exceeds strip length {}",
				idx,
				self.length
			);
			Color {
				r: self.data[(idx as usize) * 3],
				g: self.data[(idx as usize) * 3 + 1],
				b: self.data[(idx as usize) * 3 + 2],
			}
		}

		fn set_pixel(&mut self, idx: u32, r: u8, g: u8, b: u8) {
			assert!(
				idx < self.length,
				"set_pixel: index {} exceeds strip length {}",
				idx,
				self.length
			);
			self.data[(idx as usize) * 3] = r;
			self.data[(idx as usize) * 3 + 1] = g;
			self.data[(idx as usize) * 3 + 2] = b;
		}

		fn blit(&mut self) {
			self.spi.write(&self.data).unwrap();
		}
	}
}
