use pdf417::*;

const WHITE: &str = "\x1B[38;2;255;255;255m█";
const BLACK: &str = "\x1B[38;2;0;0;0m█";

const COLS: u32 = 4;
const ROWS: u32 = 6;
const LEVEL: u8 = 1;
const SCALE: (u32, u32) = (1, 1);

const W: usize = pdf417_width!(COLS, SCALE.0) as usize;
const H: usize = pdf417_height!(ROWS, SCALE.1) as usize;

fn main() {
    const S: &str = "Hello from no-std >> rust << !";
    let mut input = [0u16; (COLS*ROWS) as usize];
    let data_words = generate_ascii(S, &mut input, LEVEL);
    println!("{data_words}/{}", input.len());

    let mut storage = [false; W * H];
    let pdf417 = PDF417::scaled(&input, ROWS, COLS, LEVEL, SCALE);
    pdf417.render(&mut storage[..]);

    const PADDING: usize = 4;
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
