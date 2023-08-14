use pdf417::*;

const WHITE: &str = "\x1B[38;2;255;255;255m█";
const BLACK: &str = "\x1B[38;2;0;0;0m█";

const PADDING: usize = 4;
const COLS: u8 = 2;
const ROWS: u8 = 5;
const LEVEL: u8 = 0;
const SCALE: (u32, u32) = (2, 2);

const W: usize = pdf417_width!(COLS, SCALE.0);
const H: usize = pdf417_height!(ROWS, SCALE.1);

fn main() {
    const S: &str = "Test";
    let mut input = [0u16; (COLS*ROWS) as usize];
    let data_words = generate_ascii(S, &mut input, LEVEL);
    println!("{data_words}/{}", input.len());

    let mut storage = [0u8; (W * H) / 8 + ROWS as usize];
    let pdf417 = PDF417::new(&input, ROWS, COLS, LEVEL).scaled(SCALE);
    pdf417.render(&mut storage[..]);

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
