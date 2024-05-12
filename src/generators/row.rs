use core::iter;
use crate::generators::bitfield::Bitfield;

pub trait FreeSize {}
pub trait FixedSize {}

pub trait Row<'a>: iter::Iterator<Item = Bitfield> + Clone {
    type Info: Copy; // info must be cheap to copy
    const DEFAULT_SCALE: (u16, u16);

    fn prepare(dimensions: (u8, u8), level: u8) -> Self::Info;
    fn init(codewords: &'a [u16], row: u8, infos: Self::Info) -> Self;
    fn width(dimensions: (u8, u8)) -> u32;
}
