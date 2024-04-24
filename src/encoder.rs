use crate::hash::RunningHasher;

use crate::hash_chain::HashChain;
use crate::lz77::{Lz77, MAX_DISTANCE, MAX_LENGTH};
use std::io::Read;
use std::io::Result;

const BUF_LEN: usize = 1 << 16; // 64k
const BUF_MASK: usize = BUF_LEN - 1;
const READ_CHUNK_SIZE: usize = 1 << 14; // 16k -- must be a few bytes less than MAX_DISTANCE
const CHAIN_LEN: usize = 1 << 15; // 32k
const CHAIN_MASK: usize = CHAIN_LEN - 1;

pub struct Encoder<R> {
    read: R,
    search_pos: usize,
    cap: usize,
    buf: Vec<u8>,
    hasher: RunningHasher,
    hash_chain: HashChain,
    pos2hash: Vec<u16>,
    verbose: bool,
    state: Option<(usize, usize)>, // length, distance
}

impl<R: Read> Encoder<R> {
    pub fn new(read: R, verbose: bool) -> Self {
        Self {
            read,
            search_pos: 0,
            cap: 0,
            buf: vec![0; BUF_LEN],
            hasher: RunningHasher::new(),
            hash_chain: HashChain::new(),
            pos2hash: vec![0; CHAIN_LEN],
            verbose,
            state: None,
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
        let hash_pos = self.search_pos + 2;
        match hash_pos & BUF_MASK {
            0x0000 => {
                self.hash_chain.prune_table(0x8000..);
                self.hash_chain.prune_chain(0x8000..);
            }
            0x8000 => {
                self.hash_chain.prune_table(0x0000..0x8000);
                self.hash_chain.prune_chain(0x0000..0x8000);
            }
            _ => {}
        }

        let x = self.buf[hash_pos & BUF_MASK];
        if self.verbose {
            eprintln!("update hash for {}", hash_pos);
        }
        let h = self.hasher.update(x);

        let x = (self.search_pos & BUF_MASK) as u16;
        let prev_idx = self.hash_chain.add(h, x);
        self.pos2hash[(self.search_pos) & CHAIN_MASK] = h as u16;
        prev_idx as usize
    }

    fn match_length(&self, begin: usize, end: usize, target: usize) -> usize {
        let mut result = 0;
        for (ix1, ix2) in (begin..end).zip(target..) {
            if self.buf[ix1 & BUF_MASK] != self.buf[ix2 & BUF_MASK] {
                break;
            }
            result += 1;
        }
        result
    }

    /// search for best match that is least greater than best_length
    fn best_match(
        &self,
        mut pos: usize,
        mut best_length: usize,
        mut max_count: usize,
    ) -> (usize, usize) {
        let mut best_distance = 0;
        let mut prev_distance = 0;
        let upper_bound = MAX_LENGTH.min(self.cap - self.search_pos);

        while max_count > 0 && pos != 0 && best_length < upper_bound {
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

            if self.buf[(self.search_pos + best_length) & BUF_MASK]
                == self.buf[(pos + best_length) & BUF_MASK]
            {
                let length = self.match_length(self.search_pos, self.search_pos + upper_bound, pos);
                if length > best_length {
                    best_length = length;
                    best_distance = distance;
                }
            }

            pos = self.hash_chain.get(pos) as usize;
            prev_distance = distance;
            max_count -= 1;
        }

        (best_length, best_distance)
    }

    /// returns length so far, new (length, distance)
    fn better_match(&mut self, length: usize, max_count: usize) -> Option<(usize, usize, usize)> {
        for ix in 1..length {
            let pos = self.advance_hash();
            if ix == 1 {
                let (l, d) = self.best_match(pos, length, max_count);
                self.search_pos += 1;
                if d > 0 {
                    return Some((ix, l, d));
                }
            } else {
                self.search_pos += 1;
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.search_pos
    }

    /// read up n bytes; it will return as soon as it reads >= n bytes
    pub fn read_chunk(&mut self, n: usize, xs: &mut Vec<Lz77>) -> usize {
        xs.clear();
        let begin = self.len();
        while self.len() - begin < n {
            match self.next() {
                Some(x) => xs.push(x),
                None => break,
            }
        }
        self.len() - begin
    }
}

impl<R: Read> Iterator for Encoder<R> {
    type Item = Lz77;

    fn next(&mut self) -> Option<Self::Item> {
        self.fill_buf().unwrap();
        if self.search_pos >= self.cap {
            return None;
        }

        if self.search_pos == 0 {
            self.hasher.update(self.buf[0]);
            self.hasher.update(self.buf[1]);
        }

        let (length, distance) = if self.state.is_some() {
            self.state.take().unwrap()
        } else {
            let pos = self.advance_hash();
            let (l, d) = self.best_match(pos, 3, 1024);
            self.search_pos += 1;
            (l, d)
        };

        if length < 4 {
            let x = self.buf[(self.search_pos - 1) & BUF_MASK];
            return Some(Lz77::Literal(x));
        }

        if let Some((l1, l2, d)) = self.better_match(length, 1024) {
            self.state = Some((l2, d));
            match l1 {
                1 => {
                    let x = self.buf[(self.search_pos - 2) & BUF_MASK];
                    Some(Lz77::Literal(x))
                }
                4.. => Some(Lz77::Dictionary {
                    length: (l1 - 3) as u8,
                    distance: distance as u16,
                }),
                _ => unreachable!(),
            }
        } else {
            Some(Lz77::Dictionary {
                length: (length - 3) as u8,
                distance: distance as u16,
            })
        }
    }
}
