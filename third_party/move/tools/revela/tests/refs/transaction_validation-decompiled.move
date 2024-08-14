module 0x1::transaction_validation {
    struct TransactionValidation has key {
        module_addr: address,
        module_name: vector<u8>,
        script_prologue_name: vector<u8>,
        module_prologue_name: vector<u8>,
        multi_agent_prologue_name: vector<u8>,
        user_epilogue_name: vector<u8>,
    }

    fun epilogue(arg0: signer, arg1: u64, arg2: u64, arg3: u64, arg4: u64) {
        epilogue_gas_payer(arg0, 0x1::signer::address_of(&arg0), arg1, arg2, arg3, arg4);
    }

    fun epilogue_gas_payer(arg0: signer, arg1: address, arg2: u64, arg3: u64, arg4: u64, arg5: u64) {
        assert!(arg4 >= arg5, 0x1::error::invalid_argument(6));
        let v0 = arg4 - arg5;
        assert!((arg3 as u128) * (v0 as u128) <= 18446744073709551615, 0x1::error::out_of_range(6));
        let v1 = arg3 * v0;
        if (0x1::features::operations_default_to_fa_apt_store_enabled()) {
            assert!(0x1::aptos_account::is_fungible_balance_at_least(arg1, v1), 0x1::error::out_of_range(1005));
        } else {
            let v2 = 0x1::coin::is_balance_at_least<0x1::aptos_coin::AptosCoin>(arg1, v1);
            assert!(v2, 0x1::error::out_of_range(1005));
        };
        if (0x1::features::collect_and_distribute_gas_fees()) {
            0x1::transaction_fee::collect_fee(arg1, v1);
            arg4 = 0;
        } else {
            arg4 = v1;
        };
        if (arg4 > arg2) {
            0x1::transaction_fee::burn_fee(arg1, arg4 - arg2);
        } else if (arg4 < arg2) {
            0x1::transaction_fee::mint_and_refund(arg1, arg2 - arg4);
        };
        0x1::account::increment_sequence_number(0x1::signer::address_of(&arg0));
    }

    fun fee_payer_script_prologue(arg0: signer, arg1: u64, arg2: vector<u8>, arg3: vector<address>, arg4: vector<vector<u8>>, arg5: address, arg6: vector<u8>, arg7: u64, arg8: u64, arg9: u64, arg10: u8) {
        assert!(0x1::features::fee_payer_enabled(), 0x1::error::invalid_state(1010));
        prologue_common(arg0, arg5, arg1, arg2, arg7, arg8, arg9, arg10);
        multi_agent_common_prologue(arg3, arg4);
        assert!(arg6 == 0x1::account::get_authentication_key(arg5), 0x1::error::invalid_argument(1001));
    }

    public(friend) fun initialize(arg0: &signer, arg1: vector<u8>, arg2: vector<u8>, arg3: vector<u8>, arg4: vector<u8>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = TransactionValidation{
            module_addr               : @0x1,
            module_name               : b"transaction_validation",
            script_prologue_name      : arg1,
            module_prologue_name      : arg2,
            multi_agent_prologue_name : arg3,
            user_epilogue_name        : arg4,
        };
        move_to<TransactionValidation>(arg0, v0);
    }

    fun multi_agent_common_prologue(arg0: vector<address>, arg1: vector<vector<u8>>) {
        let v0 = 0x1::vector::length<address>(&arg0);
        assert!(0x1::vector::length<vector<u8>>(&arg1) == v0, 0x1::error::invalid_argument(1009));
        let v1 = 0;
        while (v1 < v0) {
            let v2 = *0x1::vector::borrow<address>(&arg0, v1);
            assert!(0x1::account::exists_at(v2), 0x1::error::invalid_argument(1004));
            let v3 = *0x1::vector::borrow<vector<u8>>(&arg1, v1) == 0x1::account::get_authentication_key(v2);
            assert!(v3, 0x1::error::invalid_argument(1001));
            v1 = v1 + 1;
        };
    }

    fun multi_agent_script_prologue(arg0: signer, arg1: u64, arg2: vector<u8>, arg3: vector<address>, arg4: vector<vector<u8>>, arg5: u64, arg6: u64, arg7: u64, arg8: u8) {
        prologue_common(arg0, 0x1::signer::address_of(&arg0), arg1, arg2, arg5, arg6, arg7, arg8);
        multi_agent_common_prologue(arg3, arg4);
    }

    fun prologue_common(arg0: signer, arg1: address, arg2: u64, arg3: vector<u8>, arg4: u64, arg5: u64, arg6: u64, arg7: u8) {
        assert!(0x1::timestamp::now_seconds() < arg6, 0x1::error::invalid_argument(1006));
        assert!(0x1::chain_id::get() == arg7, 0x1::error::invalid_argument(1007));
        let v0 = 0x1::signer::address_of(&arg0);
        let v1 = v0 == arg1 || 0x1::account::exists_at(v0);
        if (v1) {
            v1 = true;
        } else {
            v1 = !0x1::features::sponsored_automatic_account_creation_enabled();
        };
        if (v1) {
            v1 = true;
        } else {
            v1 = arg2 > 0;
        };
        if (v1) {
            assert!(0x1::account::exists_at(v0), 0x1::error::invalid_argument(1004));
            assert!(arg3 == 0x1::account::get_authentication_key(v0), 0x1::error::invalid_argument(1001));
            let v2 = 0x1::account::get_sequence_number(v0);
            assert!(arg2 < 9223372036854775808, 0x1::error::out_of_range(1008));
            assert!(arg2 >= v2, 0x1::error::invalid_argument(1002));
            assert!(arg2 == v2, 0x1::error::invalid_argument(1003));
        } else {
            assert!(arg2 == 0, 0x1::error::invalid_argument(1003));
            assert!(arg3 == 0x1::bcs::to_bytes<address>(&v0), 0x1::error::invalid_argument(1001));
        };
        if (0x1::features::operations_default_to_fa_apt_store_enabled()) {
            let v3 = 0x1::aptos_account::is_fungible_balance_at_least(arg1, arg4 * arg5);
            assert!(v3, 0x1::error::invalid_argument(1005));
        } else {
            let v4 = 0x1::coin::is_balance_at_least<0x1::aptos_coin::AptosCoin>(arg1, arg4 * arg5);
            assert!(v4, 0x1::error::invalid_argument(1005));
        };
    }

    fun script_prologue(arg0: signer, arg1: u64, arg2: vector<u8>, arg3: u64, arg4: u64, arg5: u64, arg6: u8, arg7: vector<u8>) {
        prologue_common(arg0, 0x1::signer::address_of(&arg0), arg1, arg2, arg3, arg4, arg5, arg6);
    }

    // decompiled from Move bytecode v7
}
