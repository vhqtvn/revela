module 0x1::gas_schedule {
    struct GasEntry has copy, drop, store {
        key: 0x1::string::String,
        val: u64,
    }
    
    struct GasSchedule has copy, drop, key {
        entries: vector<GasEntry>,
    }
    
    struct GasScheduleV2 has copy, drop, store, key {
        feature_version: u64,
        entries: vector<GasEntry>,
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: vector<u8>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::vector::is_empty<u8>(&arg1)) {
            abort 0x1::error::invalid_argument(1)
        };
        move_to<GasScheduleV2>(arg0, 0x1::util::from_bytes<GasScheduleV2>(arg1));
    }
    
    public(friend) fun on_new_epoch(arg0: &signer) acquires GasScheduleV2 {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::config_buffer::does_exist<GasScheduleV2>()) {
            if (exists<GasScheduleV2>(@0x1)) {
                *borrow_global_mut<GasScheduleV2>(@0x1) = 0x1::config_buffer::extract<GasScheduleV2>();
            } else {
                move_to<GasScheduleV2>(arg0, 0x1::config_buffer::extract<GasScheduleV2>());
            };
        };
    }
    
    public fun set_for_next_epoch(arg0: &signer, arg1: vector<u8>) acquires GasScheduleV2 {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::vector::is_empty<u8>(&arg1)) {
            abort 0x1::error::invalid_argument(1)
        };
        let v0 = 0x1::util::from_bytes<GasScheduleV2>(arg1);
        if (exists<GasScheduleV2>(@0x1)) {
            let v1 = v0.feature_version >= borrow_global<GasScheduleV2>(@0x1).feature_version;
            assert!(v1, 0x1::error::invalid_argument(2));
        };
        0x1::config_buffer::upsert<GasScheduleV2>(v0);
    }
    
    public fun set_for_next_epoch_check_hash(arg0: &signer, arg1: vector<u8>, arg2: vector<u8>) acquires GasScheduleV2 {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::vector::is_empty<u8>(&arg2)) {
            abort 0x1::error::invalid_argument(1)
        };
        let v0 = 0x1::util::from_bytes<GasScheduleV2>(arg2);
        if (exists<GasScheduleV2>(@0x1)) {
            let v1 = borrow_global<GasScheduleV2>(@0x1);
            assert!(v0.feature_version >= v1.feature_version, 0x1::error::invalid_argument(2));
            let v2 = 0x1::aptos_hash::sha3_512(0x1::bcs::to_bytes<GasScheduleV2>(v1)) == arg1;
            assert!(v2, 0x1::error::invalid_argument(3));
        };
        0x1::config_buffer::upsert<GasScheduleV2>(v0);
    }
    
    public fun set_gas_schedule(arg0: &signer, arg1: vector<u8>) acquires GasSchedule, GasScheduleV2 {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::vector::is_empty<u8>(&arg1)) {
            abort 0x1::error::invalid_argument(1)
        };
        0x1::chain_status::assert_genesis();
        if (exists<GasScheduleV2>(@0x1)) {
            let v0 = borrow_global_mut<GasScheduleV2>(@0x1);
            let v1 = 0x1::util::from_bytes<GasScheduleV2>(arg1);
            assert!(v1.feature_version >= v0.feature_version, 0x1::error::invalid_argument(2));
            *v0 = v1;
        } else {
            if (exists<GasSchedule>(@0x1)) {
                move_from<GasSchedule>(@0x1);
            };
            move_to<GasScheduleV2>(arg0, 0x1::util::from_bytes<GasScheduleV2>(arg1));
        };
        0x1::reconfiguration::reconfigure();
    }
    
    public fun set_storage_gas_config(arg0: &signer, arg1: 0x1::storage_gas::StorageGasConfig) {
        0x1::storage_gas::set_config(arg0, arg1);
        0x1::reconfiguration::reconfigure();
    }
    
    public fun set_storage_gas_config_for_next_epoch(arg0: &signer, arg1: 0x1::storage_gas::StorageGasConfig) {
        0x1::storage_gas::set_config(arg0, arg1);
    }
    
    // decompiled from Move bytecode v7
}
