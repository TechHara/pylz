pub const MAX_DISTANCE: usize = 1 << 15; // 32k
pub const MAX_LENGTH: usize = 258;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Lz77 {
    Literal(u8),
    Dictionary {
        length: u8, // 0~255 <-> 3~258
        distance: u16,
    },
}

impl Default for Lz77 {
    fn default() -> Self {
        Self::Literal(0)
    }
}
