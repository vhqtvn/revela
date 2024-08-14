module 0x1::reconfiguration_state {
    struct State has key {
        variant: 0x1::copyable_any::Any,
    }

    struct StateActive has copy, drop, store {
        start_time_secs: u64,
    }

    struct StateInactive has copy, drop, store {
        dummy_field: bool,
    }

    public fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (exists<State>(@0x1)) {
        } else {
            let v0 = StateInactive{dummy_field: false};
            let v1 = State{variant: 0x1::copyable_any::pack<StateInactive>(v0)};
            move_to<State>(arg0, v1);
        };
    }

    public fun initialize_for_testing(arg0: &signer) {
        initialize(arg0);
    }

    public(friend) fun is_in_progress() : bool acquires State {
        if (exists<State>(@0x1)) {
            let v0 = *0x1::string::bytes(0x1::copyable_any::type_name(&borrow_global<State>(@0x1).variant));
            return v0 == b"0x1::reconfiguration_state::StateActive"
        };
        false
    }

    public fun is_initialized() : bool {
        exists<State>(@0x1)
    }

    public(friend) fun on_reconfig_finish() acquires State {
        if (exists<State>(@0x1)) {
            let v0 = borrow_global_mut<State>(@0x1);
            let v1 = *0x1::string::bytes(0x1::copyable_any::type_name(&v0.variant));
            assert!(v1 == b"0x1::reconfiguration_state::StateActive", 0x1::error::invalid_state(1));
            let v2 = StateInactive{dummy_field: false};
            v0.variant = 0x1::copyable_any::pack<StateInactive>(v2);
        };
    }

    public(friend) fun on_reconfig_start() acquires State {
        if (exists<State>(@0x1)) {
            let v0 = borrow_global_mut<State>(@0x1);
            let v1 = *0x1::string::bytes(0x1::copyable_any::type_name(&v0.variant));
            if (v1 == b"0x1::reconfiguration_state::StateInactive") {
                let v2 = StateActive{start_time_secs: 0x1::timestamp::now_seconds()};
                v0.variant = 0x1::copyable_any::pack<StateActive>(v2);
            };
        };
    }

    public(friend) fun start_time_secs() : u64 acquires State {
        let v0 = borrow_global<State>(@0x1);
        let v1 = *0x1::string::bytes(0x1::copyable_any::type_name(&v0.variant));
        assert!(v1 == b"0x1::reconfiguration_state::StateActive", 0x1::error::invalid_state(1));
        let v2 = 0x1::copyable_any::unpack<StateActive>(v0.variant);
        v2.start_time_secs
    }

    // decompiled from Move bytecode v7
}
