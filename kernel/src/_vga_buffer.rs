use core::{fmt, mem::MaybeUninit, ptr};
use lazy_static::lazy_static;
use spin::Mutex;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)] // ensures actually u8
pub enum Colour {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)] // u8 interaction but cooler
struct ColourCode(u8);

impl ColourCode {
    fn new(foreground: Colour, background: Colour) -> ColourCode {
        ColourCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)] // guaratees correct field ordering, like C
struct ScreenChar {
    ascii_character: u8,
    colour_code: ColourCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[MaybeUninit<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// https://github.com/phil-opp/blog_os/issues/1301#issuecomment-2227451984
impl Buffer {
    fn write(&mut self, row: usize, col: usize, c: ScreenChar) {
        unsafe {
            // UNSAFE: all pointers in `chars` point to a valid ScreenChar in the VGA buffer.
            ptr::write_volatile(&mut self.chars[row][col], MaybeUninit::new(c));
        }
    }
}

pub struct Writer {
    column_position: usize,
    colour_code: ColourCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let colour_code = self.colour_code;
                self.buffer.write(
                    row,
                    col,
                    ScreenChar {
                        ascii_character: byte,
                        colour_code,
                    },
                );
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character =
                    unsafe { ptr::read_volatile(&self.buffer.chars[row][col].assume_init()) };
                self.buffer.write(row - 1, col, character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            colour_code: self.colour_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.write(row, col, blank);
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // either a real printable ascii character or its a newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // nuh uh get sploded
                _ => self.write_byte(0xf3),
            }
        }
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        colour_code: ColourCode::new(Colour::Magenta, Colour::Black),
        buffer: unsafe {
            (ptr::with_exposed_provenance_mut::<Buffer>(0xb8000).as_mut()).unwrap_unchecked()
        },
    });
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap(); // could cause deadlock if interrupt happens here, forever spinlock
    });
}

#[test_case]
fn test_println_simple() {
    println!("meow"); // if we dont panic then yay! we did it!
}

#[test_case]
fn test_println_lots() {
    for _ in 0..200 {
        println!(":3");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    let s = "this is very important information";
    interrupts::without_interrupts(|| {
        // prevent race condition or whatever, keep writer locked for whole test
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        // ok now verify it actually happened
        for (i, c) in s.chars().enumerate() {
            // counting iters in i and character in c
            // println! appends a newline so its the one before the last line
            let screen_char = unsafe {
                ptr::read_volatile(&writer.buffer.chars[BUFFER_HEIGHT - 2][i].assume_init())
            };
            assert_eq!(char::from(screen_char.ascii_character), c); // they equal!! or panic :(
        }
    });
}

#[test_case]
fn test_println_wrap() {
    // if we panic from boundscheck then well this didnt work
    println!(
        "this is a very long string that should start to wrap, as it does not fit on one entire row. i will now begin my performance. *ahem* meow meow meow meow meow meow meow meow meow meow. thank you."
    );
}
