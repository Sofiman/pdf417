use pdf417::*;

const WHITE: &str = "\x1B[38;2;255;255;255m█";
const BLACK: &str = "\x1B[38;2;0;0;0m█";

const PADDING: usize = 4;
const COLS: u8 = 3;
const ROWS: u8 = 4;
const LEVEL: u8 = 0;

const INPUT: [u16; 12] = [10, 900, 7 * 30 + 4, 11 * 30 + 11, 14 * 30 + 26, 22 * 30 + 14, 17 * 30 + 11, 3 * 30 + 29, 10 * 30 + 29, 900, 0, 0]; // HELLO WORLD!
//const INPUT: [u16; 6] = [4, 900, 7 * 30 + 7, 7 * 30 + 7, 0, 0]; // HELLO WORLD!
//const INPUT: [u16; 6] = [4, 900, 19 * 30 + 4, 18 * 30 + 19, 0, 0]; // TEST
//const INPUT: [u16; 20] = [16, 902, 1, 278, 827, 900, 295, 902, 2, 326, 823, 544, 900, 149, 900, 900, 0, 0, 0, 0];

const W: usize = pdf417_width!(COLS as usize);
const H: usize = pdf417_height!(ROWS as usize);

fn main() {
    let mut input = INPUT.clone();
    ecc::generate_ecc(&mut input, LEVEL);

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
