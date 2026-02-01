use crate::{SectionData, lir::func::SsaFunction};

pub mod bit_field_forwarding;
pub mod call_signature_prunning;
pub mod collect_allocations;
pub mod const_folding;
pub mod cse;
pub mod dce;
pub mod def_use;
pub mod detect_jump_tables;
pub mod fill_static_reads;
pub mod promote_known_targets;
pub mod reassociate;
pub mod reconstruct_comp;
pub(crate) mod reconstruct_flag_comp;

pub fn optimise(func: &mut SsaFunction, rdata: SectionData<'_>) {
    func.resolve();

    loop {
        let mut changed = false;

        changed |= const_folding::run(func);
        func.patch_resolve_aliases();

        changed |= reassociate::run(func);
        changed |= cse::run(func);

        func.patch_resolve_aliases();

        changed |= bit_field_forwarding::run(func);

        func.patch_resolve_aliases();

        reconstruct_comp::exec(func);
        changed |= reconstruct_flag_comp::exec(func);
        dce::exec(func);

        fill_static_reads::exec(func, rdata);
        promote_known_targets::run(func);

        if !changed {
            break;
        }
    }
}
