mod tables;

use tables::*;

const START: [bool; 17] = [true, true, true, true, true, true, true, true, false, true, false, true, false, true, false, false, false];
const END: [bool; 18] = [true, true, true, true, true, true, true, false, true, false, false, false, true, false, true, false, false, true];

macro_rules! append {
    ($sto:ident, $tb:ident, $i:ident, $bits:expr) => {
        let b = HL_TO_LL[$tb * 929 + $bits];
        $sto[$i] = true;
        $i += 1;
        let mut mask: u16 = (1 << 15);
        for _ in 0..16 {
            $sto[$i] = (b & mask) == mask;
            mask >>= 1;
            $i += 1;
        }
    }
}

pub fn generate_ecc(codewords: &mut [u16], level: u8) {
    let factors: &[u16] = match level {
        0 => &ECC_L0,
        1 => &ECC_L1,
        2 => &ECC_L2,
        3 => &ECC_L3,
        4 => &ECC_L4,
        5 => &ECC_L5,
        6 => &ECC_L6,
        7 => &ECC_L7,
        8 => &ECC_L8,
        _ => unreachable!()
    };

    assert!(codewords.len() >= factors.len());
    let (data, ecc) = codewords.split_at_mut(codewords.len() - factors.len());

    for cw in data {
        let t = (*cw + ecc[0]) % 929;

        for i in (0..factors.len()).rev() {
            let factor = ((t as usize * factors[i] as usize) % 929) as u16;
            let d = if i > 0 { ecc[factors.len() - i] } else { 0 };
            ecc[factors.len() - 1 - i] = (d + 929 - factor) % 929;
        }
    }

    for e in ecc {
        if *e != 0 {
            *e = 929 - *e;
        }
    }
}

pub fn generate_text(high_level: &[u16], rows: usize, cols: usize, level: u8, storage: &mut [bool]) {
    let mut table = 0;

    let mut i = 0;
    let mut col = 0;
    let mut row = 0;

    for &codeword in high_level {
        if col == 0 {
            // row start pattern
            storage[i..i+START.len()].copy_from_slice(&START);
            i += START.len();

            // row left pattern
            let cw = match table {
                0 => (rows - 1) / 3,
                1 => level as usize * 3 + (rows - 1) % 3,
                2 => cols - 1,
                _ => unreachable!()
            };
            append!(storage, table, i, (row / 3) * 30 + cw);
        }

        append!(storage, table, i, codeword as usize);
        col += 1;

        if col == cols {
            // row right codeword
            let cw = match table {
                0 => cols - 1,
                1 => (rows - 1) / 3,
                2 => level as usize * 3 + (rows - 1) % 3,
                _ => unreachable!()
            };
            append!(storage, table, i, (row / 3) * 30 + cw);

            // row end pattern
            storage[i..i+END.len()].copy_from_slice(&END);
            i += END.len();

            col = 0;
            row += 1;
            if table == 2 { table = 0; } else { table += 1 };
        }
    }
}

mod tests {
    use super::*;
    const WHITE: &str = "\x1B[38;2;255;255;255m█";
    const BLACK: &str = "\x1B[38;2;0;0;0m█";
    const INPUT: [u16; 12] = [10, 900, 7 * 30 + 4, 11 * 30 + 11, 14 * 30 + 26, 22 * 30 + 14, 17 * 30 + 11, 3 * 30 + 29, 10 * 30 + 29, 900, 0, 0]; // HELLO WORLD!
    //const INPUT: [u16; 6] = [4, 900, 7 * 30 + 7, 7 * 30 + 7, 0, 0]; // HELLO WORLD!
    //const INPUT: [u16; 6] = [4, 900, 19 * 30 + 4, 18 * 30 + 19, 0, 0]; // TEST
    //const INPUT: [u16; 20] = [16, 902, 1, 278, 827, 900, 295, 902, 2, 326, 823, 544, 900, 149, 900, 900, 0, 0, 0, 0];

    #[test]
    fn test_generate() {
        const PADDING: usize = 4;
        const COLS: usize = 3;
        const ROWS: usize = 4;
        const ROW_SIZE: usize = (COLS + 2) * 17 + START.len() + END.len();
        const LEVEL: u8 = 0;

        let mut input = INPUT.clone();
        generate_ecc(&mut input, LEVEL);
        let mut storage = [false; ROW_SIZE * ROWS];
        generate_text(&input, ROWS, COLS, LEVEL, &mut storage);


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
        todo!();
    }

    const inputData: [u16; 16] = [16, 902, 1, 278, 827, 900, 295, 902, 2, 326, 823, 544, 900, 149, 900, 900];

    #[test]
    fn test_ecc_l0() {
        let expected: [u16; 2] = [156, 765];
        let mut data = [0u16; inputData.len() + 2];
        data[..inputData.len()].copy_from_slice(&inputData);
        generate_ecc(&mut data, 0);
        assert_eq!(data[inputData.len()..], expected);
    }

    #[test]
    fn test_ecc_l1() {
        let expected: [u16; 4] = [168, 875, 63, 355];
        let mut data = [0u16; inputData.len() + 4];
        data[..inputData.len()].copy_from_slice(&inputData);
        generate_ecc(&mut data, 1);
        assert_eq!(data[inputData.len()..], expected);
    }

    #[test]
    fn test_ecc_l2() {
        let expected: [u16; 8] = [628, 715, 393, 299, 863, 601, 169, 708];
        let mut data = [0u16; inputData.len() + 8];
        data[..inputData.len()].copy_from_slice(&inputData);
        generate_ecc(&mut data, 2);
        assert_eq!(data[inputData.len()..], expected);
    }

    #[test]
    fn test_ecc_l3() {
        let expected: [u16; 16] = [232, 176, 793, 616, 476, 406, 855, 445, 84, 518, 522, 721, 607, 2, 42, 578];
        let mut data = [0u16; inputData.len() + 16];
        data[..inputData.len()].copy_from_slice(&inputData);
        generate_ecc(&mut data, 3);
        assert_eq!(data[inputData.len()..], expected);
    }

    #[test]
    fn test_ecc_l4() {
        let expected: [u16; 32] = [281, 156, 276, 668, 44, 252, 877, 30, 549, 856, 773, 639, 420, 330, 693, 329, 283, 723, 480, 482, 102, 925, 535, 892, 374, 472, 837, 331, 343, 608, 390, 364];
        let mut data = [0u16; inputData.len() + 32];
        data[..inputData.len()].copy_from_slice(&inputData);
        generate_ecc(&mut data, 4);
        assert_eq!(data[inputData.len()..], expected);
    }
}
