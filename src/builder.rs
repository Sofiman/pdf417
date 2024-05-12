use core::marker::PhantomData;
use crate::{generators::row::{Row, FixedSize, FreeSize}, tables::Variant};

/// Minimum number of rows in a PDF417 barcode.
pub const MIN_ROWS: u8 = 3;
/// Maximum number of rows in a PDF417 barcode.
pub const MAX_ROWS: u8 = 90;
/// Minimum number of data columns in a PDF417 barcode.
pub const MIN_COLS: u8 = 1;
/// Maximum number of data columns in a PDF417 barcode.
pub const MAX_COLS: u8 = 30;

#[derive(Debug, Clone)]
pub struct PDF417<'a, R: Row<'a> + 'a> {
    storage: &'a [u16],
    dimensions: (u8, u8),
    level: u8,
    _phantom: PhantomData<R>,
}

impl<'a, R: Row<'a> + 'a> PDF417<'a, R> {
    /// Get the number of rows of the PDF417.
    #[inline]
    pub const fn rows(&self) -> u8 {
        self.dimensions.0
    }

    /// Get the number of columns of the PDF417. This is used to lay down the 
    /// start, left, right and end indicators in the render function.
    #[inline]
    pub const fn cols(&self) -> u8 {
        self.dimensions.1
    }

    #[inline]
    pub const fn level(&self) -> u8 {
        self.level
    }

    pub fn iter(&self) -> impl Iterator<Item = R> + 'a {
        let infos = R::prepare(self.dimensions, self.level);
        // TODO: should it be chunks_exact ?
        self.storage.chunks(self.cols() as usize)
            .enumerate()
            .map(move |(row, codewords)| R::init(codewords, row as u8, infos))
    }

    pub fn bits(&self) -> impl Iterator<Item = bool> + 'a {
        self.iter()
            .flatten() // rows -> bitfields
            .flatten() // bitfield -> bits
    }

    pub const fn render(self) -> PDF417Render<'a, R> {
         PDF417Render {
             inner: self,
             scale: R::DEFAULT_SCALE,
             inverted: false
         }
    }
}

impl<'a, R: Row<'a> + 'a + FixedSize> PDF417<'a, R> {
    pub const fn from_variant(storage: &'a [u16], v: Variant) -> Self {
        let dimensions = (v.rows(), v.cols());
        assert!(storage.len() == (dimensions.0 as usize * dimensions.1 as usize),
            "The data will not fit in the provided configuration");

        Self { storage, dimensions, level: v.variant(), _phantom: PhantomData }
    }
}

impl<'a, R: Row<'a> + 'a + FreeSize> PDF417<'a, R> {
    /// Creates a new PDF417 with the user's data section (codewords slice),
    /// the level of error correction and the layout configuration
    /// (rows and cols). The total codewords capacity is calculated with 
    /// rows \* cols and must be greater or equal to the number of codewords
    /// in the `codewords` slice. Please make sure your codewords
    /// slice is valid, you can use [PDF417Encoder] to fill it accordingly.
    pub const fn new(storage: &'a [u16], rows: u8, cols: u8, level: u8) -> Self {
        assert!(rows >= MIN_ROWS && rows <= MAX_ROWS, "The number of rows must be between 3 and 90");
        assert!(cols >= MIN_COLS && cols <= MAX_COLS, "The number of columns must be between 1 and 30");
        assert!(storage.len() == (rows as usize * cols as usize),
            "The data will not fit in the provided configuration");
        assert!(level < 9, "ECC level must be between 0 and 8");

        Self { storage, dimensions: (rows, cols), level, _phantom: PhantomData }
    }

}

#[derive(Debug, Clone)]
pub struct PDF417Render<'a, R: Row<'a> + 'a> {
    inner: PDF417<'a, R>,
    scale: (u16, u16),
    inverted: bool
}

impl<'a, R: Row<'a> + 'a> From<PDF417<'a, R>> for PDF417Render<'a, R> {
    fn from(inner: PDF417<'a, R>) -> Self {
        inner.render()
    }
}

impl<'a, R: Row<'a> + 'a> PDF417Render<'a, R> {
    pub fn width(&self) -> u32 {
        R::width(self.inner.dimensions) * self.scale.0 as u32
    }

    pub const fn height(&self) -> u32 {
        self.inner.rows() as u32 * self.scale.1 as u32
    }


    /// Returns the scale of the PDF417 as (Scale X axis, Scale Y axis).
    pub const fn scale(&self) -> (u16, u16) {
        self.scale
    }

    /// Sets the scale of the PDF417 on both axis. See also [scaled](PDF417::scaled).
    pub const fn set_scale(mut self, scale: (u16, u16)) -> Self {
        self.scale = scale;
        self
    }

    /// Returns if the PDF417 is set to be rendered with inverted colors.
    pub const fn inverted(&self) -> bool {
        self.inverted
    }

    /// Marks whether this PDF417 should be rendered with pixel values inverted.
    pub const fn set_inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    pub fn bits(&self) -> impl Iterator<Item = bool> + 'a {
        let (sx, sy) = self.scale;
        let invert = self.inverted;
        self.inner.iter()
            .flat_map(move |row| core::iter::repeat(row).take(sy as usize))
            .flatten() // rows -> bitfields
            .flatten() // bitfield -> bits
            .flat_map(move |bit| core::iter::repeat(bit ^ invert).take(sx as usize))
    }

    pub fn fill<P: Clone>(&self, target: &mut [P], on: &P, off: &P) {
        for (i, bit) in self.bits().enumerate() {
            target[i] = if bit { on.clone() } else { off.clone() };
        }
    }

    pub fn fill_bits(&self, target: &mut [bool]) {
        self.fill(target, &true, &false);
    }

    pub fn fill_bitmap(&self, target: &mut [u8]) {
        let mut i = 0;
        let mut mask: u8 = 7;
        for bit in self.bits() {
            if mask == 0 {
                i += 1;
                mask = 7;
            } else {
                target[i] |= (bit as u8) << mask;
                mask -= 1;
            }
        }
    }
}
