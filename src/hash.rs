pub const TABLE_SIZE: usize = 0x8000;

pub struct RunningHasher {
    hash: usize,
}

impl Default for RunningHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl RunningHasher {
    /// 3-byte hasher, i.e., the hash is a function of the last three bytes
    /// uses 15-bits hash, so shift is 5-bits
    const MASK: usize = TABLE_SIZE - 1;
    const SHIFT: usize = 5;

    pub fn new() -> Self {
        Self { hash: 0 }
    }

    pub fn update(&mut self, x: u8) -> usize {
        self.hash = (self.hash << Self::SHIFT) ^ x as usize;
        self.hash &= Self::MASK;
        self.hash
    }

    pub fn get(&self) -> usize {
        self.hash
    }
}

#[test]
fn test_rolling_hash() {
    let mut hasher = RunningHasher::new();
    hasher.update(b'a');
    hasher.update(b'b');
    let h1 = hasher.update(b'c');

    hasher.update(b'a');
    hasher.update(b'b');
    let h2 = hasher.update(b'c');

    assert_eq!(h1, h2);
}
