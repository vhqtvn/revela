use std::collections::{HashMap, HashSet};

use move_model::symbol::Symbol;
use move_stackless_bytecode::stackless_bytecode::{AttrId, Bytecode, Label, Operation};

use super::{datastructs::BasicBlock, StacklessBlockContent};

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
        let Bytecode::Call(_, dst, Operation::TestVariant(mid, sid, sym, _), src, _) =
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
            println!("j: {}, curr: {:?}", j, curr);
            println!("bytecode: {:?}", bytecodes[j]);
            println!("true_branches: {:?}", true_branches);
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
                    let Bytecode::Branch(_, true_label, false_label, src) = bytecodes[j] else {
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
