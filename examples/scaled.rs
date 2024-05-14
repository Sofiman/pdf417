use pdf417::*;

mod utils;

const COLS: u8 = 2;
const ROWS: u8 = 5;
const SCALE: (u16, u16) = (1, 3);

const W: usize = pdf417_width!(COLS, SCALE.0);
const H: usize = pdf417_height!(ROWS, SCALE.1);

fn main() {
    let mut input = [0u16; (COLS*ROWS) as usize];
    let (level, _) = PDF417Encoder::new(&mut input, false)
        .append_ascii("Test")
        .fit_seal().unwrap();

    let mut storage = [0u8; (W + 7) / 8 * H];
    let pdf417 = PDF417::new(&input, ROWS, COLS, level)
        .render()
        .set_scale(SCALE);
    pdf417.fill_bitmap(&mut storage[..]);

    utils::display_bitmap(W, &storage);
}
