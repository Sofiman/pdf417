use pdf417::*;

const WHITE: &str = "\x1B[38;2;255;255;255m█";
const BLACK: &str = "\x1B[38;2;0;0;0m█";

const PADDING: usize = 4;
const COLS: u8 = 4;
const ROWS: u8 = 7;

const W: usize = pdf417_width!(COLS);
const H: usize = pdf417_height!(ROWS);

fn main() {
    let mut input = [0u16; (COLS*ROWS) as usize];
    let (level, _) = PDF417Encoder::new(&mut input, false)
        .append_ascii("Hello, world from no-std *rust* !")
        .fit_seal().unwrap();

    let mut storage = [false; W * H];
    let pdf417 = PDF417::new(&input, ROWS, COLS, level);
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
