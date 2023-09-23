//! User data to high level encoding conversion functions

use crate::ecc;

use awint_core::{InlAwi, Bits};
type U160 = awint_macros::inlawi_ty!(160);

pub const M_LATCH_TEXT: u16 = 900;
pub const M_LATCH_BYTE: u16 = 901;
pub const M_LATCH_NUMERIC: u16 = 902;
// 903 to 912: reserved for future use
pub const M_SHIFT_BYTE: u16 = 913;
// 914 to 920: reserved for future use
// 921: used for reader initialization or programming (barcode used to
// enable/disable specific features of the reader).
// 922 to 923: Macro PDF4l7 
pub const M_LATCH_BYTE_M6: u16 = 924;
pub const ECI_CUSTOM_ID: u16 = 925;
pub const ECI_GENERAL_ID: u16 = 926;
pub const ECI_CODE_PAGE: u16 = 927;
// 928: Block start for PDF macro

pub const CW_PADDING: u16 = M_LATCH_TEXT;

const MIXED_CHAR_SET: [u8; 15] = [
    b'&', b'\r', b'\t', b',', b':', b'#', b'-', b'.', b'$', b'/', b'+', b'%', b'*', b'=', b'^'
];
const PUNC_CHAR_SET: [u8; 29] = [
    b';', b'<', b'>', b'@', b'[', b'\\', b']', b'_', b'`', b'~', b'!', b'\r', b'\t',
    b',', b':', b'\n', b'-', b'.', b'$', b'/', b'"', b'|', b'*', b'(', b')', b'?',
    b'{', b'}', b'\''
];

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
        if $rh {
            $cws[$i] = $cws[$i] * 30 + cw;
            $rh = false;
            $i += 1;
        } else {
            $cws[$i] = cw;
            $rh = true;
        }
    }};
}

macro_rules! push_sp {
    ($cws:ident, $i:ident, $rh:ident, $cw:expr; $post:ident = $new:expr) => {{
        let cw = $cw as u16;
        if $rh {
            $cws[$i] = $cws[$i] * 30 + 29;
            $rh = false;
            $i += 1;
        }

        $cws[$i] = cw;
        $i += 1;
        $post = $new;
    }};
}

/// Use a PDF417Encoder to encode your data segements to a slice of codewords
/// ready to be rendered.
#[derive(Debug)]
pub struct PDF417Encoder<'a> {
    storage: &'a mut [u16],
    used: usize
}

impl<'a> PDF417Encoder<'a> {
    /// Create a PDF417Encoder with sufficient storage for data segments.
    pub fn new(storage: &'a mut [u16]) -> Self {
        assert!(storage.len() > 0, "storage must be able to contain at least one codeword");
        Self { storage, used: 1 } // skip the first codeword (used for length)
    }

    /// Returns the number of codewords already used
    pub fn count(&self) -> usize {
        self.used
    }

    /// Returns the number of available codewords (excluding required ECC codewords).
    pub fn capacity(&self) -> usize {
        self.storage.len()
    }

    /// Returns the number of free codeword slots including slots filled up by
    /// ECC codewords (at the specified level).
    pub fn available(&self, level: u8) -> usize {
        self.storage.len() - ecc::ecc_count(level) - self.used
    }

    /// Appends a numeric segment containing a 64-bit unsigned integer `n`. For
    /// larger numbers please use the [append_ascii] method which can handle 44+
    /// digit numbers.
    pub fn append_num(mut self, mut n: u64) -> Self {
        self.storage[self.used] = M_LATCH_NUMERIC;

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
            self.storage[self.used + 1 + nb - count - 1] = r as u16;
            count += 1;
        }

        self.used += 1 + nb;
        self
    }

    /// Appends a bytes segment.
    pub fn append_bytes(mut self, bytes: &[u8]) -> Self {
        let mut i = self.used;
        let mut k = 0;

        if bytes.len() > 1 {
            // latch to byte mode
            self.storage[i] = if bytes.len() % 6 == 0 { M_LATCH_BYTE_M6 } else { M_LATCH_BYTE };
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
                    self.storage[i + 4 - n] = r as u16;
                    s = q;
                }

                i += 5;
                k += 6;
            }
        } else {
            self.storage[i] = M_SHIFT_BYTE; // shift to byte mode (only for next codeword)
            i += 1;
        }

        // remaining
        while k < bytes.len() {
            self.storage[i] = bytes[k] as u16;
            k += 1;
            i += 1;
        }

        self.used = i;
        self
    }

    /// Appends an ASCII (text) segment. *Warning*: This function uses the
    /// PDF417 table based encoding to optimize the size of the text and
    /// therefore support only a small set of displayable characters. If you
    /// want to encode an UTF-8 string, use [append_utf8] instead (uses more
    /// space).
    pub fn append_ascii(mut self, s: &str) -> Self {
        debug_assert!(s.is_ascii(), "use append_utf8 for UTF-8 strings");
        let out = &mut self.storage;
        let s = s.as_bytes();

        let mut mode: u8 = 0; // 0: Upper, 1: Lower, 2: Mixed, 3: Punc, 4: Numeric
        let mut i = self.used;
        let mut k = 0;
        let mut right = false; // false = upper 8 bits | true = lower 8 bits

        while k < s.len() {
            let c = s[k];
            match c {
                c if c.is_ascii_uppercase() => { // b'A'..=b'Z'
                    match mode {
                        0 => (),
                        1 => if k + 1 < s.len() && s[k + 1].is_ascii_lowercase() {
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
                c if c.is_ascii_lowercase() => { // b'a'..=b'z'
                    match mode {
                        0 | 2 => push!(out, i, right, 27; mode = 1),
                        1 => (),
                        3 => push!(out, i, right, 29, 27; mode = 1),
                        _ => unreachable!("Unknown mode {mode}"),
                    }
                    push!(out, i, right, c - b'a'; k = k + 1);
                },
                c if c.is_ascii_digit() => { // b'0'..=b'9'
                    let mut end = k + 1;
                    while end < s.len() && end-k < 44 && s[end].is_ascii_digit() {
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
                        if mode != 4 { push_sp!(out, i, right, M_LATCH_NUMERIC; mode = 4); }

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

                    if mode == 4 && k < s.len() && !s[k].is_ascii_digit() {
                        push_sp!(out, i, right, M_LATCH_TEXT; mode = 0);
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
                            3 if (1..=4).contains(&p) || (6..=9).contains(&p) => (),
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
                        out[i] = M_SHIFT_BYTE;
                        out[i + 1] = c as u16;
                        i += 2;
                    }
                    k += 1;
                },
            };
        }

        if right { 
            out[i] = out[i] * 30 + 29;
            i += 1;
        }
        self.used = i;

        self
    }

    /// Appends a special segement crafted to store an __UTF-8__ string `s`.
    /// __Note that the conversion is space inefficient, if the string is
    /// composed of ASCII characters, please consider using [generate_ascii]
    /// instead.__ Internally, we use a ECI identifier (\\000026) to switch to
    /// the UTF-8 code page then append a byte segment to store the string's
    /// bytes.
    pub fn append_utf8(mut self, s: &str) -> Self {
        self.storage[self.used] = ECI_CODE_PAGE; // ECI identifier for code page
        self.storage[self.used + 1] = 26; // UTF-8 is \000026
        self.used += 2;

        self.append_bytes(s.as_bytes())
    }

    /// Append a single codeword.
    pub fn append_codeword(mut self, codeword: u16) -> Self {
        self.storage[self.used] = codeword;
        self.used += 1;
        self
    }

    /// Append a slice of codewords.
    pub fn append_raw(mut self, codewords: &[u16]) -> Self {
        self.storage[self.used..self.used+codewords.len()].copy_from_slice(codewords);
        self.used += codewords.len();
        self
    }

    /// Call this function to seal your data segments into a slice of codewords
    /// ready to be rendered to a PDF417. Both padding and ECC
    /// codewords are generated by this function.
    pub fn seal(self, level: u8) -> &'a mut [u16] {
        let total = self.capacity() - ecc::ecc_count(level);
        self.storage[0] = total as u16;
        self.storage[self.used..total].fill(CW_PADDING);

        ecc::generate_ecc(self.storage, level);

        self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::PDF417Encoder;

    #[test]
    fn test_encode_ascii_simple() {
        let mut codewords = [0u16; 3 + 1];
        PDF417Encoder::new(&mut codewords).append_ascii("Test");
        assert_eq!(&codewords, &[0, 19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29]);
    }

    #[test]
    fn test_generate_ascii_switch_modes() {
        let mut codewords = [0u16; 9];
        PDF417Encoder::new(&mut codewords).append_ascii("abc1D234\x1B");
        assert_eq!(&codewords, &[0, 27 * 30 + 0, 1 * 30 + 2, 28 * 30 + 1, 28 * 30 + 3, 28 * 30 + 2, 3 * 30 + 4, 913, 0x1B]);
    }

    #[test]
    fn test_generate_ascii_numeric() {
        let mut codewords = [0u16; 12];
        PDF417Encoder::new(&mut codewords).append_ascii("12345678987654321 num");
        assert_eq!(&codewords, &[0, 902, 190, 232, 499, 20, 504, 721, 900, 26 * 30 + 27, 13 * 30 + 20, 12 * 30 + 29]);
    }

    #[test]
    fn test_generate_ascii_numeric_big() {
        let mut codewords = [0u16; 20];
        PDF417Encoder::new(&mut codewords)
            //             [                        p1                 ][ p2 ]
            .append_ascii("123456789876543211234567898765432112345678987654321");
        assert_eq!(&codewords, &[0, 902, 491, 81, 137, 725, 651, 455, 511, 858, 135, 138, 488, 568, 447, 553, 198, /* p2 */ 21, 715, 821]);
    }

    #[test]
    fn test_encode_num() {
        let mut codewords = [0u16; 8];
        PDF417Encoder::new(&mut codewords).append_num(12345678987654321);
        assert_eq!(&codewords, &[0, 902, 190, 232, 499, 20, 504, 721]);
    }

    #[test]
    fn test_generate_ascii_with_digits() {
        let mut codewords = [0u16; 17];
        PDF417Encoder::new(&mut codewords).append_ascii("encoded 0123456789 as digits");
        assert_eq!(&codewords, &[0, 27 * 30 + 4, 13 * 30 + 2, 14 * 30 + 3, 4 * 30 + 3, 26 * 30 + 28, 0 * 30 + 1, 2 * 30 + 3, 4 * 30 + 5, 6 * 30 + 7, 8 * 30 + 9,
            26 * 30 + 27, 0 * 30 + 18, 26 * 30 + 3, 8 * 30 + 6, 8 * 30 + 19, 18 * 30 + 29]);
    }

    #[test]
    fn test_generate_ascii_punc_mixed() {
        let mut codewords = [0u16; 18];
        PDF417Encoder::new(&mut codewords).append_ascii("This! Is a `quote (100%)`.");
        assert_eq!(&codewords, &[0, 19 * 30 + 27, 7 * 30 + 8, 18 * 30 + 29, 10 * 30 + 26, 27 * 30 + 8, 18 * 30 + 26, 0 * 30 + 26, 29 * 30 + 8, 16 * 30 + 20, 14 * 30 + 19, 4 * 30 + 26, 29 * 30 + 23, 28 * 30 + 1, 0 * 30 + 0, 21 * 30 + 25, 24 * 30 + 8, 17 * 30 + 29]);
    }

    #[test]
    fn test_encode_bytes_multiple() {
        let mut codewords = [0u16; 7];
        PDF417Encoder::new(&mut codewords).append_bytes(b"alcool");
        assert_eq!(&codewords, &[0, 924, 163, 238, 432, 766, 244]);
    }

    #[test]
    fn test_encode_bytes_not_multiple() {
        let mut codewords = [0u16; 11];
        PDF417Encoder::new(&mut codewords).append_bytes(b"encode bin");
        assert_eq!(&codewords, &[0, 901, 169, 883, 224, 680, 517, 32, 98, 105, 110]);
    }

    #[test]
    fn test_multiple_segments() {
        let mut codewords = [0u16; 1 + 3 + 2 + 10];

        PDF417Encoder::new(&mut codewords)
            .append_ascii("Test")
            .append_num(42)
            .append_bytes(b"encode bin");
        assert_eq!(&codewords, &[
            0,
            19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29,
            902, 142,
            901, 169, 883, 224, 680, 517, 32, 98, 105, 110
        ]);
    }

    #[test]
    fn test_seal() {
        let mut codewords = [0u16; 1 + 3 + 2 + 10 + 2];

        PDF417Encoder::new(&mut codewords)
            .append_ascii("Test")
            .append_num(42)
            .append_bytes(b"encode bin")
            .seal(0);

        assert_eq!(&codewords, &[
            16,
            19 * 30 + 27, 4 * 30 + 18, 19 * 30 + 29,
            902, 142,
            901, 169, 883, 224, 680, 517, 32, 98, 105, 110,
            // ecc
            161, 542
        ]);
    }
}
