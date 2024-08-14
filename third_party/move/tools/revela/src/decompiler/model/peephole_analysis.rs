// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_model::model::FunctionEnv;

use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Operation},
};
use std::collections::{BTreeMap, HashMap};

pub struct PeepHoleProcessor {
    max_loop: usize,
}

impl PeepHoleProcessor {
    pub fn new(max_loop: usize) -> Box<Self> {
        Box::new(Self { max_loop })
    }
}

impl FunctionTargetProcessor for PeepHoleProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            // Nothing to do
            return data;
        }

        let code = std::mem::take(&mut data.code);

        let code = Self::transform_code(code, self.max_loop);

        data.code = code;
        data
    }

    fn name(&self) -> String {
        "peephole".to_string()
    }
}

impl PeepHoleProcessor {
    // return true if this operation produces no side-effect - so we can
    // optimize it (ie. remove it)
    fn no_side_effect(oper: &Operation) -> bool {
        match oper {
            Operation::Function(..)
            | Operation::WriteRef
            | Operation::Unpack(..)
            | Operation::Pack(..)
            | Operation::MoveTo(_, _, _)
            | Operation::MoveFrom(_, _, _) => false,

            Operation::Exists(_, _, _)
            | Operation::BorrowLoc
            | Operation::BorrowField(_, _, _, _)
            | Operation::BorrowGlobal(_, _, _)
            | Operation::GetField(_, _, _, _)
            | Operation::Drop
            | Operation::ReadRef
            | Operation::FreezeRef(_)
            | Operation::Vector
            | Operation::CastU8
            | Operation::CastU16
            | Operation::CastU32
            | Operation::CastU64
            | Operation::CastU128
            | Operation::Not
            | Operation::Add
            | Operation::Sub
            | Operation::Mul
            | Operation::Div
            | Operation::Mod
            | Operation::BitOr
            | Operation::BitAnd
            | Operation::Xor
            | Operation::Shl
            | Operation::Shr
            | Operation::Lt
            | Operation::Gt
            | Operation::Le
            | Operation::Ge
            | Operation::Or
            | Operation::And
            | Operation::Eq
            | Operation::Neq
            | Operation::CastU256
            | Operation::TestVariant(_, _, _, _)
            | Operation::PackVariant(_, _, _, _)
            | Operation::UnpackVariant(_, _, _, _)
            | Operation::BorrowVariantField(_, _, _, _, _) => true,

            // specification opcode - dont touch it
            Operation::OpaqueCallBegin(..) | Operation::OpaqueCallEnd(..) => false,
            Operation::TraceLocal(_)
            | Operation::TraceReturn(_)
            | Operation::TraceAbort
            | Operation::TraceExp(_, _)
            | Operation::TraceGlobalMem(_)
            | Operation::EmitEvent
            | Operation::EventStoreDiverge
            | Operation::GetGlobal(..)
            | Operation::UnpackRef
            | Operation::PackRef
            | Operation::UnpackRefDeep
            | Operation::PackRefDeep
            | Operation::Stop
            | Operation::Uninit
            | Operation::Release
            | Operation::IsParent(_, _)
            | Operation::WriteBack(_, _)
            | Operation::Havoc(..) => false,
        }
    }

    /**
     * This function assumes that all the jumps are labeled, so all consecutive Assigns, Loads, Calls are within the same basic block
     */
    fn remove_destroy_in_assignments_block(
        changed: &mut bool,
        mut block: Vec<Bytecode>,
    ) -> Vec<Bytecode> {
        fn next_remove(block: &Vec<Bytecode>) -> Option<(usize, Vec<usize>)> {
            let mut instruction_to_remove = HashMap::new();
            let mut removed_dsts = vec![HashMap::<usize, usize>::new(); block.len()];
            for (idx, inst) in block.iter().enumerate() {
                match inst {
                    Bytecode::Assign(_, dest, src, _) => {
                        instruction_to_remove.insert(dest, idx);
                        // src is used, so we cannot remove assignment to it
                        instruction_to_remove.remove(&src);
                    }
                    Bytecode::Load(_, dest, _) => {
                        instruction_to_remove.insert(dest, idx);
                    }
                    Bytecode::Call(_, dest, op, srcs, _) => {
                        if let Operation::Drop = op {
                            //
                            assert!(srcs.len() == 1 && dest.len() == 0);
                            if let Some(prev_idx) = instruction_to_remove.get(&srcs[0]) {
                                let prev_inst = &block[*prev_idx];
                                let can_safe_remove = match prev_inst {
                                    Bytecode::Assign(_, _, _, _) => true,
                                    Bytecode::Load(_, _, _) => true,
                                    Bytecode::Call(_, dst, _, _, _) => {
                                        assert!(!removed_dsts[*prev_idx].contains_key(&srcs[0]));
                                        removed_dsts[*prev_idx].insert(srcs[0], idx);
                                        // only remove the instruction if all the dsts are removed
                                        if dst.len() == removed_dsts[*prev_idx].len() {
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                    _ => false,
                                };
                                if can_safe_remove {
                                    return match prev_inst {
                                        Bytecode::Assign(_, _, _, _) | Bytecode::Load(_, _, _) => {
                                            Some((*prev_idx, vec![idx]))
                                        }
                                        Bytecode::Call(_, _, _, _, _) => Some((
                                            *prev_idx,
                                            removed_dsts[*prev_idx]
                                                .iter()
                                                .map(|(_, v)| *v)
                                                .collect(),
                                        )),
                                        _ => unreachable!(),
                                    };
                                }
                                instruction_to_remove.remove(&srcs[0]);
                            }
                        } else {
                            for d in dest {
                                instruction_to_remove.insert(d, idx);
                            }
                            for src in srcs {
                                instruction_to_remove.remove(src);
                            }
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        while let Some((prev, mut destroy_ops)) = next_remove(&block) {
            destroy_ops.sort();
            destroy_ops.dedup();
            assert!(prev < destroy_ops[0]);
            for destroy in destroy_ops.iter().rev() {
                block.remove(*destroy);
            }
            if match &mut block[prev] {
                Bytecode::Call(_, dst, op, _, _) => {
                    if PeepHoleProcessor::no_side_effect(op) {
                        true
                    } else {
                        dst.clear();
                        false
                    }
                }
                Bytecode::Assign(_, _, _, _) | Bytecode::Load(_, _, _) => true,
                _ => unreachable!(),
            } {
                block.remove(prev);
            }
            *changed = true;
        }
        block
    }

    // remove all Destroy insn that destroys the destination of some instruction above it that is not used in between
    fn remove_destroy(code: Vec<Bytecode>) -> (Vec<Bytecode>, bool) {
        let mut changed = false;

        // Transform code.
        let mut new_code = vec![];

        let mut current_assignment_block = vec![];

        for (_, insn) in code.iter().enumerate() {
            if let Bytecode::Call(..) | Bytecode::Assign(..) | Bytecode::Load(..) = insn {
                current_assignment_block.push(insn.clone());
                continue;
            } else {
                for insn in Self::remove_destroy_in_assignments_block(
                    &mut changed,
                    current_assignment_block.clone(),
                ) {
                    new_code.push(insn);
                }
                current_assignment_block.clear();
            }

            // This instruction should be included
            new_code.push(insn.clone());
        }
        for insn in Self::remove_destroy_in_assignments_block(
            &mut changed,
            current_assignment_block.clone(),
        ) {
            new_code.push(insn);
        }

        (new_code, changed)
    }

    // find all the JUMP right after LABEL
    fn remove_hops(code: Vec<Bytecode>) -> (Vec<Bytecode>, bool) {
        let mut changed = false;

        let mut label_map = BTreeMap::new();

        // find all the pair LABEL & JUMP as consecutive instructions
        for (code_offset, insn) in code.iter().enumerate() {
            if let Bytecode::Jump(_, new_target) = insn {
                if code_offset > 0 {
                    // let offset: u16 = code_offset as u16;

                    let last_insn = &code[code_offset - 1];
                    match last_insn {
                        // pattern: Label(AttrId(30), Label(5)); Jump(AttrId(33), Label(6))
                        Bytecode::Label(_, old_target) => {
                            if old_target != new_target {
                                label_map.insert(old_target.clone(), new_target.clone());
                            }
                        }

                        _ => {}
                    }
                }
            }
        }

        // patch all the branch instructions jumping to old target, to go to new target
        let mut new_code = vec![];

        for insn in code {
            match insn {
                Bytecode::Branch(id, then_label, else_label, cond) => {
                    if label_map.contains_key(&then_label) || label_map.contains_key(&else_label) {
                        changed = true;
                    }

                    let then_new = label_map.get(&then_label).unwrap_or(&then_label);
                    let else_new = label_map.get(&else_label).unwrap_or(&else_label);
                    let insn_new = Bytecode::Branch(id, *then_new, *else_new, cond);

                    new_code.push(insn_new);
                }

                Bytecode::Jump(id, label) => {
                    if label_map.contains_key(&label) {
                        changed = true;
                    }

                    let label_new = label_map.get(&label).unwrap_or(&label);
                    let insn_new = Bytecode::Jump(id, *label_new);

                    new_code.push(insn_new);
                }

                _ => {
                    new_code.push(insn.clone());
                }
            }
        }

        (new_code, changed)
    }

    // Change all the branches with the same else & then branch with a simple jump
    // Branch(AttrId(65), Label(13), Label(13), 23) -> Jump(AttrId(65), Label(13))
    fn patch_branch(code: Vec<Bytecode>) -> (Vec<Bytecode>, bool) {
        let mut changed = false;

        let mut new_code = vec![];

        for insn in &code {
            match insn {
                Bytecode::Branch(id, then_label, else_label, _) if then_label == else_label => {
                    let insn_new = Bytecode::Jump(*id, *then_label);

                    new_code.push(insn_new);

                    changed = true;
                }

                _ => {
                    new_code.push(insn.clone());
                }
            }
        }

        (new_code, changed)
    }

    // find consecutive labels, then change all of them to the last one
    // 17: Label(AttrId(30), Label(5))
    // 18: Label(AttrId(34), Label(4))
    // 19: Label(AttrId(37), Label(6))
    fn patch_sequence_labels(code: Vec<Bytecode>) -> (Vec<Bytecode>, bool) {
        let mut changed = false;

        let mut label_map = BTreeMap::new();

        let mut sequence_length = 0;
        let mut start: usize = 0;

        for (i, insn) in code.iter().enumerate() {
            if let Bytecode::Label(..) = insn {
                if sequence_length == 0 {
                    start = i;
                }
                sequence_length += 1;
            } else {
                if sequence_length > 1 {
                    if let Bytecode::Label(_, label_to) = code[i - 1] {
                        for index in start..i - 1 {
                            if let Bytecode::Label(_, label_from) = code[index] {
                                label_map.insert(label_from, label_to);
                            }
                        }
                    }
                }

                // new sequence
                sequence_length = 0;
            }
        }

        // corner case: the last insn can be a label
        if sequence_length > 1 {
            let i = code.len() - 1;
            if let Bytecode::Label(_, label_to) = code[i] {
                for index in start..i {
                    if let Bytecode::Label(_, label_from) = code[index] {
                        label_map.insert(label_from, label_to);
                    }
                }
            }
        }

        // change labels in all the jumps in a sequence to the last label
        let mut new_code = vec![];

        for insn in &code {
            match insn {
                Bytecode::Branch(id, then_label, else_label, cond) => {
                    if label_map.contains_key(then_label) || label_map.contains_key(else_label) {
                        changed = true;
                    }

                    let then_new = label_map.get(&then_label).unwrap_or(&then_label);
                    let else_new = label_map.get(&else_label).unwrap_or(&else_label);
                    let insn_new = Bytecode::Branch(*id, *then_new, *else_new, *cond);

                    new_code.push(insn_new);
                }

                Bytecode::Jump(id, label) => {
                    if label_map.contains_key(label) {
                        changed = true;
                    }

                    let label_new = label_map.get(&label).unwrap_or(&label);
                    let insn_new = Bytecode::Jump(*id, *label_new);

                    new_code.push(insn_new);
                }

                Bytecode::Label(_, label) => {
                    // if this label has mapping, remove it
                    if label_map.contains_key(label) {
                        // skip this instruction
                        changed = true;
                    } else {
                        new_code.push(insn.clone());
                    }
                }

                _ => {
                    new_code.push(insn.clone());
                }
            }
        }

        (new_code, changed)
    }

    // for consecutive Jumps, only keep the first Jump, but remove all the Jump right after it
    // 9: Jump(AttrId(13), Label(2)); 10: Jump(AttrId(17), Label(2))
    fn remove_sequence_jumps(code: Vec<Bytecode>) -> (Vec<Bytecode>, bool) {
        let mut changed = false;

        let mut new_code = vec![];

        for insn in &code {
            if let Bytecode::Jump(..) = insn {
                if let Some(last_insn) = new_code.last() {
                    match last_insn {
                        Bytecode::Jump(..) => {
                            // 2 consecutive Jumps, so do not keep this instruction
                            changed = true;
                            continue;
                        }

                        _ => {}
                    }
                }
            }

            // This instruction should be included
            new_code.push(insn.clone());
        }

        (new_code, changed)
    }

    // remove all JUMP code that jumps to the label right after it
    fn remove_jump(code: Vec<Bytecode>) -> (Vec<Bytecode>, bool) {
        let mut changed = false;

        let mut new_code = vec![];

        for insn in &code {
            if let Bytecode::Label(_, label) = insn {
                if let Some(last_insn) = new_code.last() {
                    match last_insn {
                        // Jump(AttrId(29), Label(6)); Label(AttrId(37), Label(6))
                        Bytecode::Jump(_, target) if target == label => {
                            // remove the previous Jump
                            new_code.pop();
                            changed = true;
                        }

                        _ => {}
                    }
                }
            }

            // This instruction should be included
            new_code.push(insn.clone());
        }

        (new_code, changed)
    }

    fn remove_labels(code: Vec<Bytecode>) -> (Vec<Bytecode>, bool) {
        let mut changed = false;

        // find all used labels
        let mut used_labels = vec![];

        for insn in &code {
            match insn {
                Bytecode::Branch(_, then_label, else_label, _) => {
                    used_labels.push(then_label);
                    used_labels.push(else_label);
                }

                Bytecode::Jump(_, label) => {
                    used_labels.push(label);
                }

                _ => {}
            }
        }

        // now remove all labels unused
        let mut new_code = vec![];

        for (_, insn) in code.iter().enumerate() {
            match insn {
                Bytecode::Label(_, label) => {
                    // if this label is not used, remove it
                    if used_labels.contains(&label) {
                        new_code.push(insn.clone());
                    } else {
                        // skip this instruction
                        changed = true;
                    }
                }

                _ => {
                    new_code.push(insn.clone());
                }
            }
        }

        (new_code, changed)
    }

    fn transform_code(code: Vec<Bytecode>, max_loop: usize) -> Vec<Bytecode> {
        let mut new_code = code;

        let mut loop_count = 0;

        while loop_count < max_loop {
            loop_count += 1;

            let (updated_code, changed1) = Self::remove_destroy(new_code);
            new_code = updated_code;

            let (updated_code, changed2) = Self::remove_hops(new_code);
            new_code = updated_code;

            let (updated_code, changed3) = Self::patch_branch(new_code);
            new_code = updated_code;

            let (updated_code, changed4) = Self::patch_sequence_labels(new_code);
            new_code = updated_code;

            let (updated_code, changed5) = Self::remove_sequence_jumps(new_code);
            new_code = updated_code;

            let (updated_code, changed6) = Self::remove_jump(new_code);
            new_code = updated_code;

            let (updated_code, changed7) = Self::remove_labels(new_code);
            new_code = updated_code;

            // continue optimizing until a fixed point is reached
            if !changed1
                && !changed2
                && !changed3
                && !changed4
                && !changed5
                && !changed6
                && !changed7
            {
                break;
            }
        }

        new_code
    }
}
