use chip8::emulator;
use ratatui::{
    prelude::{Buffer, Rect, Style},
    widgets::{Widget, WidgetRef},
};

pub struct Display<'a> {
    pixel_filled: String,
    pixel_empty: String,
    buffer: &'a [u8; emulator::GRAPHICS_BUFFER_SIZE],
    style: Style,
}

impl<'a> Widget for Display<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
    }
}

impl<'a> WidgetRef for Display<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.render_pixels(area, buf);
    }
}

impl<'a> Display<'a> {
    pub fn new(
        buffer: &'a [u8; emulator::GRAPHICS_BUFFER_SIZE],
        pixel_filled: String,
        pixel_empty: String,
        style: Style,
    ) -> Self {
        Self {
            pixel_filled,
            pixel_empty,
            buffer,
            style,
        }
    }

    ///
    /// bit-index: 0 1 2 3 4 5 6 7
    /// (0, 0) => upper bit first byte
    /// (24, 0) => upper bit, fourth byte
    /// (7, 0) => lowest bit first byte
    ///
    ///
    pub fn render_pixels(&self, area: Rect, buf: &mut Buffer) {
        let cell_width: u16 = 2;
        for y in 0..(emulator::DISPLAY_HEIGHT as u8) {
            let mut x_byte = 0;
            for x in 0..(emulator::DISPLAY_WIDTH as u8) {
                let bit_index = x % 8;
                if bit_index == 0 {
                    x_byte += 1;
                }
                let byte = (x_byte - 1) + (8 * y);
                let mask = (0x1_u32 << (7 - bit_index)) as u8;
                let bit = self.buffer[byte as usize] & mask;
                let pixel = if bit > 0 {
                    &self.pixel_filled
                } else {
                    &self.pixel_empty
                };

                for w in 0..cell_width {
                    let x = area.left() + (x as u16) * cell_width + w;
                    let y = area.top() + (y as u16);
                    if x >= area.width {
                        break;
                    }
                    if y >= area.height {
                        break;
                    }
                    buf.get_mut(x, y).set_symbol(pixel).set_style(self.style);
                }
            }
        }
    }
}
