use core::iter;

use crate::HL_TO_LL;
use crate::generators::generator::{PDF417Columns, START_PAT, END_PAT};
use crate::generators::bitfield::Bitfield;

macro_rules! cw {
    ($tb:expr, $val:expr) => {
        Bitfield::new((1 << 16) | HL_TO_LL[$tb as usize * 929 + $val as usize] as u32, 17)
    }
}

#[derive(Clone)]
#[repr(u8)]
enum RowPattern {
    Start,
    Left,
    Data,
    Right,
    End,
    None,
}

#[derive(Clone)]
pub struct PDF417Row<'a, const TRUNCATED: bool> {
    codewords: &'a [u16],
    next_pat: RowPattern,
    table: u8,
    /// (left, right)
    markers: (u16, u16)
}

impl<'a, const TRUNCATED: bool> PDF417Row<'a, TRUNCATED> {
    fn new(codewords: &'a [u16], row: u8, infos: (u8, u8, u8)) -> Self {
        let (rows_val, cols_val, level_val) = infos;
        let table = row % 3;
        let row_id = (row / 3) as u16 * 30;

        let (left, right) = match table {
            0 => (rows_val, cols_val),
            1 => (level_val, rows_val),
            2 => (cols_val, level_val),
            _ => unreachable!()
        };
        Self {
            codewords,
            table,
            markers: (left as u16 + row_id, right as u16 + row_id),
            next_pat: RowPattern::Start
        }
    }

    fn prepare((rows, cols): (u8, u8), level: u8) -> (u8, u8, u8) {
        let rows_val = (rows - 1) / 3;
        let cols_val = cols - 1;
        let level_val = level * 3 + (rows - 1) % 3;
        (rows_val, cols_val, level_val)
    }
}

impl<'a> PDF417Columns<'a> for PDF417Row<'a, false> {
    type Info = (u8, u8, u8);

    fn init(codewords: &'a [u16], row: u8, infos: (u8, u8, u8)) -> Self {
        Self::new(codewords, row, infos)
    }

    fn prepare(dimensions: (u8, u8), level: u8) -> Self::Info {
        Self::prepare(dimensions, level)
    }
}

impl<'a> iter::Iterator for PDF417Row<'a, false> {
    type Item = Bitfield;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self.next_pat {
            RowPattern::Start => (Some(START_PAT), RowPattern::Left),
            RowPattern::Left => (Some(cw!(self.table, self.markers.0)), RowPattern::Data),
            RowPattern::Data => {
                let cw = self.codewords[0];
                self.codewords = &self.codewords[1..];

                let next = if self.codewords.is_empty() { RowPattern::Right } else { RowPattern::Data };

                (Some(cw!(self.table, cw)), next)
            },
            RowPattern::Right => (Some(cw!(self.table, self.markers.1)), RowPattern::End),
            RowPattern::End => (Some(END_PAT), RowPattern::None),
            RowPattern::None => (None, RowPattern::None)
        };

        self.next_pat = next;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.codewords.len() + match self.next_pat {
            RowPattern::Start => 4,
            RowPattern::Left  => 3,
            RowPattern::Data  => 2,
            RowPattern::Right => 2,
            RowPattern::End   => 1,
            RowPattern::None  => 0,
        };
        (count, Some(count))
    }
}

impl<'a> ExactSizeIterator for PDF417Row<'a, false> {}
impl<'a> iter::FusedIterator for PDF417Row<'a, false> {}

impl<'a> PDF417Columns<'a> for PDF417Row<'a, true> {
    type Info = (u8, u8, u8);

    fn init(codewords: &'a [u16], row: u8, infos: (u8, u8, u8)) -> Self {
        Self::new(codewords, row, infos)
    }

    fn prepare(dimensions: (u8, u8), level: u8) -> Self::Info {
        Self::prepare(dimensions, level)
    }
}

impl<'a> iter::Iterator for PDF417Row<'a, true> {
    type Item = Bitfield;

    fn next(&mut self) -> Option<Self::Item> {
        let (item, next) = match self.next_pat {
            RowPattern::Start => (Some(START_PAT), RowPattern::Left),
            RowPattern::Left => (Some(cw!(self.table, self.markers.0)), RowPattern::Data),
            RowPattern::Data => {
                let cw = self.codewords[0];
                self.codewords = &self.codewords[1..];

                let next = if self.codewords.is_empty() { RowPattern::End } else { RowPattern::Data };

                (Some(cw!(self.table, cw)), next)
            },
            RowPattern::End => (Some(Bitfield::new(1, 1)), RowPattern::None),
            RowPattern::Right | RowPattern::None => (None, RowPattern::None)
        };

        self.next_pat = next;
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.codewords.len() + match self.next_pat {
            RowPattern::Start => 3,
            RowPattern::Left  => 2,
            RowPattern::Data  => 1,
            RowPattern::End   => 1,
            RowPattern::Right | RowPattern::None  => 0,
        };
        (count, Some(count))
    }
}

impl<'a> ExactSizeIterator for PDF417Row<'a, true> {}
impl<'a> iter::FusedIterator for PDF417Row<'a, true> {}
