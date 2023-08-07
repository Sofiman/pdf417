use pdf417::*;

const WHITE: &str = "\x1B[38;2;255;255;255m█";
const BLACK: &str = "\x1B[38;2;0;0;0m█";

const PADDING: usize = 4;
const COLS: usize = 4;
const ROWS: usize = 6;
const ROW_SIZE: usize = (COLS + 2) * 17 + START_PATTERN_LEN as usize + END_PATTERN_LEN as usize;
const LEVEL: u8 = 1;

fn main() {
    const S: &str = "💛ワンピース";
    let mut input = [0u16; COLS*ROWS];
    let data_words = generate_text(S, &mut input, LEVEL);
    println!("{data_words}/{}", input.len());

    let mut storage = [false; ROW_SIZE * ROWS];
    let pdf417 = PDF417::new(&input, ROWS as u32, COLS as u32, LEVEL, false);
    pdf417.render(&mut storage[..]);

    let mut col = 0;
    for _ in 0..((PADDING+1)/2) {
        println!("{}", str::repeat(WHITE, ROW_SIZE + PADDING * 2));
    }
    print!("{}", str::repeat(WHITE, PADDING));
    for on in storage {
        print!("{}", if on { BLACK } else { WHITE });
        col += 1;
        if col == ROW_SIZE {
            col = 0;
            print!("{b}\n{b}", b = str::repeat(WHITE, PADDING));
        }
    }
    println!("{}", str::repeat(WHITE, ROW_SIZE + PADDING));
    for _ in 0..((PADDING-1)/2) {
        println!("{}", str::repeat(WHITE, ROW_SIZE + PADDING * 2));
    }
    println!("\x1B[0m");
}
