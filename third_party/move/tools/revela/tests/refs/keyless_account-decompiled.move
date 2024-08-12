module 0x1::keyless_account {
    struct Configuration has copy, drop, store, key {
        override_aud_vals: vector<0x1::string::String>,
        max_signatures_per_txn: u16,
        max_exp_horizon_secs: u64,
        training_wheels_pubkey: 0x1::option::Option<vector<u8>>,
        max_commited_epk_bytes: u16,
        max_iss_val_bytes: u16,
        max_extra_field_bytes: u16,
        max_jwt_header_b64_bytes: u32,
    }
    
    struct Group {
        dummy_field: bool,
    }
    
    struct Groth16VerificationKey has drop, store, key {
        alpha_g1: vector<u8>,
        beta_g2: vector<u8>,
        gamma_g2: vector<u8>,
        delta_g2: vector<u8>,
        gamma_abc_g1: vector<vector<u8>>,
    }
    
    public fun add_override_aud(arg0: &signer, arg1: 0x1::string::String) acquires Configuration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        let v0 = &mut borrow_global_mut<Configuration>(0x1::signer::address_of(arg0)).override_aud_vals;
        0x1::vector::push_back<0x1::string::String>(v0, arg1);
    }
    
    public fun add_override_aud_for_next_epoch(arg0: &signer, arg1: 0x1::string::String) acquires Configuration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = if (0x1::config_buffer::does_exist<Configuration>()) {
            0x1::config_buffer::extract<Configuration>()
        } else {
            *borrow_global<Configuration>(0x1::signer::address_of(arg0))
        };
        0x1::vector::push_back<0x1::string::String>(&mut v0.override_aud_vals, arg1);
        set_configuration_for_next_epoch(arg0, v0);
    }
    
    public fun new_configuration(arg0: vector<0x1::string::String>, arg1: u16, arg2: u64, arg3: 0x1::option::Option<vector<u8>>, arg4: u16, arg5: u16, arg6: u16, arg7: u32) : Configuration {
        Configuration{
            override_aud_vals        : arg0, 
            max_signatures_per_txn   : arg1, 
            max_exp_horizon_secs     : arg2, 
            training_wheels_pubkey   : arg3, 
            max_commited_epk_bytes   : arg4, 
            max_iss_val_bytes        : arg5, 
            max_extra_field_bytes    : arg6, 
            max_jwt_header_b64_bytes : arg7,
        }
    }
    
    public fun new_groth16_verification_key(arg0: vector<u8>, arg1: vector<u8>, arg2: vector<u8>, arg3: vector<u8>, arg4: vector<vector<u8>>) : Groth16VerificationKey {
        Groth16VerificationKey{
            alpha_g1     : arg0, 
            beta_g2      : arg1, 
            gamma_g2     : arg2, 
            delta_g2     : arg3, 
            gamma_abc_g1 : arg4,
        }
    }
    
    public(friend) fun on_new_epoch(arg0: &signer) acquires Configuration, Groth16VerificationKey {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::config_buffer::does_exist<Groth16VerificationKey>()) {
            let v0 = 0x1::config_buffer::extract<Groth16VerificationKey>();
            if (exists<Groth16VerificationKey>(@0x1)) {
                *borrow_global_mut<Groth16VerificationKey>(@0x1) = v0;
            } else {
                move_to<Groth16VerificationKey>(arg0, v0);
            };
        };
        if (0x1::config_buffer::does_exist<Configuration>()) {
            if (exists<Configuration>(@0x1)) {
                *borrow_global_mut<Configuration>(@0x1) = 0x1::config_buffer::extract<Configuration>();
            } else {
                move_to<Configuration>(arg0, 0x1::config_buffer::extract<Configuration>());
            };
        };
    }
    
    public fun remove_all_override_auds(arg0: &signer) acquires Configuration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        let v0 = &mut borrow_global_mut<Configuration>(0x1::signer::address_of(arg0)).override_aud_vals;
        *v0 = 0x1::vector::empty<0x1::string::String>();
    }
    
    public fun remove_all_override_auds_for_next_epoch(arg0: &signer) acquires Configuration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = if (0x1::config_buffer::does_exist<Configuration>()) {
            0x1::config_buffer::extract<Configuration>()
        } else {
            *borrow_global<Configuration>(0x1::signer::address_of(arg0))
        };
        v0.override_aud_vals = 0x1::vector::empty<0x1::string::String>();
        set_configuration_for_next_epoch(arg0, v0);
    }
    
    public fun set_configuration_for_next_epoch(arg0: &signer, arg1: Configuration) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::config_buffer::upsert<Configuration>(arg1);
    }
    
    public fun set_groth16_verification_key_for_next_epoch(arg0: &signer, arg1: Groth16VerificationKey) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::config_buffer::upsert<Groth16VerificationKey>(arg1);
    }
    
    public fun update_configuration(arg0: &signer, arg1: Configuration) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        move_to<Configuration>(arg0, arg1);
    }
    
    public fun update_groth16_verification_key(arg0: &signer, arg1: Groth16VerificationKey) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        move_to<Groth16VerificationKey>(arg0, arg1);
    }
    
    public fun update_max_exp_horizon(arg0: &signer, arg1: u64) acquires Configuration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        borrow_global_mut<Configuration>(0x1::signer::address_of(arg0)).max_exp_horizon_secs = arg1;
    }
    
    public fun update_max_exp_horizon_for_next_epoch(arg0: &signer, arg1: u64) acquires Configuration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = if (0x1::config_buffer::does_exist<Configuration>()) {
            0x1::config_buffer::extract<Configuration>()
        } else {
            *borrow_global<Configuration>(0x1::signer::address_of(arg0))
        };
        v0.max_exp_horizon_secs = arg1;
        set_configuration_for_next_epoch(arg0, v0);
    }
    
    public fun update_training_wheels(arg0: &signer, arg1: 0x1::option::Option<vector<u8>>) acquires Configuration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::chain_status::assert_genesis();
        if (0x1::option::is_some<vector<u8>>(&arg1)) {
            assert!(0x1::vector::length<u8>(0x1::option::borrow<vector<u8>>(&arg1)) == 32, 1);
        };
        borrow_global_mut<Configuration>(0x1::signer::address_of(arg0)).training_wheels_pubkey = arg1;
    }
    
    public fun update_training_wheels_for_next_epoch(arg0: &signer, arg1: 0x1::option::Option<vector<u8>>) acquires Configuration {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::option::is_some<vector<u8>>(&arg1)) {
            let v0 = 0x1::ed25519::new_validated_public_key_from_bytes(*0x1::option::borrow<vector<u8>>(&arg1));
            assert!(0x1::option::is_some<0x1::ed25519::ValidatedPublicKey>(&v0), 1);
        };
        let v1 = if (0x1::config_buffer::does_exist<Configuration>()) {
            0x1::config_buffer::extract<Configuration>()
        } else {
            *borrow_global<Configuration>(0x1::signer::address_of(arg0))
        };
        v1.training_wheels_pubkey = arg1;
        set_configuration_for_next_epoch(arg0, v1);
    }
    
    fun validate_groth16_vk(arg0: &Groth16VerificationKey) {
        let v0 = 0x1::crypto_algebra::deserialize<0x1::bn254_algebra::G1, 0x1::bn254_algebra::FormatG1Compr>(&arg0.alpha_g1);
        assert!(0x1::option::is_some<0x1::crypto_algebra::Element<0x1::bn254_algebra::G1>>(&v0), 2);
        let v1 = 0x1::crypto_algebra::deserialize<0x1::bn254_algebra::G2, 0x1::bn254_algebra::FormatG2Compr>(&arg0.beta_g2);
        assert!(0x1::option::is_some<0x1::crypto_algebra::Element<0x1::bn254_algebra::G2>>(&v1), 3);
        let v2 = 0x1::crypto_algebra::deserialize<0x1::bn254_algebra::G2, 0x1::bn254_algebra::FormatG2Compr>(&arg0.gamma_g2);
        assert!(0x1::option::is_some<0x1::crypto_algebra::Element<0x1::bn254_algebra::G2>>(&v2), 3);
        let v3 = 0x1::crypto_algebra::deserialize<0x1::bn254_algebra::G2, 0x1::bn254_algebra::FormatG2Compr>(&arg0.delta_g2);
        assert!(0x1::option::is_some<0x1::crypto_algebra::Element<0x1::bn254_algebra::G2>>(&v3), 3);
        let v4 = false;
        let v5 = 0;
        while (true) {
            if (v4) {
                v5 = v5 + 1;
            } else {
                v4 = true;
            };
            if (v5 < 0x1::vector::length<vector<u8>>(&arg0.gamma_abc_g1)) {
                let v6 = 0x1::vector::borrow<vector<u8>>(&arg0.gamma_abc_g1, v5);
                let v7 = 0x1::crypto_algebra::deserialize<0x1::bn254_algebra::G1, 0x1::bn254_algebra::FormatG1Compr>(v6);
                assert!(0x1::option::is_some<0x1::crypto_algebra::Element<0x1::bn254_algebra::G1>>(&v7), 2);
                continue
            };
            break
        };
    }
    
    // decompiled from Move bytecode v7
}
