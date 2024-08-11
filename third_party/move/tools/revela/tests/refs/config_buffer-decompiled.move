module 0x1::config_buffer {
    struct PendingConfigs has key {
        configs: 0x1::simple_map::SimpleMap<0x1::string::String, 0x1::any::Any>,
    }
    
    public(friend) fun upsert<T0: drop + store>(arg0: T0) acquires PendingConfigs {
        let v0 = 0x1::type_info::type_name<T0>();
        let v1 = &mut borrow_global_mut<PendingConfigs>(@0x1).configs;
        let (_, _) = 0x1::simple_map::upsert<0x1::string::String, 0x1::any::Any>(v1, v0, 0x1::any::pack<T0>(arg0));
    }
    
    public fun does_exist<T0: store>() : bool acquires PendingConfigs {
        if (exists<PendingConfigs>(@0x1)) {
            let v1 = 0x1::type_info::type_name<T0>();
            0x1::simple_map::contains_key<0x1::string::String, 0x1::any::Any>(&borrow_global<PendingConfigs>(@0x1).configs, &v1)
        } else {
            false
        }
    }
    
    public fun extract<T0: store>() : T0 acquires PendingConfigs {
        let v0 = 0x1::type_info::type_name<T0>();
        let v1 = &mut borrow_global_mut<PendingConfigs>(@0x1).configs;
        let (_, v3) = 0x1::simple_map::remove<0x1::string::String, 0x1::any::Any>(v1, &v0);
        0x1::any::unpack<T0>(v3)
    }
    
    public fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (!exists<PendingConfigs>(@0x1)) {
            let v0 = PendingConfigs{configs: 0x1::simple_map::new<0x1::string::String, 0x1::any::Any>()};
            move_to<PendingConfigs>(arg0, v0);
        };
    }
    
    // decompiled from Move bytecode v6
}
