#![no_std]
#![allow(dead_code)]

mod tables;
pub mod ecc;

use tables::HL_TO_LL;

use bnum::BUintD32;
use num_traits::cast::ToPrimitive;
use num_integer::Integer;
type U160 = BUintD32::<5>;

const START: u32 = 0b11111111010101000;
const   END: u32 = 0b111111101000101001;
pub const START_PATTERN_LEN: u8 = 17;
pub const END_PATTERN_LEN: u8 = 18;

const MIXED_CHAR_SET: [u8; 15] = [
    b'&', b'\r', b'\t', b',', b':', b'#', b'-', b'.', b'$', b'/', b'+', b'%', b'*', b'=', b'^'
];
const PUNC_CHAR_SET: [u8; 29] = [
    b';', b'<', b'>', b'@', b'[', b'\\', b']', b'_', b'`', b'~', b'!', b'\r', b'\t',
    b',', b':', b'\n', b'-', b'.', b'$', b'/', b'"', b'|', b'*', b'(', b')', b'?',
    b'{', b'}', b'\''
];

pub fn generate_text(s: &str, out: &mut [u16], level: u8) -> usize {
    // 6 bytes = 5 codewords; +1 for length indicator + 1 for byte mode +2 for ECI mode
    let ecc_cw = ecc::ecc_count(level);
    let min = (s.len()/6)*5 + (s.len() % 6) + ecc_cw + 1 + 1 + 2;
    assert!(out.len() >= min, "output buffer not large enough to fit {min} codewords");

    // metadata
    let data_end = out.len() - ecc_cw;
    out[0] = data_end as u16;
    out[1] = 927; // ECI identifier for code page
    out[2] = 26; // UTF-8 is \000026

    let data_words = encode_bytes(s.as_bytes(), &mut out[3..data_end]);
    ecc::generate_ecc(out, level);
    return data_words + ecc_cw + 3;
}

pub fn encode_bytes(bytes: &[u8], out: &mut [u16]) -> usize {
    let mut i = 0;
    let mut k = 0;

    if bytes.len() > 1 {
        // latch to byte mode
        out[i] = if bytes.len() % 6 == 0 { 924 } else { 901 };
        i += 1;

        while bytes.len()-k >= 6 {
            // pack six bytes
            let mut s: u64 = 0;
            for n in 0..6 {
                s = (s << 8) + bytes[k + n] as u64;
            }
            // append five codewords
            for n in 0..5 {
                let (q, r) = (s / 900, s % 900);
                out[i + 4 - n] = r as u16;
                s = q;
            }

            i += 5;
            k += 6;
        }
    } else {
        out[i] = 913; // shift to byte mode (only for next codeword)
        i += 1;
    }

    // remaining
    while k < bytes.len() {
        out[i] = bytes[k] as u16;
        k += 1;
        i += 1;
    }

    out[i..].fill(900); // padding

    return i;
}

pub fn generate_ascii(s: &str, out: &mut [u16], level: u8) -> usize {
    // 2 char = 1 codeword; +1 for length indicator +4 for mode switches
    // TODO: Relax with the fixed +4 in required capacity
    let ecc_cw = ecc::ecc_count(level);
    assert!(out.len() >= s.len()/2 + ecc_cw + 1 + 4, "output buffer not large enough");

    // metadata
    let data_end = out.len() - ecc_cw;
    out[0] = data_end as u16;

    let data_words = encode_ascii(s, &mut out[1..data_end]);
    ecc::generate_ecc(out, level);
    return data_words + ecc_cw + 1;
}

macro_rules! push {
    ($cws:ident, $i:ident, $rh:ident, $($cw:expr),+; $post:ident = $new:expr) => {{
        push!($cws, $i, $rh, $($cw),+);
        $post = $new;
    }};
    ($cws:ident, $i:ident, $rh:ident, $head:expr, $($cw:expr),+) => {
        push!($cws, $i, $rh, $head);
        push!($cws, $i, $rh, $($cw),+);
    };
    ($cws:ident, $i:ident, $rh:ident, $cw:expr) => {{
        let cw = $cw as u16;
        if cw > 29 {
            if $rh {
                $cws[$i] = $cws[$i] * 30 + 29;
                $rh = false;
                $i += 1;
            }

            $cws[$i] = cw;
            $i += 1;
        } else {
            if $rh {
                $cws[$i] = $cws[$i] * 30 + cw;
                $rh = false;
                $i += 1;
            } else {
                $cws[$i] = cw;
                $rh = true;
            }
        }
    }}
}

pub fn encode_ascii(s: &str, out: &mut [u16]) -> usize {
    debug_assert!(s.is_ascii());
    let s = s.as_bytes();

    let mut mode: u8 = 0; // 0: Upper, 1: Lower, 2: Mixed, 3: Punc, 4: Numeric
    let mut i = 0;
    let mut k = 0;
    let mut right = false; // false = upper 8 bits | true = lower 8 bits

    while k < s.len() {
        let c = s[k];
        match c {
            b'A'..=b'Z' => {
                match mode {
                    0 => (),
                    1 => if k + 1 < s.len() && ((b'a'..=b'z').contains(&s[k + 1]) || s[k + 1] == b' ') {
                        push!(out, i, right, 27);
                    } else {
                        push!(out, i, right, 29, 29; mode = 0);
                    },
                    2 => push!(out, i, right, 28; mode = 0),
                    3 => push!(out, i, right, 29; mode = 0),
                    _ => unreachable!("Unknown mode {mode}"),
                }
                push!(out, i, right, c - b'A'; k = k + 1);
            },
            b'a'..=b'z' => {
                match mode {
                    0 | 2 => push!(out, i, right, 27; mode = 1),
                    1 => (),
                    3 => push!(out, i, right, 29, 27; mode = 1),
                    _ => unreachable!("Unknown mode {mode}"),
                }
                push!(out, i, right, c - b'a'; k = k + 1);
            },
            b'0'..=b'9' if mode == 2 => push!(out, i, right, c - b'0'; k = k + 1),
            b'0'..=b'9' => {
                let mut end = k + 1;
                while end < s.len() && end-k < 44 && (b'0'..=b'9').contains(&s[end]) {
                    end += 1;
                }

                if end-k <= 13 && mode != 4 {
                    match mode {
                        0 | 1 => push!(out, i, right, 28; mode = 2),
                        2 => (),
                        3 => push!(out, i, right, 29, 28; mode = 2),
                        _ => unreachable!("Unknown mode {mode}"),
                    }
                    while k < end {
                        push!(out, i, right, s[k] - b'0'; k = k + 1);
                    }
                } else {
                    if mode != 4 { push!(out, i, right, 902; mode = 4); }

                    let b900 = U160::from(900u16);
                    let mut b = U160::from_str_radix(core::str::from_utf8(&s[k..end]).expect("only ascii"), 10)
                        .expect("44 digits base 10 should fit in 160 bits");
                    b += U160::from(10u16).pow((end-k) as u32);
                    let nb = (end-k) / 3 + 1;
                    let mut count = 0;

                    while b > U160::ZERO {
                        let (q, r) = b.div_rem(&b900);
                        b = q;
                        out[i + nb - count - 1] = r.to_u16().expect("remainder is always <900");
                        count += 1;
                    }

                    k = end;
                    i += nb;
                }

                if mode == 4 && k < s.len() && !(b'0'..=b'9').contains(&s[k]) {
                    push!(out, i, right, 900; mode = 0);
                }
            },
            b' ' => {
                if mode == 3 { push!(out, i, right, 29; mode = 0) };
                push!(out, i, right, 26; k = k + 1);
            },
            c => {
                if let Some(p) = MIXED_CHAR_SET.iter().position(|&r| r == c) {
                    match mode {
                        0 | 1 => push!(out, i, right, 28; mode = 2),
                        2 => (),
                        /* no switch if the char is also present in the punc table */
                        3 if (p >= 1 && p <= 4) || (p >= 6 && p <= 9)  => (),
                        3 => push!(out, i, right, 29, 28; mode = 2),
                        _ => unreachable!("Unknown mode {mode}"),
                    }
                    push!(out, i, right, p + 10);
                } else if let Some(p) = PUNC_CHAR_SET.iter().position(|&r| r == c) {
                    if mode != 3 {
                        let mut end = k + 1;
                        while end < s.len() && end-k < 3 && PUNC_CHAR_SET.contains(&s[end]) {
                            end += 1;
                        }
                        if end-k >= 3 { // latch
                            if mode != 2 { push!(out, i, right, 28); }
                            push!(out, i, right, 25; mode = 3);
                        } else { // shift
                            push!(out, i, right, 29);
                        }
                    }
                    push!(out, i, right, p);
                } else { // switch to byte mode
                    if right {
                        out[i] = out[i] * 30 + 29;
                        i += 1;
                    }
                    // TODO: Encode multiple bytes if consecutive instead of one by one
                    i += encode_bytes(&s[k..(k+1)], &mut out[i..(i + 2)]);
                }
                k += 1;
            },
        };
    }

    if right { 
        out[i] = out[i] * 30 + 29;
        i += 1;
    }

    out[i..].fill(900); // padding
    return i;
}

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
    use super::{encode_ascii, encode_bytes, RenderTarget, PDF417};

    #[test]
    fn test_encode_ascii_simple() {
        let mut codewords = [0u16; 3];
        encode_ascii("Test", &mut codewords);
        assert_eq!(&codewords, &[19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29]);
    }

    #[test]
    fn test_encode_ascii_simple_with_padding() {
        let mut codewords = [0u16; 5];
        encode_ascii("Test", &mut codewords);
        assert_eq!(&codewords, &[19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29, 900, 900]);
    }

    #[test]
    fn test_generate_ascii_switch_modes() {
        let mut codewords = [0u16; 8];
        encode_ascii("abc1D234\x1B", &mut codewords);
        assert_eq!(&codewords, &[27 * 30 + 0, 1 * 30 + 2, 28 * 30 + 1, 28 * 30 + 3, 28 * 30 + 2, 3 * 30 + 4, 913, 0x1B]);
    }

    #[test]
    fn test_generate_ascii_numeric() {
        let mut codewords = [0u16; 11];
        encode_ascii("12345678987654321 num", &mut codewords);
        assert_eq!(&codewords, &[902, 190, 232, 499, 20, 504, 721, 900, 26 * 30 + 27, 13 * 30 + 20, 12 * 30 + 29]);
    }

    #[test]
    fn test_generate_ascii_numeric_big() {
        let mut codewords = [0u16; 19];
        //           [                        p1                 ][ p2 ]
        encode_ascii("123456789876543211234567898765432112345678987654321", &mut codewords);
        assert_eq!(&codewords, &[902, 491, 81, 137, 725, 651, 455, 511, 858, 135, 138, 488, 568, 447, 553, 198, /* next */ 21, 715, 821]);
    }

    #[test]
    fn test_generate_ascii_with_digits() {
        let mut codewords = [0u16; 16];
        encode_ascii("encoded 0123456789 as digits", &mut codewords);
        assert_eq!(&codewords, &[27 * 30 + 4, 13 * 30 + 2, 14 * 30 + 3, 4 * 30 + 3, 26 * 30 + 28, 0 * 30 + 1, 2 * 30 + 3, 4 * 30 + 5, 6 * 30 + 7, 8 * 30 + 9,
            26 * 30 + 27, 0 * 30 + 18, 26 * 30 + 3, 8 * 30 + 6, 8 * 30 + 19, 18 * 30 + 29]);
    }

    #[test]
    fn test_generate_ascii_punc_mixed() {
        let mut codewords = [0u16; 17];
        encode_ascii("This! Is a `quote (100%)`.", &mut codewords);
        assert_eq!(&codewords, &[19 * 30 + 27, 7 * 30 + 8, 18 * 30 + 29, 10 * 30 + 26, 27 * 30 + 8, 18 * 30 + 26, 0 * 30 + 26, 29 * 30 + 8, 16 * 30 + 20, 14 * 30 + 19, 4 * 30 + 26, 29 * 30 + 23, 28 * 30 + 1, 0 * 30 + 0, 21 * 30 + 25, 24 * 30 + 8, 17 * 30 + 29]);
    }

    #[test]
    fn test_encode_bytes_multiple() {
        let mut codewords = [0u16; 6];
        encode_bytes(b"alcool", &mut codewords);
        assert_eq!(&codewords, &[924, 163, 238, 432, 766, 244]);
    }

    #[test]
    fn test_encode_bytes_not_multiple() {
        let mut codewords = [0u16; 10];
        encode_bytes(b"encode bin", &mut codewords);
        assert_eq!(&codewords, &[901, 169, 883, 224, 680, 517, 32, 98, 105, 110]);
    }

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
