use std::{collections::HashMap, hash::Hash};

use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use crate::{
    Binary, DataObject, DataObjectKind,
    addr::Addr,
    cfg::{Block, BlockType, looks_like_jump_table},
    ins::{self, Op},
};

pub fn promote_functions_based_on_object_function_refs(
    objects: &[DataObject],
    blocks: &mut [Block],
) {
    for obj in objects {
        let DataObjectKind::FunctionsRef(addr) = &obj.kind else {
            continue;
        };

        let Some(block) = get_block_mut(blocks, *addr) else {
            continue;
        };

        block.set_typ(BlockType::Function);
    }
}

fn get_block(blocks: &[Block], addr: Addr) -> Option<&Block> {
    blocks
        .binary_search_by(|b| b.addr().cmp(&addr))
        .ok()
        .and_then(|idx| blocks.get(idx))
}

fn get_block_mut(blocks: &mut [Block], addr: Addr) -> Option<&mut Block> {
    blocks
        .binary_search_by(|b| b.addr().cmp(&addr))
        .ok()
        .and_then(|idx| blocks.get_mut(idx))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FunctionAddr(Addr);

#[derive(PartialEq, Eq)]
enum EndFlowType {
    Break,
    Fallthrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeType {
    FuncEntry,
    Jump,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    index: NodeIndex,
    #[allow(unused)]
    typ: NodeType,
}

pub struct GraphFunctionCollector {
    digraph: DiGraph<Addr, ()>,
    addr_to_node: HashMap<Addr, Node>,
}

impl GraphFunctionCollector {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            digraph: DiGraph::with_capacity(capacity, 0),
            addr_to_node: HashMap::with_capacity(capacity),
        }
    }

    pub fn insert_nodes(&mut self, blocks: &[Block]) {
        for block in blocks {
            if let BlockType::Entry | BlockType::Function | BlockType::Jump = block.typ() {
                let index = self.digraph.add_node(block.addr());
                let typ = if let BlockType::Entry | BlockType::Function = block.typ() {
                    NodeType::FuncEntry
                } else {
                    NodeType::Jump
                };
                self.addr_to_node.insert(block.addr(), Node { index, typ });
            }
        }
    }

    pub fn insert_edges(&mut self, binary: &Binary<'_>, blocks: &[Block]) {
        let mut last_flow = EndFlowType::Break;
        let mut last_block = blocks[0].addr();

        for block in blocks {
            if let BlockType::Entry | BlockType::Function | BlockType::Jump = block.typ() {
                if last_flow == EndFlowType::Fallthrough {
                    let last_node = &self.addr_to_node[&last_block];
                    let this_node = &self.addr_to_node[&block.addr()];
                    self.digraph.add_edge(last_node.index, this_node.index, ());
                }

                last_flow = self.insert_edges_from_block(binary, *block, blocks);
                last_block = block.addr();
            } else {
                last_flow = EndFlowType::Break;
            }
        }
    }

    fn insert_edges_from_block(
        &mut self,
        binary: &Binary<'_>,
        block: Block,
        blocks: &[Block],
    ) -> EndFlowType {
        use ins::Instruction as Ins;

        let code = binary.sections.text.slice(block.addr(), block.size());
        let decoder = ins::Decoder::new(code, block.addr());

        let this_node = self.addr_to_node.get(&block.addr()).unwrap();

        for ins in decoder {
            match ins.instr {
                Ins::Return(_) => return EndFlowType::Break,
                Ins::Jump(Op::Addr(addr)) => {
                    if let Some(block) = get_block(blocks, addr)
                        && block.typ() == BlockType::Jump
                    {
                        let node = self.addr_to_node[&addr];
                        self.digraph.add_edge(this_node.index, node.index, ());
                    }

                    return EndFlowType::Break;
                }
                Ins::JumpConditional(Op::Addr(addr), _) | Ins::Loop(addr) => {
                    if let Some(block) = get_block(blocks, addr)
                        && block.typ() == BlockType::Jump
                    {
                        let node = self.addr_to_node[&addr];
                        self.digraph.add_edge(this_node.index, node.index, ());
                    }
                }
                Ins::Jump(Op::Mem(mem)) | Ins::JumpConditional(Op::Mem(mem), _)
                    if looks_like_jump_table(&mem) =>
                {
                    let Some(jump_table) = get_block(blocks, Addr(mem.disp)) else {
                        continue;
                    };

                    let Some(jump_table_entries) = jump_table.jump_targets(binary) else {
                        continue;
                    };

                    for jump_entry in jump_table_entries {
                        if let Some(node) = self.addr_to_node.get(&jump_entry.target) {
                            self.digraph.add_edge(this_node.index, node.index, ());
                        }
                    }

                    if let Ins::Jump(_) = ins.instr {
                        return EndFlowType::Break;
                    }
                }
                Ins::Int3 => return EndFlowType::Break,
                Ins::Invalid => return EndFlowType::Break,
                _ => {}
            }
        }

        EndFlowType::Fallthrough
    }

    pub fn find_blocks_shared_between_functions(&self, blocks: &[Block]) -> Vec<Addr> {
        let block_to_func = self.assign_blocks_to_functions(blocks);

        block_to_func
            .iter()
            .filter_map(|(addr, functions)| {
                if functions.len() > 1 {
                    Some(*addr)
                } else {
                    None
                }
            })
            .collect()
    }

    fn assign_blocks_to_functions(&self, blocks: &[Block]) -> HashMap<Addr, Vec<FunctionAddr>> {
        let mut block_to_func = HashMap::with_capacity(blocks.len());

        for block in blocks {
            if !matches!(block.typ(), BlockType::Entry | BlockType::Function) {
                continue;
            };

            self.process_function(&mut block_to_func, FunctionAddr(block.addr()), block.addr());
        }

        block_to_func
    }

    fn process_function(
        &self,
        block_to_func: &mut HashMap<Addr, Vec<FunctionAddr>>,
        func_addr: FunctionAddr,
        block_addr: Addr,
    ) {
        if let Some(funcs) = block_to_func.get_mut(&block_addr) {
            if !funcs.contains(&func_addr) {
                funcs.push(func_addr);
            }
            return;
        }

        block_to_func.insert(block_addr, vec![func_addr]);

        let idx = &self.addr_to_node[&block_addr];

        for edge in self
            .digraph
            .edges_directed(idx.index, petgraph::Direction::Outgoing)
        {
            let target = self.digraph.node_weight(edge.target()).unwrap();

            self.process_function(block_to_func, func_addr, *target);
        }
    }

    pub fn promote_functions_based_on_tail_calls(&self, blocks: &mut [Block]) {
        let funcs = self.find_blocks_shared_between_functions(blocks);

        for func_addr in funcs {
            get_block_mut(blocks, func_addr)
                .unwrap()
                .set_typ(BlockType::Function);
        }
    }

    pub fn print_block_to_func(&self, blocks: &[Block]) {
        let block_to_func = self
            .assign_blocks_to_functions(blocks)
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();

        for (addr, funcs) in block_to_func {
            if funcs.len() > 1 {
                println!("  - {addr} <- {funcs:?}");
            }
        }
    }
}
