use crate::hash::RunningHasher;

use crate::hash_chain::HashChain;
use crate::lz77::{Lz77, MAX_DISTANCE, MAX_LENGTH};
use std::io::Result;
use std::io::{Cursor, Read};

const BUF_LEN: usize = 1 << 16; // 64k
const BUF_MASK: usize = BUF_LEN - 1;
const READ_CHUNK_SIZE: usize = 1 << 14; // 16k -- must be a few bytes less than MAX_DISTANCE
const CHAIN_LEN: usize = 1 << 15; // 32k
const CHAIN_MASK: usize = CHAIN_LEN - 1;

pub struct Encoder<R> {
    read: R,
    hash_pos: usize,
    search_pos: usize,
    cap: usize,
    buf: Vec<u8>,
    hasher: RunningHasher,
    hash_chain: HashChain,
    pos2hash: Vec<u16>,
    verbose: bool,
}

impl<R: Read> Encoder<R> {
    pub fn new(read: R, verbose: bool) -> Self {
        Self {
            read,
            hash_pos: 0,
            search_pos: 0,
            cap: 0,
            buf: vec![0; BUF_LEN],
            hasher: RunningHasher::new(),
            hash_chain: HashChain::new(),
            pos2hash: vec![0; CHAIN_LEN],
            verbose,
        }
    }

    /// fill up the buffer by at most READ_CHUNK_SIZE
    fn fill_buf(&mut self) -> Result<()> {
        if self.cap >= self.search_pos + MAX_LENGTH {
            return Ok(());
        }
        let cap = self.cap & BUF_MASK;
        let n = (BUF_LEN - cap).min(READ_CHUNK_SIZE);
        let n = self.read.read(&mut self.buf[cap..cap + n])?;
        self.cap += n;
        Ok(())
    }

    /// update the hash and return the most recent position that has hash clash
    fn advance_hash(&mut self) -> usize {
        match self.hash_pos & BUF_MASK {
            0x0000 => self.hash_chain.prune_table(0x8000..),
            0x8000 => self.hash_chain.prune_table(0x0000..0x8000),
            _ => {}
        }

        let x = self.buf[self.hash_pos & BUF_MASK];
        let h = self.hasher.update(x);
        if self.hash_pos < 2 {
            self.hash_pos += 1;
            return 0;
        }

        let new_idx = self.hash_pos - 2;
        match new_idx & BUF_MASK {
            0x0000 => self.hash_chain.prune_chain(0x8000..),
            0x8000 => self.hash_chain.prune_chain(0x0000..0x8000),
            _ => {}
        }

        let x = (new_idx & BUF_MASK) as u16;
        let prev_idx = self.hash_chain.add(h, x);
        self.pos2hash[(self.hash_pos - 2) & CHAIN_MASK] = h as u16;
        self.hash_pos += 1;
        prev_idx as usize
    }

    fn match_length(&mut self, begin: usize, end: usize, target: usize) -> usize {
        let mut result = 0;
        for (ix1, ix2) in (begin..end).zip(target..) {
            if self.buf[ix1 & BUF_MASK] != self.buf[ix2 & BUF_MASK] {
                break;
            }
            result += 1;
        }
        result
    }
}

impl<R: Read> Iterator for Encoder<R> {
    type Item = Lz77;

    fn next(&mut self) -> Option<Self::Item> {
        while self.search_pos + 2 > self.hash_pos {
            self.fill_buf().unwrap();
            self.advance_hash();
        }

        self.fill_buf().unwrap();
        if self.search_pos >= self.cap {
            return None;
        }
        let mut pos = self.advance_hash();

        let mut best_distance = 0;
        let mut best_length = 0;
        let upper_bound = MAX_LENGTH.min(self.cap - self.search_pos);
        let mut prev_distance = 0;

        while pos != 0 {
            let distance = (self.search_pos - pos) & 0xFFFF;
            debug_assert_eq!(
                distance & BUF_MASK,
                (self.search_pos - (pos & BUF_MASK)) & BUF_MASK
            );

            debug_assert_ne!(distance, 0);
            if prev_distance >= distance || distance > MAX_DISTANCE {
                break;
            }

            debug_assert_eq!(self.pos2hash[pos & CHAIN_MASK], self.hasher.get() as u16);

            if self.verbose {
                eprintln!("pos: {}\tdistance: {}", self.search_pos, distance);
            }

            if self.buf[(self.search_pos + best_length + 1) & BUF_MASK]
                == self.buf[(pos + best_length + 1) & BUF_MASK]
            {
                let length = self.match_length(self.search_pos, self.search_pos + upper_bound, pos);
                if length > best_length {
                    best_length = length;
                    best_distance = distance;
                }
                if best_length == upper_bound {
                    break;
                }
            }

            pos = self.hash_chain.get(pos) as usize;
            prev_distance = distance;
        }

        if best_length < 4 {
            let x = self.buf[self.search_pos & BUF_MASK];
            self.search_pos += 1;
            Some(Lz77::Literal(x))
        } else {
            self.search_pos += best_length;
            let distance = best_distance as u16;
            let length = (best_length - 3) as u8;
            Some(Lz77::Dictionary { length, distance })
        }
    }
}

/// pos is where to start encoding
/// any data before is used as history
pub fn deflate(mut xs: &[u8], mut pos: usize) -> Vec<Lz77> {
    if pos > MAX_DISTANCE {
        xs = &xs[pos - MAX_DISTANCE..];
        pos = MAX_DISTANCE;
    }

    let cursor = Cursor::new(xs);
    let mut encoder = Encoder::new(cursor, false);
    encoder.search_pos = pos;

    encoder.collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encoder() {
        let xs = vec![0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5];
        let encoder = Encoder::new(std::io::Cursor::new(xs), false);
        assert_eq!(
            encoder.collect::<Vec<_>>(),
            vec![
                Lz77::Literal(0),
                Lz77::Literal(1),
                Lz77::Literal(2),
                Lz77::Literal(3),
                Lz77::Literal(4),
                Lz77::Literal(5),
                Lz77::Literal(0),
                Lz77::Dictionary {
                    length: 5 - 3,
                    distance: 6
                },
            ]
        );
    }

    #[test]
    fn test_deflate() {
        let xs = vec![0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5];
        assert_eq!(
            deflate(&xs, 6),
            vec![
                Lz77::Literal(0),
                Lz77::Dictionary {
                    length: 5 - 3,
                    distance: 6
                },
            ]
        );
    }
}
