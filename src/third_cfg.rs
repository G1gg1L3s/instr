use crate::{SectionData, addr::Addr, flat_ir};

pub fn walk_code_blocks(text: SectionData<'_>, start: Addr) {
    let code = text.slice_to_end(start);

    let block = flat_ir::lower_block(code, start);
    eprintln!("{block}");
}
