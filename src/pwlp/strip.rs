pub struct Color {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

pub trait Strip {
	fn length(&self) -> u8;
	fn blit(&mut self);
	fn set_pixel(&mut self, idx: u8, r: u8, g: u8, b: u8);
	fn get_pixel(&self, idx: u8) -> Color;
}

pub struct DummyStrip {
	trace: bool,
	length: u8,
	data: Vec<u8>,
}

impl DummyStrip {
	pub fn new(length: u8, trace: bool) -> DummyStrip {
		DummyStrip {
			trace,
			length,
			data: vec![0u8; (length as usize) * 3],
		}
	}
}

impl Strip for DummyStrip {
	fn length(&self) -> u8 {
		self.length
	}

	fn set_pixel(&mut self, idx: u8, r: u8, g: u8, b: u8) {
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

	fn get_pixel(&self, idx: u8) -> Color {
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
		length: u8,
	}

	impl SPIStrip {
		pub fn new(spi: Spi, length: u8) -> SPIStrip {
			SPIStrip {
				spi,
				length,
				data: vec![0u8; (length as usize) * 3],
			}
		}
	}

	impl super::Strip for SPIStrip {
		fn length(&self) -> u8 {
			self.length
		}

		fn get_pixel(&self, idx: u8) -> Color {
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

		fn set_pixel(&mut self, idx: u8, r: u8, g: u8, b: u8) {
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
