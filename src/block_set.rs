use std::collections::BTreeMap;

use crate::{
    addr::Addr,
    new_cfg::{Block, CodeBlockType},
};

#[derive(Debug)]
pub struct BlockSet {
    blocks: BTreeMap<Addr, Block>,
}

#[derive(Debug, Clone, Copy)]
pub enum SplitError {
    OutOfRange,
    Unalighed,
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

    pub fn get_split(
        &mut self,
        addr: Addr,
        new_type: CodeBlockType,
    ) -> Result<&mut Block, SplitError> {
        let target_addr = self.find_block_with_addr(addr)?;
        if target_addr == addr {
            return Ok(self.blocks.get_mut(&addr).unwrap());
        }

        let block = self.blocks.remove(&target_addr).unwrap();

        let (left, right) = self.split_block_or_insert_back(addr, block, new_type)?;

        let addr = right.addr();
        self.blocks.insert(left.addr(), left);
        self.blocks.insert(addr, right);

        Ok(self.blocks.get_mut(&addr).unwrap())
    }

    fn split_block_or_insert_back(
        &mut self,
        addr: Addr,
        block: Block,
        new_type: CodeBlockType,
    ) -> Result<(Block, Block), SplitError> {
        match block.split(addr, new_type) {
            Ok((left, right)) => Ok((left, right)),
            Err(block) => {
                self.blocks.insert(block.addr(), block);
                return Err(SplitError::Unalighed);
            }
        }
    }

    pub fn remove_split(&mut self, addr: Addr, size: usize) -> Result<Block, SplitError> {
        let block_addr = self.find_block_with_addr(addr)?;
        let mut block = self.blocks.remove(&block_addr).unwrap();
        let typ = if let Block::Code(code) = &block {
            code.typ
        } else {
            CodeBlockType::Filler
        };

        let block_end = block.addr() + u32::try_from(block.size()).unwrap();
        let mut requested_end = addr + u32::try_from(size).unwrap();

        if block_end < requested_end {
            requested_end = block_end;
        }

        if block_addr < addr {
            let (left, right) = self.split_block_or_insert_back(addr, block, typ)?;
            self.blocks.insert(left.addr(), left);
            block = right;
        }

        if requested_end < block_end {
            let (left, right) = self.split_block_or_insert_back(requested_end, block, typ)?;
            self.blocks.insert(right.addr(), right);
            block = left;
        }

        Ok(block)
    }

    fn find_block_with_addr(&mut self, addr: Addr) -> Result<Addr, SplitError> {
        let (target_addr, target) = self
            .blocks
            .range(..=addr)
            .next_back()
            .ok_or(SplitError::OutOfRange)?;
        if *target_addr == addr {
            return Ok(addr);
        }

        if target.contains(addr) {
            Ok(*target_addr)
        } else {
            Err(SplitError::OutOfRange)
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
