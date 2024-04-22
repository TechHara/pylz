use std::ops::RangeBounds;

use crate::hash::TABLE_SIZE;

const CHAIN_LEN: u16 = 1 << 15; // 32k
const CHAIN_MASK: u16 = CHAIN_LEN - 1;

pub struct HashChain {
    table: Vec<u16>,
    chain: Vec<u16>,
}

impl HashChain {
    pub fn new() -> Self {
        Self {
            table: vec![0; TABLE_SIZE],
            chain: vec![0; CHAIN_LEN as usize],
        }
    }

    /// hash: masked hash
    /// x: unmasked new item
    pub fn add(&mut self, hash: usize, x: u16) -> u16 {
        let prev = self.table[hash];
        self.table[hash] = x;
        self.chain[(x & CHAIN_MASK) as usize] = prev;
        prev
    }

    pub fn get(&self, x: usize) -> u16 {
        self.chain[x & (CHAIN_MASK as usize)]
    }

    /// remove any entries if not in the range
    pub fn prune_table<R: RangeBounds<u16>>(&mut self, range: R) {
        for x in &mut self.table {
            if !range.contains(x) {
                *x = 0;
            }
        }
    }

    pub fn prune_chain<R: RangeBounds<u16>>(&mut self, range: R) {
        for x in &mut self.chain {
            if !range.contains(x) {
                *x = 0;
            }
        }
    }
}
