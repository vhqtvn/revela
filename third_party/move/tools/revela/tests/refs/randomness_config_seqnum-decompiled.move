module 0x1::randomness_config_seqnum {
    struct RandomnessConfigSeqNum has drop, store, key {
        seq_num: u64,
    }

    public fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (exists<RandomnessConfigSeqNum>(@0x1)) {
        } else {
            let v0 = RandomnessConfigSeqNum{seq_num: 0};
            move_to<RandomnessConfigSeqNum>(arg0, v0);
        };
    }

    public(friend) fun on_new_epoch(arg0: &signer) acquires RandomnessConfigSeqNum {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (0x1::config_buffer::does_exist<RandomnessConfigSeqNum>()) {
            let v0 = 0x1::config_buffer::extract<RandomnessConfigSeqNum>();
            if (exists<RandomnessConfigSeqNum>(@0x1)) {
                *borrow_global_mut<RandomnessConfigSeqNum>(@0x1) = v0;
            } else {
                move_to<RandomnessConfigSeqNum>(arg0, v0);
            };
        };
    }

    public fun set_for_next_epoch(arg0: &signer, arg1: u64) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = RandomnessConfigSeqNum{seq_num: arg1};
        0x1::config_buffer::upsert<RandomnessConfigSeqNum>(v0);
    }

    // decompiled from Move bytecode v7
}
