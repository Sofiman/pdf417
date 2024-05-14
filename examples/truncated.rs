use pdf417::*;

mod utils;

const COLS: u8 = 3;
const ROWS: u8 = 6;

const W: usize = pdf417_width!(COLS, 1, true);
const H: usize = pdf417_height!(ROWS, 1);

fn main() {
    let mut input = [0u16; (COLS*ROWS) as usize];
    let (level, _) = PDF417Encoder::new(&mut input, false)
        .append_ascii("Truncated PDF417")
        .fit_seal().unwrap();

    let mut storage = [false; W * H];
    let pdf417 = TruncatedPDF417::new(&input, ROWS, COLS, level).render();
    pdf417.fill_bits(&mut storage[..]);

    utils::display_bits(W, &storage);
}
