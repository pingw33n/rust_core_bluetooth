use enumflags2::{BitFlags, RawBitFlags};
use std::fmt;

pub struct BitFlagsDebug<T: RawBitFlags>(pub BitFlags<T>);

impl<T: RawBitFlags + fmt::Debug> fmt::Debug for BitFlagsDebug<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut it = self.0.iter().peekable();
        write!(f, "BitFlags(")?;
        while let Some(v) = it.next() {
            write!(f, "{:?}", v)?;
            if it.peek().is_some() {
                write!(f, " | ")?;
            }
        }
        write!(f, ")")
    }
}