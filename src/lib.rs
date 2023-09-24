//! # PDF417 Generator
//!
//! A no-std and no-alloc PDF417 encoder for embedded applications (also works for
//! std). This library implements mutliple encoding modes for numbers, strings and
//! bytes according to the specification. You can also customize the rendering of
//! the barcodes (size, storage and inverted) and supports the Truncated PDF417.
//!
//! #### Basic Example
//! ```
//! # use pdf417::*;
//! const COLS: u8 = 3;
//! const ROWS: u8 = 5;
//! const ECC_LEVEL: u8 = 1;
//! const WIDTH: usize = pdf417_width!(COLS);
//! const HEIGHT: usize = pdf417_height!(ROWS);
//! 
//! // High-level encoding
//! let mut input = [0u16; (ROWS * COLS) as usize];
//! PDF417Encoder::new(&mut input, false)
//!     .append_ascii("Hello, world!").seal(ECC_LEVEL);
//! 
//! // Rendering
//! let mut storage = [false; WIDTH * HEIGHT];
//! PDF417::new(&input, ROWS, COLS, ECC_LEVEL).render(&mut storage[..]);
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

#![no_std]
#![feature(const_mut_refs)]

mod tables;
use tables::{HL_TO_LL, M_PDF417_VARIANTS, M_PDF417_VARIANTS_COUNT, M_PDF417_RAP, M_PDF417_SIDE, M_PDF417_CENTER};

pub mod ecc;
pub mod high_level;
pub use high_level::*;
pub use tables::{get_variant, find_variant, variant_dim};

const START: u32 = 0b11111111010101000;
const   END: u32 = 0b111111101000101001;

/// Size in pixels of the PDF417 start pattern
pub const START_PATTERN_LEN: u8 = 17;
/// Size in pixels of the PDF417 end pattern
pub const END_PATTERN_LEN: u8 = 18;

/// Minimum number of rows in a PDF417 barcode.
pub const MIN_ROWS: u8 = 3;
/// Maximum number of rows in a PDF417 barcode.
pub const MAX_ROWS: u8 = 90;
/// Minimum number of data columns in a PDF417 barcode.
pub const MIN_COLS: u8 = 1;
/// Maximum number of data columns in a PDF417 barcode.
pub const MAX_COLS: u8 = 30;

/// (rows, cols, (scaleX, scaleY), inverted)
pub type PDF417Config = (u8, u8, (u32, u32), bool);

/// A receiver for rendering PDF417 barcodes.
pub trait RenderTarget {
    /// User defined type for storing the progress of the rendering and/or
    /// various configuration values.
    type State;

    /// Called at the beginning of the rendering of an PDF417 passed by
    /// reference. You can store any state you want which you will be able to
    /// use later in the next functions.
    fn begin(&self, config: PDF417Config) -> Self::State;

    /// Called at the beginning of a row (before the start pattern and left
    /// codeword are appended).
    fn row_start(&mut self, state: &mut Self::State);

    /// Called at the end of a row (after the right codeword and end
    /// pattern are appended).
    fn row_end(&mut self, state: &mut Self::State);

    /// Append the `count` least significant bits stored in `value` as pixels.
    /// The `count` is guaranteed to be less or equal than 32. A set bit
    /// represents a black pixel and a unset bit a white pixel.
    fn append_bits(&mut self, state: &mut Self::State, value: u32, count: u8);

    #[allow(unused_variables)]
    /// Called at the end of the rendering of an PDF417. The last state is
    /// moved into this function.
    fn end(&mut self, state: Self::State) {}
}

#[derive(Debug, Default)]
/// Struct used to implement RenderTarget for \[bool\]
///
/// This allows passing an \[bool\] to [PDF417::render].
pub struct BoolSliceRenderConfig {
    i: usize,
    row_start: usize,
    scale: (u32, u32),
    inverted: bool
}

impl RenderTarget for [bool] {
    type State = BoolSliceRenderConfig;

    fn begin(&self, (_, _, scale, inverted): PDF417Config) -> Self::State {
        BoolSliceRenderConfig { scale, inverted, ..Default::default() }
    }

    fn row_start(&mut self, state: &mut Self::State) {
        state.row_start = state.i;
    }

    fn row_end(&mut self, state: &mut Self::State) {
        let w = state.scale.1 as usize;
        if w > 1 {
            let mut i = state.i;
            let len = state.i - state.row_start;
            for _ in 0..(w - 1) {
                self.copy_within((state.row_start)..(state.row_start+len), i);
                i += len;
            }
            state.i = i;
        }
    }

    fn append_bits(&mut self, state: &mut Self::State, mut value: u32, count: u8) {
        if state.inverted {
            value = !value;
        }
        let w = state.scale.0 as usize;
        let i = &mut state.i;
        let mut mask = 1 << (count as u32 - 1);
        for _ in 0..count {
            self[(*i)..(*i + w)].fill((value & mask) != 0);
            *i += w;
            mask >>= 1;
        }
    }
}

#[derive(Debug, Default)]
struct BitShifter {
    cursor: usize,
    bit: u8
}

impl BitShifter {
    #[inline(always)]
    pub fn shift(&mut self, storage: &mut [u8], v: bool) {
        if v { storage[self.cursor] |= 1 << (7 - self.bit); }
        self.bit += 1;
        if self.bit == 8 {
            self.cursor += 1;
            self.bit = 0;
        }
    }

    #[inline(always)]
    pub fn skip(&mut self) {
        self.cursor += 1;
        self.bit = 0;
    }

    #[inline(always)]
    pub fn move_to(&mut self, cursor: usize) {
        self.cursor = cursor;
        self.bit = 0;
    }
}

#[derive(Debug, Default)]
/// Struct used to implement RenderTarget for \[u8\].
///
/// This allows passing an \[u8\] to [PDF417::render]. Please note that
/// after each row there is some padding zeros at the end of the current byte.
/// Therefore, when using the slice of bytes to renders you must skip these
/// bytes by checking if we reached the end of the row and discarding the end of
/// the byte being rendered.
pub struct ByteSliceRenderConfig {
    bs: BitShifter,
    row_start: usize,
    inverted: bool,
    scale: (u32, u32)
}

impl RenderTarget for [u8] {
    type State = ByteSliceRenderConfig;

    fn begin(&self, (_, _, scale, inverted): PDF417Config) -> Self::State {
        ByteSliceRenderConfig { scale, inverted, ..Default::default() }
    }

    fn row_start(&mut self, state: &mut Self::State) {
        state.row_start = state.bs.cursor;
    }

    fn row_end(&mut self, state: &mut Self::State) {
        if state.bs.bit > 0 {
            // add padding to the last byte of the row
            state.bs.skip();
        }

        let h = state.scale.1;
        if h > 1 {
            let mut i = state.bs.cursor;
            let j = state.row_start;
            let len = i - j;
            for _ in 0..(h - 1) {
                self.copy_within(j..(j+len), i);
                i += len;
            }
            state.bs.move_to(i);
        }
    }

    fn append_bits(&mut self, state: &mut Self::State, mut value: u32, mut count: u8) {
        if state.inverted {
            value = !value;
        }
        let w = state.scale.0 as usize;
        while count > 0 {
            // get upper 8 bits
            count -= 1;
            let is_set = (value >> count) & 1 == 1;
            for _ in 0..w {
                state.bs.shift(self, is_set);
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Configuration and Rendering of PDF417 barcodes.
pub struct PDF417<'a> {
    codewords: &'a [u16],
    rows: u8,
    cols: u8,
    level: u8,
    scale: (u32, u32),
    truncated: bool,
    inverted: bool
}

const LEADING_ONE: u32 = 1 << 16;
macro_rules! cw {
    ($tb:ident, $val:expr) => {
        LEADING_ONE + HL_TO_LL[$tb * 929 + $val as usize] as u32
    }
}

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
            (pdf417::START_PATTERN_LEN as usize + 17 + $cols as usize * 17 + 1)
                * $scale_x as usize
        } else {
            (pdf417::START_PATTERN_LEN as usize + 17 + $cols as usize * 17 + 17 + pdf417::END_PATTERN_LEN as usize)
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

impl<'a> PDF417<'a> {
    /// Creates a new PDF417 with the user's data section (codewords slice),
    /// the level of error correction and the layout configuration
    /// (rows and cols). The total codewords capacity is calculated with 
    /// rows \* cols and must be greater or equal to the number of codewords
    /// in the `codewords` slice.
    pub const fn new(codewords: &'a [u16], rows: u8, cols: u8, level: u8) -> Self {
        assert!(rows >= MIN_ROWS && rows <= MAX_ROWS, "The number of rows must be between 3 and 90");
        assert!(cols >= MIN_COLS && cols <= MAX_COLS, "The number of columns must be between 1 and 30");
        assert!(codewords.len() == (rows as usize * cols as usize),
            "The data will not fit in the provided configuration");
        assert!(level < 9, "ECC level must be between 0 and 8 inclusive");

        PDF417 { codewords, rows, cols, level, scale: (1, 1), truncated: false, inverted: false }
    }

    /// Returns if the PDF417 is set to be rendered as a Truncated PDF417.
    ///
    /// See also [set_truncated](PDF417::set_truncated).
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Marks whether this PDF417 should be rendered as a Truncated PDF417.
    ///
    /// See also [set_truncated](PDF417::set_truncated).
    pub const fn truncated(mut self, truncated: bool) -> Self {
        self.truncated = truncated;
        self
    }

    /// Marks whether this PDF417 should be rendered as a Truncated PDF417.
    /// Quote from the PDF417 specification:
    /// > In a relatively "clean" environment where label damage is unlikely
    /// > (e.g., an office), the right row indicators can be omitted and the stop
    /// > pattern can be reduced to one module width bar.
    /// > This truncation reduces the non-data overhead from 4 codewords per row
    /// > to 2 codewords per row, with a trade-off in decode performance and
    /// > robustness, or the ability to withstand degradation.
    pub const fn set_truncated(&mut self, truncated: bool) -> &mut Self {
        self.truncated = truncated;
        self
    }

    /// Returns the scale of the PDF417 as (Scale X axis, Scale Y axis).
    pub const fn scale(&self) -> (u32, u32) {
        self.scale
    }

    /// Sets the scale of the PDF417 on both axis. The scale tuple stores
    /// the non-zero scale values as (Scale X axis, Scale Y axis).
    pub const fn scaled(mut self, scale: (u32, u32)) -> Self {
        assert!(scale.0 != 0 && scale.1 != 0, "scale cannot be zero");
        self.scale = scale;
        self
    }

    /// Sets the scale of the PDF417 on both axis. See also [scaled](PDF417::scaled).
    pub const fn set_scaled(&mut self, scale: (u32, u32)) -> &mut Self {
        assert!(scale.0 != 0 && scale.1 != 0, "scale cannot be zero");
        self.scale = scale;
        self
    }

    /// Returns if the PDF417 is set to be rendered with inverted colors.
    pub const fn is_inverted(&self) -> bool {
        self.inverted
    }

    /// Marks whether this PDF417 should be rendered with pixel values inverted.
    pub const fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    /// Marks whether this PDF417 should be rendered with pixel values inverted.
    pub const fn set_inverted(&mut self, inverted: bool) -> &mut Self {
        self.inverted = inverted;
        self
    }

    /// Get the number of rows of the PDF417.
    pub const fn rows(&self) -> u8 {
        self.rows
    }

    /// Get the number of columns of the PDF417. This is used to lay down the 
    /// start, left, right and end indicators in the render function.
    pub const fn cols(&self) -> u8 {
        self.cols
    }

    /// Set the dimensions of the PDF417 as (number of rows, number of cols).
    pub const fn with_dimensions(mut self, (rows, cols): (u8, u8)) -> Self {
        assert!(rows >= MIN_ROWS && rows <= MAX_ROWS, "The number of rows must be between 3 and 90");
        assert!(cols >= MIN_COLS && cols <= MAX_COLS, "The number of columns must be between 1 and 30");
        assert!(self.codewords.len() <= (rows as usize * cols as usize),
            "The data will not fit in the provided configuration");
        self.rows = rows;
        self.cols = cols;
        self
    } 

    /// Set the number of columns of the PDF417.
    pub const fn set_dimensions(&mut self, (rows, cols): (u8, u8)) -> &mut Self {
        assert!(rows >= MIN_ROWS && rows <= MAX_ROWS, "The number of rows must be between 3 and 90");
        assert!(cols >= MIN_COLS && cols <= MAX_COLS, "The number of columns must be between 1 and 30");
        assert!(self.codewords.len() <= (rows as usize * cols as usize),
            "The data will not fit in the provided configuration");
        self.rows = rows;
        self.cols = cols;
        self
    } 

    /// Render the PDF417 to a suitable render target. The scale, truncated,
    /// cols, and rows configuration values are used here to lay down the
    /// pixels to construct a valid barcode according to the specification.
    ///
    /// **Note**: The scale parameter is handled by the RenderTarget which allows
    /// for specialized ways of copying the pixel values.
    pub fn render<Target: RenderTarget + ?Sized>(&self, storage: &mut Target) {
        let rows_val = (self.rows as u32 - 1) / 3;
        let cols_val = self.cols as u32 - 1;
        let level_val = self.level as u32 * 3 + (self.rows as u32 - 1) % 3;

        let mut table = 0;
        let mut state = storage.begin((self.rows, self.cols, self.scale, self.inverted));
        let mut col = 0;
        let mut row = 0;

        for &codeword in self.codewords {
            if col == 0 {
                storage.row_start(&mut state);

                // row start pattern
                storage.append_bits(&mut state, START, START_PATTERN_LEN);

                // row left pattern
                let cw = match table {
                    0 => rows_val,
                    1 => level_val,
                    2 => cols_val,
                    _ => unreachable!()
                };
                storage.append_bits(&mut state, cw!(table, (row / 3) * 30 + cw), 17);
            }

            storage.append_bits(&mut state, cw!(table, codeword), 17);
            col += 1;

            if col == self.cols {
                if self.truncated {
                    // stop pattern reduced to one module width bar
                    storage.append_bits(&mut state, 1, 1);
                } else {
                    // row right codeword
                    let cw = match table {
                        0 => cols_val,
                        1 => rows_val,
                        2 => level_val,
                        _ => unreachable!()
                    };
                    storage.append_bits(&mut state, cw!(table, (row / 3) * 30 + cw), 17);

                    storage.append_bits(&mut state, END, END_PATTERN_LEN);
                }

                storage.row_end(&mut state);

                col = 0;
                row += 1;
                if table == 2 { table = 0; } else { table += 1 };
            }
        }

        storage.end(state);
    }
}

#[macro_export]
/// Calculate the width in pixels of a PDF417 barcode according to the
/// configuration (Columns, X scale, Is Truncated). Only the number of columns
/// is required, other parameters can be omitted in order.
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
/// Calculate the height in pixels of a PDF417 barcode according to the
/// configuration (Rows, Y scale). Only the number of rows is required, other
/// parameters can be omitted in order.
macro_rules! m_pdf417_height {
    ($rows:expr) => {
        m_pdf417_height!($rows, 2);
    };
    ($rows:expr, $scale_y:expr) => {
        $rows as usize * $scale_y as usize
    };
}

pub struct MicroPDF417<'a> {
    codewords: &'a [u16],
    variant: u8
}

impl<'a> MicroPDF417<'a> {
    /// Creates a new PDF417 with the user's data section (codewords slice),
    /// the level of error correction and the layout configuration
    /// (rows and cols). The total codewords capacity is calculated with 
    /// rows \* cols and must be greater or equal to the number of codewords
    /// in the `codewords` slice.
    pub const fn new(codewords: &'a [u16], variant: u8) -> Self {
        assert!(variant <= 34);
        MicroPDF417 { codewords, variant }
    }

    pub fn render<Target: RenderTarget + ?Sized>(&self, storage: &mut Target) {
        let variant = self.variant as usize;
        let (cols, rows, mut left, mut center, mut right, mut table) = (
            M_PDF417_VARIANTS[variant] as usize,
            M_PDF417_VARIANTS[M_PDF417_VARIANTS_COUNT + variant] as usize,
            M_PDF417_RAP[0 * M_PDF417_VARIANTS_COUNT + variant] as usize - 1,
            M_PDF417_RAP[1 * M_PDF417_VARIANTS_COUNT + variant] as usize - 1,
            M_PDF417_RAP[2 * M_PDF417_VARIANTS_COUNT + variant] as usize - 1,
            M_PDF417_RAP[3 * M_PDF417_VARIANTS_COUNT + variant] as usize,
        );
        let mut state = storage.begin((rows as u8, cols as u8, (1, 1), false));

        for row in 0..rows {
            storage.row_start(&mut state);
            storage.append_bits(&mut state, M_PDF417_SIDE[left] as u32, 10);

            let mut col = 0;
            while col < cols && col < 2 {
                storage.append_bits(&mut state, cw!(table, self.codewords[row * cols + col]), 17);
                col += 1;
            }

            if col < cols {
                storage.append_bits(&mut state, M_PDF417_CENTER[center] as u32, 10);

                while col < cols {
                    storage.append_bits(&mut state, cw!(table, self.codewords[row * cols + col]), 17);
                    col += 1;
                }
            }

            storage.append_bits(&mut state, ((M_PDF417_SIDE[right] as u32) << 1) | 1, 11);
            storage.row_end(&mut state);

            if left == 51 { left = 0; } else { left += 1; }
            if center == 51 { center = 0; } else { center += 1; }
            if right == 51 { right = 0; } else { right += 1; }
            if table == 2 { table = 0; } else { table += 1; }
        }

        storage.end(state);
    }
}

#[cfg(test)]
mod tests {
    use super::RenderTarget;

    #[test]
    fn test_append_bits_to_bool_slice() {
        let mut t = [false; 13];
        let mut state = t.begin((3, 1, (1, 1), false));
        t.append_bits(&mut state, 0b110001, 6);
        t.append_bits(&mut state, 0b11, 2);
        t.append_bits(&mut state, 0b00111, 5);

        assert_eq!(&t, &[true, true, false, false, false, true, true, true, false, false, true, true, true]);
    }

    #[test]
    fn test_append_bits_to_byte_slice() {
        let mut t = [0u8; 5];
        let mut state = t.begin((3, 1, (1, 1), false));
        t.append_bits(&mut state, 0b10101010_10101010_1, 17);
        t.append_bits(&mut state, 0b1110001_110001, 13);
        t.append_bits(&mut state, 0b11, 2);
        t.append_bits(&mut state, 0b0000111, 7);

        assert_eq!(&t, &[0b10101010, 0b10101010, 0b11110001, 0b11000111, 0b00001110]);
    }

    #[test]
    fn test_append_bits_to_bool_slice_scaled() {
        let mut t = [false; 6 * 3 * 2];
        let mut state = t.begin((3, 1, (3, 2), false));
        t.row_start(&mut state);
        t.append_bits(&mut state, 0b110001, 6);

        assert_eq!(&t[..(t.len()/2)], &[true, true, true, true, true, true, false, false, false, false, false, false, false, false, false, true, true, true], "Testing X scale");

        t.row_end(&mut state);
        assert_eq!(&t[(t.len()/2)..], &[true, true, true, true, true, true, false, false, false, false, false, false, false, false, false, true, true, true],"Testing Y scale");
    }

    #[test]
    fn test_append_bits_to_byte_slice_scaled() {
        let mut t = [0u8; (8 * 3 * 2) / 8];
        let mut state = t.begin((3, 1, (3, 2), false));
        t.row_start(&mut state);
        t.append_bits(&mut state, 0b01000111, 8);

        assert_eq!(&t[..(t.len()/2)], &[0b00011100, 0b00000001, 0b11111111], "Testing X scale");

        t.row_end(&mut state);
        assert_eq!(&t[(t.len()/2)..], &[0b00011100, 0b00000001, 0b11111111], "Testing Y scale");
    }
}
