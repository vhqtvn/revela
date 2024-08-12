module 0x1::block {
    struct BlockResource has key {
        height: u64,
        epoch_interval: u64,
        new_block_events: 0x1::event::EventHandle<NewBlockEvent>,
        update_epoch_interval_events: 0x1::event::EventHandle<UpdateEpochIntervalEvent>,
    }
    
    struct CommitHistory has key {
        max_capacity: u32,
        next_idx: u32,
        table: 0x1::table_with_length::TableWithLength<u32, NewBlockEvent>,
    }
    
    struct NewBlock has drop, store {
        hash: address,
        epoch: u64,
        round: u64,
        height: u64,
        previous_block_votes_bitvec: vector<u8>,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        time_microseconds: u64,
    }
    
    struct NewBlockEvent has copy, drop, store {
        hash: address,
        epoch: u64,
        round: u64,
        height: u64,
        previous_block_votes_bitvec: vector<u8>,
        proposer: address,
        failed_proposer_indices: vector<u64>,
        time_microseconds: u64,
    }
    
    struct UpdateEpochInterval has drop, store {
        old_epoch_interval: u64,
        new_epoch_interval: u64,
    }
    
    struct UpdateEpochIntervalEvent has drop, store {
        old_epoch_interval: u64,
        new_epoch_interval: u64,
    }
    
    fun block_prologue(arg0: signer, arg1: address, arg2: u64, arg3: u64, arg4: address, arg5: vector<u64>, arg6: vector<u8>, arg7: u64) acquires BlockResource, CommitHistory {
        let v0 = block_prologue_common(&arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7);
        0x1::randomness::on_new_block(&arg0, arg2, arg3, 0x1::option::none<vector<u8>>());
        if (arg7 - 0x1::reconfiguration::last_reconfiguration_time() >= v0) {
            0x1::reconfiguration::reconfigure();
        };
    }
    
    fun block_prologue_common(arg0: &signer, arg1: address, arg2: u64, arg3: u64, arg4: address, arg5: vector<u64>, arg6: vector<u8>, arg7: u64) : u64 acquires BlockResource, CommitHistory {
        0x1::system_addresses::assert_vm(arg0);
        assert!(arg4 == @0x3001 || 0x1::stake::is_current_epoch_validator(arg4), 0x1::error::permission_denied(2));
        let v0 = 0x1::option::none<u64>();
        if (arg4 != @0x3001) {
            v0 = 0x1::option::some<u64>(0x1::stake::get_validator_index(arg4));
        };
        let v1 = borrow_global_mut<BlockResource>(@0x1);
        v1.height = 0x1::event::counter<NewBlockEvent>(&v1.new_block_events);
        let v2 = v1.height;
        let v3 = NewBlockEvent{
            hash                        : arg1, 
            epoch                       : arg2, 
            round                       : arg3, 
            height                      : v2, 
            previous_block_votes_bitvec : arg6, 
            proposer                    : arg4, 
            failed_proposer_indices     : arg5, 
            time_microseconds           : arg7,
        };
        let v4 = v1.height;
        let v5 = NewBlock{
            hash                        : arg1, 
            epoch                       : arg2, 
            round                       : arg3, 
            height                      : v4, 
            previous_block_votes_bitvec : arg6, 
            proposer                    : arg4, 
            failed_proposer_indices     : arg5, 
            time_microseconds           : arg7,
        };
        emit_new_block_event(arg0, &mut v1.new_block_events, v3, v5);
        if (0x1::features::collect_and_distribute_gas_fees()) {
            0x1::transaction_fee::process_collected_fees();
            0x1::transaction_fee::register_proposer_for_fee_collection(arg4);
        };
        0x1::stake::update_performance_statistics(v0, arg5);
        0x1::state_storage::on_new_block(0x1::reconfiguration::current_epoch());
        v1.epoch_interval
    }
    
    fun block_prologue_ext(arg0: signer, arg1: address, arg2: u64, arg3: u64, arg4: address, arg5: vector<u64>, arg6: vector<u8>, arg7: u64, arg8: 0x1::option::Option<vector<u8>>) acquires BlockResource, CommitHistory {
        let v0 = block_prologue_common(&arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7);
        0x1::randomness::on_new_block(&arg0, arg2, arg3, arg8);
        if (arg7 - 0x1::reconfiguration::last_reconfiguration_time() >= v0) {
            0x1::reconfiguration_with_dkg::try_start();
        };
    }
    
    fun emit_genesis_block_event(arg0: signer) acquires BlockResource, CommitHistory {
        let v0 = 0x1::vector::empty<u8>();
        let v1 = 0x1::vector::empty<u64>();
        let v2 = NewBlockEvent{
            hash                        : @0x0, 
            epoch                       : 0, 
            round                       : 0, 
            height                      : 0, 
            previous_block_votes_bitvec : v0, 
            proposer                    : @0x3001, 
            failed_proposer_indices     : v1, 
            time_microseconds           : 0,
        };
        let v3 = 0x1::vector::empty<u8>();
        let v4 = 0x1::vector::empty<u64>();
        let v5 = NewBlock{
            hash                        : @0x0, 
            epoch                       : 0, 
            round                       : 0, 
            height                      : 0, 
            previous_block_votes_bitvec : v3, 
            proposer                    : @0x3001, 
            failed_proposer_indices     : v4, 
            time_microseconds           : 0,
        };
        emit_new_block_event(&arg0, &mut borrow_global_mut<BlockResource>(@0x1).new_block_events, v2, v5);
    }
    
    fun emit_new_block_event(arg0: &signer, arg1: &mut 0x1::event::EventHandle<NewBlockEvent>, arg2: NewBlockEvent, arg3: NewBlock) acquires CommitHistory {
        if (exists<CommitHistory>(@0x1)) {
            let v0 = borrow_global_mut<CommitHistory>(@0x1);
            let v1 = v0.next_idx;
            if (0x1::table_with_length::contains<u32, NewBlockEvent>(&v0.table, v1)) {
                0x1::table_with_length::remove<u32, NewBlockEvent>(&mut v0.table, v1);
            };
            0x1::table_with_length::add<u32, NewBlockEvent>(&mut v0.table, v1, arg2);
            v0.next_idx = (v1 + 1) % v0.max_capacity;
        };
        0x1::timestamp::update_global_time(arg0, arg2.proposer, arg2.time_microseconds);
        assert!(0x1::event::counter<NewBlockEvent>(arg1) == arg2.height, 0x1::error::invalid_argument(1));
        if (0x1::features::module_event_migration_enabled()) {
            0x1::event::emit<NewBlock>(arg3);
        };
        0x1::event::emit_event<NewBlockEvent>(arg1, arg2);
    }
    
    public fun emit_writeset_block_event(arg0: &signer, arg1: address) acquires BlockResource, CommitHistory {
        0x1::system_addresses::assert_vm(arg0);
        let v0 = borrow_global_mut<BlockResource>(@0x1);
        v0.height = 0x1::event::counter<NewBlockEvent>(&v0.new_block_events);
        let v1 = 0x1::reconfiguration::current_epoch();
        let v2 = v0.height;
        let v3 = 0x1::vector::empty<u8>();
        let v4 = 0x1::vector::empty<u64>();
        let v5 = 0x1::timestamp::now_microseconds();
        let v6 = NewBlockEvent{
            hash                        : arg1, 
            epoch                       : v1, 
            round                       : 18446744073709551615, 
            height                      : v2, 
            previous_block_votes_bitvec : v3, 
            proposer                    : @0x3001, 
            failed_proposer_indices     : v4, 
            time_microseconds           : v5,
        };
        let v7 = 0x1::reconfiguration::current_epoch();
        let v8 = v0.height;
        let v9 = 0x1::vector::empty<u8>();
        let v10 = 0x1::vector::empty<u64>();
        let v11 = 0x1::timestamp::now_microseconds();
        let v12 = NewBlock{
            hash                        : arg1, 
            epoch                       : v7, 
            round                       : 18446744073709551615, 
            height                      : v8, 
            previous_block_votes_bitvec : v9, 
            proposer                    : @0x3001, 
            failed_proposer_indices     : v10, 
            time_microseconds           : v11,
        };
        emit_new_block_event(arg0, &mut v0.new_block_events, v6, v12);
    }
    
    public fun get_current_block_height() : u64 acquires BlockResource {
        borrow_global<BlockResource>(@0x1).height
    }
    
    public fun get_epoch_interval_secs() : u64 acquires BlockResource {
        borrow_global<BlockResource>(@0x1).epoch_interval / 1000000
    }
    
    public(friend) fun initialize(arg0: &signer, arg1: u64) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(arg1 > 0, 0x1::error::invalid_argument(3));
        let v0 = 0x1::table_with_length::new<u32, NewBlockEvent>();
        let v1 = CommitHistory{
            max_capacity : 2000, 
            next_idx     : 0, 
            table        : v0,
        };
        move_to<CommitHistory>(arg0, v1);
        let v2 = 0x1::account::new_event_handle<NewBlockEvent>(arg0);
        let v3 = 0x1::account::new_event_handle<UpdateEpochIntervalEvent>(arg0);
        let v4 = BlockResource{
            height                       : 0, 
            epoch_interval               : arg1, 
            new_block_events             : v2, 
            update_epoch_interval_events : v3,
        };
        move_to<BlockResource>(arg0, v4);
    }
    
    public fun initialize_commit_history(arg0: &signer, arg1: u32) {
        assert!(arg1 > 0, 0x1::error::invalid_argument(3));
        let v0 = 0x1::table_with_length::new<u32, NewBlockEvent>();
        let v1 = CommitHistory{
            max_capacity : arg1, 
            next_idx     : 0, 
            table        : v0,
        };
        move_to<CommitHistory>(arg0, v1);
    }
    
    public fun update_epoch_interval_microsecs(arg0: &signer, arg1: u64) acquires BlockResource {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(arg1 > 0, 0x1::error::invalid_argument(3));
        let v0 = borrow_global_mut<BlockResource>(@0x1);
        let v1 = v0.epoch_interval;
        v0.epoch_interval = arg1;
        if (0x1::features::module_event_migration_enabled()) {
            let v2 = UpdateEpochInterval{
                old_epoch_interval : v1, 
                new_epoch_interval : arg1,
            };
            0x1::event::emit<UpdateEpochInterval>(v2);
        };
        let v3 = UpdateEpochIntervalEvent{
            old_epoch_interval : v1, 
            new_epoch_interval : arg1,
        };
        0x1::event::emit_event<UpdateEpochIntervalEvent>(&mut v0.update_epoch_interval_events, v3);
    }
    
    // decompiled from Move bytecode v7
}
