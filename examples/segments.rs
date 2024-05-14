use pdf417::*;

mod utils;

const COLS: u8 = 4;
const ROWS: u8 = 6;

const W: usize = pdf417_width!(COLS);
const H: usize = pdf417_height!(ROWS);

fn main() {
    let mut input = [0u16; (COLS*ROWS) as usize];
    let (level, _) = PDF417Encoder::new(&mut input, false)
        .append_ascii("AsciiSegment ")
        .append_num(42)
        .append_bytes(b" ByteSegment")
        .fit_seal().unwrap();

    let mut storage = [false; W * H];
    let pdf417 = PDF417::new(&input, ROWS, COLS, level).render();
    pdf417.fill_bits(&mut storage[..]);

    utils::display_bits(W, &storage);
}
