use pdf417::*;

mod utils;

const COLS: u8 = 4;
const ROWS: u8 = 7;

const W: usize = pdf417_width!(COLS);
const H: usize = pdf417_height!(ROWS);

fn main() {
    let mut input = [0u16; (COLS*ROWS) as usize];

    // \x1d - GS, Group Separator, ASCII Code 29 (Hex 1D)
    // \x1e - RS, Record Separator, ASCII Code 30 (Hex 1E)
    // \x04 - EOT, End of Transmission, ASCII Code 04 (Hex 04)
    let (level, _) = PDF417Encoder::new(&mut input, false)
        .append_ascii("\x1e06\x1d66831000\x1d9117327\x1e\x04")
        .fit_seal().unwrap();

    let mut storage = [false; W * H];
    let pdf417 = PDF417::new(&input, ROWS, COLS, level).render();
    pdf417.fill_bits(&mut storage[..]);

    utils::display_bits(W, &storage);
}
