use std::ops::{Index, IndexMut};

use crate::lir::{
    ins::{InsId, Terminator},
    value::ValueId,
};

#[derive(Debug, Clone)]
pub struct Blocks(Vec<Block>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(u16);

#[derive(Debug, Clone)]
pub struct Block {
    pub id: BlockId,
    pub params: Vec<ValueId>,
    pub ins: Vec<InsId>,
    pub predecessors: Vec<BlockId>,
    pub terminator: Option<Terminator>,
}

impl Block {
    pub fn terminator(&self) -> &Terminator {
        self.terminator.as_ref().unwrap()
    }

    pub fn terminator_mut(&mut self) -> &mut Terminator {
        self.terminator.as_mut().unwrap()
    }
}

impl Blocks {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self) -> BlockId {
        let id = self.0.len().try_into().expect("to much Blocks");
        self.0.push(Block {
            id: BlockId(id),
            params: vec![],
            ins: vec![],
            predecessors: vec![],
            terminator: None,
        });
        BlockId(id)
    }

    pub fn keys(&self) -> impl Iterator<Item = BlockId> + use<> {
        let max = self.0.len().try_into().unwrap();
        BlockKeys(0..max)
    }

    pub fn iter(&self) -> impl Iterator<Item = (BlockId, &Block)> {
        self.keys().zip(self.0.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (BlockId, &mut Block)> {
        let keys = self.keys();
        keys.zip(self.0.iter_mut())
    }
}

#[derive(Debug, Clone)]
pub struct BlockKeys(std::ops::Range<u16>);

impl Iterator for BlockKeys {
    type Item = BlockId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(BlockId)
    }
}

impl Index<BlockId> for Blocks {
    type Output = Block;

    fn index(&self, index: BlockId) -> &Self::Output {
        self.0.index(usize::from(index.0))
    }
}

impl IndexMut<BlockId> for Blocks {
    fn index_mut(&mut self, index: BlockId) -> &mut Self::Output {
        self.0.index_mut(usize::from(index.0))
    }
}

impl std::fmt::Debug for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "block{}", self.0)
    }
}

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
