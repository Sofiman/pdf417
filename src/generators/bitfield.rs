use core::iter;

#[derive(Debug, Clone, Copy)]
pub struct Bitfield(u32);

impl Bitfield {
    pub const fn new(bits: u32, count: u8) -> Self {
        debug_assert!(count <= 24, "count is too big");

        Self((bits << 8) | count as u32)
    }

    #[inline]
    pub const fn size(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    #[inline]
    pub const fn bits(&self) -> u32 {
        self.0 >> 8
    }

    #[inline]
    pub const fn as_pair(&self) -> (u32, u32) {
        (self.0 >> 8, self.0 & 0xFF)
    }
}

impl iter::IntoIterator for Bitfield {
    type Item = bool;
    type IntoIter = Bits;

    fn into_iter(self) -> Self::IntoIter {
        let (value, count) = self.as_pair();
        Bits { value, count }
    }
}

pub struct Bits {
    value: u32,
    count: u32,
}

impl iter::Iterator for Bits {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            let bit = (self.value >> self.count) & 1 != 0;
            self.count -= 1;
            Some(bit)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.count as usize;
        (count, Some(count))
    }
}

impl iter::DoubleEndedIterator for Bits {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            let bit = (self.value & 1) != 0;
            self.value >>= 1;
            self.count -= 1;
            Some(bit)
        } else {
            None
        }
    }
}

impl iter::ExactSizeIterator for Bits {}
impl iter::FusedIterator for Bits {}
