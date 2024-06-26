use crate::vga_buffer::Color::{Black, LightGrey, Pink};
use lazy_static::lazy_static;
use volatile::Volatile;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGrey = 7,
    DarkGrey = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_char: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const INDENT_SIZE: usize = 4;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// Implement writer
pub struct Writer {
    column_pos: usize,
    //Store the position of latest char
    color_code: ColorCode,
    buffer: &'static mut Buffer, //ยังงงๆอยู่ว่าทำไมต้อง ref มา
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\t' => self.indent(),
            byte => {
                // In case of any other bytes, we will put into byte variable
                if self.column_pos >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1; // The default position of buffer
                let col = self.column_pos;

                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_char: byte,
                    color_code: self.color_code,
                });
                self.column_pos += 1;
            }
        }
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' | b'\t' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_pos = 0;
    }

    fn indent(&mut self) {
        // I must find next column_pos that divisible by INDENT_SIZE
        // Here is the equation that comes up on my mind
        // self.column_pos = self.column_pos + (INDENT_SIZE - (self.column_pos % INDENT_SIZE))
        // And here is the gemini help me to simplify the equation
        self.column_pos = {
            if self.column_pos < BUFFER_WIDTH - INDENT_SIZE {
                ((self.column_pos / INDENT_SIZE) + 1) * INDENT_SIZE
            } else {
                BUFFER_WIDTH - 1
                // May be new line instead?
            }
        }
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_char: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    fn set_color(&mut self, color_code: ColorCode){
        self.color_code = color_code;
    }
}

use core::fmt::{self, Write};
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.write_byte(c as u8);
        Ok(())
    }

    // fn write_fmt(&mut self, args: Arguments<'_>) -> fmt::Result {
    //     Ok(())
    // }
}

use spin::Mutex;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_pos: 0,
        color_code: ColorCode::new(Color::LightGrey, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

// Implementation of print macro
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print_panic {
    ($($arg:tt)*) => ($crate::vga_buffer::_print_panic(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print_important {
    ($($arg:tt)*) => ($crate::vga_buffer::_print_important(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    WRITER.lock().write_fmt(args).unwrap();
}

pub fn _print_panic(args: fmt::Arguments) {
    let orig_color = WRITER.lock().color_code;
    let panic_color = ColorCode::new(Color::LightRed, Color::Black);
    WRITER.lock().set_color(panic_color);
    WRITER.lock().write_fmt(args).unwrap();
    WRITER.lock().set_color(orig_color);
}

pub fn _print_important(args: fmt::Arguments) {
    let orig_color = WRITER.lock().color_code;
    let panic_color = ColorCode::new(Color::White, Color::Black);
    WRITER.lock().set_color(panic_color);
    WRITER.lock().write_fmt(args).unwrap();
    WRITER.lock().set_color(orig_color);
}

#[allow(dead_code)]
pub fn demo_printing() {
    let mut writer = Writer {
        column_pos: 0,
        color_code: ColorCode::new(LightGrey, Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    };

    writer.write_byte(b'H');
    writer.write_string("ello from");
    writer.write_string(" my_os v.0.0.1");

    write!(writer, "Numbers are {} and {}", 42, 1.0 / 3.0).unwrap();
}
