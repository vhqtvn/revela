module 0x1::randomness_api_v0_config {
    struct AllowCustomMaxGasFlag has drop, store, key {
        value: bool,
    }
    
    struct RequiredGasDeposit has drop, store, key {
        gas_amount: 0x1::option::Option<u64>,
    }
    
    fun initialize(arg0: &signer, arg1: RequiredGasDeposit, arg2: AllowCustomMaxGasFlag) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        move_to<RequiredGasDeposit>(arg0, arg1);
        move_to<AllowCustomMaxGasFlag>(arg0, arg2);
    }
    
    public fun on_new_epoch(arg0: &signer) acquires AllowCustomMaxGasFlag, RequiredGasDeposit {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::config_buffer::does_exist<RequiredGasDeposit>()) {
            if (exists<RequiredGasDeposit>(@0x1)) {
                *borrow_global_mut<RequiredGasDeposit>(@0x1) = 0x1::config_buffer::extract<RequiredGasDeposit>();
            } else {
                move_to<RequiredGasDeposit>(arg0, 0x1::config_buffer::extract<RequiredGasDeposit>());
            };
        };
        if (0x1::config_buffer::does_exist<AllowCustomMaxGasFlag>()) {
            let v0 = 0x1::config_buffer::extract<AllowCustomMaxGasFlag>();
            if (exists<AllowCustomMaxGasFlag>(@0x1)) {
                *borrow_global_mut<AllowCustomMaxGasFlag>(@0x1) = v0;
            } else {
                move_to<AllowCustomMaxGasFlag>(arg0, v0);
            };
        };
    }
    
    public fun set_allow_max_gas_flag_for_next_epoch(arg0: &signer, arg1: bool) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = AllowCustomMaxGasFlag{value: arg1};
        0x1::config_buffer::upsert<AllowCustomMaxGasFlag>(v0);
    }
    
    public fun set_for_next_epoch(arg0: &signer, arg1: 0x1::option::Option<u64>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = RequiredGasDeposit{gas_amount: arg1};
        0x1::config_buffer::upsert<RequiredGasDeposit>(v0);
    }
    
    // decompiled from Move bytecode v7
}
