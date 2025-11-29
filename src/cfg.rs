use std::collections::BTreeMap;

use crate::{Binary, RDataObject, RDataObjectKind, addr::Addr};

pub fn derive_blocks(binary: &Binary<'_>) {
    let mut to_process = derive_functions_from_rdata(&binary.robjects);
    to_process.push(binary.entry_point);

    let mut processed = BTreeMap::<Addr, ()>::new();

    while let Some(addr) = to_process.pop() {
        if processed.contains_key(&addr) {
            continue;
        }

        let block = binary.sections.data.slice_to_end(addr);
    }
}

pub fn derive_blocks_from_code(code: &[u8]) {
    todo!()
}

pub fn derive_functions_from_rdata(robjects: &[RDataObject]) -> Vec<Addr> {
    let mut result = Vec::with_capacity(robjects.len() / 4);
    for obj in robjects {
        if let RDataObjectKind::FunctionsRef(addr) = &obj.kind {
            result.push(*addr);
        }
    }
    result
}
