use crate::tables::*;

pub const fn ecc_count(level: u8) -> usize {
    assert!(level < 9, "ECC level must be between 0 and 8 inclusive");
    1 << (level as usize + 1)
}

pub fn generate_ecc(codewords: &mut [u16], level: u8) {
    assert!(level < 9, "ECC level must be between 0 and 8 inclusive");

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
    ecc.fill(0);

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

#[cfg(test)]
mod tests {
    use super::generate_ecc;

    const INPUT_DATA: [u16; 16] = [16, 902, 1, 278, 827, 900, 295, 902, 2, 326, 823, 544, 900, 149, 900, 900];

    #[test]
    fn test_ecc_l0() {
        let expected: [u16; 2] = [156, 765];
        let mut data = [0u16; INPUT_DATA.len() + 2];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 0);
        assert_eq!(data[INPUT_DATA.len()..], expected);
    }

    #[test]
    fn test_ecc_l1() {
        let expected: [u16; 4] = [168, 875, 63, 355];
        let mut data = [0u16; INPUT_DATA.len() + 4];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 1);
        assert_eq!(data[INPUT_DATA.len()..], expected);
    }

    #[test]
    fn test_ecc_l2() {
        let expected: [u16; 8] = [628, 715, 393, 299, 863, 601, 169, 708];
        let mut data = [0u16; INPUT_DATA.len() + 8];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 2);
        assert_eq!(data[INPUT_DATA.len()..], expected);
    }

    #[test]
    fn test_ecc_l3() {
        let expected: [u16; 16] = [232, 176, 793, 616, 476, 406, 855, 445, 84, 518, 522, 721, 607, 2, 42, 578];
        let mut data = [0u16; INPUT_DATA.len() + 16];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 3);
        assert_eq!(data[INPUT_DATA.len()..], expected);
    }

    #[test]
    fn test_ecc_l4() {
        let expected: [u16; 32] = [281, 156, 276, 668, 44, 252, 877, 30, 549, 856, 773, 639, 420, 330, 693, 329, 283, 723, 480, 482, 102, 925, 535, 892, 374, 472, 837, 331, 343, 608, 390, 364];
        let mut data = [0u16; INPUT_DATA.len() + 32];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 4);
        assert_eq!(data[INPUT_DATA.len()..], expected);
    }
}
