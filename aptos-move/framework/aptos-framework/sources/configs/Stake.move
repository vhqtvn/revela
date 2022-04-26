module AptosFramework::Stake {
    use Std::Vector;
    use Std::Signer;
    use AptosFramework::SystemAddresses;
    use AptosFramework::Timestamp;
    use AptosFramework::TestCoin::{Self, Coin};

    friend AptosFramework::Reconfiguration;
    friend AptosFramework::Genesis;

    const MINIMUM_LOCK_PERIOD: u64 = 86400;

    /// Basic unit of stake delegation, it's stored in StakePool.
    struct Delegation has store {
        coins: Coin,
        from: address,
        locked_until_secs: u64,
    }

    /// Aggregation of delegation and represent a validator's voting power, stored in ValidatorInfo.
    /// Invariants:
    /// 1. current_stake = sum(active + pending_inactive)
    /// 2. user interact with pending_active and inactive if it's in the ValidatorSet.
    /// 3. user interact with active, inactive if it's not in the ValidatorSet.
    /// 4. pending_active and pending_inactive are empty if it's not in the ValidatorSet.
    struct StakePool has key, store {
        // sum of active and pending_inactive stakes, updated on epoch boundary.
        current_stake: u64,
        // active stake
        active: vector<Delegation>,
        // inactive stake, can be withdrawn
        inactive: vector<Delegation>,
        // pending activation for next epoch
        pending_active: vector<Delegation>,
        // pending deactivation for next epoch
        pending_inactive: vector<Delegation>,
    }

    /// Validator info stored in validator address.
    struct ValidatorConfig has key, copy, store, drop {
        consensus_pubkey: vector<u8>,
        network_address: vector<u8>,
        // to make it compatible with previous definition, remove later
        fullnode_address: vector<u8>,
    }

    /// Consensus information per validator, stored in ValidatorSet.
    struct ValidatorInfo has copy, store, drop {
        addr: address,
        voting_power: u64,
        config: ValidatorConfig,
    }

    /// Full ValidatorSet, stored in @CoreResource.
    struct ValidatorSet has key {
        consensus_scheme: u8,
        // minimum stakes required to join validator set
        minimum_stake: u64,
        // maximum stakes allowed to join validator set
        maximum_stake: u64,
        // active validators for the current epoch
        active_validators: vector<ValidatorInfo>,
        // pending validators to leave in next epoch (still active)
        pending_inactive: vector<ValidatorInfo>,
        // pending validators to join in next epoch
        pending_active: vector<ValidatorInfo>,
    }

    /// Any user can delegate a stake.
    public fun delegate_stake(account: &signer, to: address, amount: u64, locked_until_secs: u64) acquires StakePool, ValidatorSet {
        let coins = TestCoin::withdraw(account, amount);
        let current_time = Timestamp::now_seconds();
        assert!(current_time + MINIMUM_LOCK_PERIOD < locked_until_secs, 0);
        let stake_pool = borrow_global_mut<StakePool>(to);
        let delegation = Delegation {
            coins,
            locked_until_secs,
            from: Signer::address_of(account),
        };
        // add to pending_active if it's a current validator otherwise add to active directly
        if (is_current_validator(to)) {
            Vector::push_back(&mut stake_pool.pending_active, delegation);
        } else {
            stake_pool.current_stake = stake_pool.current_stake + TestCoin::value(&delegation.coins);
            Vector::push_back(&mut stake_pool.active, delegation);
        }
    }

    /// Withdraw from active delegation, it's moved to pending_inactive if locked_until_secs < current_time or
    /// directly deposit if it's not from an active validator.
    public fun withdraw_active(account: &signer, from: address) acquires StakePool, ValidatorSet {
        let addr = Signer::address_of(account);
        let current_time = Timestamp::now_seconds();
        let stake_pool = borrow_global_mut<StakePool>(from);
        let d = withdraw_internal(&mut stake_pool.active, addr);
        let is_current_validator = is_current_validator(from);
        if (!is_current_validator) {
            // directly deposit if it's not active validator
            let Delegation {coins, from: _, locked_until_secs: _} = d;
            TestCoin::deposit(addr, coins);
        } else if (d.locked_until_secs < current_time) {
            // move to pending_inactive if it can be unlocked
            Vector::push_back(&mut stake_pool.pending_inactive, d);
        } else {
            // not allowed to withdraw
            abort 0
        };
    }

    /// Withdraw from inactive delegation, directly deposited to the account's balance.
    public fun withdraw_inactive(account: &signer, from: address) acquires StakePool {
        let addr = Signer::address_of(account);
        let stake_pool = borrow_global_mut<StakePool>(from);
        let d = withdraw_internal(&mut stake_pool.inactive, addr);
        let Delegation {coins, from: _, locked_until_secs: _} = d;
        TestCoin::deposit(addr, coins);
    }

    /// Initialize the ValidatorInfo for account.
    public fun register_validator_candidate(
        account: &signer,
        consensus_pubkey: vector<u8>,
        network_address: vector<u8>,
        fullnode_address: vector<u8>
    ) {
        move_to(account, StakePool {
            current_stake: 0,
            active: Vector::empty(),
            pending_active: Vector::empty(),
            pending_inactive: Vector::empty(),
            inactive: Vector::empty(),
        });
        move_to(account, ValidatorConfig {
            consensus_pubkey,
            network_address,
            fullnode_address,
        });
    }

    /// Rotate the consensus key of the validator, it'll take effect in next epoch.
    public fun rotate_consensus_key(account: &signer, consensus_pubkey: vector<u8>) acquires ValidatorConfig {
        let addr = Signer::address_of(account);
        let validator_info = borrow_global_mut<ValidatorConfig>(addr);
        validator_info.consensus_pubkey = consensus_pubkey;
    }

    /// Initialize validator set to the core resource account.
    public fun initialize_validator_set(account: &signer, minimum_stake: u64, maximum_stake: u64) {
        SystemAddresses::assert_core_resource(account);
        move_to(account, ValidatorSet {
            consensus_scheme: 0,
            minimum_stake,
            maximum_stake,
            active_validators: Vector::empty(),
            pending_active: Vector::empty(),
            pending_inactive: Vector::empty(),
        });
    }

    /// Initiate by the validator info owner
    public fun join_validator_set(account: &signer) acquires StakePool, ValidatorConfig, ValidatorSet {
        let addr = Signer::address_of(account);
        let stake_pool = borrow_global<StakePool>(addr);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        assert!(stake_pool.current_stake >= validator_set.minimum_stake, 0);
        assert!(stake_pool.current_stake <= validator_set.maximum_stake, 0);
        let (exist, _) = find_validator(&validator_set.active_validators, addr);
        assert!(!exist, 0);
        let (exist, _) = find_validator(&validator_set.pending_inactive, addr);
        assert!(!exist, 0);
        let (exist, _) = find_validator(&validator_set.pending_active, addr);
        assert!(!exist, 0);

        Vector::push_back(&mut validator_set.pending_active, generate_validator_info(addr));
    }

    /// Initiate by the validator info owner.
    public fun leave_validator_set(account: &signer) acquires ValidatorSet {
        let addr = Signer::address_of(account);
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);

        let (exist, index) = find_validator(&validator_set.active_validators, addr);
        assert!(exist, 0);

        let validator_info = Vector::swap_remove(&mut validator_set.active_validators, index);
        assert!(Vector::length(&validator_set.active_validators) > 0, 0);
        Vector::push_back(&mut validator_set.pending_inactive, validator_info);
    }

    /// Triggers at epoch boundary.
    /// 1. distribute rewards to stake pool of active and pending inactive validators
    /// 2. purge pending queues
    /// 3. update the validator info from owners' address
    public(friend) fun on_new_epoch() acquires StakePool, ValidatorConfig, ValidatorSet {
        let validator_set = borrow_global_mut<ValidatorSet>(@CoreResources);
        // distribute reward
        let i = 0;
        let len = Vector::length(&validator_set.active_validators);
        while (i < len) {
            let addr = Vector::borrow(&validator_set.active_validators, i).addr;
            update_stake_pool(addr);
            i = i + 1;
        };
        let i = 0;
        let len = Vector::length(&validator_set.pending_inactive);
        while (i < len) {
            let addr = Vector::borrow(&validator_set.pending_inactive, i).addr;
            update_stake_pool(addr);
            i = i + 1;
        };
        // purge pending queue
        append(&mut validator_set.active_validators, &mut validator_set.pending_active);
        validator_set.pending_inactive = Vector::empty();
        // update validator info (so network address/public key change takes effect)
        let i = 0;
        let len = Vector::length(&validator_set.active_validators);
        while (i < len) {
            let old_validator_info = Vector::borrow_mut(&mut validator_set.active_validators, i);
            *old_validator_info = generate_validator_info(old_validator_info.addr);
            i = i + 1;
        }
    }

    /// Update individual validator's stake pool
    /// 1. distribute rewards to active/pending_inactive delegations
    /// 2. process pending_active, pending_inactive correspondingly
    /// 3. update the current stake
    fun update_stake_pool(addr: address) acquires StakePool {
        let stake_pool = borrow_global_mut<StakePool>(addr);
        distribute_reward( &mut stake_pool.active);
        distribute_reward( &mut stake_pool.pending_inactive);
        // move pending_active to active
        append(&mut stake_pool.active, &mut stake_pool.pending_active);
        // move pending_inactive to inactive
        append(&mut stake_pool.inactive, &mut stake_pool.pending_inactive);
        let current_stake = 0;
        let i = 0;
        let len = Vector::length(&stake_pool.active);
        while (i < len) {
            current_stake = current_stake + TestCoin::value(&Vector::borrow(&stake_pool.active, i).coins);
            i = i + 1;
        };
        stake_pool.current_stake = current_stake;
    }

    /// Mint the reward and add to the delegation based on some formula
    fun distribute_reward(v: &mut vector<Delegation>) {
        let i = 0;
        let len = Vector::length(v);
        while (i < len) {
            let d = Vector::borrow_mut(v, i);
            let reward = TestCoin::zero(); // mint some coins based on delegation, timestamp, maybe also total stakes
            TestCoin::merge(&mut d.coins, reward);
            i = i + 1;
        };
    }

    fun append<T>(v1: &mut vector<T>, v2: &mut vector<T>) {
        while (!Vector::is_empty(v2)) {
            Vector::push_back(v1, Vector::pop_back(v2));
        }
    }

    fun find_delegation(v: &vector<Delegation>, addr: address): u64 {
        let i = 0;
        let len =  Vector::length(v);
        while (i < len) {
            let d = Vector::borrow(v, i);
            if (d.from == addr) {
                return i
            };
            i = i + 1;
        };
        abort 0
    }

    fun find_validator(v: &vector<ValidatorInfo>, addr: address): (bool, u64) {
        let i = 0;
        let len = Vector::length(v);
        while (i < len) {
            if (Vector::borrow(v, i).addr == addr) {
                return (true, i)
            };
            i = i + 1;
        };
        (false, 0)
    }

    public fun is_current_validator(addr: address): bool acquires ValidatorSet{
        let validator_set = borrow_global<ValidatorSet>(@CoreResources);
        let (exist_1, _) = find_validator(&validator_set.active_validators, addr);
        if (exist_1) {
            return true
        };
        let (exist_2, _) = find_validator(&validator_set.pending_inactive, addr);
        exist_2
    }

    fun withdraw_internal(v: &mut vector<Delegation>, addr: address): Delegation {
        let index = find_delegation(v, addr);
        Vector::swap_remove(v, index)
    }

    fun generate_validator_info(addr: address): ValidatorInfo acquires StakePool, ValidatorConfig {
        let config = *borrow_global<ValidatorConfig>(addr);
        let voting_power = borrow_global<StakePool>(addr).current_stake;
        ValidatorInfo {
            addr,
            voting_power,
            config,
        }
    }

    #[test(core_resources = @CoreResources, account_1 = @0x123, account_2 = @0x234)]
    fun test_basic_delegation(
        core_resources: signer,
        account_1: signer,
        account_2: signer
    ) acquires StakePool, ValidatorConfig, ValidatorSet {
        initialize_validator_set(&core_resources, 100, 10000);
        Timestamp::set_time_has_started_for_testing(&core_resources);
        TestCoin::mint_for_test(&account_1, 10000);
        TestCoin::mint_for_test(&account_2, 10000);
        register_validator_candidate(&account_1, Vector::empty(), Vector::empty(), Vector::empty());
        let addr1 = Signer::address_of(&account_1);
        let addr2 = Signer::address_of(&account_2);
        delegate_stake(&account_1, addr1, 100, 100000);
        // delegation when the address is not a validator
        assert!(TestCoin::value(&Vector::borrow(&borrow_global<StakePool>(addr1).active, 0).coins) == 100, 0);
        assert!(Vector::borrow(&borrow_global<StakePool>(addr1).active, 0).from == addr1, 0);
        delegate_stake(&account_2, addr1, 101, 100000);
        assert!(TestCoin::value(&Vector::borrow(&borrow_global<StakePool>(addr1).active, 1).coins) == 101, 0);
        assert!(Vector::borrow(&borrow_global<StakePool>(addr1).active, 1).from == addr2, 0);
        // join the validator set with enough stake
        join_validator_set(&account_1);
        on_new_epoch();
        // delegation when the address is active valdiator
        assert!(is_current_validator(addr1), 0);
        delegate_stake(&account_2, addr1, 102, 100000);
        assert!(borrow_global<StakePool>(addr1).current_stake == 201, 0);
        assert!(TestCoin::value(&Vector::borrow(&borrow_global<StakePool>(addr1).pending_active, 0).coins) == 102, 0);
        // withdraw active stakes
        Timestamp::update_global_time_for_test(100001000000);
        withdraw_active(&account_1, addr1);
        assert!(TestCoin::value(&Vector::borrow(&borrow_global<StakePool>(addr1).pending_inactive, 0).coins) == 100, 0);
        // total stake doesn't change until next epoch
        assert!(borrow_global<StakePool>(addr1).current_stake == 201, 0);
        // pending delegations are processed on new epoch
        on_new_epoch();
        assert!(Vector::length(&borrow_global<StakePool>(addr1).pending_active) == 0, 0);
        assert!(Vector::length(&borrow_global<StakePool>(addr1).pending_inactive) == 0, 0);
        assert!(TestCoin::value(&Vector::borrow(&borrow_global<StakePool>(addr1).inactive, 0).coins) == 100, 0);
        assert!(TestCoin::value(&Vector::borrow(&borrow_global<StakePool>(addr1).active, 1).coins) == 102, 0);
        assert!(borrow_global<StakePool>(addr1).current_stake == 203, 0);
        // withdraw inactive
        withdraw_inactive(&account_1, addr1);
        assert!(TestCoin::balance_of(addr1) == 10000, 0);
    }

    #[test(core_resources = @CoreResources, account_1 = @0x123, account_2 = @0x234, account_3 = @0x345)]
    fun test_validator_join_leave(
        core_resources: signer,
        account_1: signer,
        account_2: signer,
        account_3: signer
    ) acquires StakePool, ValidatorConfig, ValidatorSet {
        initialize_validator_set(&core_resources, 100, 10000);
        Timestamp::set_time_has_started_for_testing(&core_resources);
        TestCoin::mint_for_test(&account_1, 10000);
        let addr1 = Signer::address_of(&account_1);
        let addr2 = Signer::address_of(&account_2);
        let addr3 = Signer::address_of(&account_3);
        register_validator_candidate(&account_1, Vector::empty(), Vector::empty(), Vector::empty());
        register_validator_candidate(&account_2, Vector::empty(), Vector::empty(), Vector::empty());
        register_validator_candidate(&account_3, Vector::empty(), Vector::empty(), Vector::empty());
        delegate_stake(&account_1, addr1, 100, 100000);
        delegate_stake(&account_1, addr2, 100, 100000);
        delegate_stake(&account_1, addr3, 100, 100000);
        join_validator_set(&account_1);
        join_validator_set(&account_2);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 0).addr == addr1, 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 1).addr == addr2, 0);
        on_new_epoch();
        assert!(is_current_validator(addr1), 0);
        assert!(is_current_validator(addr2), 0);
        // changes don't take effect until next epoch
        leave_validator_set(&account_2);
        join_validator_set(&account_3);
        rotate_consensus_key(&account_1, x"1234");
        assert!(is_current_validator(addr2), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_inactive, 0).addr == addr2, 0);
        assert!(!is_current_validator(addr3), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).pending_active, 0).addr == addr3, 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).active_validators, 0).config.consensus_pubkey == Vector::empty(), 0);
        // changes applied after new epoch
        on_new_epoch();
        assert!(!is_current_validator(addr2), 0);
        assert!(is_current_validator(addr3), 0);
        assert!(Vector::borrow(&borrow_global<ValidatorSet>(@CoreResources).active_validators, 0).config.consensus_pubkey == x"1234", 0);
    }
}
