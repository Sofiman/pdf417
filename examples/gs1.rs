use pdf417::*;

const WHITE: &str = "\x1B[38;2;255;255;255m█";
const BLACK: &str = "\x1B[38;2;0;0;0m█";

const PADDING: usize = 4;
const COLS: u8 = 4;
const ROWS: u8 = 7;
const LEVEL: u8 = 1;

const W: usize = pdf417_width!(COLS);
const H: usize = pdf417_height!(ROWS);

fn main() {
    let mut input = [0u16; (COLS*ROWS) as usize];

    // \x1d - GS, Group Separator, ASCII Code 29 (Hex 1D)
    // \x1e - RS, Record Separator, ASCII Code 30 (Hex 1E)
    // \x04 - EOT, End of Transmission, ASCII Code 04 (Hex 04)
    PDF417Encoder::new(&mut input)
        .append_ascii("\x1e06\x1d66831000\x1d9117327\x1e\x04")
        .seal(LEVEL);

    let mut storage = [false; W * H];
    let pdf417 = PDF417::new(&input, ROWS, COLS, LEVEL);
    pdf417.render(&mut storage[..]);

    let mut col = 0;
    for _ in 0..((PADDING+1)/2) {
        println!("{}", str::repeat(WHITE, W + PADDING * 2));
    }
    print!("{}", str::repeat(WHITE, PADDING));
    for on in storage {
        print!("{}", if on { BLACK } else { WHITE });
        col += 1;
        if col == W {
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
