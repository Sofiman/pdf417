#![no_std]
#![allow(dead_code)]

mod tables;
use tables::HL_TO_LL;

pub mod ecc;
pub mod high_level;
pub use high_level::*;

const START: u32 = 0b11111111010101000;
const   END: u32 = 0b111111101000101001;
pub const START_PATTERN_LEN: u8 = 17;
pub const END_PATTERN_LEN: u8 = 18;

pub trait RenderTarget {
    type State;
    fn init_state(&self, config: &PDF417) -> Self::State;

    fn row_start(&mut self, state: &mut Self::State);
    fn row_end(&mut self, state: &mut Self::State);
    fn append_bits(&mut self, state: &mut Self::State, value: u32, count: u8);
}

#[derive(Debug, Default)]
pub struct BoolSliceRenderConfig {
    i: usize,
    row_start: usize,
    scale: (u32, u32)
}

impl RenderTarget for [bool] {
    type State = BoolSliceRenderConfig;

    fn init_state(&self, config: &PDF417) -> Self::State {
        BoolSliceRenderConfig { scale: config.scale, ..Default::default() }
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

    fn append_bits(&mut self, state: &mut Self::State, value: u32, count: u8) {
        let w = state.scale.0 as usize;
        let i = &mut state.i;
        let mut mask = 1 << (count as u32 - 1);
        for _ in 0..count {
            self[(*i)..(*i + w)].fill((value & mask) == mask);
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
    #[inline]
    pub fn shift(&mut self, storage: &mut [u8], v: bool) {
        storage[self.cursor] |= (v as u8) << (7 - self.bit);
        self.bit += 1;
        if self.bit == 8 {
            self.cursor += 1;
            self.bit = 0;
        }
    }

    #[inline]
    pub fn skip(&mut self) {
        self.cursor += 1;
        self.bit = 0;
    }

    #[inline]
    pub fn move_to(&mut self, cursor: usize) {
        self.cursor = cursor;
        self.bit = 0;
    }
}

#[derive(Debug, Default)]
pub struct ByteSliceRenderConfig {
    bs: BitShifter,
    row_start: usize,
    scale: (u32, u32)
}

impl RenderTarget for [u8] {
    type State = ByteSliceRenderConfig;

    fn init_state(&self, config: &PDF417) -> Self::State {
        ByteSliceRenderConfig { scale: config.scale, ..Default::default() }
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

    fn append_bits(&mut self, state: &mut Self::State, value: u32, mut count: u8) {
        let w = state.scale.0 as usize;
        while count > 0 {
            // get upper 8 bits
            count -= 1;
            for _ in 0..w {
                state.bs.shift(self, (value >> count) & 1 == 1);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PDF417<'a> {
    codewords: &'a [u16],
    /// 3 to 90
    rows: u32,
    /// Up to 583 
    cols: u32,
    level: u8,
    scale: (u32, u32),

    /// In a relatively "clean" environment where label damage is unlikely
    /// (e.g., an office), the right row indicators can be omitted and the stop
    /// pattern can be reduced to one module width bar.
    /// This truncation reduces the non-data overhead from 4 codewords per row
    /// to 2 codewords per row, with a trade-off in decode performance and
    /// robustness, or the ability to withstand degradation.
    ///
    /// This version is called Truncated PDF417, which is fully reader
    /// compatible with standard PDF41.7.
    truncated: bool
}

const LEADING_ONE: u32 = 1 << 16;
macro_rules! cw {
    ($tb:ident, $val:expr) => {
        LEADING_ONE + HL_TO_LL[$tb * 929 + $val as usize] as u32
    }
}

#[macro_export]
macro_rules! pdf417_width {
    ($cols:expr) => {
        pdf417_width!($cols, 1);
    };
    ($cols:expr, $scale_x:expr) => {
        pdf417_width!($cols, $scale_x, 0);
    };
    ($cols:expr, $scale_x:expr, $pad:expr) => {
        pdf417_width!($cols, $scale_x, $pad, false);
    };
    ($cols:expr, $scale_x:expr, $pad:expr, $truncated:expr) => {
        if $truncated {
            (17 + 17 + $cols * 17) * $scale_x + $pad
        } else {
            (17 + 17 + $cols * 17 + 17 + 18) * $scale_x + $pad
        }
    };
}

#[macro_export]
macro_rules! pdf417_height {
    ($rows:expr) => {
        pdf417_height!($rows, 1);
    };
    ($rows:expr, $scale_y:expr) => {
        pdf417_height!($rows, $scale_y, 0);
    };
    ($rows:expr, $scale_y:expr, $pad:expr) => {
        $rows * $scale_y + $pad
    };
}

impl<'a> PDF417<'a> {
    pub const fn new(codewords: &'a [u16], rows: u32, cols: u32, level: u8) -> Self {
        assert!(level < 9, "ECC level must be between 0 and 8 inclusive");
        assert!(codewords.len() <= (rows*cols) as usize,
            "codewords will not fit in a the provided configuration");

        PDF417 { codewords, rows, cols, level, truncated: false, scale: (1, 1) }
    }

    pub const fn scaled(codewords: &'a [u16], rows: u32, cols: u32, level: u8, scale: (u32, u32)) -> Self {
        assert!(level < 9, "ECC level must be between 0 and 8 inclusive");
        assert!(codewords.len() <= (rows*cols) as usize,
            "codewords will not fit in a the provided configuration");
        assert!(scale.0 > 0 && scale.1 > 0);

        PDF417 { codewords, rows, cols, level, scale, truncated: false }
    }

    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub const fn truncated(self, truncated: bool) -> Self {
        Self { truncated, ..self }
    }

    pub const fn rows(&self) -> u32 {
        self.rows
    }

    pub const fn cols(&self) -> u32 {
        self.cols
    }

    pub const fn scale(&self) -> (u32, u32) {
        self.scale
    }

    pub fn render<Target: RenderTarget + ?Sized>(&self, storage: &mut Target) {
        let rows_val = (self.rows - 1) / 3;
        let cols_val = self.cols - 1;
        let level_val = self.level as u32 * 3 + (self.rows - 1) % 3;

        let mut table = 0;
        let mut state = storage.init_state(self);
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
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderTarget, PDF417};

    #[test]
    fn test_append_bits_to_bool_slice() {
        let mut t = [false; 13];
        let mut state = t.init_state(&PDF417::new(&[0u16; 1], 1, 1, 0));
        t.append_bits(&mut state, 0b110001, 6);
        t.append_bits(&mut state, 0b11, 2);
        t.append_bits(&mut state, 0b00111, 5);

        assert_eq!(&t, &[true, true, false, false, false, true, true, true, false, false, true, true, true]);
    }

    #[test]
    fn test_append_bits_to_byte_slice() {
        let mut t = [0u8; 5];
        let mut state = t.init_state(&PDF417::new(&[0u16; 1], 1, 1, 0));
        t.append_bits(&mut state, 0b10101010_10101010_1, 17);
        t.append_bits(&mut state, 0b1110001_110001, 13);
        t.append_bits(&mut state, 0b11, 2);
        t.append_bits(&mut state, 0b0000111, 7);

        assert_eq!(&t, &[0b10101010, 0b10101010, 0b11110001, 0b11000111, 0b00001110]);
    }

    #[test]
    fn test_append_bits_to_bool_slice_scaled() {
        let mut t = [false; 6 * 3 * 2];
        let mut state = t.init_state(&PDF417::scaled(&[0u16; 1], 1, 1, 0, (3, 2)));
        t.row_start(&mut state);
        t.append_bits(&mut state, 0b110001, 6);

        assert_eq!(&t[..(t.len()/2)], &[true, true, true, true, true, true, false, false, false, false, false, false, false, false, false, true, true, true], "Testing X scale");

        t.row_end(&mut state);
        assert_eq!(&t[(t.len()/2)..], &[true, true, true, true, true, true, false, false, false, false, false, false, false, false, false, true, true, true],"Testing Y scale");
    }

    #[test]
    fn test_append_bits_to_byte_slice_scaled() {
        let mut t = [0u8; (8 * 3 * 2) / 8];
        let mut state = t.init_state(&PDF417::scaled(&[0u16; 1], 1, 1, 0, (3, 2)));
        t.row_start(&mut state);
        t.append_bits(&mut state, 0b01000111, 8);

        assert_eq!(&t[..(t.len()/2)], &[0b00011100, 0b00000001, 0b11111111], "Testing X scale");

        t.row_end(&mut state);
        assert_eq!(&t[(t.len()/2)..], &[0b00011100, 0b00000001, 0b11111111], "Testing Y scale");
    }
}
