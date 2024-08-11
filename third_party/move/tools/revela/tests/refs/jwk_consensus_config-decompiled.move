module 0x1::jwk_consensus_config {
    struct ConfigOff has copy, drop, store {
        dummy_field: bool,
    }
    
    struct ConfigV1 has copy, drop, store {
        oidc_providers: vector<OIDCProvider>,
    }
    
    struct JWKConsensusConfig has drop, store, key {
        variant: 0x1::copyable_any::Any,
    }
    
    struct OIDCProvider has copy, drop, store {
        name: 0x1::string::String,
        config_url: 0x1::string::String,
    }
    
    public fun initialize(arg0: &signer, arg1: JWKConsensusConfig) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (!exists<JWKConsensusConfig>(@0x1)) {
            move_to<JWKConsensusConfig>(arg0, arg1);
        };
    }
    
    public fun new_off() : JWKConsensusConfig {
        let v0 = ConfigOff{dummy_field: false};
        JWKConsensusConfig{variant: 0x1::copyable_any::pack<ConfigOff>(v0)}
    }
    
    public fun new_oidc_provider(arg0: 0x1::string::String, arg1: 0x1::string::String) : OIDCProvider {
        OIDCProvider{
            name       : arg0, 
            config_url : arg1,
        }
    }
    
    public fun new_v1(arg0: vector<OIDCProvider>) : JWKConsensusConfig {
        let v0 = 0x1::simple_map::new<0x1::string::String, u64>();
        let v1 = &arg0;
        let v2 = 0;
        while (v2 < 0x1::vector::length<OIDCProvider>(v1)) {
            let v3 = 0x1::vector::borrow<OIDCProvider>(v1, v2).name;
            let (_, v5) = 0x1::simple_map::upsert<0x1::string::String, u64>(&mut v0, v3, 0);
            let v6 = v5;
            if (0x1::option::is_some<u64>(&v6)) {
                abort 0x1::error::invalid_argument(1)
            };
            v2 = v2 + 1;
        };
        let v7 = ConfigV1{oidc_providers: arg0};
        JWKConsensusConfig{variant: 0x1::copyable_any::pack<ConfigV1>(v7)}
    }
    
    public(friend) fun on_new_epoch(arg0: &signer) acquires JWKConsensusConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::config_buffer::does_exist<JWKConsensusConfig>()) {
            if (exists<JWKConsensusConfig>(@0x1)) {
                *borrow_global_mut<JWKConsensusConfig>(@0x1) = 0x1::config_buffer::extract<JWKConsensusConfig>();
            } else {
                move_to<JWKConsensusConfig>(arg0, 0x1::config_buffer::extract<JWKConsensusConfig>());
            };
        };
    }
    
    public fun set_for_next_epoch(arg0: &signer, arg1: JWKConsensusConfig) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::config_buffer::upsert<JWKConsensusConfig>(arg1);
    }
    
    // decompiled from Move bytecode v6
}
