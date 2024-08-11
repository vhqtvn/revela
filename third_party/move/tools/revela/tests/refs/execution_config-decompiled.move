module 0x1::execution_config {
    struct ExecutionConfig has drop, store, key {
        config: vector<u8>,
    }
    
    public(friend) fun on_new_epoch(arg0: &signer) acquires ExecutionConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::config_buffer::does_exist<ExecutionConfig>()) {
            if (exists<ExecutionConfig>(@0x1)) {
                *borrow_global_mut<ExecutionConfig>(@0x1) = 0x1::config_buffer::extract<ExecutionConfig>();
            } else {
                move_to<ExecutionConfig>(arg0, 0x1::config_buffer::extract<ExecutionConfig>());
            };
        };
    }
    
    public fun set(arg0: &signer, arg1: vector<u8>) acquires ExecutionConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        assert!(0x1::vector::length<u8>(&arg1) > 0, 0x1::error::invalid_argument(1));
        if (exists<ExecutionConfig>(@0x1)) {
            borrow_global_mut<ExecutionConfig>(@0x1).config = arg1;
        } else {
            let v0 = ExecutionConfig{config: arg1};
            move_to<ExecutionConfig>(arg0, v0);
        };
        0x1::reconfiguration::reconfigure();
    }
    
    public fun set_for_next_epoch(arg0: &signer, arg1: vector<u8>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(0x1::vector::length<u8>(&arg1) > 0, 0x1::error::invalid_argument(1));
        let v0 = ExecutionConfig{config: arg1};
        0x1::config_buffer::upsert<ExecutionConfig>(v0);
    }
    
    // decompiled from Move bytecode v6
}
