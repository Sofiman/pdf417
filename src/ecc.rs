//! ECC generation for PDF417

use crate::tables::*;

/// Returns the number of ECC codewords required by the specified level (0-8)
/// of a regular PDF417.
///
/// | ECC Level | Number of ECC codewords |
/// |-----------|-------------------------|
/// | 0         | 2                       |
/// | 1         | 4                       |
/// | 2         | 8                       |
/// | 3         | 16                      |
/// | 4         | 32                      |
/// | 5         | 64                      |
/// | 6         | 128                     |
/// | 7         | 256                     |
/// | 8         | 512                     |
pub const fn ecc_count(level: u8) -> usize {
    assert!(level < 9, "ECC level must be between 0 and 8 inclusive");
    1 << (level as usize + 1)
}

/// Calculate and stores the ECC codewords in the slice `codewords` in-place.
/// The last **N** codewords are overwritten by the ECC codewords where **N**
/// is the number of ECC codewords to insert according to the specified level.
/// The ECC is calculated for the (total-N) first codewords where total is the
/// length of the codewords slice.
pub fn generate_ecc(codewords: &mut [u16], level: u8) {
    assert!(level <= 8, "ECC level must be between 0 and 8 inclusive");
    let factors: &'static [u16] = match level {
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

    assert!(codewords.len() >= factors.len(), "ECC codewords could not fit in buffer");
    generate_ecc_codewords(factors, codewords);
}

/// Calculate and stores the ECC codewords in the slice `codewords` in-place.
/// The last **N** codewords are overwritten by the ECC codewords where **N**
/// is the number of ECC codewords to insert according to the k offset
/// (depending on the variant of the MicroPDF417). The ECC is calculated for the
/// (total-N) first codewords where total is the length of the codewords slice.
pub fn generate_micro_ecc(codewords: &mut [u16], count: usize, k: usize) {
    assert!(count > 0, "count cannot be empty");
    assert!(codewords.len() >= count);
    generate_ecc_codewords(&ECC_MICRO[k..(k+count)], codewords);
}

fn generate_ecc_codewords(factors: &'static [u16], codewords: &mut [u16]) {
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
    use super::{generate_ecc, ecc_count};

    const INPUT_DATA: [u16; 16] = [16, 902, 1, 278, 827, 900, 295, 902, 2, 326, 823, 544, 900, 149, 900, 900];

    #[test]
    fn test_ecc_l0() {
        const EXPECTED: [u16; ecc_count(0)] = [156, 765];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 0);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }

    #[test]
    fn test_ecc_l1() {
        const EXPECTED: [u16; ecc_count(1)] = [168, 875, 63, 355];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 1);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }

    #[test]
    fn test_ecc_l2() {
        const EXPECTED: [u16; ecc_count(2)] = [628, 715, 393, 299, 863, 601, 169, 708];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 2);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }

    #[test]
    fn test_ecc_l3() {
        const EXPECTED: [u16; ecc_count(3)] = [232, 176, 793, 616, 476, 406, 855, 445, 84, 518, 522, 721, 607, 2, 42, 578];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 3);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }

    #[test]
    fn test_ecc_l4() {
        const EXPECTED: [u16; ecc_count(4)] = [281, 156, 276, 668, 44, 252, 877, 30, 549, 856, 773, 639, 420, 330, 693, 329, 283, 723, 480, 482, 102, 925, 535, 892, 374, 472, 837, 331, 343, 608, 390, 364];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 4);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }

    #[test]
    fn test_ecc_l5() {
        const EXPECTED: [u16; ecc_count(5)] = [31, 850, 18, 870, 53, 477, 837, 130, 533, 186, 266, 450, 39, 492, 542, 653, 499, 887, 618, 103, 364, 313, 906, 396, 270, 735, 593, 81, 557, 712, 810, 48, 167, 533, 205, 577, 503, 126, 449, 189, 859, 471, 493, 849, 554, 76, 878, 893, 168, 497, 251, 704, 311, 650, 283, 268, 462, 223, 659, 763, 176, 34, 544, 304];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 5);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }

    #[test]
    fn test_ecc_l6() {
        const EXPECTED: [u16; ecc_count(6)] = [345, 775, 909, 489, 650, 568, 869, 577, 574, 349, 885, 317, 492, 222, 783, 451, 647, 385, 168, 366, 118, 655, 643, 551, 179, 880, 880, 752, 132, 206, 765, 862, 727, 240, 32, 266, 911, 287, 813, 437, 868, 201, 681, 867, 567, 398, 508, 564, 504, 676, 785, 554, 831, 566, 424, 93, 515, 275, 61, 544, 272, 621, 374, 922, 779, 663, 789, 295, 631, 536, 755, 465, 485, 416, 76, 412, 76, 431, 28, 614, 767, 419, 600, 779, 94, 584, 647, 846, 121, 97, 790, 205, 424, 793, 263, 271, 694, 522, 437, 817, 382, 164, 113, 849, 178, 602, 554, 261, 415, 737, 401, 675, 203, 271, 649, 120, 765, 209, 522, 687, 420, 32, 60, 266, 270, 228, 304, 270];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 6);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }

    #[test]
    fn test_ecc_l7() {
        const EXPECTED: [u16; ecc_count(7)] = [142, 203, 799, 4, 105, 137, 793, 914, 225, 636, 60, 171, 490, 180, 414, 141, 399, 599, 829, 288, 108, 268, 444, 481, 795, 146, 655, 778, 189, 32, 597, 206, 208, 711, 845, 608, 642, 636, 540, 795, 845, 466, 492, 659, 138, 800, 912, 171, 92, 438, 225, 301, 777, 449, 230, 448, 326, 182, 892, 681, 543, 582, 732, 758, 162, 587, 685, 378, 646, 356, 354, 25, 839, 839, 556, 253, 501, 771, 745, 616, 473, 293, 669, 822, 613, 684, 229, 265, 110, 438, 144, 727, 317, 605, 414, 497, 82, 278, 267, 323, 43, 894, 624, 282, 790, 579, 430, 255, 802, 553, 922, 604, 68, 692, 809, 909, 663, 589, 735, 670, 298, 158, 201, 68, 124, 64, 67, 338, 694, 373, 225, 579, 309, 699, 920, 432, 717, 72, 126, 819, 142, 755, 473, 630, 331, 758, 730, 65, 359, 451, 236, 16, 56, 31, 87, 587, 125, 385, 384, 197, 352, 383, 173, 271, 38, 558, 810, 260, 521, 680, 7, 319, 650, 334, 695, 708, 0, 562, 365, 204, 114, 185, 560, 746, 767, 449, 797, 688, 63, 135, 818, 805, 3, 536, 908, 532, 400, 698, 49, 212, 630, 93, 157, 275, 3, 20, 611, 179, 302, 282, 876, 665, 241, 206, 474, 80, 217, 460, 462, 751, 719, 571, 536, 794, 522, 385, 598, 756, 162, 212, 758, 662, 361, 223, 587, 857, 503, 382, 615, 86, 283, 541, 847, 518, 406, 736, 486, 408, 226, 342, 784, 772, 211, 888, 234, 335];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 7);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }

    #[test]
    fn test_ecc_l8() {
        const EXPECTED: [u16; ecc_count(8)] = [538, 446, 840, 510, 163, 708, 177, 666, 423, 600, 707, 913, 770, 571, 156, 683, 676, 697, 898, 776, 128, 851, 163, 854, 135, 661, 880, 279, 92, 324, 397, 207, 379, 223, 574, 9, 70, 858, 878, 579, 61, 551, 261, 388, 315, 856, 266, 865, 923, 38, 313, 62, 381, 198, 265, 256, 385, 878, 347, 532, 821, 53, 855, 225, 697, 826, 263, 334, 207, 565, 460, 496, 705, 599, 383, 289, 178, 168, 401, 268, 555, 190, 922, 284, 180, 810, 891, 832, 636, 813, 894, 495, 701, 484, 204, 793, 129, 164, 444, 228, 636, 98, 809, 57, 736, 697, 727, 534, 889, 480, 898, 773, 234, 851, 880, 843, 714, 443, 412, 489, 578, 468, 367, 663, 11, 686, 319, 352, 345, 670, 106, 106, 219, 466, 439, 350, 538, 66, 852, 175, 465, 731, 332, 110, 926, 491, 18, 422, 736, 797, 624, 376, 728, 526, 735, 200, 502, 923, 789, 529, 923, 706, 384, 869, 172, 548, 520, 463, 813, 384, 793, 231, 190, 653, 864, 351, 400, 525, 487, 828, 654, 307, 141, 638, 770, 775, 282, 54, 758, 197, 492, 320, 86, 790, 275, 237, 923, 25, 591, 605, 61, 824, 79, 631, 532, 337, 867, 423, 340, 597, 682, 923, 287, 408, 503, 361, 881, 196, 468, 759, 746, 389, 124, 784, 198, 865, 538, 451, 178, 772, 653, 121, 497, 598, 711, 716, 241, 159, 429, 88, 799, 761, 639, 105, 54, 807, 351, 435, 793, 873, 360, 8, 881, 479, 693, 576, 849, 875, 771, 621, 134, 863, 8, 171, 799, 924, 103, 63, 491, 538, 597, 855, 697, 499, 7, 886, 286, 85, 107, 220, 319, 124, 197, 150, 729, 899, 585, 540, 676, 414, 256, 856, 596, 259, 882, 436, 26, 273, 753, 127, 679, 390, 654, 42, 276, 420, 247, 629, 116, 803, 131, 25, 403, 645, 462, 897, 151, 622, 108, 167, 227, 831, 887, 662, 739, 263, 829, 56, 624, 317, 908, 378, 39, 393, 861, 338, 202, 179, 907, 109, 360, 736, 554, 342, 594, 125, 433, 394, 195, 698, 844, 912, 530, 842, 337, 294, 528, 231, 735, 93, 8, 579, 42, 148, 609, 233, 782, 887, 888, 915, 620, 78, 137, 161, 282, 217, 775, 564, 33, 195, 36, 584, 679, 775, 476, 309, 230, 303, 708, 143, 679, 502, 814, 193, 508, 532, 542, 580, 603, 641, 338, 361, 542, 537, 810, 394, 764, 136, 167, 611, 881, 775, 267, 433, 142, 202, 828, 363, 101, 728, 660, 583, 483, 786, 717, 190, 809, 422, 567, 741, 695, 310, 120, 177, 47, 494, 345, 508, 16, 639, 402, 625, 286, 298, 358, 54, 705, 916, 291, 424, 375, 883, 655, 675, 498, 498, 884, 862, 365, 310, 805, 763, 855, 354, 777, 543, 53, 773, 120, 408, 234, 728, 438, 914, 3, 670, 546, 465, 449, 923, 51, 546, 709, 648, 96, 320, 682, 326, 848, 234, 855, 791, 20, 97, 901, 351, 317, 764, 767, 312, 206, 139, 610, 578, 646, 264, 389, 238, 675, 595, 430, 88];
        let mut data = [0u16; INPUT_DATA.len() + EXPECTED.len()];
        data[..INPUT_DATA.len()].copy_from_slice(&INPUT_DATA);
        generate_ecc(&mut data, 8);
        assert_eq!(data[INPUT_DATA.len()..], EXPECTED);
    }
}
