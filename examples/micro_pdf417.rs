use pdf417::*;

mod utils;

const COLS: u8 = 4;
const ROWS: u8 = 4;
const V: Option<Variant> = Variant::with_dimensions(ROWS, COLS);

const W: usize = m_pdf417_width!(COLS);
const H: usize = m_pdf417_height!(ROWS);

fn main() {
    let variant = V.unwrap();
    let mut input = [0u16; (COLS*ROWS) as usize];
    PDF417Encoder::new(&mut input, true).append_num(12345678)
        .seal(variant.into());

    let mut storage = [false; W * H];
    let pdf417 = MicroPDF417::from_variant(&input, variant).render();
    pdf417.fill_bits(&mut storage[..]);

    utils::display_bits(W, &storage);
}
