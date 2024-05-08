use core::iter;
use crate::generators::bitfield::Bitfield;

pub const START_PAT: Bitfield = Bitfield::new(0b11111111010101000, 17);
pub const   END_PAT: Bitfield = Bitfield::new(0b111111101000101001, 18);

pub trait PDF417Columns<'a>: iter::Iterator<Item = Bitfield> + Clone {
    type Info: Copy; // info must be cheap to copy

    fn init(codewords: &'a [u16], row: u8, infos: Self::Info) -> Self;
    fn prepare(dimensions: (u8, u8), level: u8) -> Self::Info;
}

pub struct PDF417RowIterator<'a, Columns: PDF417Columns<'a>> {
    codewords: &'a [u16],
    dimensions: (u8, u8),
    infos: Columns::Info,

    row: u8,
}

impl<'a, Columns: PDF417Columns<'a>> PDF417RowIterator<'a, Columns> {
    pub fn new(codewords: &'a [u16], dimensions: (u8, u8), level: u8) -> Self {
        Self {
            codewords,
            dimensions,
            infos: Columns::prepare(dimensions, level),
            row: 0,
        }
    }
}

impl<'a, Columns: PDF417Columns<'a>> iter::Iterator for PDF417RowIterator<'a, Columns> {
    type Item = Columns;

    fn next(&mut self) -> Option<Self::Item> {
        let (rows, cols) = self.dimensions;
        if self.row == rows {
            return None;
        }

        let start = self.row as usize * cols as usize;
        let end = start + cols as usize;
        let row = Columns::init(&self.codewords[start..end], self.row, self.infos);

        self.row += 1;
        Some(row)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = (self.dimensions.0 - self.row) as usize;
        (count, Some(count))
    }
}

impl<'a, Columns: PDF417Columns<'a>> ExactSizeIterator for PDF417RowIterator<'a, Columns> {}
impl<'a, Columns: PDF417Columns<'a>> iter::FusedIterator for PDF417RowIterator<'a, Columns> {}
