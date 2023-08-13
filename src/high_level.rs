//! User data to high level encoding conversion functions

use crate::ecc;

use awint_core::{InlAwi, Bits};
type U160 = awint_macros::inlawi_ty!(160);

const MIXED_CHAR_SET: [u8; 15] = [
    b'&', b'\r', b'\t', b',', b':', b'#', b'-', b'.', b'$', b'/', b'+', b'%', b'*', b'=', b'^'
];
const PUNC_CHAR_SET: [u8; 29] = [
    b';', b'<', b'>', b'@', b'[', b'\\', b']', b'_', b'`', b'~', b'!', b'\r', b'\t',
    b',', b':', b'\n', b'-', b'.', b'$', b'/', b'"', b'|', b'*', b'(', b')', b'?',
    b'{', b'}', b'\''
];

/// Encode a 64-bit unsigned integer `n` to the `out` codewords slice for later
/// rendering to a PDF417. This function returns the number of codewords
/// written. Please note that the remaining codeword slots are filled with the
/// padding codeword (900).
pub fn encode_num(mut n: u64, out: &mut [u16]) -> usize {
    out[0] = 902;

    let mut digits = 0;

    // Append a leading 1 to the number to do the base 900
    // conversion. We need to calculate and add 10^(digits).
    // Power of 10 (see https://stackoverflow.com/a/44103598)
    {
        let mut val = n;
        let mut p1 = 1;

        while val > 0 {
            p1 += p1 << 2; // *5
            val /= 10;
            digits += 1;
        }
        p1 <<= digits;
        n += p1;
    }

    let nb = digits / 3 + 1;
    let mut count = 0;

    while n > 0 {
        let (q, r) = (n / 900, n % 900);
        n = q;
        out[1 + nb - count - 1] = r as u16;
        count += 1;
    }

    out[(1 + nb)..].fill(900); // padding

    1 + nb
}

/// Generates the required codewords to store an __UTF-8__ string `s` to the `out`
/// codewords slice ready to be rendered to a PDF417. __Note that the conversion
/// is space inefficient, is the string is composed of ASCII characters, please
/// consider using [generate_ascii] instead.__ Generated codewords include
/// metadata codewords such as ECC codewords, which count is determined by
/// `level` (0 to 8). The free codewords slots are filled by padding codeword
/// (900). This function returns the total number of codewords written to the
/// `out` slice excluding padding.
pub fn generate_text(s: &str, out: &mut [u16], level: u8) -> usize {
    // 6 bytes = 5 codewords; +1 for length indicator + 1 for byte mode +2 for ECI mode
    let ecc_cw = ecc::ecc_count(level);
    let min = (s.len()/6)*5 + (s.len() % 6) + ecc_cw + 1 + 1 + 2;
    assert!(out.len() >= min, "output buffer not large enough to fit {min} codewords");

    // metadata
    let data_end = out.len() - ecc_cw;
    out[0] = data_end as u16;
    out[1] = 927; // ECI identifier for code page
    out[2] = 26; // UTF-8 is \000026

    let data_words = encode_bytes(s.as_bytes(), &mut out[3..data_end]);
    ecc::generate_ecc(out, level);
    return data_words + ecc_cw + 3;
}

/// Encodes the `bytes` byte slice to the `out` codewords slice for later
/// rendering to a PDF417. This function returns the number of codewords
/// written. Please note that the remaining codeword slots are filled with the
/// padding codeword (900).
pub fn encode_bytes(bytes: &[u8], out: &mut [u16]) -> usize {
    let mut i = 0;
    let mut k = 0;

    if bytes.len() > 1 {
        // latch to byte mode
        out[i] = if bytes.len() % 6 == 0 { 924 } else { 901 };
        i += 1;

        while bytes.len()-k >= 6 {
            // pack six bytes
            let mut s: u64 = 0;
            for n in 0..6 {
                s = (s << 8) + bytes[k + n] as u64;
            }
            // append five codewords
            for n in 0..5 {
                let (q, r) = (s / 900, s % 900);
                out[i + 4 - n] = r as u16;
                s = q;
            }

            i += 5;
            k += 6;
        }
    } else {
        out[i] = 913; // shift to byte mode (only for next codeword)
        i += 1;
    }

    // remaining
    while k < bytes.len() {
        out[i] = bytes[k] as u16;
        k += 1;
        i += 1;
    }

    out[i..].fill(900); // padding

    return i;
}

/// Generates the required codewords to store the __ASCII__ string `s` to the
/// `out` codewords slice ready to be rendered to a PDF417. Generated codewords
/// include metadata codewords such as ECC codewords, which count is determined
/// by `level` (0 to 8). The free codewords slots are filled by padding codeword
/// (900). This function returns the total number of codewords written to the
/// `out` slice excluding padding.
pub fn generate_ascii(s: &str, out: &mut [u16], level: u8) -> usize {
    // 2 char = 1 codeword; +1 for length indicator +4 for mode switches
    // TODO: Relax with the fixed +4 in required capacity
    let ecc_cw = ecc::ecc_count(level);
    assert!(out.len() >= s.len()/2 + ecc_cw + 1 + 4, "output buffer not large enough");

    // metadata
    let data_end = out.len() - ecc_cw;
    out[0] = data_end as u16;

    let data_words = encode_ascii(s, &mut out[1..data_end]);
    ecc::generate_ecc(out, level);
    return data_words + ecc_cw + 1;
}

macro_rules! push {
    ($cws:ident, $i:ident, $rh:ident, $($cw:expr),+; $post:ident = $new:expr) => {{
        push!($cws, $i, $rh, $($cw),+);
        $post = $new;
    }};
    ($cws:ident, $i:ident, $rh:ident, $head:expr, $($cw:expr),+) => {
        push!($cws, $i, $rh, $head);
        push!($cws, $i, $rh, $($cw),+);
    };
    ($cws:ident, $i:ident, $rh:ident, $cw:expr) => {{
        let cw = $cw as u16;
        if cw > 29 {
            if $rh {
                $cws[$i] = $cws[$i] * 30 + 29;
                $rh = false;
                $i += 1;
            }

            $cws[$i] = cw;
            $i += 1;
        } else {
            if $rh {
                $cws[$i] = $cws[$i] * 30 + cw;
                $rh = false;
                $i += 1;
            } else {
                $cws[$i] = cw;
                $rh = true;
            }
        }
    }}
}

/// Encode the ASCII string to the `out` codewords slice for later rendering to
/// a PDF417. *Warning*: This function uses the PDF417 table based encoding to
/// reduce the size of the text. If you want to encode an UTF-8 string, use
/// [generate_text] instead.
///
/// This function returns the number of codewords written. Please note that the
/// remaining codeword slots are filled with the padding codeword (900).
pub fn encode_ascii(s: &str, out: &mut [u16]) -> usize {
    debug_assert!(s.is_ascii());
    let s = s.as_bytes();

    let mut mode: u8 = 0; // 0: Upper, 1: Lower, 2: Mixed, 3: Punc, 4: Numeric
    let mut i = 0;
    let mut k = 0;
    let mut right = false; // false = upper 8 bits | true = lower 8 bits

    while k < s.len() {
        let c = s[k];
        match c {
            b'A'..=b'Z' => {
                match mode {
                    0 => (),
                    1 => if k + 1 < s.len() && ((b'a'..=b'z').contains(&s[k + 1]) || s[k + 1] == b' ') {
                        push!(out, i, right, 27);
                    } else {
                        push!(out, i, right, 29, 29; mode = 0);
                    },
                    2 => push!(out, i, right, 28; mode = 0),
                    3 => push!(out, i, right, 29; mode = 0),
                    _ => unreachable!("Unknown mode {mode}"),
                }
                push!(out, i, right, c - b'A'; k = k + 1);
            },
            b'a'..=b'z' => {
                match mode {
                    0 | 2 => push!(out, i, right, 27; mode = 1),
                    1 => (),
                    3 => push!(out, i, right, 29, 27; mode = 1),
                    _ => unreachable!("Unknown mode {mode}"),
                }
                push!(out, i, right, c - b'a'; k = k + 1);
            },
            b'0'..=b'9' if mode == 2 => push!(out, i, right, c - b'0'; k = k + 1),
            b'0'..=b'9' => {
                let mut end = k + 1;
                while end < s.len() && end-k < 44 && (b'0'..=b'9').contains(&s[end]) {
                    end += 1;
                }
                let digits = end - k;

                if digits <= 13 && mode != 4 {
                    match mode {
                        0 | 1 => push!(out, i, right, 28; mode = 2),
                        2 => (),
                        3 => push!(out, i, right, 29, 28; mode = 2),
                        _ => unreachable!("Unknown mode {mode}"),
                    }
                    while k < end {
                        push!(out, i, right, s[k] - b'0'; k = k + 1);
                    }
                } else {
                    if mode != 4 { push!(out, i, right, 902; mode = 4); }

                    let mut b = U160::zero();
                    {
                        let mut p0 = U160::zero();
                        let mut p1 = U160::zero();
                        b.bytes_radix_(None, &s[k..end], 10, &mut p0, &mut p1)
                            .expect("45 digits base 10 should fit in 160 bits");

                        // Append a leading 1 to the number to do the base 900
                        // conversion. We need to calculate and add 10^(digits).
                        // Power of 10 (see https://stackoverflow.com/a/44103598)
                        p1.uone_();
                        p1.shl_(digits).unwrap();
                        for _ in 0..digits {
                            p0.copy_(&p1).unwrap();
                            p0.shl_(2).unwrap();
                            p1.add_(&p0).unwrap();
                        }
                        b.add_(&p1).unwrap();
                    }
                    let nb = digits / 3 + 1;
                    let mut count = 0;

                    while !b.is_zero() {
                        let r = b.digit_udivide_inplace_(900).expect("900 > 0");
                        out[i + nb - count - 1] = r as u16;
                        count += 1;
                    }

                    k = end;
                    i += nb;
                }

                if mode == 4 && k < s.len() && !(b'0'..=b'9').contains(&s[k]) {
                    push!(out, i, right, 900; mode = 0);
                }
            },
            b' ' => {
                if mode == 3 { push!(out, i, right, 29; mode = 0) };
                push!(out, i, right, 26; k = k + 1);
            },
            c => {
                if let Some(p) = MIXED_CHAR_SET.iter().position(|&r| r == c) {
                    match mode {
                        0 | 1 => push!(out, i, right, 28; mode = 2),
                        2 => (),
                        /* no switch if the char is also present in the punc table */
                        3 if (p >= 1 && p <= 4) || (p >= 6 && p <= 9)  => (),
                        3 => push!(out, i, right, 29, 28; mode = 2),
                        _ => unreachable!("Unknown mode {mode}"),
                    }
                    push!(out, i, right, p + 10);
                } else if let Some(p) = PUNC_CHAR_SET.iter().position(|&r| r == c) {
                    if mode != 3 {
                        let mut end = k + 1;
                        while end < s.len() && end-k < 3 && PUNC_CHAR_SET.contains(&s[end]) {
                            end += 1;
                        }
                        if end-k >= 3 { // latch
                            if mode != 2 { push!(out, i, right, 28); }
                            push!(out, i, right, 25; mode = 3);
                        } else { // shift
                            push!(out, i, right, 29);
                        }
                    }
                    push!(out, i, right, p);
                } else { // switch to byte mode
                    if right {
                        out[i] = out[i] * 30 + 29;
                        i += 1;
                    }
                    // TODO: Encode multiple bytes if consecutive instead of one by one
                    i += encode_bytes(&s[k..(k+1)], &mut out[i..(i + 2)]);
                }
                k += 1;
            },
        };
    }

    if right { 
        out[i] = out[i] * 30 + 29;
        i += 1;
    }

    out[i..].fill(900); // padding
    return i;
}

#[cfg(test)]
mod tests {
    use super::{encode_ascii, encode_bytes, encode_num};

    #[test]
    fn test_encode_ascii_simple() {
        let mut codewords = [0u16; 3];
        encode_ascii("Test", &mut codewords);
        assert_eq!(&codewords, &[19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29]);
    }

    #[test]
    fn test_encode_ascii_simple_with_padding() {
        let mut codewords = [0u16; 5];
        encode_ascii("Test", &mut codewords);
        assert_eq!(&codewords, &[19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29, 900, 900]);
    }

    #[test]
    fn test_generate_ascii_switch_modes() {
        let mut codewords = [0u16; 8];
        encode_ascii("abc1D234\x1B", &mut codewords);
        assert_eq!(&codewords, &[27 * 30 + 0, 1 * 30 + 2, 28 * 30 + 1, 28 * 30 + 3, 28 * 30 + 2, 3 * 30 + 4, 913, 0x1B]);
    }

    #[test]
    fn test_generate_ascii_numeric() {
        let mut codewords = [0u16; 11];
        encode_ascii("12345678987654321 num", &mut codewords);
        assert_eq!(&codewords, &[902, 190, 232, 499, 20, 504, 721, 900, 26 * 30 + 27, 13 * 30 + 20, 12 * 30 + 29]);
    }

    #[test]
    fn test_generate_ascii_numeric_big() {
        let mut codewords = [0u16; 19];
        //           [                        p1                 ][ p2 ]
        encode_ascii("123456789876543211234567898765432112345678987654321", &mut codewords);
        assert_eq!(&codewords, &[902, 491, 81, 137, 725, 651, 455, 511, 858, 135, 138, 488, 568, 447, 553, 198, /* next */ 21, 715, 821]);
    }

    #[test]
    fn test_encode_num() {
        let mut codewords = [0u16; 7];
        encode_num(12345678987654321, &mut codewords);
        assert_eq!(&codewords, &[902, 190, 232, 499, 20, 504, 721]);
    }

    #[test]
    fn test_generate_ascii_with_digits() {
        let mut codewords = [0u16; 16];
        encode_ascii("encoded 0123456789 as digits", &mut codewords);
        assert_eq!(&codewords, &[27 * 30 + 4, 13 * 30 + 2, 14 * 30 + 3, 4 * 30 + 3, 26 * 30 + 28, 0 * 30 + 1, 2 * 30 + 3, 4 * 30 + 5, 6 * 30 + 7, 8 * 30 + 9,
            26 * 30 + 27, 0 * 30 + 18, 26 * 30 + 3, 8 * 30 + 6, 8 * 30 + 19, 18 * 30 + 29]);
    }

    #[test]
    fn test_generate_ascii_punc_mixed() {
        let mut codewords = [0u16; 17];
        encode_ascii("This! Is a `quote (100%)`.", &mut codewords);
        assert_eq!(&codewords, &[19 * 30 + 27, 7 * 30 + 8, 18 * 30 + 29, 10 * 30 + 26, 27 * 30 + 8, 18 * 30 + 26, 0 * 30 + 26, 29 * 30 + 8, 16 * 30 + 20, 14 * 30 + 19, 4 * 30 + 26, 29 * 30 + 23, 28 * 30 + 1, 0 * 30 + 0, 21 * 30 + 25, 24 * 30 + 8, 17 * 30 + 29]);
    }

    #[test]
    fn test_encode_bytes_multiple() {
        let mut codewords = [0u16; 6];
        encode_bytes(b"alcool", &mut codewords);
        assert_eq!(&codewords, &[924, 163, 238, 432, 766, 244]);
    }

    #[test]
    fn test_encode_bytes_not_multiple() {
        let mut codewords = [0u16; 10];
        encode_bytes(b"encode bin", &mut codewords);
        assert_eq!(&codewords, &[901, 169, 883, 224, 680, 517, 32, 98, 105, 110]);
    }
}
