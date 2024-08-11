module 0x1::delegation_pool {
    struct AddStake has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_added: u64,
        add_stake_fee: u64,
    }
    
    struct AddStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_added: u64,
        add_stake_fee: u64,
    }
    
    struct AllowlistDelegator has drop, store {
        pool_address: address,
        delegator_address: address,
    }
    
    struct BeneficiaryForOperator has key {
        beneficiary_for_operator: address,
    }
    
    struct CommissionPercentageChange has drop, store {
        pool_address: address,
        owner: address,
        commission_percentage_next_lockup_cycle: u64,
    }
    
    struct CreateProposal has drop, store {
        proposal_id: u64,
        voter: address,
        delegation_pool: address,
    }
    
    struct CreateProposalEvent has drop, store {
        proposal_id: u64,
        voter: address,
        delegation_pool: address,
    }
    
    struct DelegateVotingPower has drop, store {
        pool_address: address,
        delegator: address,
        voter: address,
    }
    
    struct DelegateVotingPowerEvent has drop, store {
        pool_address: address,
        delegator: address,
        voter: address,
    }
    
    struct DelegatedVotes has copy, drop, store {
        active_shares: u128,
        pending_inactive_shares: u128,
        active_shares_next_lockup: u128,
        last_locked_until_secs: u64,
    }
    
    struct DelegationPool has key {
        active_shares: 0x1::pool_u64_unbound::Pool,
        observed_lockup_cycle: ObservedLockupCycle,
        inactive_shares: 0x1::table::Table<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>,
        pending_withdrawals: 0x1::table::Table<address, ObservedLockupCycle>,
        stake_pool_signer_cap: 0x1::account::SignerCapability,
        total_coins_inactive: u64,
        operator_commission_percentage: u64,
        add_stake_events: 0x1::event::EventHandle<AddStakeEvent>,
        reactivate_stake_events: 0x1::event::EventHandle<ReactivateStakeEvent>,
        unlock_stake_events: 0x1::event::EventHandle<UnlockStakeEvent>,
        withdraw_stake_events: 0x1::event::EventHandle<WithdrawStakeEvent>,
        distribute_commission_events: 0x1::event::EventHandle<DistributeCommissionEvent>,
    }
    
    struct DelegationPoolAllowlisting has key {
        allowlist: 0x1::smart_table::SmartTable<address, bool>,
    }
    
    struct DelegationPoolOwnership has store, key {
        pool_address: address,
    }
    
    struct DisableDelegatorsAllowlisting has drop, store {
        pool_address: address,
    }
    
    struct DistributeCommission has drop, store {
        pool_address: address,
        operator: address,
        beneficiary: address,
        commission_active: u64,
        commission_pending_inactive: u64,
    }
    
    struct DistributeCommissionEvent has drop, store {
        pool_address: address,
        operator: address,
        commission_active: u64,
        commission_pending_inactive: u64,
    }
    
    struct EnableDelegatorsAllowlisting has drop, store {
        pool_address: address,
    }
    
    struct EvictDelegator has drop, store {
        pool_address: address,
        delegator_address: address,
    }
    
    struct GovernanceRecords has key {
        votes: 0x1::smart_table::SmartTable<VotingRecordKey, u64>,
        votes_per_proposal: 0x1::smart_table::SmartTable<u64, u64>,
        vote_delegation: 0x1::smart_table::SmartTable<address, VoteDelegation>,
        delegated_votes: 0x1::smart_table::SmartTable<address, DelegatedVotes>,
        vote_events: 0x1::event::EventHandle<VoteEvent>,
        create_proposal_events: 0x1::event::EventHandle<CreateProposalEvent>,
        delegate_voting_power_events: 0x1::event::EventHandle<DelegateVotingPowerEvent>,
    }
    
    struct NextCommissionPercentage has key {
        commission_percentage_next_lockup_cycle: u64,
        effective_after_secs: u64,
    }
    
    struct ObservedLockupCycle has copy, drop, store {
        index: u64,
    }
    
    struct ReactivateStake has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_reactivated: u64,
    }
    
    struct ReactivateStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_reactivated: u64,
    }
    
    struct RemoveDelegatorFromAllowlist has drop, store {
        pool_address: address,
        delegator_address: address,
    }
    
    struct SetBeneficiaryForOperator has drop, store {
        operator: address,
        old_beneficiary: address,
        new_beneficiary: address,
    }
    
    struct UnlockStake has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_unlocked: u64,
    }
    
    struct UnlockStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_unlocked: u64,
    }
    
    struct Vote has drop, store {
        voter: address,
        proposal_id: u64,
        delegation_pool: address,
        num_votes: u64,
        should_pass: bool,
    }
    
    struct VoteDelegation has copy, drop, store {
        voter: address,
        pending_voter: address,
        last_locked_until_secs: u64,
    }
    
    struct VoteEvent has drop, store {
        voter: address,
        proposal_id: u64,
        delegation_pool: address,
        num_votes: u64,
        should_pass: bool,
    }
    
    struct VotingRecordKey has copy, drop, store {
        voter: address,
        proposal_id: u64,
    }
    
    struct WithdrawStake has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_withdrawn: u64,
    }
    
    struct WithdrawStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_withdrawn: u64,
    }
    
    public fun partial_governance_voting_enabled(arg0: address) : bool {
        exists<GovernanceRecords>(arg0) && 0x1::stake::get_delegated_voter(arg0) == arg0
    }
    
    public entry fun add_stake(arg0: &signer, arg1: address, arg2: u64) acquires BeneficiaryForOperator, DelegationPool, DelegationPoolAllowlisting, GovernanceRecords, NextCommissionPercentage {
        if (arg2 == 0) {
            return
        };
        let v0 = 0x1::signer::address_of(arg0);
        assert_delegator_allowlisted(arg1, v0);
        synchronize_delegation_pool(arg1);
        let v1 = get_add_stake_fee(arg1, arg2);
        let v2 = borrow_global_mut<DelegationPool>(arg1);
        0x1::aptos_account::transfer(arg0, arg1, arg2);
        let v3 = retrieve_stake_pool_owner(v2);
        0x1::stake::add_stake(&v3, arg2);
        buy_in_active_shares(v2, v0, arg2 - v1);
        assert_min_active_balance(v2, v0);
        buy_in_active_shares(v2, @0x0, v1);
        if (0x1::features::module_event_migration_enabled()) {
            let v4 = AddStake{
                pool_address      : arg1, 
                delegator_address : v0, 
                amount_added      : arg2, 
                add_stake_fee     : v1,
            };
            0x1::event::emit<AddStake>(v4);
        };
        let v5 = AddStakeEvent{
            pool_address      : arg1, 
            delegator_address : v0, 
            amount_added      : arg2, 
            add_stake_fee     : v1,
        };
        0x1::event::emit_event<AddStakeEvent>(&mut v2.add_stake_events, v5);
    }
    
    public fun get_stake(arg0: address, arg1: address) : (u64, u64, u64) acquires BeneficiaryForOperator, DelegationPool {
        assert_delegation_pool_exists(arg0);
        let v0 = borrow_global<DelegationPool>(arg0);
        let (v1, v2, _, v4, v5) = calculate_stake_pool_drift(v0);
        let v6 = 0x1::pool_u64_unbound::total_shares(&v0.active_shares);
        let v7 = v6;
        let v8 = 0x1::pool_u64_unbound::shares(&v0.active_shares, arg1);
        let (_, _, v11, _) = 0x1::stake::get_stake(arg0);
        if (v11 == 0) {
            v7 = v6 - 0x1::pool_u64_unbound::shares(&v0.active_shares, @0x0);
            if (arg1 == @0x0) {
                v8 = 0;
            };
        };
        let v13 = 0x1::pool_u64_unbound::shares_to_amount_with_total_stats(&v0.active_shares, v8, v2 - v4, v7);
        let v14 = v13;
        let (v15, v16) = get_pending_withdrawal(arg0, arg1);
        let (v17, v18) = if (v15) {
            (v16, 0)
        } else {
            (0, v16)
        };
        let v19 = v18;
        let v20 = v17;
        let v21 = beneficiary_for_operator(0x1::stake::get_operator(arg0));
        if (arg1 == v21) {
            v14 = v13 + v4;
            if (v1) {
                v20 = v17 + v5;
            } else {
                v19 = v18 + v5;
            };
        };
        (v14, v20, v19)
    }
    
    public entry fun reactivate_stake(arg0: &signer, arg1: address, arg2: u64) acquires BeneficiaryForOperator, DelegationPool, DelegationPoolAllowlisting, GovernanceRecords, NextCommissionPercentage {
        if (arg2 == 0) {
            return
        };
        let v0 = 0x1::signer::address_of(arg0);
        assert_delegator_allowlisted(arg1, v0);
        synchronize_delegation_pool(arg1);
        let v1 = borrow_global_mut<DelegationPool>(arg1);
        let v2 = coins_to_transfer_to_ensure_min_stake(pending_inactive_shares_pool(v1), &v1.active_shares, v0, arg2);
        let v3 = redeem_inactive_shares(v1, v0, v2, v1.observed_lockup_cycle);
        let v4 = retrieve_stake_pool_owner(v1);
        0x1::stake::reactivate_stake(&v4, v3);
        buy_in_active_shares(v1, v0, v3);
        assert_min_active_balance(v1, v0);
        if (0x1::features::module_event_migration_enabled()) {
            let v5 = ReactivateStake{
                pool_address       : arg1, 
                delegator_address  : v0, 
                amount_reactivated : v3,
            };
            0x1::event::emit<ReactivateStake>(v5);
        };
        let v6 = ReactivateStakeEvent{
            pool_address       : arg1, 
            delegator_address  : v0, 
            amount_reactivated : v3,
        };
        0x1::event::emit_event<ReactivateStakeEvent>(&mut v1.reactivate_stake_events, v6);
    }
    
    public entry fun set_delegated_voter(arg0: &signer, arg1: address) acquires BeneficiaryForOperator, DelegationPool, DelegationPoolOwnership, GovernanceRecords, NextCommissionPercentage {
        let v0 = !0x1::features::delegation_pool_partial_governance_voting_enabled();
        assert!(v0, 0x1::error::invalid_state(12));
        let v1 = get_owned_pool_address(0x1::signer::address_of(arg0));
        synchronize_delegation_pool(v1);
        let v2 = retrieve_stake_pool_owner(borrow_global<DelegationPool>(v1));
        0x1::stake::set_delegated_voter(&v2, arg1);
    }
    
    public entry fun set_operator(arg0: &signer, arg1: address) acquires BeneficiaryForOperator, DelegationPool, DelegationPoolOwnership, GovernanceRecords, NextCommissionPercentage {
        let v0 = get_owned_pool_address(0x1::signer::address_of(arg0));
        synchronize_delegation_pool(v0);
        let v1 = retrieve_stake_pool_owner(borrow_global<DelegationPool>(v0));
        0x1::stake::set_operator(&v1, arg1);
    }
    
    public entry fun unlock(arg0: &signer, arg1: address, arg2: u64) acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        if (arg2 == 0) {
            return
        };
        synchronize_delegation_pool(arg1);
        unlock_internal(0x1::signer::address_of(arg0), arg1, arg2);
    }
    
    public entry fun withdraw(arg0: &signer, arg1: address, arg2: u64) acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert!(arg2 > 0, 0x1::error::invalid_argument(11));
        synchronize_delegation_pool(arg1);
        withdraw_internal(borrow_global_mut<DelegationPool>(arg1), 0x1::signer::address_of(arg0), arg2);
    }
    
    public entry fun allowlist_delegator(arg0: &signer, arg1: address) acquires DelegationPoolAllowlisting, DelegationPoolOwnership {
        let v0 = get_owned_pool_address(0x1::signer::address_of(arg0));
        assert_allowlisting_enabled(v0);
        if (delegator_allowlisted(v0, arg1)) {
            return
        };
        let v1 = &mut borrow_global_mut<DelegationPoolAllowlisting>(v0).allowlist;
        0x1::smart_table::add<address, bool>(v1, arg1, true);
        let v2 = AllowlistDelegator{
            pool_address      : v0, 
            delegator_address : arg1,
        };
        0x1::event::emit<AllowlistDelegator>(v2);
    }
    
    public fun allowlisting_enabled(arg0: address) : bool {
        assert_delegation_pool_exists(arg0);
        exists<DelegationPoolAllowlisting>(arg0)
    }
    
    fun amount_to_shares_to_redeem(arg0: &0x1::pool_u64_unbound::Pool, arg1: address, arg2: u64) : u128 {
        if (arg2 >= 0x1::pool_u64_unbound::balance(arg0, arg1)) {
            0x1::pool_u64_unbound::shares(arg0, arg1)
        } else {
            0x1::pool_u64_unbound::amount_to_shares(arg0, arg2)
        }
    }
    
    fun assert_allowlisting_enabled(arg0: address) {
        assert!(allowlisting_enabled(arg0), 0x1::error::invalid_state(24));
    }
    
    fun assert_delegation_pool_exists(arg0: address) {
        assert!(delegation_pool_exists(arg0), 0x1::error::invalid_argument(3));
    }
    
    fun assert_delegator_allowlisted(arg0: address, arg1: address) acquires DelegationPoolAllowlisting {
        assert!(delegator_allowlisted(arg0, arg1), 0x1::error::permission_denied(25));
    }
    
    fun assert_min_active_balance(arg0: &DelegationPool, arg1: address) {
        let v0 = 0x1::pool_u64_unbound::balance(&arg0.active_shares, arg1) >= 1000000000;
        assert!(v0, 0x1::error::invalid_argument(8));
    }
    
    fun assert_min_pending_inactive_balance(arg0: &DelegationPool, arg1: address) {
        let v0 = 0x1::pool_u64_unbound::balance(pending_inactive_shares_pool(arg0), arg1) >= 1000000000;
        assert!(v0, 0x1::error::invalid_argument(9));
    }
    
    fun assert_owner_cap_exists(arg0: address) {
        assert!(owner_cap_exists(arg0), 0x1::error::not_found(1));
    }
    
    fun assert_partial_governance_voting_enabled(arg0: address) {
        assert_delegation_pool_exists(arg0);
        assert!(partial_governance_voting_enabled(arg0), 0x1::error::invalid_state(14));
    }
    
    public fun beneficiary_for_operator(arg0: address) : address acquires BeneficiaryForOperator {
        if (exists<BeneficiaryForOperator>(arg0)) {
            return borrow_global<BeneficiaryForOperator>(arg0).beneficiary_for_operator
        };
        arg0
    }
    
    fun buy_in_active_shares(arg0: &mut DelegationPool, arg1: address, arg2: u64) : u128 acquires GovernanceRecords {
        let v0 = 0x1::pool_u64_unbound::amount_to_shares(&arg0.active_shares, arg2);
        if (v0 == 0) {
            return 0
        };
        let v1 = get_pool_address(arg0);
        if (partial_governance_voting_enabled(v1)) {
            update_governance_records_for_buy_in_active_shares(arg0, v1, v0, arg1);
        };
        0x1::pool_u64_unbound::buy_in(&mut arg0.active_shares, arg1, arg2);
        v0
    }
    
    fun buy_in_pending_inactive_shares(arg0: &mut DelegationPool, arg1: address, arg2: u64) : u128 acquires GovernanceRecords {
        let v0 = 0x1::pool_u64_unbound::amount_to_shares(pending_inactive_shares_pool(arg0), arg2);
        if (v0 == 0) {
            return 0
        };
        let v1 = get_pool_address(arg0);
        if (partial_governance_voting_enabled(v1)) {
            update_governance_records_for_buy_in_pending_inactive_shares(arg0, v1, v0, arg1);
        };
        0x1::pool_u64_unbound::buy_in(pending_inactive_shares_pool_mut(arg0), arg1, arg2);
        execute_pending_withdrawal(arg0, arg1);
        let v2 = arg0.observed_lockup_cycle;
        let v3 = &mut arg0.pending_withdrawals;
        let v4 = 0x1::table::borrow_mut_with_default<address, ObservedLockupCycle>(v3, arg1, v2);
        assert!(*v4 == v2, 0x1::error::invalid_state(4));
        v0
    }
    
    fun calculate_and_update_delegated_votes(arg0: &DelegationPool, arg1: &mut GovernanceRecords, arg2: address) : u64 {
        calculate_total_voting_power(arg0, update_and_borrow_mut_delegated_votes(arg0, arg1, arg2))
    }
    
    public fun calculate_and_update_delegator_voter(arg0: address, arg1: address) : address acquires DelegationPool, GovernanceRecords {
        assert_partial_governance_voting_enabled(arg0);
        let v0 = borrow_global_mut<GovernanceRecords>(arg0);
        calculate_and_update_delegator_voter_internal(borrow_global<DelegationPool>(arg0), v0, arg1)
    }
    
    fun calculate_and_update_delegator_voter_internal(arg0: &DelegationPool, arg1: &mut GovernanceRecords, arg2: address) : address {
        update_and_borrow_mut_delegator_vote_delegation(arg0, arg1, arg2).voter
    }
    
    public fun calculate_and_update_remaining_voting_power(arg0: address, arg1: address, arg2: u64) : u64 acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert_partial_governance_voting_enabled(arg0);
        if (0x1::aptos_governance::get_remaining_voting_power(arg0, arg2) == 0) {
            return 0
        };
        let v0 = calculate_and_update_voter_total_voting_power(arg0, arg1);
        v0 - get_used_voting_power(borrow_global<GovernanceRecords>(arg0), arg1, arg2)
    }
    
    public fun calculate_and_update_voter_total_voting_power(arg0: address, arg1: address) : u64 acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert_partial_governance_voting_enabled(arg0);
        synchronize_delegation_pool(arg0);
        let v0 = borrow_global<DelegationPool>(arg0);
        let v1 = update_and_borrow_mut_delegated_votes(v0, borrow_global_mut<GovernanceRecords>(arg0), arg1);
        calculate_total_voting_power(v0, v1)
    }
    
    public fun calculate_and_update_voting_delegation(arg0: address, arg1: address) : (address, address, u64) acquires DelegationPool, GovernanceRecords {
        assert_partial_governance_voting_enabled(arg0);
        let v0 = borrow_global_mut<GovernanceRecords>(arg0);
        let v1 = update_and_borrow_mut_delegator_vote_delegation(borrow_global<DelegationPool>(arg0), v0, arg1);
        (v1.voter, v1.pending_voter, v1.last_locked_until_secs)
    }
    
    fun calculate_stake_pool_drift(arg0: &DelegationPool) : (bool, u64, u64, u64, u64) {
        let (v0, v1, v2, v3) = 0x1::stake::get_stake(get_pool_address(arg0));
        let v4 = v3;
        assert!(v1 >= arg0.total_coins_inactive, 0x1::error::invalid_state(7));
        let v5 = v1 > arg0.total_coins_inactive;
        let v6 = v0 + v2;
        if (v5) {
            v4 = v1 - arg0.total_coins_inactive;
        };
        let v7 = 0x1::pool_u64_unbound::total_coins(&arg0.active_shares);
        let v8 = if (v6 > v7) {
            let v9 = 10000;
            assert!(v9 != 0, 0x1::error::invalid_argument(4));
            ((((v6 - v7) as u128) * (arg0.operator_commission_percentage as u128) / (v9 as u128)) as u64)
        } else {
            0
        };
        let v10 = 0x1::pool_u64_unbound::total_coins(pending_inactive_shares_pool(arg0));
        let v11 = if (v4 > v10) {
            let v12 = 10000;
            assert!(v12 != 0, 0x1::error::invalid_argument(4));
            ((((v4 - v10) as u128) * (arg0.operator_commission_percentage as u128) / (v12 as u128)) as u64)
        } else {
            0
        };
        (v5, v6, v4, v8, v11)
    }
    
    fun calculate_total_voting_power(arg0: &DelegationPool, arg1: &DelegatedVotes) : u64 {
        let v0 = 0x1::pool_u64_unbound::shares_to_amount(&arg0.active_shares, arg1.active_shares);
        let v1 = pending_inactive_shares_pool(arg0);
        v0 + 0x1::pool_u64_unbound::shares_to_amount(v1, arg1.pending_inactive_shares)
    }
    
    public fun can_withdraw_pending_inactive(arg0: address) : bool {
        let v0 = 0x1::stake::get_validator_state(arg0) == 4;
        v0 && 0x1::timestamp::now_seconds() >= 0x1::stake::get_lockup_secs(arg0)
    }
    
    fun coins_to_redeem_to_ensure_min_stake(arg0: &0x1::pool_u64_unbound::Pool, arg1: address, arg2: u64) : u64 {
        let v0 = 0x1::pool_u64_unbound::balance(arg0, arg1);
        let v1 = v0 - 0x1::pool_u64_unbound::shares_to_amount(arg0, amount_to_shares_to_redeem(arg0, arg1, arg2));
        if (v1 < 1000000000) {
            arg2 = v0;
        };
        arg2
    }
    
    fun coins_to_transfer_to_ensure_min_stake(arg0: &0x1::pool_u64_unbound::Pool, arg1: &0x1::pool_u64_unbound::Pool, arg2: address, arg3: u64) : u64 {
        let v0 = 0x1::pool_u64_unbound::balance(arg1, arg2);
        let v1 = v0 + 0x1::pool_u64_unbound::shares_to_amount(arg0, amount_to_shares_to_redeem(arg0, arg2, arg3));
        if (v1 < 1000000000) {
            arg3 = 1000000000 - v0 + 1;
        };
        coins_to_redeem_to_ensure_min_stake(arg0, arg2, arg3)
    }
    
    public entry fun create_proposal(arg0: &signer, arg1: address, arg2: vector<u8>, arg3: vector<u8>, arg4: vector<u8>, arg5: bool) acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert_partial_governance_voting_enabled(arg1);
        synchronize_delegation_pool(arg1);
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = borrow_global<DelegationPool>(arg1);
        let v2 = 0x1::aptos_governance::get_required_proposer_stake();
        let v3 = calculate_and_update_delegated_votes(v1, borrow_global_mut<GovernanceRecords>(arg1), v0) >= v2;
        assert!(v3, 0x1::error::invalid_argument(15));
        let v4 = retrieve_stake_pool_owner(borrow_global<DelegationPool>(arg1));
        let v5 = 0x1::aptos_governance::create_proposal_v2_impl(&v4, arg1, arg2, arg3, arg4, arg5);
        if (0x1::features::module_event_migration_enabled()) {
            let v6 = CreateProposal{
                proposal_id     : v5, 
                voter           : v0, 
                delegation_pool : arg1,
            };
            0x1::event::emit<CreateProposal>(v6);
        };
        let v7 = &mut borrow_global_mut<GovernanceRecords>(arg1).create_proposal_events;
        let v8 = CreateProposalEvent{
            proposal_id     : v5, 
            voter           : v0, 
            delegation_pool : arg1,
        };
        0x1::event::emit_event<CreateProposalEvent>(v7, v8);
    }
    
    fun create_resource_account_seed(arg0: vector<u8>) : vector<u8> {
        let v0 = 0x1::vector::empty<u8>();
        0x1::vector::append<u8>(&mut v0, b"aptos_framework::delegation_pool");
        0x1::vector::append<u8>(&mut v0, arg0);
        v0
    }
    
    public entry fun delegate_voting_power(arg0: &signer, arg1: address, arg2: address) acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert_partial_governance_voting_enabled(arg1);
        synchronize_delegation_pool(arg1);
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = borrow_global<DelegationPool>(arg1);
        let v2 = borrow_global_mut<GovernanceRecords>(arg1);
        let v3 = update_and_borrow_mut_delegator_vote_delegation(v1, v2, v0);
        let v4 = v3.pending_voter;
        if (v4 != arg2) {
            v3.pending_voter = arg2;
            let v5 = get_delegator_active_shares(v1, v0);
            let v6 = update_and_borrow_mut_delegated_votes(v1, v2, v4);
            v6.active_shares_next_lockup = v6.active_shares_next_lockup - v5;
            let v7 = update_and_borrow_mut_delegated_votes(v1, v2, arg2);
            v7.active_shares_next_lockup = v7.active_shares_next_lockup + v5;
        };
        if (0x1::features::module_event_migration_enabled()) {
            let v8 = DelegateVotingPower{
                pool_address : arg1, 
                delegator    : v0, 
                voter        : arg2,
            };
            0x1::event::emit<DelegateVotingPower>(v8);
        };
        let v9 = DelegateVotingPowerEvent{
            pool_address : arg1, 
            delegator    : v0, 
            voter        : arg2,
        };
        0x1::event::emit_event<DelegateVotingPowerEvent>(&mut v2.delegate_voting_power_events, v9);
    }
    
    public fun delegation_pool_exists(arg0: address) : bool {
        exists<DelegationPool>(arg0)
    }
    
    public fun delegator_allowlisted(arg0: address, arg1: address) : bool acquires DelegationPoolAllowlisting {
        if (!allowlisting_enabled(arg0)) {
            return true
        };
        let v0 = &mut borrow_global_mut<DelegationPoolAllowlisting>(arg0).allowlist;
        0x1::smart_table::contains<address, bool>(v0, arg1)
    }
    
    public entry fun disable_delegators_allowlisting(arg0: &signer) acquires DelegationPoolAllowlisting, DelegationPoolOwnership {
        let v0 = get_owned_pool_address(0x1::signer::address_of(arg0));
        assert_allowlisting_enabled(v0);
        let DelegationPoolAllowlisting { allowlist: v1 } = move_from<DelegationPoolAllowlisting>(v0);
        0x1::smart_table::destroy<address, bool>(v1);
        let v2 = DisableDelegatorsAllowlisting{pool_address: v0};
        0x1::event::emit<DisableDelegatorsAllowlisting>(v2);
    }
    
    public entry fun enable_delegators_allowlisting(arg0: &signer) acquires DelegationPool, DelegationPoolOwnership {
        assert!(0x1::features::delegation_pool_allowlisting_enabled(), 0x1::error::invalid_state(23));
        let v0 = get_owned_pool_address(0x1::signer::address_of(arg0));
        if (allowlisting_enabled(v0)) {
            return
        };
        let v1 = retrieve_stake_pool_owner(borrow_global<DelegationPool>(v0));
        let v2 = DelegationPoolAllowlisting{allowlist: 0x1::smart_table::new<address, bool>()};
        move_to<DelegationPoolAllowlisting>(&v1, v2);
        let v3 = EnableDelegatorsAllowlisting{pool_address: v0};
        0x1::event::emit<EnableDelegatorsAllowlisting>(v3);
    }
    
    public entry fun enable_partial_governance_voting(arg0: address) acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert!(0x1::features::partial_governance_voting_enabled(), 0x1::error::invalid_state(13));
        assert!(0x1::features::delegation_pool_partial_governance_voting_enabled(), 0x1::error::invalid_state(13));
        assert_delegation_pool_exists(arg0);
        synchronize_delegation_pool(arg0);
        let v0 = retrieve_stake_pool_owner(borrow_global<DelegationPool>(arg0));
        0x1::stake::set_delegated_voter(&v0, 0x1::signer::address_of(&v0));
        let v1 = 0x1::smart_table::new<VotingRecordKey, u64>();
        let v2 = 0x1::smart_table::new<u64, u64>();
        let v3 = 0x1::smart_table::new<address, VoteDelegation>();
        let v4 = 0x1::smart_table::new<address, DelegatedVotes>();
        let v5 = 0x1::account::new_event_handle<VoteEvent>(&v0);
        let v6 = 0x1::account::new_event_handle<CreateProposalEvent>(&v0);
        let v7 = 0x1::account::new_event_handle<DelegateVotingPowerEvent>(&v0);
        let v8 = GovernanceRecords{
            votes                        : v1, 
            votes_per_proposal           : v2, 
            vote_delegation              : v3, 
            delegated_votes              : v4, 
            vote_events                  : v5, 
            create_proposal_events       : v6, 
            delegate_voting_power_events : v7,
        };
        move_to<GovernanceRecords>(&v0, v8);
    }
    
    public entry fun evict_delegator(arg0: &signer, arg1: address) acquires BeneficiaryForOperator, DelegationPool, DelegationPoolAllowlisting, DelegationPoolOwnership, GovernanceRecords, NextCommissionPercentage {
        let v0 = get_owned_pool_address(0x1::signer::address_of(arg0));
        assert_allowlisting_enabled(v0);
        let v1 = delegator_allowlisted(v0, arg1);
        assert!(!v1, 0x1::error::invalid_state(26));
        synchronize_delegation_pool(v0);
        let v2 = borrow_global<DelegationPool>(v0);
        if (get_delegator_active_shares(v2, arg1) == 0) {
            return
        };
        unlock_internal(arg1, v0, 0x1::pool_u64_unbound::balance(&v2.active_shares, arg1));
        let v3 = EvictDelegator{
            pool_address      : v0, 
            delegator_address : arg1,
        };
        0x1::event::emit<EvictDelegator>(v3);
    }
    
    fun execute_pending_withdrawal(arg0: &mut DelegationPool, arg1: address) acquires GovernanceRecords {
        let (v0, v1) = pending_withdrawal_exists(arg0, arg1);
        let v2 = v1;
        if (v0 && v2.index < arg0.observed_lockup_cycle.index) {
            withdraw_internal(arg0, arg1, 18446744073709551615);
        };
    }
    
    public fun get_add_stake_fee(arg0: address, arg1: u64) : u64 acquires DelegationPool, NextCommissionPercentage {
        if (0x1::stake::is_current_epoch_validator(arg0)) {
            let v1 = 0x1::staking_config::get();
            let (v2, v3) = 0x1::staking_config::get_reward_rate(&v1);
            let v4 = if (v3 > 0) {
                assert_delegation_pool_exists(arg0);
                let v5 = operator_commission_percentage(arg0);
                let v6 = v2 * (10000 - v5);
                (((arg1 as u128) * (v6 as u128) / ((v6 as u128) + ((v3 * 10000) as u128))) as u64)
            } else {
                0
            };
            v4
        } else {
            0
        }
    }
    
    public fun get_delegation_pool_stake(arg0: address) : (u64, u64, u64, u64) {
        assert_delegation_pool_exists(arg0);
        0x1::stake::get_stake(arg0)
    }
    
    fun get_delegator_active_shares(arg0: &DelegationPool, arg1: address) : u128 {
        0x1::pool_u64_unbound::shares(&arg0.active_shares, arg1)
    }
    
    fun get_delegator_pending_inactive_shares(arg0: &DelegationPool, arg1: address) : u128 {
        0x1::pool_u64_unbound::shares(pending_inactive_shares_pool(arg0), arg1)
    }
    
    public fun get_delegators_allowlist(arg0: address) : vector<address> acquires DelegationPoolAllowlisting {
        assert_allowlisting_enabled(arg0);
        let v0 = vector[];
        let v1 = &mut borrow_global_mut<DelegationPoolAllowlisting>(arg0).allowlist;
        let v2 = 0;
        while (v2 < 0x1::smart_table::num_buckets<address, bool>(v1)) {
            let v3 = 0x1::smart_table::borrow_buckets<address, bool>(v1);
            let v4 = 0x1::table_with_length::borrow<u64, vector<0x1::smart_table::Entry<address, bool>>>(v3, v2);
            let v5 = 0;
            while (v5 < 0x1::vector::length<0x1::smart_table::Entry<address, bool>>(v4)) {
                let v6 = 0x1::vector::borrow<0x1::smart_table::Entry<address, bool>>(v4, v5);
                let (v7, _) = 0x1::smart_table::borrow_kv<address, bool>(v6);
                0x1::vector::push_back<address>(&mut v0, *v7);
                v5 = v5 + 1;
            };
            v2 = v2 + 1;
        };
        v0
    }
    
    public fun get_expected_stake_pool_address(arg0: address, arg1: vector<u8>) : address {
        0x1::account::create_resource_address(&arg0, create_resource_account_seed(arg1))
    }
    
    public fun get_owned_pool_address(arg0: address) : address acquires DelegationPoolOwnership {
        assert_owner_cap_exists(arg0);
        borrow_global<DelegationPoolOwnership>(arg0).pool_address
    }
    
    public fun get_pending_withdrawal(arg0: address, arg1: address) : (bool, u64) acquires DelegationPool {
        assert_delegation_pool_exists(arg0);
        let v0 = borrow_global<DelegationPool>(arg0);
        let (v1, _, v3, _, v5) = calculate_stake_pool_drift(v0);
        let (v6, v7) = pending_withdrawal_exists(v0, arg1);
        let v8 = v7;
        if (!v6) {
            (false, 0)
        } else {
            let v11 = 0x1::table::borrow<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>(&v0.inactive_shares, v8);
            let (v12, v13) = if (v8.index < v0.observed_lockup_cycle.index) {
                (true, 0x1::pool_u64_unbound::balance(v11, arg1))
            } else {
                (v1, 0x1::pool_u64_unbound::shares_to_amount_with_total_coins(v11, 0x1::pool_u64_unbound::shares(v11, arg1), v3 - v5))
            };
            (v12, v13)
        }
    }
    
    fun get_pool_address(arg0: &DelegationPool) : address {
        0x1::account::get_signer_capability_address(&arg0.stake_pool_signer_cap)
    }
    
    fun get_used_voting_power(arg0: &GovernanceRecords, arg1: address, arg2: u64) : u64 {
        let v0 = VotingRecordKey{
            voter       : arg1, 
            proposal_id : arg2,
        };
        let v1 = 0;
        *0x1::smart_table::borrow_with_default<VotingRecordKey, u64>(&arg0.votes, v0, &v1)
    }
    
    public entry fun initialize_delegation_pool(arg0: &signer, arg1: u64, arg2: vector<u8>) acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert!(0x1::features::delegation_pools_enabled(), 0x1::error::invalid_state(10));
        let v0 = 0x1::signer::address_of(arg0);
        assert!(!owner_cap_exists(v0), 0x1::error::already_exists(2));
        assert!(arg1 <= 10000, 0x1::error::invalid_argument(5));
        let (v1, v2) = 0x1::account::create_resource_account(arg0, create_resource_account_seed(arg2));
        let v3 = v1;
        0x1::coin::register<0x1::aptos_coin::AptosCoin>(&v3);
        let v4 = 0x1::signer::address_of(&v3);
        0x1::stake::initialize_stake_owner(&v3, 0, v0, v0);
        let v5 = 0x1::table::new<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>();
        0x1::table::add<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>(&mut v5, olc_with_index(0), 0x1::pool_u64_unbound::create_with_scaling_factor(10000000000000000));
        let v6 = 0x1::pool_u64_unbound::create_with_scaling_factor(10000000000000000);
        let v7 = olc_with_index(0);
        let v8 = 0x1::table::new<address, ObservedLockupCycle>();
        let v9 = 0x1::account::new_event_handle<AddStakeEvent>(&v3);
        let v10 = 0x1::account::new_event_handle<ReactivateStakeEvent>(&v3);
        let v11 = 0x1::account::new_event_handle<UnlockStakeEvent>(&v3);
        let v12 = 0x1::account::new_event_handle<WithdrawStakeEvent>(&v3);
        let v13 = 0x1::account::new_event_handle<DistributeCommissionEvent>(&v3);
        let v14 = DelegationPool{
            active_shares                  : v6, 
            observed_lockup_cycle          : v7, 
            inactive_shares                : v5, 
            pending_withdrawals            : v8, 
            stake_pool_signer_cap          : v2, 
            total_coins_inactive           : 0, 
            operator_commission_percentage : arg1, 
            add_stake_events               : v9, 
            reactivate_stake_events        : v10, 
            unlock_stake_events            : v11, 
            withdraw_stake_events          : v12, 
            distribute_commission_events   : v13,
        };
        move_to<DelegationPool>(&v3, v14);
        let v15 = DelegationPoolOwnership{pool_address: v4};
        move_to<DelegationPoolOwnership>(arg0, v15);
        if (0x1::features::partial_governance_voting_enabled() && 0x1::features::delegation_pool_partial_governance_voting_enabled()) {
            enable_partial_governance_voting(v4);
        };
    }
    
    public fun is_next_commission_percentage_effective(arg0: address) : bool acquires NextCommissionPercentage {
        let v0 = exists<NextCommissionPercentage>(arg0);
        v0 && 0x1::timestamp::now_seconds() >= borrow_global<NextCommissionPercentage>(arg0).effective_after_secs
    }
    
    public fun min_remaining_secs_for_commission_change() : u64 {
        let v0 = 0x1::staking_config::get();
        0x1::staking_config::get_recurring_lockup_duration(&v0) / 4
    }
    
    public fun multiply_then_divide(arg0: u64, arg1: u64, arg2: u64) : u64 {
        assert!(arg2 != 0, 0x1::error::invalid_argument(4));
        (((arg0 as u128) * (arg1 as u128) / (arg2 as u128)) as u64)
    }
    
    public fun observed_lockup_cycle(arg0: address) : u64 acquires DelegationPool {
        assert_delegation_pool_exists(arg0);
        borrow_global<DelegationPool>(arg0).observed_lockup_cycle.index
    }
    
    fun olc_with_index(arg0: u64) : ObservedLockupCycle {
        ObservedLockupCycle{index: arg0}
    }
    
    public fun operator_commission_percentage(arg0: address) : u64 acquires DelegationPool, NextCommissionPercentage {
        assert_delegation_pool_exists(arg0);
        if (is_next_commission_percentage_effective(arg0)) {
            operator_commission_percentage_next_lockup_cycle(arg0)
        } else {
            borrow_global<DelegationPool>(arg0).operator_commission_percentage
        }
    }
    
    public fun operator_commission_percentage_next_lockup_cycle(arg0: address) : u64 acquires DelegationPool, NextCommissionPercentage {
        assert_delegation_pool_exists(arg0);
        if (exists<NextCommissionPercentage>(arg0)) {
            borrow_global<NextCommissionPercentage>(arg0).commission_percentage_next_lockup_cycle
        } else {
            borrow_global<DelegationPool>(arg0).operator_commission_percentage
        }
    }
    
    public fun owner_cap_exists(arg0: address) : bool {
        exists<DelegationPoolOwnership>(arg0)
    }
    
    fun pending_inactive_shares_pool(arg0: &DelegationPool) : &0x1::pool_u64_unbound::Pool {
        let v0 = arg0.observed_lockup_cycle;
        0x1::table::borrow<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>(&arg0.inactive_shares, v0)
    }
    
    fun pending_inactive_shares_pool_mut(arg0: &mut DelegationPool) : &mut 0x1::pool_u64_unbound::Pool {
        let v0 = arg0.observed_lockup_cycle;
        let v1 = &mut arg0.inactive_shares;
        0x1::table::borrow_mut<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>(v1, v0)
    }
    
    fun pending_withdrawal_exists(arg0: &DelegationPool, arg1: address) : (bool, ObservedLockupCycle) {
        if (0x1::table::contains<address, ObservedLockupCycle>(&arg0.pending_withdrawals, arg1)) {
            (true, *0x1::table::borrow<address, ObservedLockupCycle>(&arg0.pending_withdrawals, arg1))
        } else {
            (false, olc_with_index(0))
        }
    }
    
    fun redeem_active_shares(arg0: &mut DelegationPool, arg1: address, arg2: u64) : u64 acquires GovernanceRecords {
        let v0 = amount_to_shares_to_redeem(&arg0.active_shares, arg1, arg2);
        if (v0 == 0) {
            return 0
        };
        let v1 = get_pool_address(arg0);
        if (partial_governance_voting_enabled(v1)) {
            update_governanace_records_for_redeem_active_shares(arg0, v1, v0, arg1);
        };
        0x1::pool_u64_unbound::redeem_shares(&mut arg0.active_shares, arg1, v0)
    }
    
    fun redeem_inactive_shares(arg0: &mut DelegationPool, arg1: address, arg2: u64, arg3: ObservedLockupCycle) : u64 acquires GovernanceRecords {
        let v0 = 0x1::table::borrow<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>(&arg0.inactive_shares, arg3);
        let v1 = amount_to_shares_to_redeem(v0, arg1, arg2);
        if (v1 == 0) {
            return 0
        };
        let v2 = get_pool_address(arg0);
        if (partial_governance_voting_enabled(v2) && arg3.index == arg0.observed_lockup_cycle.index) {
            update_governanace_records_for_redeem_pending_inactive_shares(arg0, v2, v1, arg1);
        };
        let v3 = &mut arg0.inactive_shares;
        let v4 = 0x1::table::borrow_mut<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>(v3, arg3);
        if (0x1::pool_u64_unbound::shares(v4, arg1) == 0) {
            0x1::table::remove<address, ObservedLockupCycle>(&mut arg0.pending_withdrawals, arg1);
        };
        if (arg3.index < arg0.observed_lockup_cycle.index && 0x1::pool_u64_unbound::total_coins(v4) == 0) {
            let v5 = arg3;
            let v6 = 0x1::table::remove<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>(&mut arg0.inactive_shares, v5);
            0x1::pool_u64_unbound::destroy_empty(v6);
        };
        0x1::pool_u64_unbound::redeem_shares(v4, arg1, v1)
    }
    
    public entry fun remove_delegator_from_allowlist(arg0: &signer, arg1: address) acquires DelegationPoolAllowlisting, DelegationPoolOwnership {
        let v0 = get_owned_pool_address(0x1::signer::address_of(arg0));
        assert_allowlisting_enabled(v0);
        let v1 = delegator_allowlisted(v0, arg1);
        if (!v1) {
            return
        };
        let v2 = &mut borrow_global_mut<DelegationPoolAllowlisting>(v0).allowlist;
        0x1::smart_table::remove<address, bool>(v2, arg1);
        let v3 = RemoveDelegatorFromAllowlist{
            pool_address      : v0, 
            delegator_address : arg1,
        };
        0x1::event::emit<RemoveDelegatorFromAllowlist>(v3);
    }
    
    fun retrieve_stake_pool_owner(arg0: &DelegationPool) : signer {
        0x1::account::create_signer_with_capability(&arg0.stake_pool_signer_cap)
    }
    
    public entry fun set_beneficiary_for_operator(arg0: &signer, arg1: address) acquires BeneficiaryForOperator {
        assert!(0x1::features::operator_beneficiary_change_enabled(), 0x1::error::invalid_state(19));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = beneficiary_for_operator(v0);
        if (exists<BeneficiaryForOperator>(v0)) {
            borrow_global_mut<BeneficiaryForOperator>(v0).beneficiary_for_operator = arg1;
        } else {
            let v2 = BeneficiaryForOperator{beneficiary_for_operator: arg1};
            move_to<BeneficiaryForOperator>(arg0, v2);
        };
        let v3 = SetBeneficiaryForOperator{
            operator        : v0, 
            old_beneficiary : v1, 
            new_beneficiary : arg1,
        };
        0x1::event::emit<SetBeneficiaryForOperator>(v3);
    }
    
    public fun shareholders_count_active_pool(arg0: address) : u64 acquires DelegationPool {
        assert_delegation_pool_exists(arg0);
        0x1::pool_u64_unbound::shareholders_count(&borrow_global<DelegationPool>(arg0).active_shares)
    }
    
    public entry fun synchronize_delegation_pool(arg0: address) acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert_delegation_pool_exists(arg0);
        let v0 = borrow_global_mut<DelegationPool>(arg0);
        let (v1, v2, v3, v4, v5) = calculate_stake_pool_drift(v0);
        let (_, _, v8, _) = 0x1::stake::get_stake(arg0);
        if (v8 == 0) {
            redeem_active_shares(v0, @0x0, 18446744073709551615);
        };
        0x1::pool_u64_unbound::update_total_coins(&mut v0.active_shares, v2 - v4);
        0x1::pool_u64_unbound::update_total_coins(pending_inactive_shares_pool_mut(v0), v3 - v5);
        let v10 = beneficiary_for_operator(0x1::stake::get_operator(arg0));
        buy_in_active_shares(v0, v10, v4);
        let v11 = beneficiary_for_operator(0x1::stake::get_operator(arg0));
        buy_in_pending_inactive_shares(v0, v11, v5);
        let v12 = 0x1::stake::get_operator(arg0);
        let v13 = DistributeCommissionEvent{
            pool_address                : arg0, 
            operator                    : v12, 
            commission_active           : v4, 
            commission_pending_inactive : v5,
        };
        0x1::event::emit_event<DistributeCommissionEvent>(&mut v0.distribute_commission_events, v13);
        if (0x1::features::operator_beneficiary_change_enabled()) {
            let v14 = 0x1::stake::get_operator(arg0);
            let v15 = beneficiary_for_operator(0x1::stake::get_operator(arg0));
            let v16 = DistributeCommission{
                pool_address                : arg0, 
                operator                    : v14, 
                beneficiary                 : v15, 
                commission_active           : v4, 
                commission_pending_inactive : v5,
            };
            0x1::event::emit<DistributeCommission>(v16);
        };
        if (v1) {
            let (_, v18, _, _) = 0x1::stake::get_stake(arg0);
            v0.total_coins_inactive = v18;
            v0.observed_lockup_cycle.index = v0.observed_lockup_cycle.index + 1;
            let v21 = 0x1::pool_u64_unbound::create_with_scaling_factor(10000000000000000);
            0x1::table::add<ObservedLockupCycle, 0x1::pool_u64_unbound::Pool>(&mut v0.inactive_shares, v0.observed_lockup_cycle, v21);
        };
        if (is_next_commission_percentage_effective(arg0)) {
            v0.operator_commission_percentage = borrow_global<NextCommissionPercentage>(arg0).commission_percentage_next_lockup_cycle;
        };
    }
    
    fun unlock_internal(arg0: address, arg1: address, arg2: u64) acquires DelegationPool, GovernanceRecords {
        assert!(arg0 != @0x0, 0x1::error::invalid_argument(27));
        let (v0, _, _, _) = 0x1::stake::get_stake(arg1);
        assert!(arg2 <= v0, 0x1::error::invalid_argument(6));
        let v4 = borrow_global_mut<DelegationPool>(arg1);
        let v5 = &v4.active_shares;
        let v6 = coins_to_transfer_to_ensure_min_stake(v5, pending_inactive_shares_pool(v4), arg0, arg2);
        let v7 = redeem_active_shares(v4, arg0, v6);
        let v8 = retrieve_stake_pool_owner(v4);
        0x1::stake::unlock(&v8, v7);
        buy_in_pending_inactive_shares(v4, arg0, v7);
        assert_min_pending_inactive_balance(v4, arg0);
        if (0x1::features::module_event_migration_enabled()) {
            let v9 = UnlockStake{
                pool_address      : arg1, 
                delegator_address : arg0, 
                amount_unlocked   : v7,
            };
            0x1::event::emit<UnlockStake>(v9);
        };
        let v10 = UnlockStakeEvent{
            pool_address      : arg1, 
            delegator_address : arg0, 
            amount_unlocked   : v7,
        };
        0x1::event::emit_event<UnlockStakeEvent>(&mut v4.unlock_stake_events, v10);
    }
    
    fun update_and_borrow_mut_delegated_votes(arg0: &DelegationPool, arg1: &mut GovernanceRecords, arg2: address) : &mut DelegatedVotes {
        let v0 = 0x1::stake::get_lockup_secs(get_pool_address(arg0));
        let v1 = &mut arg1.delegated_votes;
        if (!0x1::smart_table::contains<address, DelegatedVotes>(v1, arg2)) {
            let v2 = get_delegator_active_shares(arg0, arg2);
            let v3 = get_delegator_pending_inactive_shares(arg0, arg2);
            let v4 = DelegatedVotes{
                active_shares             : v2, 
                pending_inactive_shares   : v3, 
                active_shares_next_lockup : v2, 
                last_locked_until_secs    : v0,
            };
            return 0x1::smart_table::borrow_mut_with_default<address, DelegatedVotes>(v1, arg2, v4)
        };
        let v5 = 0x1::smart_table::borrow_mut<address, DelegatedVotes>(v1, arg2);
        if (v5.last_locked_until_secs < v0) {
            v5.active_shares = v5.active_shares_next_lockup;
            v5.pending_inactive_shares = 0;
            v5.last_locked_until_secs = v0;
        };
        v5
    }
    
    fun update_and_borrow_mut_delegator_vote_delegation(arg0: &DelegationPool, arg1: &mut GovernanceRecords, arg2: address) : &mut VoteDelegation {
        let v0 = 0x1::stake::get_lockup_secs(get_pool_address(arg0));
        let v1 = &mut arg1.vote_delegation;
        if (!0x1::smart_table::contains<address, VoteDelegation>(v1, arg2)) {
            let v2 = VoteDelegation{
                voter                  : arg2, 
                pending_voter          : arg2, 
                last_locked_until_secs : v0,
            };
            return 0x1::smart_table::borrow_mut_with_default<address, VoteDelegation>(v1, arg2, v2)
        };
        let v3 = 0x1::smart_table::borrow_mut<address, VoteDelegation>(v1, arg2);
        if (v3.last_locked_until_secs < v0) {
            v3.voter = v3.pending_voter;
            v3.last_locked_until_secs = v0;
        };
        v3
    }
    
    public entry fun update_commission_percentage(arg0: &signer, arg1: u64) acquires BeneficiaryForOperator, DelegationPool, DelegationPoolOwnership, GovernanceRecords, NextCommissionPercentage {
        assert!(0x1::features::commission_change_delegation_pool_enabled(), 0x1::error::invalid_state(22));
        assert!(arg1 <= 10000, 0x1::error::invalid_argument(5));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = get_owned_pool_address(v0);
        let v2 = operator_commission_percentage(v1);
        assert!(v2 + 1000 >= arg1, 0x1::error::invalid_argument(20));
        let v3 = 0x1::stake::get_remaining_lockup_secs(v1) >= min_remaining_secs_for_commission_change();
        assert!(v3, 0x1::error::invalid_state(21));
        synchronize_delegation_pool(v1);
        if (exists<NextCommissionPercentage>(v1)) {
            let v4 = borrow_global_mut<NextCommissionPercentage>(v1);
            v4.commission_percentage_next_lockup_cycle = arg1;
            v4.effective_after_secs = 0x1::stake::get_lockup_secs(v1);
        } else {
            let v5 = 0x1::account::create_signer_with_capability(&borrow_global<DelegationPool>(v1).stake_pool_signer_cap);
            let v6 = 0x1::stake::get_lockup_secs(v1);
            let v7 = NextCommissionPercentage{
                commission_percentage_next_lockup_cycle : arg1, 
                effective_after_secs                    : v6,
            };
            move_to<NextCommissionPercentage>(&v5, v7);
        };
        let v8 = CommissionPercentageChange{
            pool_address                            : v1, 
            owner                                   : v0, 
            commission_percentage_next_lockup_cycle : arg1,
        };
        0x1::event::emit<CommissionPercentageChange>(v8);
    }
    
    fun update_governanace_records_for_redeem_active_shares(arg0: &DelegationPool, arg1: address, arg2: u128, arg3: address) acquires GovernanceRecords {
        let v0 = borrow_global_mut<GovernanceRecords>(arg1);
        let v1 = update_and_borrow_mut_delegator_vote_delegation(arg0, v0, arg3);
        let v2 = v1.voter;
        let v3 = v1.pending_voter;
        let v4 = update_and_borrow_mut_delegated_votes(arg0, v0, v2);
        v4.active_shares = v4.active_shares - arg2;
        if (v2 == v3) {
            v4.active_shares_next_lockup = v4.active_shares_next_lockup - arg2;
        } else {
            let v5 = update_and_borrow_mut_delegated_votes(arg0, v0, v3);
            v5.active_shares_next_lockup = v5.active_shares_next_lockup - arg2;
        };
    }
    
    fun update_governanace_records_for_redeem_pending_inactive_shares(arg0: &DelegationPool, arg1: address, arg2: u128, arg3: address) acquires GovernanceRecords {
        let v0 = borrow_global_mut<GovernanceRecords>(arg1);
        let v1 = calculate_and_update_delegator_voter_internal(arg0, v0, arg3);
        let v2 = update_and_borrow_mut_delegated_votes(arg0, v0, v1);
        v2.pending_inactive_shares = v2.pending_inactive_shares - arg2;
    }
    
    fun update_governance_records_for_buy_in_active_shares(arg0: &DelegationPool, arg1: address, arg2: u128, arg3: address) acquires GovernanceRecords {
        let v0 = borrow_global_mut<GovernanceRecords>(arg1);
        let v1 = update_and_borrow_mut_delegator_vote_delegation(arg0, v0, arg3);
        let v2 = v1.voter;
        let v3 = v1.pending_voter;
        let v4 = update_and_borrow_mut_delegated_votes(arg0, v0, v2);
        v4.active_shares = v4.active_shares + arg2;
        if (v3 == v2) {
            v4.active_shares_next_lockup = v4.active_shares_next_lockup + arg2;
        } else {
            let v5 = update_and_borrow_mut_delegated_votes(arg0, v0, v3);
            v5.active_shares_next_lockup = v5.active_shares_next_lockup + arg2;
        };
    }
    
    fun update_governance_records_for_buy_in_pending_inactive_shares(arg0: &DelegationPool, arg1: address, arg2: u128, arg3: address) acquires GovernanceRecords {
        let v0 = borrow_global_mut<GovernanceRecords>(arg1);
        let v1 = calculate_and_update_delegator_voter_internal(arg0, v0, arg3);
        let v2 = update_and_borrow_mut_delegated_votes(arg0, v0, v1);
        v2.pending_inactive_shares = v2.pending_inactive_shares + arg2;
    }
    
    public entry fun vote(arg0: &signer, arg1: address, arg2: u64, arg3: u64, arg4: bool) acquires BeneficiaryForOperator, DelegationPool, GovernanceRecords, NextCommissionPercentage {
        assert_partial_governance_voting_enabled(arg1);
        synchronize_delegation_pool(arg1);
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = calculate_and_update_remaining_voting_power(arg1, v0, arg2);
        if (arg3 > v1) {
            arg3 = v1;
        };
        assert!(arg3 > 0, 0x1::error::invalid_argument(16));
        let v2 = borrow_global_mut<GovernanceRecords>(arg1);
        let v3 = 0x1::aptos_governance::get_remaining_voting_power(arg1, arg2);
        let v4 = 0x1::smart_table::borrow_mut_with_default<u64, u64>(&mut v2.votes_per_proposal, arg2, 0);
        let v5 = 0x1::aptos_governance::get_voting_power(arg1) - v3 == *v4;
        assert!(v5, 0x1::error::invalid_argument(17));
        *v4 = *v4 + arg3;
        let v6 = VotingRecordKey{
            voter       : v0, 
            proposal_id : arg2,
        };
        let v7 = 0x1::smart_table::borrow_mut_with_default<VotingRecordKey, u64>(&mut v2.votes, v6, 0);
        *v7 = *v7 + arg3;
        let v8 = retrieve_stake_pool_owner(borrow_global<DelegationPool>(arg1));
        0x1::aptos_governance::partial_vote(&v8, arg1, arg2, arg3, arg4);
        if (0x1::features::module_event_migration_enabled()) {
            let v9 = Vote{
                voter           : v0, 
                proposal_id     : arg2, 
                delegation_pool : arg1, 
                num_votes       : arg3, 
                should_pass     : arg4,
            };
            0x1::event::emit<Vote>(v9);
        };
        let v10 = VoteEvent{
            voter           : v0, 
            proposal_id     : arg2, 
            delegation_pool : arg1, 
            num_votes       : arg3, 
            should_pass     : arg4,
        };
        0x1::event::emit_event<VoteEvent>(&mut v2.vote_events, v10);
    }
    
    fun withdraw_internal(arg0: &mut DelegationPool, arg1: address, arg2: u64) acquires GovernanceRecords {
        if (arg2 == 0) {
            return
        };
        let v0 = get_pool_address(arg0);
        let (v1, v2) = pending_withdrawal_exists(arg0, arg1);
        let v3 = v2;
        let v4 = if (v1) {
            let v5 = v3.index < arg0.observed_lockup_cycle.index || can_withdraw_pending_inactive(v0);
            v5
        } else {
            false
        };
        if (!v4) {
            return
        };
        if (v3.index == arg0.observed_lockup_cycle.index) {
            arg2 = coins_to_redeem_to_ensure_min_stake(pending_inactive_shares_pool(arg0), arg1, arg2);
        };
        let v6 = redeem_inactive_shares(arg0, arg1, arg2, v3);
        let v7 = retrieve_stake_pool_owner(arg0);
        let v8 = &v7;
        if (can_withdraw_pending_inactive(v0)) {
            let (_, _, _, v12) = 0x1::stake::get_stake(v0);
            let v13 = v12;
            if (v3.index == arg0.observed_lockup_cycle.index) {
                v13 = v12 - v6;
            };
            0x1::stake::reactivate_stake(v8, v13);
            0x1::stake::withdraw(v8, v6);
            0x1::stake::unlock(v8, v13);
        } else {
            0x1::stake::withdraw(v8, v6);
        };
        0x1::aptos_account::transfer(v8, arg1, v6);
        let (_, v15, _, _) = 0x1::stake::get_stake(v0);
        arg0.total_coins_inactive = v15;
        if (0x1::features::module_event_migration_enabled()) {
            let v18 = WithdrawStake{
                pool_address      : v0, 
                delegator_address : arg1, 
                amount_withdrawn  : v6,
            };
            0x1::event::emit<WithdrawStake>(v18);
        };
        let v19 = WithdrawStakeEvent{
            pool_address      : v0, 
            delegator_address : arg1, 
            amount_withdrawn  : v6,
        };
        0x1::event::emit_event<WithdrawStakeEvent>(&mut arg0.withdraw_stake_events, v19);
    }
    
    // decompiled from Move bytecode v6
}
