#![no_std]
#![allow(dead_code)]

mod tables;
pub mod ecc;

use tables::HL_TO_LL;

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

#[derive(Debug, Clone)]
pub struct PDF417<'a> {
    codewords: &'a [u16],
    rows: usize,
    cols: usize,
    level: u8
}

impl<'a> PDF417<'a> {
    pub fn new(codewords: &'a [u16], rows: usize, cols: usize, level: u8) -> Self {
        assert!(level < 9, "ECC level must be between 0 and 8 inclusive");
        assert!(codewords.len() == rows*cols,
            "codewords will not fit in a {rows}x{cols} configuration");

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
