use bootloader_api::info::{FrameBuffer, PixelFormat};
use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{Rgb888, RgbColor},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colour {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Colour {
    pub fn new(colour: Rgb888) -> Self {
        Colour {
            red: colour.r(),
            green: colour.g(),
            blue: colour.b(),
        }
    }
}

pub fn set_pixel_in(framebuffer: &mut FrameBuffer, position: Position, colour: Colour) {
    let info = framebuffer.info();

    let byte_offset = {
        let line_offset = position.y * info.stride;
        let pixel_offset = line_offset + position.x;
        pixel_offset * info.bytes_per_pixel
    };

    let pixel_buffer = &mut framebuffer.buffer_mut()[byte_offset..];
    match info.pixel_format {
        PixelFormat::Rgb => {
            pixel_buffer[0] = colour.red;
            pixel_buffer[1] = colour.green;
            pixel_buffer[2] = colour.blue;
        }
        PixelFormat::Bgr => {
            pixel_buffer[0] = colour.blue;
            pixel_buffer[1] = colour.green;
            pixel_buffer[2] = colour.red;
        }
        PixelFormat::U8 => {
            let grey = colour.red / 3 + colour.green / 3 + colour.blue / 3;
            pixel_buffer[0] = grey;
        }
        other => panic!("unknown pixel format {other:?}"),
    }
}

pub struct Display<'f> {
    framebuffer: &'f mut FrameBuffer,
}

impl<'f> Display<'f> {
    pub fn new(framebuffer: &'f mut FrameBuffer) -> Self {
        Display { framebuffer }
    }

    fn draw_pixel(&mut self, Pixel(coordinates, colour): Pixel<Rgb888>) {
        let (width, height) = {
            let info = self.framebuffer.info();
            (info.width, info.height)
        };

        let (x, y) = {
            let c: (i32, i32) = coordinates.into();
            (c.0 as usize, c.1 as usize)
        };

        if (0..width).contains(&x) && (0..height).contains(&y) {
            set_pixel_in(self.framebuffer, Position { x, y }, Colour::new(colour));
        }
    }
}

impl<'f> DrawTarget for Display<'f> {
    type Color = Rgb888;

    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels.into_iter() {
            self.draw_pixel(pixel);
        }

        Ok(())
    }
}

impl <'f> OriginDimensions for Display<'f> {
    fn size(&self) -> Size {
        let info = self.framebuffer.info();

        Size::new(info.width as u32, info.height as u32)
    }
}
