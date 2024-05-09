use pdf417::*;

const WHITE: &str = "\x1B[38;2;255;255;255m█";
const BLACK: &str = "\x1B[38;2;0;0;0m█";

const PADDING: usize = 4;
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

    let mut storage = [0u8; ((W - 1) / 8 + 1) * H];
    let pdf417 = PDF417::new(&input, ROWS, COLS, level)
        .render()
        .set_scale(SCALE);
    pdf417.render_bitmap(&mut storage[..]);

    let mut col = 0;
    for _ in 0..((PADDING+1)/2) {
        println!("{}", str::repeat(WHITE, W + PADDING * 2));
    }
    print!("{}", str::repeat(WHITE, PADDING));
    for bits in storage {
        for k in (0..8).rev() {
            if col + 7 - k == W { break; }
            print!("{}", if (bits & (1 << k)) != 0 { BLACK } else { WHITE });
        }
        col += 8;
        if col >= W {
            col = 0;
            print!("{b}\n{b}", b = str::repeat(WHITE, PADDING));
        }
    }
    println!("{}", str::repeat(WHITE, W + PADDING));
    for _ in 0..((PADDING-1)/2) {
        println!("{}", str::repeat(WHITE, W + PADDING * 2));
    }
    println!("\x1B[0m");
}
