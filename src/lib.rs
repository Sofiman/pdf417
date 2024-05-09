//! # PDF417 Generator
//!
//! A no-std and no-alloc PDF417 encoder for embedded applications (also works for
//! std). This library implements mutliple encoding modes for numbers, strings and
//! bytes according to the specification. You can also customize the rendering of
//! the barcodes (size, storage and inverted) and supports both Truncated PDF417
//! and MicroPDF417.
//!
//! #### Basic Example
//! ```
//! # use pdf417::*;
//! const COLS: u8 = 3;
//! const ROWS: u8 = 5;
//! const WIDTH: usize = pdf417_width!(COLS);
//! const HEIGHT: usize = pdf417_height!(ROWS);
//! 
//! // High-level encoding
//! let mut input = [0u16; (ROWS * COLS) as usize];
//! let (level, _) = PDF417Encoder::new(&mut input, false)
//!     .append_ascii("Hello, world!").fit_seal().unwrap();
//! 
//! // Rendering
//! let mut storage = [false; WIDTH * HEIGHT];
//! PDF417::new(&input, ROWS, COLS, level).render(&mut storage[..]);
//! ```
//!
//! ### Data Segments
//!
//! You can multiple data segments (aka encoding modes) on a single barcode. The
//! available types are:
//! - **numeric**: efficient encoding of 44+ digit numbers
//! - **ascii**: efficient encoding of text (alphanumeric + punctuation) with
//!     support for non-displyable ASCII values which are encoded as raw bytes.
//! - **bytes**: binary data as bytes
//! 
//! An additional **UTF-8** mode is available which allows encoding of UTF-8 strings
//! using an ECI identifier and byte encoding mode (note that this encoding takes
//! significantly more space than the ASCII mode).
//! 
//! > See the different methods available on [PDF417Encoder] struct.
//!
//! ### MicroPDF417
//!
//! This library also supports the generation of MicroPDF417. Here is an
//! example:
//!
//! ```
//! # use pdf417::*;
//! const COLS: u8 = 1;
//! const ROWS: u8 = 11;
//! const WIDTH: usize = m_pdf417_width!(COLS);
//! const HEIGHT: usize = m_pdf417_height!(ROWS);
//!
//! // High-level encoding
//! let variant = get_variant(ROWS, COLS).unwrap();
//! let mut input = [0u16; (ROWS * COLS) as usize];
//! PDF417Encoder::new(&mut input, true)
//!     .append_num(12345678).seal(variant);
//!
//! // Rendering
//! let mut storage = [false; WIDTH * HEIGHT];
//! MicroPDF417::new(&input, variant).render(&mut storage[..]);
//! ```
//!
//! Do not forget to set the `micro` parameter to true in [PDF417Encoder::new].

#![no_std]
//#![warn(missing_docs)]

mod tables;
pub mod generators;
pub mod builder;
pub mod ecc;
pub mod high_level;

use tables::*;
use generators::{bitfield::Bitfield, PDF417Row, TruncatedPDF417Row, MicroPDF417Row};

pub use high_level::*;
pub use tables::Variant;

pub const START_PATTERN: Bitfield = Bitfield::new(0b11111111010101000, 17);
pub const   END_PATTERN: Bitfield = Bitfield::new(0b111111101000101001, 18);

#[macro_export]
/// Calculate the width in pixels of a PDF417 barcode according to the
/// configuration (Columns, X scale, Is Truncated). Only the number of columns
/// is required, other parameters can be omitted in order.
macro_rules! pdf417_width {
    ($cols:expr) => {
        pdf417_width!($cols, 1);
    };
    ($cols:expr, $scale_x:expr) => {
        pdf417_width!($cols, $scale_x, false);
    };
    ($cols:expr, $scale_x:expr, $truncated:expr) => {
        if $truncated {
            (pdf417::START_PATTERN.size() as usize + 17 + $cols as usize * 17 + 1)
                * $scale_x as usize
        } else {
            (pdf417::START_PATTERN.size() as usize + 17 + $cols as usize * 17 + 17 + pdf417::END_PATTERN.size() as usize)
                * $scale_x as usize
        }
    };
}

#[macro_export]
/// Calculate the height in pixels of a PDF417 barcode according to the
/// configuration (Rows, Y scale). Only the number of rows is required, other
/// parameters can be omitted in order.
macro_rules! pdf417_height {
    ($rows:expr) => {
        pdf417_height!($rows, 1);
    };
    ($rows:expr, $scale_y:expr) => {
        $rows as usize * $scale_y as usize
    };
}

#[macro_export]
/// Calculate the width in pixels of a MicroPDF417 barcode according to the
/// configuration (Columns, X scale). Only the number of columns is required,
/// other parameters can be omitted in order.
macro_rules! m_pdf417_width {
    ($cols:expr) => {
        m_pdf417_width!($cols, 1);
    };
    ($cols:expr, $scale_x:expr) => {
        (10 + $cols as usize * 17 + ($cols as usize / 3) * 10 + 10 + 1)
            * $scale_x as usize
    };
}

#[macro_export]
/// Calculate the height in pixels of a MicroPDF417 barcode according to the
/// configuration (Rows, Y scale). Only the number of rows is required, other
/// parameters can be omitted in order. Note that the default Y scale is 2.
macro_rules! m_pdf417_height {
    ($rows:expr) => {
        m_pdf417_height!($rows, 2);
    };
    ($rows:expr, $scale_y:expr) => {
        $rows as usize * $scale_y as usize
    };
}

pub type PDF417<'a> = builder::PDF417<'a, PDF417Row<'a>>;
pub type TruncatedPDF417<'a> = builder::PDF417<'a, TruncatedPDF417Row<'a>>;
pub type MicroPDF417<'a> = builder::PDF417<'a, MicroPDF417Row<'a>>;
