module 0x1::reconfiguration_with_dkg {
    public(friend) fun finish(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        0x1::dkg::try_clear_incomplete_session(arg0);
        0x1::consensus_config::on_new_epoch(arg0);
        0x1::execution_config::on_new_epoch(arg0);
        0x1::gas_schedule::on_new_epoch(arg0);
        0x1::version::on_new_epoch(arg0);
        0x1::features::on_new_epoch(arg0);
        0x1::jwk_consensus_config::on_new_epoch(arg0);
        0x1::jwks::on_new_epoch(arg0);
        0x1::keyless_account::on_new_epoch(arg0);
        0x1::randomness_config_seqnum::on_new_epoch(arg0);
        0x1::randomness_config::on_new_epoch(arg0);
        0x1::randomness_api_v0_config::on_new_epoch(arg0);
        0x1::reconfiguration::reconfigure();
    }
    
    fun finish_with_dkg_result(arg0: &signer, arg1: vector<u8>) {
        0x1::dkg::finish(arg1);
        finish(arg0);
    }
    
    public(friend) fun try_start() {
        let v0 = 0x1::dkg::incomplete_session();
        if (0x1::option::is_some<0x1::dkg::DKGSessionState>(&v0)) {
            let v1 = 0x1::dkg::session_dealer_epoch(0x1::option::borrow<0x1::dkg::DKGSessionState>(&v0));
            if (v1 == 0x1::reconfiguration::current_epoch()) {
                return
            };
        };
        0x1::reconfiguration_state::on_reconfig_start();
        let v2 = 0x1::stake::cur_validator_consensus_infos();
        let v3 = 0x1::stake::next_validator_consensus_infos();
        0x1::dkg::start(0x1::reconfiguration::current_epoch(), 0x1::randomness_config::current(), v2, v3);
    }
    
    // decompiled from Move bytecode v6
}
