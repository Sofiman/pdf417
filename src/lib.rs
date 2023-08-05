#![no_std]
#![allow(dead_code)]

mod tables;
pub mod ecc;

use tables::HL_TO_LL;

use bnum::BUintD32;
use num_traits::cast::ToPrimitive;
use num_integer::Integer;
type U160 = BUintD32::<5>;

const START: [bool; 17] = [true, true, true, true, true, true, true, true, false, true, false, true, false, true, false, false, false];
const END: [bool; 18] = [true, true, true, true, true, true, true, false, true, false, false, false, true, false, true, false, false, true];
pub const START_PATTERN_LEN: usize = START.len();
pub const END_PATTERN_LEN: usize = END.len();

macro_rules! append {
    ($sto:ident, $tb:ident, $i:ident, $bits:expr) => {
        let b = HL_TO_LL[$tb * 929 + $bits];
        $sto[$i] = true;
        $i += 1;
        let mut mask: u16 = (1 << 15);
        for _ in 0..16 {
            $sto[$i] = (b & mask) == mask;
            mask >>= 1;
            $i += 1;
        }
    }
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

pub fn encode_text(s: &str, out: &mut [u16]) -> Result<(), ()> {
    assert!(s.is_ascii());
    let s = s.as_bytes();

    let mut mode: u8 = 0; // 0: Upper, 1: Lower, 2: Mixed, 3: Punc, 4: Numeric
    let mut i = 1;
    let mut k = 0;
    let mut right = false; // left = upper 8 bits | right: lower 8 bits

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
                    if mode != 4 {
                        push!(out, i, right, 902; mode = 4);
                    }

                    let b900 = U160::from(900u16);
                    let mut b = U160::from_str_radix(core::str::from_utf8(&s[k..end]).unwrap(), 10)
                        .expect("should fit");
                    b += U160::from(10u16).pow((end-k) as u32);
                    let nb = (end-k) / 3 + 1;
                    let mut count = 0;

                    while b > U160::ZERO {
                        let (q, r) = b.div_rem(&b900);
                        b = q;
                        out[i + nb - count - 1] = r.to_u16().ok_or(())?;
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
            _ => unreachable!()
        };
    }

    if right { 
        out[i] = out[i] * 30 + 29;
        i += 1;
    }

    out[0] = i as u16; // length indicator
    out[i..].fill(900);

    Ok(())
}

fn encode_bigint() {
}

#[derive(Debug, Clone)]
pub struct PDF417<'a> {
    codewords: &'a [u16],
    rows: usize,
    cols: usize,
    level: u8
}

impl<'a> PDF417<'a> {
    pub const fn new(codewords: &'a [u16], rows: usize, cols: usize, level: u8) -> Self {
        assert!(level < 9, "ECC level must be between 0 and 8 inclusive");
        assert!(codewords.len() == rows*cols,
            "codewords will not fit in a the provided configuration");

        PDF417 { codewords, rows, cols, level }
    }

    pub fn render(&self, storage: &mut [bool]) {
        let rows_val = (self.rows - 1) / 3;
        let cols_val = self.cols - 1;
        let level_val = self.level as usize * 3 + (self.rows - 1) % 3;

        let mut table = 0;
        let mut i = 0;
        let mut col = 0;
        let mut row = 0;

        for &codeword in self.codewords {
            if col == 0 {
                // row start pattern
                storage[i..i+START.len()].copy_from_slice(&START);
                i += START.len();

                // row left pattern
                let cw = match table {
                    0 => rows_val,
                    1 => level_val,
                    2 => cols_val,
                    _ => unreachable!()
                };
                append!(storage, table, i, (row / 3) * 30 + cw);
            }

            append!(storage, table, i, codeword as usize);
            col += 1;

            if col == self.cols {
                // row right codeword
                let cw = match table {
                    0 => cols_val,
                    1 => rows_val,
                    2 => level_val,
                    _ => unreachable!()
                };
                append!(storage, table, i, (row / 3) * 30 + cw);

                // row end pattern
                storage[i..i+END.len()].copy_from_slice(&END);
                i += END.len();

                col = 0;
                row += 1;
                if table == 2 { table = 0; } else { table += 1 };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::encode_text;

    #[test]
    fn test_generate_text_simple() {
        let mut codewords = [0u16; 4];
        encode_text("Test", &mut codewords).unwrap();
        assert_eq!(&codewords, &[4, 19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29]);
    }

    #[test]
    fn test_generate_text_simple_with_padding() {
        let mut codewords = [0u16; 6];
        encode_text("Test", &mut codewords).unwrap();
        assert_eq!(&codewords, &[4, 19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29, 900, 900]);
    }

    #[test]
    fn test_generate_test_switch_modes() {
        let mut codewords = [0u16; 7];
        encode_text("abc1D234", &mut codewords).unwrap();
        assert_eq!(&codewords, &[7, 27 * 30 + 0, 1 * 30 + 2, 28 * 30 + 1, 28 * 30 + 3, 28 * 30 + 2, 3 * 30 + 4]);
    }

    #[test]
    fn test_generate_test_numeric() {
        let mut codewords = [0u16; 12];
        encode_text("12345678987654321 num", &mut codewords).unwrap();
        assert_eq!(&codewords, &[12, 902, 190, 232, 499, 20, 504, 721, 900, 26 * 30 + 27, 13 * 30 + 20, 12 * 30 + 29]);
    }

    #[test]
    fn test_generate_test_numeric_big() {
        let mut codewords = [0u16; 20];
        //           [                        p1                 ][ p2 ]
        encode_text("123456789876543211234567898765432112345678987654321", &mut codewords).unwrap();
        assert_eq!(&codewords, &[20, 902, 491, 81, 137, 725, 651, 455, 511, 858, 135, 138, 488, 568, 447, 553, 198, /* next */ 21, 715, 821]);
    }

    #[test]
    fn test_generate_test_text_with_digits() {
        let mut codewords = [0u16; 17];
        encode_text("encoded 0123456789 as digits", &mut codewords).unwrap();
        assert_eq!(&codewords, &[17, 27 * 30 + 4, 13 * 30 + 2, 14 * 30 + 3, 4 * 30 + 3, 26 * 30 + 28, 0 * 30 + 1, 2 * 30 + 3, 4 * 30 + 5, 6 * 30 + 7, 8 * 30 + 9,
            26 * 30 + 27, 0 * 30 + 18, 26 * 30 + 3, 8 * 30 + 6, 8 * 30 + 19, 18 * 30 + 29]);
    }

    /*
    #[test]
    fn test_generate_test_shift() {
        let mut codewords = [0u16; 7];
        encode_text("This! Is a quote.", &mut codewords).unwrap();
        assert_eq!(&codewords, &[7, 27 * 30 + 0, 1 * 30 + 2, 28 * 30 + 1, 27 * 30 + 3, 28 * 30 + 2, 3 * 30 + 4]);
    }*/
}
