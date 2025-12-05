use crate::{
    DataObject, DataObjectKind,
    cfg::{Block, BlockType},
};

pub fn promote_functions_based_on_object_function_refs(
    objects: &[DataObject],
    blocks: &mut [Block],
) {
    for obj in objects {
        let DataObjectKind::FunctionsRef(addr) = &obj.kind else {
            continue;
        };

        let Ok(block_idx) = blocks.binary_search_by(|b| b.addr().cmp(addr)) else {
            continue;
        };

        blocks[block_idx].set_typ(BlockType::Function);
    }
}

pub fn promote_functions_based_on_fillers(blocks: &mut [Block]) {
    blocks[0].set_typ(BlockType::Function);

    let mut last = BlockType::Function;

    for block in blocks {
        if last == BlockType::Filler {
            block.set_typ(BlockType::Function);
        }
        last = block.typ()
    }
}
