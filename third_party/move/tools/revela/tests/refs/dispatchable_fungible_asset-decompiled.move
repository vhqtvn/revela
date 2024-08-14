module 0x1::dispatchable_fungible_asset {
    struct TransferRefStore has key {
        transfer_ref: 0x1::fungible_asset::TransferRef,
    }

    public fun register_derive_supply_dispatch_function(arg0: &0x1::object::ConstructorRef, arg1: 0x1::option::Option<0x1::function_info::FunctionInfo>) {
        0x1::fungible_asset::register_derive_supply_dispatch_function(arg0, arg1);
    }

    public fun register_dispatch_functions(arg0: &0x1::object::ConstructorRef, arg1: 0x1::option::Option<0x1::function_info::FunctionInfo>, arg2: 0x1::option::Option<0x1::function_info::FunctionInfo>, arg3: 0x1::option::Option<0x1::function_info::FunctionInfo>) {
        0x1::fungible_asset::register_dispatch_functions(arg0, arg1, arg2, arg3);
        let v0 = 0x1::object::generate_signer(arg0);
        let v1 = TransferRefStore{transfer_ref: 0x1::fungible_asset::generate_transfer_ref(arg0)};
        move_to<TransferRefStore>(&v0, v1);
    }

    public fun deposit<T0: key>(arg0: 0x1::object::Object<T0>, arg1: 0x1::fungible_asset::FungibleAsset) acquires TransferRefStore {
        0x1::fungible_asset::deposit_sanity_check<T0>(arg0, false);
        let v0 = 0x1::fungible_asset::deposit_dispatch_function<T0>(arg0);
        if (0x1::option::is_some<0x1::function_info::FunctionInfo>(&v0)) {
            assert!(0x1::features::dispatchable_fungible_asset_enabled(), 0x1::error::aborted(3));
            let v1 = 0x1::option::borrow<0x1::function_info::FunctionInfo>(&v0);
            0x1::function_info::load_module_from_function(v1);
            let v2 = 0x1::fungible_asset::store_metadata<T0>(arg0);
            let v3 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v2);
            assert!(exists<TransferRefStore>(v3), 0x1::error::not_found(1));
            dispatchable_deposit<T0>(arg0, arg1, &borrow_global<TransferRefStore>(v3).transfer_ref, v1);
        } else {
            0x1::fungible_asset::deposit_internal(0x1::object::object_address<T0>(&arg0), arg1);
        };
    }

    public fun derived_balance<T0: key>(arg0: 0x1::object::Object<T0>) : u64 {
        let v0 = 0x1::fungible_asset::derived_balance_dispatch_function<T0>(arg0);
        if (0x1::option::is_some<0x1::function_info::FunctionInfo>(&v0)) {
            assert!(0x1::features::dispatchable_fungible_asset_enabled(), 0x1::error::aborted(3));
            let v2 = 0x1::option::borrow<0x1::function_info::FunctionInfo>(&v0);
            0x1::function_info::load_module_from_function(v2);
            dispatchable_derived_balance<T0>(arg0, v2)
        } else {
            0x1::fungible_asset::balance<T0>(arg0)
        }
    }

    public fun derived_supply<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::option::Option<u128> {
        let v0 = 0x1::fungible_asset::derived_supply_dispatch_function<T0>(arg0);
        if (0x1::option::is_some<0x1::function_info::FunctionInfo>(&v0)) {
            assert!(0x1::features::dispatchable_fungible_asset_enabled(), 0x1::error::aborted(3));
            let v2 = 0x1::option::borrow<0x1::function_info::FunctionInfo>(&v0);
            0x1::function_info::load_module_from_function(v2);
            dispatchable_derived_supply<T0>(arg0, v2)
        } else {
            0x1::fungible_asset::supply<T0>(arg0)
        }
    }

    native fun dispatchable_deposit<T0: key>(arg0: 0x1::object::Object<T0>, arg1: 0x1::fungible_asset::FungibleAsset, arg2: &0x1::fungible_asset::TransferRef, arg3: &0x1::function_info::FunctionInfo);
    native fun dispatchable_derived_balance<T0: key>(arg0: 0x1::object::Object<T0>, arg1: &0x1::function_info::FunctionInfo) : u64;
    native fun dispatchable_derived_supply<T0: key>(arg0: 0x1::object::Object<T0>, arg1: &0x1::function_info::FunctionInfo) : 0x1::option::Option<u128>;
    native fun dispatchable_withdraw<T0: key>(arg0: 0x1::object::Object<T0>, arg1: u64, arg2: &0x1::fungible_asset::TransferRef, arg3: &0x1::function_info::FunctionInfo) : 0x1::fungible_asset::FungibleAsset;
    public entry fun transfer<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>, arg2: 0x1::object::Object<T0>, arg3: u64) acquires TransferRefStore {
        let v0 = withdraw<T0>(arg0, arg1, arg3);
        deposit<T0>(arg2, v0);
    }

    public entry fun transfer_assert_minimum_deposit<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>, arg2: 0x1::object::Object<T0>, arg3: u64, arg4: u64) acquires TransferRefStore {
        let v0 = withdraw<T0>(arg0, arg1, arg3);
        deposit<T0>(arg2, v0);
        let v1 = 0x1::fungible_asset::balance<T0>(arg2) - 0x1::fungible_asset::balance<T0>(arg2) >= arg4;
        assert!(v1, 0x1::error::aborted(2));
    }

    public fun withdraw<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>, arg2: u64) : 0x1::fungible_asset::FungibleAsset acquires TransferRefStore {
        0x1::fungible_asset::withdraw_sanity_check<T0>(arg0, arg1, false);
        let v0 = 0x1::fungible_asset::withdraw_dispatch_function<T0>(arg1);
        if (0x1::option::is_some<0x1::function_info::FunctionInfo>(&v0)) {
            assert!(0x1::features::dispatchable_fungible_asset_enabled(), 0x1::error::aborted(3));
            let v2 = 0x1::option::borrow<0x1::function_info::FunctionInfo>(&v0);
            0x1::function_info::load_module_from_function(v2);
            let v3 = 0x1::fungible_asset::store_metadata<T0>(arg1);
            let v4 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v3);
            assert!(exists<TransferRefStore>(v4), 0x1::error::not_found(1));
            let v5 = arg2 <= 0x1::fungible_asset::balance<T0>(arg1) - 0x1::fungible_asset::balance<T0>(arg1);
            assert!(v5, 0x1::error::aborted(2));
            dispatchable_withdraw<T0>(arg1, arg2, &borrow_global<TransferRefStore>(v4).transfer_ref, v2)
        } else {
            0x1::fungible_asset::withdraw_internal(0x1::object::object_address<T0>(&arg1), arg2)
        }
    }

    // decompiled from Move bytecode v7
}
