use core::iter;

use crate::{HL_TO_LL, M_PDF417_VARIANTS_COUNT, M_PDF417_RAP, M_PDF417_SIDE, M_PDF417_CENTER};
use crate::generators::{row::{Row, FixedSize}, bitfield::Bitfield};

macro_rules! cw {
    ($val:expr) => {
        M_PDF417_SIDE[$val as usize] as u32
    };
    ($tb:expr, $val:expr) => {
        Bitfield::new((1 << 16) | HL_TO_LL[$tb as usize][$val as usize] as u32, 17)
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum RowPattern {
    Start = 0,
    LeftData = 1,
    RightData = 2,
    None,
}

#[derive(Clone)]
pub struct MicroPDF417Row<'a> {
    codewords: &'a [u16],
    next_pat: RowPattern,

    start_ind: u8,
    center_ind: u8,
    end_ind: u8,
    table: u8,
}

impl<'a> FixedSize for MicroPDF417Row<'a> {}

impl<'a> Row<'a> for MicroPDF417Row<'a> {
    type Info = (u8, u8, u8, u8);
    const DEFAULT_SCALE: (u16, u16) = (1, 2);

    fn init(codewords: &'a [u16], row: u8, infos: Self::Info) -> Self {
        Self {
            codewords,
            next_pat: RowPattern::Start,

            start_ind: (infos.0 + row) % 51,
            center_ind: (infos.1 + row) % 51, 
            end_ind: (infos.2 + row) % 51,
            table: (infos.3 + row) % 3
        }
    }

    fn prepare(_dimensions: (u8, u8), variant: u8) -> Self::Info {
        // TODO: assert that dimensions are the same as the variant
        (
            M_PDF417_RAP[0 * M_PDF417_VARIANTS_COUNT + variant as usize] - 1,
            M_PDF417_RAP[1 * M_PDF417_VARIANTS_COUNT + variant as usize] - 1,
            M_PDF417_RAP[2 * M_PDF417_VARIANTS_COUNT + variant as usize] - 1,
            M_PDF417_RAP[3 * M_PDF417_VARIANTS_COUNT + variant as usize],
        )
    }
}

impl<'a> iter::Iterator for MicroPDF417Row<'a> {
    type Item = Bitfield;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self.next_pat {
            RowPattern::Start => {
                let next = if self.codewords.len() > 2 { RowPattern::LeftData } else { RowPattern::RightData };
                (Some(Bitfield::new(cw![self.start_ind], 10)), next)
            },

            // middle
            RowPattern::LeftData if self.codewords.len() < 2 =>
                (Some(Bitfield::new(M_PDF417_CENTER[self.center_ind as usize] as u32, 10)), RowPattern::RightData),
            // end
            RowPattern::RightData if self.codewords.is_empty() =>
                (Some(Bitfield::new((cw![self.end_ind] << 1) | 1, 11)), RowPattern::None),

            RowPattern::LeftData | RowPattern::RightData => {
                let cw = self.codewords[0];
                self.codewords = &self.codewords[1..];

                (Some(cw!(self.table, cw)), self.next_pat)
            },

            RowPattern::None => (None, RowPattern::None)
        };

        self.next_pat = next;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let RowPattern::None = self.next_pat {
            (0, Some(0))
        } else {
            let len = self.codewords.len();
            let count = 2 + len + (len >> 1) & 1; // if len > 2 then +1
            (count, Some(count))
        }
    }
}

impl<'a> ExactSizeIterator for MicroPDF417Row<'a> {}
impl<'a> iter::FusedIterator for MicroPDF417Row<'a> {}
