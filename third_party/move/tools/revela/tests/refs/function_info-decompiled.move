module 0x1::function_info {
    struct FunctionInfo has copy, drop, store {
        module_address: address,
        module_name: 0x1::string::String,
        function_name: 0x1::string::String,
    }

    public(friend) fun check_dispatch_type_compatibility(arg0: &FunctionInfo, arg1: &FunctionInfo) : bool {
        assert!(0x1::features::dispatchable_fungible_asset_enabled(), 0x1::error::aborted(3));
        load_function_impl(arg1);
        check_dispatch_type_compatibility_impl(arg0, arg1)
    }

    native fun check_dispatch_type_compatibility_impl(arg0: &FunctionInfo, arg1: &FunctionInfo) : bool;
    native fun is_identifier(arg0: &vector<u8>) : bool;
    native fun load_function_impl(arg0: &FunctionInfo);
    public(friend) fun load_module_from_function(arg0: &FunctionInfo) {
        load_function_impl(arg0);
    }

    public fun new_function_info(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String) : FunctionInfo {
        new_function_info_from_address(0x1::signer::address_of(arg0), arg1, arg2)
    }

    public(friend) fun new_function_info_from_address(arg0: address, arg1: 0x1::string::String, arg2: 0x1::string::String) : FunctionInfo {
        assert!(is_identifier(0x1::string::bytes(&arg1)), 1);
        assert!(is_identifier(0x1::string::bytes(&arg2)), 1);
        FunctionInfo{
            module_address : arg0,
            module_name    : arg1,
            function_name  : arg2,
        }
    }

    // decompiled from Move bytecode v7
}
