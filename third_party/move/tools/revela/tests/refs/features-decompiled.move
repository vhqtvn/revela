module 0x1::features {
    struct Features has key {
        features: vector<u8>,
    }
    
    struct PendingFeatures has key {
        features: vector<u8>,
    }
    
    fun contains(arg0: &vector<u8>, arg1: u64) : bool {
        let v0 = arg1 / 8;
        let v1 = v0 < 0x1::vector::length<u8>(arg0);
        v1 && *0x1::vector::borrow<u8>(arg0, v0) & 1 << ((arg1 % 8) as u8) != 0
    }
    
    public fun abort_if_multisig_payload_mismatch_enabled() : bool acquires Features {
        is_enabled(70)
    }
    
    public fun aggregator_snapshots_enabled() : bool {
        abort 0x1::error::invalid_argument(1)
    }
    
    public fun aggregator_v2_api_enabled() : bool {
        true
    }
    
    public fun aggregator_v2_is_at_least_api_enabled() : bool acquires Features {
        is_enabled(66)
    }
    
    public fun allow_vm_binary_format_v6() : bool acquires Features {
        is_enabled(5)
    }
    
    fun apply_diff(arg0: &mut vector<u8>, arg1: vector<u64>, arg2: vector<u64>) {
        let v0 = arg1;
        0x1::vector::reverse<u64>(&mut v0);
        let v1 = v0;
        let v2 = 0x1::vector::length<u64>(&v1);
        while (v2 > 0) {
            set(arg0, 0x1::vector::pop_back<u64>(&mut v1), true);
            v2 = v2 - 1;
        };
        0x1::vector::destroy_empty<u64>(v1);
        let v3 = arg2;
        0x1::vector::reverse<u64>(&mut v3);
        let v4 = v3;
        v2 = 0x1::vector::length<u64>(&v4);
        while (v2 > 0) {
            set(arg0, 0x1::vector::pop_back<u64>(&mut v4), false);
            v2 = v2 - 1;
        };
        0x1::vector::destroy_empty<u64>(v4);
    }
    
    public fun aptos_stdlib_chain_id_enabled() : bool acquires Features {
        is_enabled(4)
    }
    
    public fun auids_enabled() : bool {
        true
    }
    
    public fun blake2b_256_enabled() : bool acquires Features {
        is_enabled(8)
    }
    
    public fun bls12_381_structures_enabled() : bool acquires Features {
        is_enabled(13)
    }
    
    public fun bn254_structures_enabled() : bool acquires Features {
        is_enabled(43)
    }
    
    public fun bulletproofs_enabled() : bool acquires Features {
        is_enabled(24)
    }
    
    public fun change_feature_flags(arg0: &signer, arg1: vector<u64>, arg2: vector<u64>) {
        abort 0x1::error::invalid_state(2)
    }
    
    public fun change_feature_flags_for_next_epoch(arg0: &signer, arg1: vector<u64>, arg2: vector<u64>) acquires Features, PendingFeatures {
        assert!(0x1::signer::address_of(arg0) == @0x1, 0x1::error::permission_denied(1));
        let v0 = if (exists<PendingFeatures>(@0x1)) {
            let PendingFeatures { features: v0 } = move_from<PendingFeatures>(@0x1);
            v0
        } else {
            if (exists<Features>(@0x1)) {
                borrow_global<Features>(@0x1).features
            } else {
                0x1::vector::empty<u8>()
            }
        };
        apply_diff(&mut v0, arg1, arg2);
        let v1 = PendingFeatures{features: v0};
        move_to<PendingFeatures>(arg0, v1);
    }
    
    fun change_feature_flags_internal(arg0: &signer, arg1: vector<u64>, arg2: vector<u64>) acquires Features {
        assert!(0x1::signer::address_of(arg0) == @0x1, 0x1::error::permission_denied(1));
        if (exists<Features>(@0x1)) {
        } else {
            let v0 = Features{features: 0x1::vector::empty<u8>()};
            move_to<Features>(arg0, v0);
        };
        let v1 = &mut borrow_global_mut<Features>(@0x1).features;
        let v2 = &arg1;
        let v3 = 0;
        while (v3 < 0x1::vector::length<u64>(v2)) {
            let v4 = *0x1::vector::borrow<u64>(v2, v3);
            set(v1, v4, true);
            v3 = v3 + 1;
        };
        let v5 = &arg2;
        v3 = 0;
        while (v3 < 0x1::vector::length<u64>(v5)) {
            set(v1, *0x1::vector::borrow<u64>(v5, v3), false);
            v3 = v3 + 1;
        };
    }
    
    public fun code_dependency_check_enabled() : bool acquires Features {
        is_enabled(1)
    }
    
    public fun coin_to_fungible_asset_migration_feature_enabled() : bool acquires Features {
        is_enabled(60)
    }
    
    public fun collect_and_distribute_gas_fees() : bool acquires Features {
        is_enabled(6)
    }
    
    public fun commission_change_delegation_pool_enabled() : bool acquires Features {
        is_enabled(42)
    }
    
    public fun concurrent_assets_enabled() : bool {
        abort 0x1::error::invalid_argument(3)
    }
    
    public fun concurrent_fungible_assets_enabled() : bool acquires Features {
        is_enabled(50)
    }
    
    public fun concurrent_fungible_balance_enabled() : bool acquires Features {
        is_enabled(67)
    }
    
    public fun concurrent_token_v2_enabled() : bool {
        true
    }
    
    public fun cryptography_algebra_enabled() : bool acquires Features {
        is_enabled(12)
    }
    
    public fun default_to_concurrent_fungible_balance_enabled() : bool acquires Features {
        is_enabled(68)
    }
    
    public fun delegation_pool_allowlisting_enabled() : bool acquires Features {
        is_enabled(56)
    }
    
    public fun delegation_pool_partial_governance_voting_enabled() : bool acquires Features {
        is_enabled(21)
    }
    
    public fun delegation_pools_enabled() : bool acquires Features {
        is_enabled(11)
    }
    
    public fun dispatchable_fungible_asset_enabled() : bool acquires Features {
        is_enabled(63)
    }
    
    fun ensure_framework_signer(arg0: &signer) {
        assert!(0x1::signer::address_of(arg0) == @0x1, 0x1::error::permission_denied(1));
    }
    
    public fun fee_payer_enabled() : bool acquires Features {
        is_enabled(22)
    }
    
    public fun get_abort_if_multisig_payload_mismatch_feature() : u64 {
        70
    }
    
    public fun get_aggregator_snapshots_feature() : u64 {
        abort 0x1::error::invalid_argument(1)
    }
    
    public fun get_aggregator_v2_api_feature() : u64 {
        abort 0x1::error::invalid_argument(3)
    }
    
    public fun get_aptos_stdlib_chain_id_feature() : u64 {
        4
    }
    
    public fun get_auids() : u64 {
        0x1::error::invalid_argument(3)
    }
    
    public fun get_blake2b_256_feature() : u64 {
        8
    }
    
    public fun get_bls12_381_strutures_feature() : u64 {
        13
    }
    
    public fun get_bn254_strutures_feature() : u64 {
        43
    }
    
    public fun get_bulletproofs_feature() : u64 {
        24
    }
    
    public fun get_coin_to_fungible_asset_migration_feature() : u64 {
        60
    }
    
    public fun get_collect_and_distribute_gas_fees_feature() : u64 {
        6
    }
    
    public fun get_commission_change_delegation_pool_feature() : u64 {
        42
    }
    
    public fun get_concurrent_assets_feature() : u64 {
        abort 0x1::error::invalid_argument(3)
    }
    
    public fun get_concurrent_fungible_assets_feature() : u64 {
        50
    }
    
    public fun get_concurrent_fungible_balance_feature() : u64 {
        67
    }
    
    public fun get_concurrent_token_v2_feature() : u64 {
        0x1::error::invalid_argument(3)
    }
    
    public fun get_cryptography_algebra_natives_feature() : u64 {
        12
    }
    
    public fun get_default_to_concurrent_fungible_balance_feature() : u64 {
        68
    }
    
    public fun get_delegation_pool_allowlisting_feature() : u64 {
        56
    }
    
    public fun get_delegation_pool_partial_governance_voting() : u64 {
        21
    }
    
    public fun get_delegation_pools_feature() : u64 {
        11
    }
    
    public fun get_dispatchable_fungible_asset_feature() : u64 {
        63
    }
    
    public fun get_jwk_consensus_feature() : u64 {
        49
    }
    
    public fun get_keyless_accounts_feature() : u64 {
        46
    }
    
    public fun get_keyless_accounts_with_passkeys_feature() : u64 {
        54
    }
    
    public fun get_keyless_but_zkless_accounts_feature() : u64 {
        47
    }
    
    public fun get_max_object_nesting_check_feature() : u64 {
        53
    }
    
    public fun get_module_event_feature() : u64 {
        26
    }
    
    public fun get_module_event_migration_feature() : u64 {
        57
    }
    
    public fun get_multisig_accounts_feature() : u64 {
        10
    }
    
    public fun get_multisig_v2_enhancement_feature() : u64 {
        55
    }
    
    public fun get_new_accounts_default_to_fa_apt_store_feature() : u64 {
        64
    }
    
    public fun get_object_native_derived_address_feature() : u64 {
        62
    }
    
    public fun get_operations_default_to_fa_apt_store_feature() : u64 {
        65
    }
    
    public fun get_operator_beneficiary_change_feature() : u64 {
        39
    }
    
    public fun get_partial_governance_voting() : u64 {
        17
    }
    
    public fun get_periodical_reward_rate_decrease_feature() : u64 {
        16
    }
    
    public fun get_primary_apt_fungible_store_at_user_address_feature() : u64 {
        abort 0x1::error::invalid_argument(1)
    }
    
    public fun get_reconfigure_with_dkg_feature() : u64 {
        45
    }
    
    public fun get_resource_groups_feature() : u64 {
        9
    }
    
    public fun get_sha_512_and_ripemd_160_feature() : u64 {
        3
    }
    
    public fun get_signer_native_format_fix_feature() : u64 {
        25
    }
    
    public fun get_sponsored_automatic_account_creation() : u64 {
        34
    }
    
    public fun get_transaction_context_extension_feature() : u64 {
        59
    }
    
    public fun get_vm_binary_format_v6() : u64 {
        5
    }
    
    public fun is_enabled(arg0: u64) : bool acquires Features {
        exists<Features>(@0x1) && contains(&borrow_global<Features>(@0x1).features, arg0)
    }
    
    public fun is_object_code_deployment_enabled() : bool acquires Features {
        is_enabled(52)
    }
    
    public fun jwk_consensus_enabled() : bool acquires Features {
        is_enabled(49)
    }
    
    public fun keyless_accounts_enabled() : bool acquires Features {
        is_enabled(46)
    }
    
    public fun keyless_accounts_with_passkeys_feature_enabled() : bool acquires Features {
        is_enabled(54)
    }
    
    public fun keyless_but_zkless_accounts_feature_enabled() : bool acquires Features {
        is_enabled(47)
    }
    
    public fun max_object_nesting_check_enabled() : bool acquires Features {
        is_enabled(53)
    }
    
    public fun module_event_enabled() : bool acquires Features {
        is_enabled(26)
    }
    
    public fun module_event_migration_enabled() : bool acquires Features {
        is_enabled(57)
    }
    
    public fun multi_ed25519_pk_validate_v2_enabled() : bool acquires Features {
        is_enabled(7)
    }
    
    public fun multi_ed25519_pk_validate_v2_feature() : u64 {
        7
    }
    
    public fun multisig_accounts_enabled() : bool acquires Features {
        is_enabled(10)
    }
    
    public fun multisig_v2_enhancement_feature_enabled() : bool acquires Features {
        is_enabled(55)
    }
    
    public fun new_accounts_default_to_fa_apt_store_enabled() : bool acquires Features {
        is_enabled(64)
    }
    
    public fun object_native_derived_address_enabled() : bool acquires Features {
        is_enabled(62)
    }
    
    public fun on_new_epoch(arg0: &signer) acquires Features, PendingFeatures {
        ensure_framework_signer(arg0);
        if (exists<PendingFeatures>(@0x1)) {
            let PendingFeatures { features: v0 } = move_from<PendingFeatures>(@0x1);
            if (exists<Features>(@0x1)) {
                borrow_global_mut<Features>(@0x1).features = v0;
            } else {
                let v1 = Features{features: v0};
                move_to<Features>(arg0, v1);
            };
        };
    }
    
    public fun operations_default_to_fa_apt_store_enabled() : bool acquires Features {
        is_enabled(65)
    }
    
    public fun operator_beneficiary_change_enabled() : bool acquires Features {
        is_enabled(39)
    }
    
    public fun partial_governance_voting_enabled() : bool acquires Features {
        is_enabled(17)
    }
    
    public fun periodical_reward_rate_decrease_enabled() : bool acquires Features {
        is_enabled(16)
    }
    
    public fun primary_apt_fungible_store_at_user_address_enabled() : bool acquires Features {
        is_enabled(61)
    }
    
    public fun reconfigure_with_dkg_enabled() : bool acquires Features {
        is_enabled(45)
    }
    
    public fun resource_groups_enabled() : bool acquires Features {
        is_enabled(9)
    }
    
    fun set(arg0: &mut vector<u8>, arg1: u64, arg2: bool) {
        let v0 = arg1 / 8;
        while (0x1::vector::length<u8>(arg0) <= v0) {
            0x1::vector::push_back<u8>(arg0, 0);
        };
        let v1 = 0x1::vector::borrow_mut<u8>(arg0, v0);
        if (arg2) {
            *v1 = *v1 | 1 << ((arg1 % 8) as u8);
        } else {
            *v1 = *v1 & (255 ^ 1 << ((arg1 % 8) as u8));
        };
    }
    
    public fun sha_512_and_ripemd_160_enabled() : bool acquires Features {
        is_enabled(3)
    }
    
    public fun signer_native_format_fix_enabled() : bool acquires Features {
        is_enabled(25)
    }
    
    public fun sponsored_automatic_account_creation_enabled() : bool acquires Features {
        is_enabled(34)
    }
    
    public fun transaction_context_extension_enabled() : bool acquires Features {
        is_enabled(59)
    }
    
    public fun treat_friend_as_private() : bool acquires Features {
        is_enabled(2)
    }
    
    // decompiled from Move bytecode v7
}
