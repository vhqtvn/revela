use std::collections::{HashMap, HashSet};

use move_model::{symbol::Symbol, well_known};
use move_stackless_bytecode::stackless_bytecode::{AttrId, Bytecode, Constant, Label, Operation};

use super::{
    algo::blocks_stackless::{AnnotatedBytecodeData, StacklessBlockContent},
    datastructs::{BasicBlock, JumpType},
    metadata::{WithMetadata, WithMetadataExt},
};

/**
 * Combine the test variants operations into a single operation.
 *
 * Example:
 *         $t4 := test_variant m::CommonFieldsAtDifferentOffset::Bar($t3)
 *         if ($t4) goto L0 else goto L1
 *         label L1
 *         $t5 := test_variant m::CommonFieldsAtDifferentOffset::Balt($t3)
 *         if ($t5) goto L0 else goto L3
 *         label L0
 *
 * Into:
 *         $t4 := test_variant m::CommonFieldsAtDifferentOffset::(Bar|Balt)($t3)
 *         if ($t4) goto L0 else goto L3
 */
pub struct StacklessVariantsTestSimplifier {
    simplified_bytecodes: Vec<Bytecode>,
    bytecode_to_variants: HashMap<AttrId, HashSet<Symbol>>,
}

#[derive(Default, Debug)]
pub struct StacklessVariantTestVariantsData {
    pub variants: Vec<Symbol>,
}

impl StacklessVariantsTestSimplifier {
    pub fn for_bytecodes(bytecodes: &[Bytecode]) -> Self {
        let single_use_test_variant = Self::find_single_use_test_variant_bool_variables(bytecodes);

        let mut simplified_bytecodes = vec![];
        let mut bytecode_to_variants = HashMap::new();

        let mut idx = 0;
        while idx < bytecodes.len() {
            let Some((last_idx, symbols, true_branch, false_branch)) =
                Self::expand_test_variant_chain(bytecodes, idx, &single_use_test_variant)
            else {
                simplified_bytecodes.push(bytecodes[idx].clone());
                idx += 1;
                continue;
            };
            let Bytecode::Call(aid, dst, op, src, None) = bytecodes[idx].clone() else {
                unreachable!();
            };
            simplified_bytecodes.push(Bytecode::Call(aid, dst.clone(), op, src.clone(), None));
            bytecode_to_variants.insert(aid, symbols);
            let Bytecode::Branch(bid, _, _, src) = bytecodes[idx + 1].clone() else {
                unreachable!();
            };
            simplified_bytecodes.push(Bytecode::Branch(
                bid,
                true_branch,
                false_branch,
                src.clone(),
            ));
            idx = last_idx + 1;
        }

        Self {
            simplified_bytecodes,
            bytecode_to_variants,
        }
    }

    pub fn simplified_bytecodes(&self) -> &[Bytecode] {
        &self.simplified_bytecodes
    }

    fn find_single_use_test_variant_bool_variables(bytecodes: &[Bytecode]) -> HashSet<usize> {
        let mut possible = HashSet::new();
        let mut last_test_var = None;
        let mut read_cnt = HashMap::new();
        let mut write_cnt = HashMap::new();
        for bytecode in bytecodes {
            match bytecode {
                Bytecode::Assign(_, dst, src, _) => {
                    *write_cnt.entry(dst).or_insert(0) += 1;
                    *read_cnt.entry(src).or_insert(0) += 1;
                }
                Bytecode::Call(_, dsts, _, srcs, _) => {
                    for d in dsts {
                        *write_cnt.entry(d).or_insert(0) += 1;
                    }
                    for s in srcs {
                        *read_cnt.entry(s).or_insert(0) += 1;
                    }
                }
                Bytecode::Ret(_, srcs) => {
                    for s in srcs {
                        *read_cnt.entry(s).or_insert(0) += 1;
                    }
                }
                Bytecode::Load(_, dst, _) => {
                    *write_cnt.entry(dst).or_insert(0) += 1;
                }
                Bytecode::Abort(_, src) | Bytecode::Branch(_, _, _, src) => {
                    *read_cnt.entry(src).or_insert(0) += 1;
                }
                Bytecode::Jump(_, _)
                | Bytecode::Label(_, _)
                | Bytecode::Nop(_)
                | Bytecode::SpecBlock(_, _)
                | Bytecode::SaveMem(_, _, _)
                | Bytecode::SaveSpecVar(_, _, _)
                | Bytecode::Prop(_, _, _) => {}
            }
            match bytecode {
                Bytecode::Call(_, dst, op, src, _)
                    if dst.len() == 1
                        && src.len() == 1
                        && matches!(op, Operation::TestVariant(..)) =>
                {
                    last_test_var = Some(dst[0]);
                }
                Bytecode::Branch(..) if last_test_var.is_some() => {
                    possible.insert(last_test_var.unwrap());
                    last_test_var = None;
                }
                _ => {
                    last_test_var = None;
                }
            }
        }
        possible
            .into_iter()
            .filter(|v| read_cnt[v] == 1 && write_cnt[v] == 1)
            .collect()
    }

    fn expand_test_variant_chain(
        bytecodes: &[Bytecode],
        idx: usize,
        single_use_test_variant: &HashSet<usize>,
    ) -> Option<(usize, HashSet<Symbol>, Label, Label)> {
        let Bytecode::Call(_, _dst, Operation::TestVariant(mid, sid, sym, _), src, _) =
            &bytecodes[idx]
        else {
            return None;
        };
        #[derive(Clone, Debug)]
        enum ReqOp {
            TestVariant,
            Branch,
            Label,
        }
        let mut curr = ReqOp::Branch;
        let mut j = idx + 1;
        let mut test_variants = HashSet::new();
        test_variants.insert(sym.clone());
        let mut true_branches = HashSet::new();
        let mut last_false_branch = None;
        while j < bytecodes.len() && true_branches.len() <= 1 {
            match curr.clone() {
                ReqOp::TestVariant => {
                    let Bytecode::Call(
                        _,
                        cdst,
                        Operation::TestVariant(cmid, csid, csym, _),
                        csrc,
                        _,
                    ) = &bytecodes[j]
                    else {
                        break;
                    };
                    if cdst.len() != 1 || csrc.len() != 1 {
                        break;
                    }
                    if !single_use_test_variant.contains(&cdst[0]) {
                        break;
                    }
                    if mid != cmid || sid != csid || src != csrc {
                        break;
                    }
                    test_variants.insert(csym.clone());
                    curr = ReqOp::Branch;
                }
                ReqOp::Branch => {
                    let Bytecode::Branch(_, true_label, false_label, _src) = bytecodes[j] else {
                        break;
                    };
                    last_false_branch = Some(false_label);
                    true_branches.insert(true_label);
                    curr = ReqOp::Label;
                }
                ReqOp::Label => {
                    let Bytecode::Label(_, label) = bytecodes[j] else {
                        break;
                    };
                    if true_branches.contains(&label) && last_false_branch.is_some() {
                        return Some((
                            j - 1,
                            test_variants,
                            true_branches.into_iter().next().unwrap(),
                            last_false_branch.unwrap(),
                        ));
                    }
                    if last_false_branch.is_some() && last_false_branch.unwrap() != label {
                        break;
                    }
                    curr = ReqOp::TestVariant;
                    last_false_branch = None;
                }
            }
            j += 1;
        }
        return None;
    }

    pub(crate) fn annotate_variants(
        &self,
        blocks: &mut [BasicBlock<usize, StacklessBlockContent>],
    ) {
        for block in blocks {
            for bytecode in &mut block.content.code {
                let attr_id = bytecode.bytecode.get_attr_id();
                if let Some(variants) = self.bytecode_to_variants.get(&attr_id) {
                    bytecode
                        .meta_mut()
                        .get_or_default::<StacklessVariantTestVariantsData>()
                        .variants = variants.into_iter().cloned().collect();
                }
            }
        }
    }
}

fn is_match_abort_block(code: &[WithMetadata<AnnotatedBytecodeData>]) -> Option<(Label, usize)> {
    if code.len() != 3 {
        return None;
    }
    let (
        Bytecode::Label(_, label),
        Bytecode::Load(_, load_var, Constant::U64(well_known::INCOMPLETE_MATCH_ABORT_CODE)),
        Bytecode::Abort(_, abort_var),
    ) = (&code[0].bytecode, &code[1].bytecode, &code[2].bytecode)
    else {
        return None;
    };
    if load_var != abort_var {
        return None;
    }
    Some((label.clone(), *load_var))
}

fn find_next_free_label_idx(
    blocks: &[WithMetadata<
        BasicBlock<usize, super::algo::blocks_stackless::StacklessBlockContent>,
    >],
) -> usize {
    let mut max_label = 0;
    for block in blocks {
        for bytecode in &block.content.code {
            if let Bytecode::Label(_, label) = bytecode.bytecode {
                max_label = max_label.max(label.as_usize());
            }
        }
    }
    max_label + 1
}

fn find_next_free_attr_id(
    blocks: &[WithMetadata<
        BasicBlock<usize, super::algo::blocks_stackless::StacklessBlockContent>,
    >],
) -> usize {
    let mut max_attr_id = 0;
    for block in blocks {
        for bytecode in &block.content.code {
            max_attr_id = max_attr_id.max(bytecode.bytecode.get_attr_id().as_usize());
        }
    }
    max_attr_id + 1
}

fn create_match_abort_block(
    next_idx: &mut usize,
    next_label_idx: &mut usize,
    next_attr_id: &mut usize,
    variable_to_use: usize,
) -> (
    usize,
    WithMetadata<BasicBlock<usize, StacklessBlockContent>>,
    Label,
) {
    let label = Label::new(*next_label_idx);
    *next_label_idx += 1;
    let base_attr_id = *next_attr_id;
    *next_attr_id += 3;
    let idx = *next_idx;
    *next_idx += 1;
    let mut basic_block: BasicBlock<usize, StacklessBlockContent> = BasicBlock::new(idx);
    basic_block.content.code = vec![
        (AnnotatedBytecodeData {
            removed: false,
            jump_type: JumpType::Unknown,
            original_offset: base_attr_id,
            bytecode: Bytecode::Label(AttrId::new(base_attr_id), label.clone()),
        })
        .with_metadata(),
        (AnnotatedBytecodeData {
            removed: false,
            jump_type: JumpType::Unknown,
            original_offset: base_attr_id + 1,
            bytecode: Bytecode::Load(
                AttrId::new(base_attr_id + 1),
                variable_to_use,
                Constant::U64(well_known::INCOMPLETE_MATCH_ABORT_CODE),
            ),
        })
        .with_metadata(),
        (AnnotatedBytecodeData {
            removed: false,
            jump_type: JumpType::Unknown,
            original_offset: base_attr_id + 2,
            bytecode: Bytecode::Abort(AttrId::new(base_attr_id + 2), variable_to_use),
        })
        .with_metadata(),
    ];
    basic_block.next = super::datastructs::Terminator::Abort;
    (idx, basic_block.with_metadata(), label)
}

pub fn duplicate_match_aborts(
    blocks: &mut Vec<WithMetadata<BasicBlock<usize, StacklessBlockContent>>>,
) -> Result<(), anyhow::Error> {
    let mut match_aborts_idx = HashSet::new();
    struct MatchAbortInfo {
        label: Label,
        variable: usize,
    }
    let mut match_aborts_info = HashMap::new();
    for (idx, block) in blocks.iter().enumerate() {
        assert!(block.idx == idx);
        if let Some((label, variable)) = is_match_abort_block(&block.content.code) {
            match_aborts_idx.insert(block.idx);
            match_aborts_info.insert(block.idx, MatchAbortInfo { label, variable });
        }
    }
    let mut match_abort_usages = HashMap::new();
    let mut insert_index = usize::MAX;
    for block in blocks.iter() {
        match &block.next {
            super::datastructs::Terminator::Normal => {
                // abort blocks can only be "jumped" to
                assert!(!match_aborts_idx.contains(&(block.idx + 1)));
            }
            super::datastructs::Terminator::IfElse {
                if_block,
                else_block,
            } => {
                assert!(if_block != else_block);
                if match_aborts_idx.contains(&if_block.target) {
                    match_abort_usages
                        .entry(if_block.target)
                        .or_insert_with(Vec::new)
                        .push(block.idx);
                }
                if match_aborts_idx.contains(&else_block.target) {
                    match_abort_usages
                        .entry(else_block.target)
                        .or_insert_with(Vec::new)
                        .push(block.idx);
                }
            }
            super::datastructs::Terminator::Branch { target } => {
                if match_aborts_idx.contains(&target) {
                    match_abort_usages
                        .entry(*target)
                        .or_insert_with(Vec::new)
                        .push(block.idx);
                }
            }
            super::datastructs::Terminator::While { .. } 
            | super::datastructs::Terminator::Break { .. }
            | super::datastructs::Terminator::Continue { .. } => {
                unreachable!()
            }
            super::datastructs::Terminator::Ret => {}
            super::datastructs::Terminator::Abort => {
                insert_index = block.idx + 1;
            }
        }
    }

    let insert_size = match_abort_usages.iter().fold(0, |acc, (_, usages)| {
        acc + if usages.len() > 1 {
            usages.len() - 1
        } else {
            0
        }
    });

    if insert_size == 0 {
        return Ok(());
    }

    assert!(insert_index != usize::MAX);

    fn update_index(idx: &mut usize, break_index: usize, insert_size: usize) {
        if *idx >= break_index {
            *idx += insert_size;
        }
    }

    for block in blocks.iter_mut() {
        if block.idx >= insert_index {
            block.idx += insert_size;
        }
        match &mut block.next {
            crate::decompiler::cfg::datastructs::Terminator::IfElse {
                if_block,
                else_block,
            } => {
                update_index(&mut if_block.target, insert_index, insert_size);
                update_index(&mut else_block.target, insert_index, insert_size);
            }
            crate::decompiler::cfg::datastructs::Terminator::Branch { target } => {
                update_index(target, insert_index, insert_size);
            }
            crate::decompiler::cfg::datastructs::Terminator::While { .. } |
            crate::decompiler::cfg::datastructs::Terminator::Break { .. } |
            crate::decompiler::cfg::datastructs::Terminator::Continue { .. } => unreachable!(),
            crate::decompiler::cfg::datastructs::Terminator::Normal |
            crate::decompiler::cfg::datastructs::Terminator::Ret |
            crate::decompiler::cfg::datastructs::Terminator::Abort => {}
        }
    }

    let mut next_idx_ptr = insert_index;
    let mut next_label_idx_ptr = find_next_free_label_idx(blocks);
    let mut next_attr_id_ptr = find_next_free_attr_id(blocks);

    let mut new_blocks = Vec::new();

    for (mut idx, mut usages) in match_abort_usages {
        if usages.len() <= 1 {
            continue;
        }
        usages.pop();
        let abort_info = match_aborts_info.get(&idx).expect("match abort block");
        update_index(&mut idx, insert_index, insert_size);
        for block_idx in usages {
            let (next_idx, new_abort_block, label) = create_match_abort_block(
                &mut next_idx_ptr,
                &mut next_label_idx_ptr,
                &mut next_attr_id_ptr,
                abort_info.variable,
            );
            enum UpdatedAt {
                IfBlock,
                ElseBlock,
                Branch,
            }
            let mut updated_at = None;
            match &mut blocks[block_idx].next {
                super::datastructs::Terminator::IfElse {
                    if_block,
                    else_block,
                } => {
                    assert!(if_block.target == idx || else_block.target == idx);
                    if if_block.target == idx {
                        if_block.target = next_idx;
                        updated_at = Some(UpdatedAt::IfBlock);
                    }
                    if else_block.target == idx {
                        else_block.target = next_idx;
                        updated_at = Some(UpdatedAt::ElseBlock);
                    }
                }
                super::datastructs::Terminator::Branch { target } => {
                    assert!(*target == idx);
                    *target = next_idx;
                    updated_at = Some(UpdatedAt::Branch);
                }
                _ => unreachable!(),
            }
            match updated_at {
                Some(UpdatedAt::IfBlock) => {
                    let last_op = &mut blocks[block_idx].content.code.last_mut().unwrap().bytecode;
                    if let Bytecode::Branch(_, true_label, _, _) = last_op {
                        assert!(*true_label == abort_info.label);
                        *true_label = label.clone();
                    } else {
                        unreachable!();
                    }
                }
                Some(UpdatedAt::ElseBlock) => {
                    let last_op = &mut blocks[block_idx].content.code.last_mut().unwrap().bytecode;
                    if let Bytecode::Branch(_, _, false_label, _) = last_op {
                        assert!(*false_label == abort_info.label);
                        *false_label = label.clone();
                    } else {
                        unreachable!();
                    }
                }
                Some(UpdatedAt::Branch) => {
                    let last_op = &mut blocks[block_idx].content.code.last_mut().unwrap().bytecode;
                    if let Bytecode::Jump(_, label) = last_op {
                        assert!(*label == abort_info.label);
                        *label = label.clone();
                    } else {
                        unreachable!();
                    }
                }
                None => unreachable!(),
            }
            new_blocks.push(new_abort_block);
        }
    }

    blocks.splice(insert_index..insert_index, new_blocks);

    Ok(())
}
