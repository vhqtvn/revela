module 0x1::randomness_config {
    struct ConfigOff has copy, drop, store {
        dummy_field: bool,
    }
    
    struct ConfigV1 has copy, drop, store {
        secrecy_threshold: 0x1::fixed_point64::FixedPoint64,
        reconstruction_threshold: 0x1::fixed_point64::FixedPoint64,
    }
    
    struct ConfigV2 has copy, drop, store {
        secrecy_threshold: 0x1::fixed_point64::FixedPoint64,
        reconstruction_threshold: 0x1::fixed_point64::FixedPoint64,
        fast_path_secrecy_threshold: 0x1::fixed_point64::FixedPoint64,
    }
    
    struct RandomnessConfig has copy, drop, store, key {
        variant: 0x1::copyable_any::Any,
    }
    
    public fun current() : RandomnessConfig acquires RandomnessConfig {
        if (exists<RandomnessConfig>(@0x1)) {
            *borrow_global<RandomnessConfig>(@0x1)
        } else {
            new_off()
        }
    }
    
    public fun enabled() : bool acquires RandomnessConfig {
        let v0 = exists<RandomnessConfig>(@0x1);
        v0 && *0x1::string::bytes(0x1::copyable_any::type_name(&borrow_global<RandomnessConfig>(@0x1).variant)) != b"0x1::randomness_config::ConfigOff"
    }
    
    public fun initialize(arg0: &signer, arg1: RandomnessConfig) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (exists<RandomnessConfig>(@0x1)) {
        } else {
            move_to<RandomnessConfig>(arg0, arg1);
        };
    }
    
    public fun new_off() : RandomnessConfig {
        let v0 = ConfigOff{dummy_field: false};
        RandomnessConfig{variant: 0x1::copyable_any::pack<ConfigOff>(v0)}
    }
    
    public fun new_v1(arg0: 0x1::fixed_point64::FixedPoint64, arg1: 0x1::fixed_point64::FixedPoint64) : RandomnessConfig {
        let v0 = ConfigV1{
            secrecy_threshold        : arg0, 
            reconstruction_threshold : arg1,
        };
        RandomnessConfig{variant: 0x1::copyable_any::pack<ConfigV1>(v0)}
    }
    
    public fun new_v2(arg0: 0x1::fixed_point64::FixedPoint64, arg1: 0x1::fixed_point64::FixedPoint64, arg2: 0x1::fixed_point64::FixedPoint64) : RandomnessConfig {
        let v0 = ConfigV2{
            secrecy_threshold           : arg0, 
            reconstruction_threshold    : arg1, 
            fast_path_secrecy_threshold : arg2,
        };
        RandomnessConfig{variant: 0x1::copyable_any::pack<ConfigV2>(v0)}
    }
    
    public(friend) fun on_new_epoch(arg0: &signer) acquires RandomnessConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::config_buffer::does_exist<RandomnessConfig>()) {
            if (exists<RandomnessConfig>(@0x1)) {
                *borrow_global_mut<RandomnessConfig>(@0x1) = 0x1::config_buffer::extract<RandomnessConfig>();
            } else {
                move_to<RandomnessConfig>(arg0, 0x1::config_buffer::extract<RandomnessConfig>());
            };
        };
    }
    
    public fun set_for_next_epoch(arg0: &signer, arg1: RandomnessConfig) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::config_buffer::upsert<RandomnessConfig>(arg1);
    }
    
    // decompiled from Move bytecode v7
}
