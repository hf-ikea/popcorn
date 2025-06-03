use core::{fmt::{self, Write}, slice};

use limine::framebuffer::MemoryModel;
use noto_sans_mono_bitmap::{
    get_raster, get_raster_width, FontWeight, RasterHeight, RasterizedChar,
};
use spin::Mutex;

use crate::framebuffer::font_constants::BACKUP_CHAR;

const LINE_SPACING: usize = 2;
const LETTER_SPACING: usize = 0;
const BORDER_PADDING: usize = 1;

mod font_constants {
    use super::*;
    pub const FONT_WEIGHT: FontWeight = FontWeight::Regular;
    pub const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;
    pub const CHAR_RASTER_WIDTH: usize = get_raster_width(FONT_WEIGHT, CHAR_RASTER_HEIGHT);
    pub const BACKUP_CHAR: char = '?';
}

fn get_char_raster(c: char) -> RasterizedChar {
    fn get(c: char) -> Option<RasterizedChar> {
        get_raster(c, font_constants::FONT_WEIGHT, font_constants::CHAR_RASTER_HEIGHT)
    }
    get(c).unwrap_or_else(|| get(BACKUP_CHAR).expect("couldnt get raster of backup char"))
}

struct FramebufferInfo {
    height: usize,
    width: usize,
    pitch: usize,
    _buffer_length: usize,
    bytes_per_pixel: usize,
    _memory_model: MemoryModel,
}

pub struct FramebufferWriter {
    buffer: &'static mut [u8],
    info: FramebufferInfo,
    x_pos: usize,
    y_pos: usize,
}

impl FramebufferWriter {
    pub unsafe fn new(framebuffer: limine::framebuffer::Framebuffer) -> Self {
        let bytes_per_pixel = framebuffer.bpp() as usize / 8;
        let buffer_length = bytes_per_pixel * framebuffer.pitch() as usize * framebuffer.height() as usize; // in bytes 
        let mut ret = FramebufferWriter {
            buffer: unsafe { slice::from_raw_parts_mut::<'static, u8>(framebuffer.addr(), buffer_length) },
            info: FramebufferInfo {
                height: framebuffer.height() as usize,
                width: framebuffer.width() as usize,
                pitch: framebuffer.pitch() as usize / bytes_per_pixel,
                _buffer_length: buffer_length,
                bytes_per_pixel: bytes_per_pixel,
                _memory_model: framebuffer.memory_model(),
            },
            x_pos: 0,
            y_pos: 0,
        };
        ret.clear();
        ret
    }

    fn new_line(&mut self) {
        self.y_pos += font_constants::CHAR_RASTER_HEIGHT.val() + LINE_SPACING;
        self.carriage_return()
    }

    fn carriage_return(&mut self) {
        self.x_pos = BORDER_PADDING;
    }

    pub fn clear(&mut self) {
        self.x_pos = BORDER_PADDING;
        self.y_pos = BORDER_PADDING;
        //self.buffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.width
    }

    fn height(&self) -> usize {
        self.info.height
    }

    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.new_line(),
            '\r' => self.carriage_return(),
            c => {
                let x = self.x_pos + font_constants::CHAR_RASTER_WIDTH;
                if x >= self.width() {
                    self.new_line();
                }
                let y = self.y_pos + font_constants::CHAR_RASTER_HEIGHT.val() + BORDER_PADDING;
                if y >= self.height() {
                    self.clear();
                }
                self.write_rendered_char(get_char_raster(c));
            }
        }
    }

    fn write_rendered_char(&mut self, raster_char: RasterizedChar) {
        for (y, row) in raster_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.x_pos += raster_char.width() + LETTER_SPACING;
    }

    /// Writes a pixel in accordance to the PixelBlueGreenRedReserved8BitPerColor format defined by
    /// the UEFI specifications.
    /// 
    /// https://uefi.org/specs/UEFI/2.11/12_Protocols_Console_Support.html#graphics-output-protocol
    pub fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.info.pitch + x;
        let byte_offset = pixel_offset * self.info.bytes_per_pixel;
        self.buffer[byte_offset] = intensity; // blue
        self.buffer[byte_offset + 1] = intensity; //green
        self.buffer[byte_offset + 2] = intensity; //red
        self.buffer[byte_offset + 3] = intensity; // ??
        self.buffer[byte_offset..byte_offset + self.info.bytes_per_pixel].copy_from_slice(&[intensity, intensity, intensity, intensity]);
        //let _ = unsafe { ptr::read_volatile(&self.buffer[byte_offset]) };
    }
}

impl fmt::Write for FramebufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        Ok(())
    }
}

pub struct LockedLogger {
    framebuffer: Mutex<FramebufferWriter>,
}

impl LockedLogger {
    pub fn new(framebuffer: limine::framebuffer::Framebuffer) -> Self {
        LockedLogger { framebuffer: Mutex::new(unsafe { FramebufferWriter::new(framebuffer) }) }
    }
}

impl log::Log for LockedLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut framebuffer = self.framebuffer.lock();
        writeln!(framebuffer, "{:5}: {}", record.level(), record.args()).unwrap();
    }
    
    fn flush(&self) {
        todo!()
    }
}