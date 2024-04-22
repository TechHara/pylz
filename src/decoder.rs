use crate::lz77::{Lz77, MAX_DISTANCE, MAX_LENGTH};

const BUF_LEN: usize = 1 << 16;
const BUF_MASK: usize = BUF_LEN - 1;

pub struct Decoder<I> {
    iter: I,
    buf: Vec<u8>,
    pos: usize,
    cap: usize,
}

impl<I> Decoder<I> {
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            buf: vec![0; BUF_LEN],
            pos: 0,
            cap: 0,
        }
    }
}

impl<I: Iterator<Item = Lz77>> Iterator for Decoder<I> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        // slide back if necessary
        if self.cap >= BUF_LEN - MAX_LENGTH {
            let n = self.cap - MAX_DISTANCE;
            self.buf.copy_within(self.cap - MAX_DISTANCE..self.cap, 0);
            self.cap -= n;
            self.pos -= n;
        }

        if self.pos < self.cap {
            let x = self.buf[self.pos & BUF_MASK];
            self.pos += 1;
            return Some(x);
        }

        // self.pos == self.cap
        match self.iter.next()? {
            Lz77::Literal(x) => {
                self.buf[self.cap & BUF_MASK] = x;
                self.cap += 1;
                self.pos += 1;
                Some(x)
            }
            Lz77::Dictionary { length, distance } => {
                let mut length = length as usize + 3;
                let mut distance = distance as usize;
                let mut idx = self.cap & BUF_MASK;
                self.cap += length;
                let begin = idx - distance;
                while length > 0 {
                    let n = distance.min(length);
                    self.buf.copy_within(begin..begin + n, idx);
                    idx += n;
                    length -= n;
                    distance += n;
                }

                let x = self.buf[self.pos];
                self.pos += 1;
                Some(x)
            }
        }
    }
}
