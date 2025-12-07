use std::collections::BTreeMap;

use crate::{addr::Addr, cfg::Block};

#[derive(Debug)]
pub struct BlockSet {
    blocks: BTreeMap<Addr, Block>,
}

#[derive(Debug, Clone, Copy)]
pub enum SplitError {
    OutOfRange,
    Unalighed,
}

impl Default for BlockSet {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockSet {
    pub fn new() -> Self {
        Self {
            blocks: Default::default(),
        }
    }

    pub fn contains_block_with_addr(&self, addr: Addr) -> bool {
        self.blocks.contains_key(&addr)
    }

    pub fn insert_non_overlaping_unchecked(&mut self, block: Block) {
        self.blocks.insert(block.addr(), block);
    }

    /// Insert without any checks -- caller must ensure correctness.
    pub fn insert_unchecked(&mut self, block: Block) {
        self.blocks.insert(block.addr(), block);
    }

    /// Return immutable covering block
    fn covering_block(&self, addr: Addr) -> Option<&Block> {
        let (_, block) = self.blocks.range(..=addr).next_back()?;
        if block.contains(addr) {
            Some(block)
        } else {
            None
        }
    }

    /// Return mutable covering block
    fn covering_block_mut(&mut self, addr: Addr) -> Option<(Addr, &mut Block)> {
        let key = {
            let (start, block) = self.blocks.range(..=addr).next_back()?;
            if block.contains(addr) {
                *start
            } else {
                return None;
            }
        };
        let block = self.blocks.get_mut(&key).unwrap();
        Some((key, block))
    }

    /// Split the block that contains `addr`.
    ///
    /// - If no block covers it → None
    /// - If addr == block.start → no split → None
    /// - Otherwise splits using Block::split()
    ///
    /// On success returns mutable reference to the NEW right block.
    pub fn split_at(&mut self, addr: Addr) -> Option<&mut Block> {
        let (start_addr, old_block) = self.covering_block_mut(addr)?;

        if addr == old_block.addr() {
            return self.blocks.get_mut(&start_addr);
        }

        let old_block_copy = *old_block; // we must own it because split() consumes self

        // Remove original block
        self.blocks.remove(&start_addr).unwrap();

        // Use Block::split()
        let (left, right) = old_block_copy.split(addr, old_block_copy.typ()).unwrap();

        // Insert both halves
        self.blocks.insert(left.addr(), left);
        self.blocks.insert(right.addr(), right);

        // Return mutable reference to right block
        self.blocks.get_mut(&addr)
    }

    /// Insert a block, removing/splitting any overlapping blocks.
    pub fn insert_clean(&mut self, new: Block) {
        let new_start = new.addr();
        let new_end = new.end_addr();

        self.split_range(new_start, new.size());

        // ---- Remove blocks fully inside the new block range ----
        let to_remove: Vec<Addr> = self
            .blocks
            .range(new_start..new_end)
            .map(|(k, _)| *k)
            .collect();

        for k in to_remove {
            self.blocks.remove(&k);
        }

        // ---- Insert the new block ----
        self.blocks.insert(new.addr(), new);
    }

    /// Ensures that the interval [start, end) is perfectly aligned in the BlockSet.
    ///
    /// This means:
    /// - If a block crosses `start`, it is split at `start`
    /// - If a block crosses `end`, it is split at `end`
    ///
    /// This keeps all data; no block is removed.
    ///
    /// Returns:
    /// - (Option<&mut Block>, Option<&mut Block>)
    ///     references to the RIGHT fragments created by splitting at start/end.
    pub fn split_range(&mut self, start: Addr, size: usize) {
        let end = start + u32::try_from(size).unwrap();

        if let Some(block) = self.covering_block(start)
            && block.contains(start) && block.addr() != start {
                self.split_at(start);
            }

        if let Some(block) = self.covering_block(end)
            && block.contains(end) && block.addr() != end {
                self.split_at(end);
            }
    }
}

#[derive(Debug)]
pub struct Iter {
    inner: <BTreeMap<Addr, Block> as IntoIterator>::IntoIter,
}

impl IntoIterator for BlockSet {
    type Item = Block;

    type IntoIter = Iter;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            inner: self.blocks.into_iter(),
        }
    }
}

impl Iterator for Iter {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, block)| block)
    }
}
