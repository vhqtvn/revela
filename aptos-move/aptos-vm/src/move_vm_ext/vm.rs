// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::NativeCodeContext;
use crate::{
    move_vm_ext::{MoveResolverExt, NativeTransactionContext, SessionExt, SessionId},
    natives::aptos_natives,
};
use move_deps::{
    move_binary_format::errors::VMResult,
    move_table_extension::NativeTableContext,
    move_vm_runtime::{move_vm::MoveVM, native_extensions::NativeContextExtensions},
};
use std::ops::Deref;

pub struct MoveVmExt {
    inner: MoveVM,
}

impl MoveVmExt {
    pub fn new() -> VMResult<Self> {
        Ok(Self {
            inner: MoveVM::new(aptos_natives())?,
        })
    }

    pub fn new_session<'r, S: MoveResolverExt>(
        &self,
        remote: &'r S,
        session_id: SessionId,
    ) -> SessionExt<'r, '_, S> {
        let mut extensions = NativeContextExtensions::default();
        extensions.add(NativeTableContext::new(session_id.as_uuid(), remote));

        let script_hash = match session_id {
            SessionId::Txn {
                sender: _,
                sequence_number: _,
                script_hash,
            } => script_hash,
            _ => vec![],
        };
        extensions.add(NativeTransactionContext::new(script_hash));
        extensions.add(NativeCodeContext::default());

        SessionExt::new(self.inner.new_session_with_extensions(remote, extensions))
    }
}

impl Deref for MoveVmExt {
    type Target = MoveVM;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
