module 0x1::dkg {
    struct DKGSessionMetadata has copy, drop, store {
        dealer_epoch: u64,
        randomness_config: 0x1::randomness_config::RandomnessConfig,
        dealer_validator_set: vector<0x1::validator_consensus_info::ValidatorConsensusInfo>,
        target_validator_set: vector<0x1::validator_consensus_info::ValidatorConsensusInfo>,
    }
    
    struct DKGSessionState has copy, drop, store {
        metadata: DKGSessionMetadata,
        start_time_us: u64,
        transcript: vector<u8>,
    }
    
    struct DKGStartEvent has drop, store {
        session_metadata: DKGSessionMetadata,
        start_time_us: u64,
    }
    
    struct DKGState has key {
        last_completed: 0x1::option::Option<DKGSessionState>,
        in_progress: 0x1::option::Option<DKGSessionState>,
    }
    
    public(friend) fun finish(arg0: vector<u8>) acquires DKGState {
        let v0 = borrow_global_mut<DKGState>(@0x1);
        assert!(0x1::option::is_some<DKGSessionState>(&v0.in_progress), 0x1::error::invalid_state(2));
        let v1 = 0x1::option::extract<DKGSessionState>(&mut v0.in_progress);
        v1.transcript = arg0;
        v0.last_completed = 0x1::option::some<DKGSessionState>(v1);
        v0.in_progress = 0x1::option::none<DKGSessionState>();
    }
    
    public fun incomplete_session() : 0x1::option::Option<DKGSessionState> acquires DKGState {
        if (exists<DKGState>(@0x1)) {
            borrow_global<DKGState>(@0x1).in_progress
        } else {
            0x1::option::none<DKGSessionState>()
        }
    }
    
    public fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (!exists<DKGState>(@0x1)) {
            let v0 = 0x1::option::none<DKGSessionState>();
            let v1 = DKGState{
                last_completed : 0x1::option::none<DKGSessionState>(), 
                in_progress    : v0,
            };
            move_to<DKGState>(arg0, v1);
        };
    }
    
    public fun session_dealer_epoch(arg0: &DKGSessionState) : u64 {
        arg0.metadata.dealer_epoch
    }
    
    public(friend) fun start(arg0: u64, arg1: 0x1::randomness_config::RandomnessConfig, arg2: vector<0x1::validator_consensus_info::ValidatorConsensusInfo>, arg3: vector<0x1::validator_consensus_info::ValidatorConsensusInfo>) acquires DKGState {
        let v0 = DKGSessionMetadata{
            dealer_epoch         : arg0, 
            randomness_config    : arg1, 
            dealer_validator_set : arg2, 
            target_validator_set : arg3,
        };
        let v1 = 0x1::timestamp::now_microseconds();
        let v2 = DKGSessionState{
            metadata      : v0, 
            start_time_us : v1, 
            transcript    : b"",
        };
        borrow_global_mut<DKGState>(@0x1).in_progress = 0x1::option::some<DKGSessionState>(v2);
        let v3 = DKGStartEvent{
            session_metadata : v0, 
            start_time_us    : v1,
        };
        0x1::event::emit<DKGStartEvent>(v3);
    }
    
    public fun try_clear_incomplete_session(arg0: &signer) acquires DKGState {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (exists<DKGState>(@0x1)) {
            borrow_global_mut<DKGState>(@0x1).in_progress = 0x1::option::none<DKGSessionState>();
        };
    }
    
    // decompiled from Move bytecode v6
}
