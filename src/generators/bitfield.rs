use core::iter;

#[derive(Debug, Clone, Copy)]
pub struct Bitfield(u32);

impl Bitfield {
    pub const fn new(bits: u32, count: u8) -> Self {
        debug_assert!(count <= 24, "count is too big");

        let mask = (1 << count) - 1;
        debug_assert!(bits & !mask == 0, "too many bits in bitfield");

        Self((bits & mask) << 8 | count as u32)
    }

    #[inline]
    pub const fn size(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// This function allows us to directly modify the count value without any fancy bit fiddling.
    #[inline]
    fn count_mut(&mut self) -> &mut u8 {
        unsafe {
            // SAFETY: An u32 is composed of 4 bytes so we can safely get a pointer to the
            // first or last byte depending on the target's endianness.

            let mut base_ptr = (&mut self.0) as *mut u32;
            if cfg!(target_endian = "big") {
                base_ptr = base_ptr.offset(3); // get last byte
            }

            &mut *(base_ptr as *mut u8)
        }
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

impl iter::Iterator for Bitfield {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let count = self.0 & 0xFF;
        if count > 0 {
            let bit = self.0 & (1 << (7 + count));
            *self.count_mut() -= 1;
            Some(bit != 0)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.count() as usize;
        (count, Some(count))
    }
}

impl iter::DoubleEndedIterator for Bitfield {
    fn next_back(&mut self) -> Option<Self::Item> {
        let count = self.0 & 0xFF;
        if count > 0 {
            let bit = self.0 & (1 << (u32::BITS - 7 + count));
            *self.count_mut() -= 1;
            Some(bit != 0)
        } else {
            None
        }
    }
}

impl iter::ExactSizeIterator for Bitfield {}
