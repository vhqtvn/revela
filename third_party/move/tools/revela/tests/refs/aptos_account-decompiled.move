module 0x1::aptos_account {
    struct DirectCoinTransferConfigUpdated has drop, store {
        account: address,
        new_allow_direct_transfers: bool,
    }

    struct DirectCoinTransferConfigUpdatedEvent has drop, store {
        new_allow_direct_transfers: bool,
    }

    struct DirectTransferConfig has key {
        allow_arbitrary_coin_transfers: bool,
        update_coin_transfer_events: 0x1::event::EventHandle<DirectCoinTransferConfigUpdatedEvent>,
    }

    public entry fun create_account(arg0: address) {
        let v0 = 0x1::account::create_account(arg0);
        register_apt(&v0);
    }

    public entry fun transfer(arg0: &signer, arg1: address, arg2: u64) {
        if (0x1::account::exists_at(arg1)) {
        } else {
            create_account(arg1);
        };
        if (0x1::features::operations_default_to_fa_apt_store_enabled()) {
            fungible_transfer_only(arg0, arg1, arg2);
        } else {
            if (0x1::coin::is_account_registered<0x1::aptos_coin::AptosCoin>(arg1)) {
            } else {
                let v0 = 0x1::create_signer::create_signer(arg1);
                0x1::coin::register<0x1::aptos_coin::AptosCoin>(&v0);
            };
            0x1::coin::transfer<0x1::aptos_coin::AptosCoin>(arg0, arg1, arg2);
        };
    }

    public fun assert_account_exists(arg0: address) {
        assert!(0x1::account::exists_at(arg0), 0x1::error::not_found(1));
    }

    public fun assert_account_is_registered_for_apt(arg0: address) {
        assert_account_exists(arg0);
        let v0 = 0x1::coin::is_account_registered<0x1::aptos_coin::AptosCoin>(arg0);
        assert!(v0, 0x1::error::not_found(2));
    }

    public entry fun batch_transfer(arg0: &signer, arg1: vector<address>, arg2: vector<u64>) {
        let v0 = 0x1::vector::length<address>(&arg1) == 0x1::vector::length<u64>(&arg2);
        assert!(v0, 0x1::error::invalid_argument(5));
        let v1 = &arg1;
        let v2 = 0;
        while (v2 < 0x1::vector::length<address>(v1)) {
            transfer(arg0, *0x1::vector::borrow<address>(v1, v2), *0x1::vector::borrow<u64>(&arg2, v2));
            v2 = v2 + 1;
        };
    }

    public entry fun batch_transfer_coins<T0>(arg0: &signer, arg1: vector<address>, arg2: vector<u64>) acquires DirectTransferConfig {
        let v0 = 0x1::vector::length<address>(&arg1) == 0x1::vector::length<u64>(&arg2);
        assert!(v0, 0x1::error::invalid_argument(5));
        let v1 = &arg1;
        let v2 = 0;
        while (v2 < 0x1::vector::length<address>(v1)) {
            let v3 = *0x1::vector::borrow<address>(v1, v2);
            transfer_coins<T0>(arg0, v3, *0x1::vector::borrow<u64>(&arg2, v2));
            v2 = v2 + 1;
        };
    }

    public(friend) fun burn_from_fungible_store(arg0: &0x1::fungible_asset::BurnRef, arg1: address, arg2: u64) {
        if (arg2 != 0) {
            let v0 = 0x1::object::create_user_derived_object_address(arg1, @0xa);
            0x1::fungible_asset::address_burn_from(arg0, v0, arg2);
        };
    }

    public fun can_receive_direct_coin_transfers(arg0: address) : bool acquires DirectTransferConfig {
        let v0 = exists<DirectTransferConfig>(arg0);
        v0 && borrow_global<DirectTransferConfig>(arg0).allow_arbitrary_coin_transfers || true
    }

    public fun deposit_coins<T0>(arg0: address, arg1: 0x1::coin::Coin<T0>) acquires DirectTransferConfig {
        if (!0x1::account::exists_at(arg0)) {
            create_account(arg0);
        };
        if (!0x1::coin::is_account_registered<T0>(arg0)) {
            assert!(can_receive_direct_coin_transfers(arg0), 0x1::error::permission_denied(3));
            let v0 = 0x1::create_signer::create_signer(arg0);
            0x1::coin::register<T0>(&v0);
        };
        0x1::coin::deposit<T0>(arg0, arg1);
    }

    fun fungible_transfer_only(arg0: &signer, arg1: address, arg2: u64) {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = 0x1::object::create_user_derived_object_address(v0, @0xa);
        let v2 = if (0x1::fungible_asset::store_exists(v1)) {
            v1
        } else {
            let v3 = 0x1::object::address_to_object<0x1::fungible_asset::Metadata>(@0xa);
            let v4 = 0x1::primary_fungible_store::create_primary_store<0x1::fungible_asset::Metadata>(v0, v3);
            0x1::object::object_address<0x1::fungible_asset::FungibleStore>(&v4)
        };
        let v5 = 0x1::object::create_user_derived_object_address(arg1, @0xa);
        let v6 = if (0x1::fungible_asset::store_exists(v5)) {
            v5
        } else {
            let v7 = 0x1::object::address_to_object<0x1::fungible_asset::Metadata>(@0xa);
            let v8 = 0x1::primary_fungible_store::create_primary_store<0x1::fungible_asset::Metadata>(arg1, v7);
            0x1::object::object_address<0x1::fungible_asset::FungibleStore>(&v8)
        };
        0x1::fungible_asset::deposit_internal(v6, 0x1::fungible_asset::withdraw_internal(v2, arg2));
    }

    public(friend) fun is_fungible_balance_at_least(arg0: address, arg1: u64) : bool {
        let v0 = 0x1::object::create_user_derived_object_address(arg0, @0xa);
        0x1::fungible_asset::is_address_balance_at_least(v0, arg1)
    }

    public(friend) fun register_apt(arg0: &signer) {
        if (0x1::features::new_accounts_default_to_fa_apt_store_enabled()) {
            let v0 = 0x1::signer::address_of(arg0);
            if (0x1::fungible_asset::store_exists(0x1::object::create_user_derived_object_address(v0, @0xa))) {
            } else {
                let v1 = 0x1::object::address_to_object<0x1::fungible_asset::Metadata>(@0xa);
                let v2 = 0x1::primary_fungible_store::create_primary_store<0x1::fungible_asset::Metadata>(v0, v1);
                0x1::object::object_address<0x1::fungible_asset::FungibleStore>(&v2);
            };
        } else {
            0x1::coin::register<0x1::aptos_coin::AptosCoin>(arg0);
        };
    }

    public entry fun set_allow_direct_coin_transfers(arg0: &signer, arg1: bool) acquires DirectTransferConfig {
        let v0 = 0x1::signer::address_of(arg0);
        if (exists<DirectTransferConfig>(v0)) {
            let v1 = borrow_global_mut<DirectTransferConfig>(v0);
            if (v1.allow_arbitrary_coin_transfers == arg1) {
                return
            };
            v1.allow_arbitrary_coin_transfers = arg1;
            if (0x1::features::module_event_migration_enabled()) {
                let v2 = DirectCoinTransferConfigUpdated{
                    account                    : v0,
                    new_allow_direct_transfers : arg1,
                };
                0x1::event::emit<DirectCoinTransferConfigUpdated>(v2);
            };
            let v3 = &mut v1.update_coin_transfer_events;
            let v4 = DirectCoinTransferConfigUpdatedEvent{new_allow_direct_transfers: arg1};
            0x1::event::emit_event<DirectCoinTransferConfigUpdatedEvent>(v3, v4);
        } else {
            let v5 = 0x1::account::new_event_handle<DirectCoinTransferConfigUpdatedEvent>(arg0);
            let v6 = DirectTransferConfig{
                allow_arbitrary_coin_transfers : arg1,
                update_coin_transfer_events    : v5,
            };
            if (0x1::features::module_event_migration_enabled()) {
                let v7 = DirectCoinTransferConfigUpdated{
                    account                    : v0,
                    new_allow_direct_transfers : arg1,
                };
                0x1::event::emit<DirectCoinTransferConfigUpdated>(v7);
            };
            let v8 = &mut v6.update_coin_transfer_events;
            let v9 = DirectCoinTransferConfigUpdatedEvent{new_allow_direct_transfers: arg1};
            0x1::event::emit_event<DirectCoinTransferConfigUpdatedEvent>(v8, v9);
            move_to<DirectTransferConfig>(arg0, v6);
        };
    }

    public entry fun transfer_coins<T0>(arg0: &signer, arg1: address, arg2: u64) acquires DirectTransferConfig {
        deposit_coins<T0>(arg1, 0x1::coin::withdraw<T0>(arg0, arg2));
    }

    // decompiled from Move bytecode v7
}
