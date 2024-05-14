use pdf417::*;

mod utils;

const COLS: u8 = 4;
const ROWS: u8 = 7;

const W: usize = pdf417_width!(COLS);

fn main() {
    let mut input = [0u16; (COLS*ROWS) as usize];
    let (level, _) = PDF417Encoder::new(&mut input, false)
        .append_ascii("Hello, world from no-std *rust* !")
        .fit_seal().unwrap();

    let pdf417 = PDF417::new(&input, ROWS, COLS, level);
    let bits: Vec<bool> = pdf417.bits().collect();

    utils::display_bits(W, &bits);
}
