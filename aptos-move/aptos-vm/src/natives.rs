// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
use anyhow::Error;
#[cfg(feature = "testing")]
use aptos_aggregator::resolver::{AggregatorReadMode, TAggregatorView};
#[cfg(feature = "testing")]
use aptos_framework::natives::{cryptography::algebra::AlgebraContext, event::NativeEventContext};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_native_interface::SafeNativeBuilder;
#[cfg(feature = "testing")]
use aptos_table_natives::{TableHandle, TableResolver};
#[cfg(feature = "testing")]
use aptos_types::aggregator::AggregatorID;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    on_chain_config::{Features, TimedFeatures, TimedFeaturesBuilder},
};
#[cfg(feature = "testing")]
use aptos_types::{
    chain_id::ChainId,
    state_store::{state_key::StateKey, state_value::StateValue},
};
#[cfg(feature = "testing")]
use bytes::Bytes;
use move_vm_runtime::native_functions::NativeFunctionTable;
#[cfg(feature = "testing")]
use {
    aptos_framework::natives::{
        aggregator_natives::NativeAggregatorContext, code::NativeCodeContext,
        cryptography::ristretto255_point::NativeRistrettoPointContext,
        transaction_context::NativeTransactionContext,
    },
    move_vm_runtime::native_extensions::NativeContextExtensions,
    once_cell::sync::Lazy,
};

#[cfg(feature = "testing")]
struct AptosBlankStorage;

#[cfg(feature = "testing")]
impl TAggregatorView for AptosBlankStorage {
    type IdentifierV1 = StateKey;
    type IdentifierV2 = AggregatorID;

    fn get_aggregator_v1_state_value(
        &self,
        _id: &Self::IdentifierV1,
        _mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        Ok(None)
    }
}

#[cfg(feature = "testing")]
impl TableResolver for AptosBlankStorage {
    fn resolve_table_entry(
        &self,
        _handle: &TableHandle,
        _key: &[u8],
    ) -> Result<Option<Bytes>, Error> {
        Ok(None)
    }
}

#[cfg(feature = "testing")]
static DUMMY_RESOLVER: Lazy<AptosBlankStorage> = Lazy::new(|| AptosBlankStorage);

pub fn aptos_natives(
    gas_feature_version: u64,
    native_gas_params: NativeGasParameters,
    misc_gas_params: MiscGasParameters,
    timed_features: TimedFeatures,
    features: Features,
) -> NativeFunctionTable {
    let mut builder = SafeNativeBuilder::new(
        gas_feature_version,
        native_gas_params,
        misc_gas_params,
        timed_features,
        features,
    );

    aptos_natives_with_builder(&mut builder)
}

pub fn aptos_natives_with_builder(builder: &mut SafeNativeBuilder) -> NativeFunctionTable {
    #[allow(unreachable_code)]
    aptos_move_stdlib::natives::all_natives(CORE_CODE_ADDRESS, builder)
        .into_iter()
        .filter(|(_, name, _, _)| name.as_str() != "vector")
        .chain(aptos_framework::natives::all_natives(
            CORE_CODE_ADDRESS,
            builder,
        ))
        .chain(aptos_table_natives::table_natives(
            CORE_CODE_ADDRESS,
            builder,
        ))
        .collect()
}

pub fn assert_no_test_natives(err_msg: &str) {
    assert!(
        aptos_natives(
            LATEST_GAS_FEATURE_VERSION,
            NativeGasParameters::zeros(),
            MiscGasParameters::zeros(),
            TimedFeaturesBuilder::enable_all().build(),
            Features::default()
        )
        .into_iter()
        .all(|(_, module_name, func_name, _)| {
            !(module_name.as_str() == "unit_test"
                && func_name.as_str() == "create_signers_for_testing"
                || module_name.as_str() == "ed25519"
                    && func_name.as_str() == "generate_keys_internal"
                || module_name.as_str() == "ed25519" && func_name.as_str() == "sign_internal"
                || module_name.as_str() == "multi_ed25519"
                    && func_name.as_str() == "generate_keys_internal"
                || module_name.as_str() == "multi_ed25519" && func_name.as_str() == "sign_internal"
                || module_name.as_str() == "bls12381"
                    && func_name.as_str() == "generate_keys_internal"
                || module_name.as_str() == "bls12381" && func_name.as_str() == "sign_internal"
                || module_name.as_str() == "bls12381"
                    && func_name.as_str() == "generate_proof_of_possession_internal"
                || module_name.as_str() == "event"
                    && func_name.as_str() == "emitted_events_internal")
        }),
        "{}",
        err_msg
    )
}

#[cfg(feature = "testing")]
pub fn configure_for_unit_test() {
    move_unit_test::extensions::set_extension_hook(Box::new(unit_test_extensions_hook))
}

#[cfg(feature = "testing")]
fn unit_test_extensions_hook(exts: &mut NativeContextExtensions) {
    use aptos_table_natives::NativeTableContext;

    exts.add(NativeTableContext::new([0u8; 32], &*DUMMY_RESOLVER));
    exts.add(NativeCodeContext::default());
    exts.add(NativeTransactionContext::new(
        vec![1],
        vec![1],
        ChainId::test().id(),
    )); // We use the testing environment chain ID here
    exts.add(NativeAggregatorContext::new([0; 32], &*DUMMY_RESOLVER));
    exts.add(NativeRistrettoPointContext::new());
    exts.add(AlgebraContext::new());
    exts.add(NativeEventContext::default());
}
