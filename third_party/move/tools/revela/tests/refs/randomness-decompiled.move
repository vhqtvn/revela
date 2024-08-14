module 0x1::randomness {
    struct PerBlockRandomness has drop, key {
        epoch: u64,
        round: u64,
        seed: 0x1::option::Option<vector<u8>>,
    }

    struct RandomnessGeneratedEvent has drop, store {
        dummy_field: bool,
    }

    public fun bytes(arg0: u64) : vector<u8> acquires PerBlockRandomness {
        let v0 = 0x1::vector::empty<u8>();
        let v1 = 0;
        while (v1 < arg0) {
            let v2 = next_32_bytes();
            0x1::vector::reverse_append<u8>(&mut v0, v2);
            v1 = v1 + 32;
        };
        if (v1 > arg0) {
            0x1::vector::trim<u8>(&mut v0, arg0);
        };
        let v3 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v3);
        v0
    }

    native fun fetch_and_increment_txn_counter() : vector<u8>;
    public fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (exists<PerBlockRandomness>(@0x1)) {
        } else {
            let v0 = PerBlockRandomness{
                epoch : 0,
                round : 0,
                seed  : 0x1::option::none<vector<u8>>(),
            };
            move_to<PerBlockRandomness>(arg0, v0);
        };
    }

    native fun is_unbiasable() : bool;
    fun next_32_bytes() : vector<u8> acquires PerBlockRandomness {
        assert!(is_unbiasable(), 1);
        let v0 = b"APTOS_RANDOMNESS";
        let v1 = *0x1::option::borrow<vector<u8>>(&borrow_global<PerBlockRandomness>(@0x1).seed);
        0x1::vector::append<u8>(&mut v0, v1);
        0x1::vector::append<u8>(&mut v0, 0x1::transaction_context::get_transaction_hash());
        0x1::vector::append<u8>(&mut v0, fetch_and_increment_txn_counter());
        0x1::hash::sha3_256(v0)
    }

    public(friend) fun on_new_block(arg0: &signer, arg1: u64, arg2: u64, arg3: 0x1::option::Option<vector<u8>>) acquires PerBlockRandomness {
        0x1::system_addresses::assert_vm(arg0);
        if (exists<PerBlockRandomness>(@0x1)) {
            let v0 = borrow_global_mut<PerBlockRandomness>(@0x1);
            v0.epoch = arg1;
            v0.round = arg2;
            v0.seed = arg3;
        };
    }

    public fun permutation(arg0: u64) : vector<u64> acquires PerBlockRandomness {
        let v0 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v0);
        let v1 = 0x1::vector::empty<u64>();
        if (arg0 == 0) {
            return 0x1::vector::empty<u64>()
        };
        let v2 = 0;
        while (v2 < arg0) {
            0x1::vector::push_back<u64>(&mut v1, v2);
            v2 = v2 + 1;
        };
        let v3 = arg0 - 1;
        while (v3 > 0) {
            let v4 = u64_range_internal(0, v3 + 1);
            0x1::vector::swap<u64>(&mut v1, v4, v3);
            v3 = v3 - 1;
        };
        v1
    }

    fun safe_add_mod(arg0: u256, arg1: u256, arg2: u256) : u256 {
        let v0 = arg2 - arg1;
        let v1 = arg0 < v0;
        let v2 = if (v1) {
            arg0 + arg1
        } else {
            arg0 - v0
        };
        let v3 = if (v1) {
            arg0 + arg1
        } else {
            arg0 - v0
        };
        take_first(v2, v3)
    }

    fun take_first(arg0: u256, arg1: u256) : u256 {
        arg0
    }

    public fun u128_integer() : u128 acquires PerBlockRandomness {
        let v0 = next_32_bytes();
        let v1 = 0;
        let v2 = 0;
        while (v2 < 16) {
            let v3 = v1 * 256;
            v1 = v3 + (0x1::vector::pop_back<u8>(&mut v0) as u128);
            v2 = v2 + 1;
        };
        let v4 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v4);
        v1
    }

    public fun u128_range(arg0: u128, arg1: u128) : u128 acquires PerBlockRandomness {
        let v0 = u256_integer_internal();
        let v1 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v1);
        arg0 + ((v0 % ((arg1 - arg0) as u256)) as u128)
    }

    public fun u16_integer() : u16 acquires PerBlockRandomness {
        let v0 = next_32_bytes();
        let v1 = 0;
        let v2 = 0;
        while (v2 < 2) {
            let v3 = v1 * 256;
            v1 = v3 + (0x1::vector::pop_back<u8>(&mut v0) as u16);
            v2 = v2 + 1;
        };
        let v4 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v4);
        v1
    }

    public fun u16_range(arg0: u16, arg1: u16) : u16 acquires PerBlockRandomness {
        let v0 = u256_integer_internal();
        let v1 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v1);
        arg0 + ((v0 % ((arg1 - arg0) as u256)) as u16)
    }

    public fun u256_integer() : u256 acquires PerBlockRandomness {
        let v0 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v0);
        u256_integer_internal()
    }

    fun u256_integer_internal() : u256 acquires PerBlockRandomness {
        let v0 = next_32_bytes();
        let v1 = 0;
        let v2 = 0;
        while (v2 < 32) {
            let v3 = v1 * 256;
            v1 = v3 + (0x1::vector::pop_back<u8>(&mut v0) as u256);
            v2 = v2 + 1;
        };
        v1
    }

    public fun u256_range(arg0: u256, arg1: u256) : u256 acquires PerBlockRandomness {
        let v0 = arg1 - arg0;
        let v1 = u256_integer_internal();
        let v2 = u256_integer_internal();
        let v3 = v2 % v0;
        let v4 = 0;
        while (v4 < 256) {
            v3 = safe_add_mod(v3, v3, v0);
            v4 = v4 + 1;
        };
        let v5 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v5);
        arg0 + safe_add_mod(v3, v1 % v0, v0)
    }

    public fun u32_integer() : u32 acquires PerBlockRandomness {
        let v0 = next_32_bytes();
        let v1 = 0;
        let v2 = 0;
        while (v2 < 4) {
            let v3 = v1 * 256;
            v1 = v3 + (0x1::vector::pop_back<u8>(&mut v0) as u32);
            v2 = v2 + 1;
        };
        let v4 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v4);
        v1
    }

    public fun u32_range(arg0: u32, arg1: u32) : u32 acquires PerBlockRandomness {
        let v0 = u256_integer_internal();
        let v1 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v1);
        arg0 + ((v0 % ((arg1 - arg0) as u256)) as u32)
    }

    public fun u64_integer() : u64 acquires PerBlockRandomness {
        let v0 = next_32_bytes();
        let v1 = 0;
        let v2 = 0;
        while (v2 < 8) {
            let v3 = v1 * 256;
            v1 = v3 + (0x1::vector::pop_back<u8>(&mut v0) as u64);
            v2 = v2 + 1;
        };
        let v4 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v4);
        v1
    }

    public fun u64_range(arg0: u64, arg1: u64) : u64 acquires PerBlockRandomness {
        let v0 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v0);
        u64_range_internal(arg0, arg1)
    }

    public fun u64_range_internal(arg0: u64, arg1: u64) : u64 acquires PerBlockRandomness {
        let v0 = u256_integer_internal();
        arg0 + ((v0 % ((arg1 - arg0) as u256)) as u64)
    }

    public fun u8_integer() : u8 acquires PerBlockRandomness {
        let v0 = next_32_bytes();
        let v1 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v1);
        0x1::vector::pop_back<u8>(&mut v0)
    }

    public fun u8_range(arg0: u8, arg1: u8) : u8 acquires PerBlockRandomness {
        let v0 = u256_integer_internal();
        let v1 = RandomnessGeneratedEvent{dummy_field: false};
        0x1::event::emit<RandomnessGeneratedEvent>(v1);
        arg0 + ((v0 % ((arg1 - arg0) as u256)) as u8)
    }

    // decompiled from Move bytecode v7
}
