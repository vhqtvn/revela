module 0x1::consensus_config {
    struct ConsensusConfig has drop, store, key {
        config: vector<u8>,
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: vector<u8>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(0x1::vector::length<u8>(&arg1) > 0, 0x1::error::invalid_argument(1));
        let v0 = ConsensusConfig{config: arg1};
        move_to<ConsensusConfig>(arg0, v0);
    }
    
    public(friend) fun on_new_epoch(arg0: &signer) acquires ConsensusConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::config_buffer::does_exist<ConsensusConfig>()) {
            if (exists<ConsensusConfig>(@0x1)) {
                *borrow_global_mut<ConsensusConfig>(@0x1) = 0x1::config_buffer::extract<ConsensusConfig>();
            } else {
                move_to<ConsensusConfig>(arg0, 0x1::config_buffer::extract<ConsensusConfig>());
            };
        };
    }
    
    public fun set(arg0: &signer, arg1: vector<u8>) acquires ConsensusConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        assert!(0x1::vector::length<u8>(&arg1) > 0, 0x1::error::invalid_argument(1));
        borrow_global_mut<ConsensusConfig>(@0x1).config = arg1;
        0x1::reconfiguration::reconfigure();
    }
    
    public fun set_for_next_epoch(arg0: &signer, arg1: vector<u8>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(0x1::vector::length<u8>(&arg1) > 0, 0x1::error::invalid_argument(1));
        let v0 = ConsensusConfig{config: arg1};
        0x1::config_buffer::upsert<ConsensusConfig>(v0);
    }
    
    public fun validator_txn_enabled() : bool acquires ConsensusConfig {
        validator_txn_enabled_internal(borrow_global<ConsensusConfig>(@0x1).config)
    }
    
    native fun validator_txn_enabled_internal(arg0: vector<u8>) : bool;
    // decompiled from Move bytecode v6
}
