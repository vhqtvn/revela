module 0x1::validator_consensus_info {
    struct ValidatorConsensusInfo has copy, drop, store {
        addr: address,
        pk_bytes: vector<u8>,
        voting_power: u64,
    }
    
    public fun default() : ValidatorConsensusInfo {
        ValidatorConsensusInfo{
            addr         : @0x0, 
            pk_bytes     : 0x1::vector::empty<u8>(), 
            voting_power : 0,
        }
    }
    
    public fun get_addr(arg0: &ValidatorConsensusInfo) : address {
        arg0.addr
    }
    
    public fun get_pk_bytes(arg0: &ValidatorConsensusInfo) : vector<u8> {
        arg0.pk_bytes
    }
    
    public fun get_voting_power(arg0: &ValidatorConsensusInfo) : u64 {
        arg0.voting_power
    }
    
    public fun new(arg0: address, arg1: vector<u8>, arg2: u64) : ValidatorConsensusInfo {
        ValidatorConsensusInfo{
            addr         : arg0, 
            pk_bytes     : arg1, 
            voting_power : arg2,
        }
    }
    
    // decompiled from Move bytecode v7
}
