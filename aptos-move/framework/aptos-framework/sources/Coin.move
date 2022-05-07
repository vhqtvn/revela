/// This module provides the foundation for typesafe Coins.
module AptosFramework::Coin {
    use Std::ASCII;
    use Std::Errors;
    use Std::Event::{Self, EventHandle};
    use Std::Option::{Self, Option};
    use Std::Signer;

    use AptosFramework::TypeInfo;

    const ECOIN_INFO_ADDRESS_MISMATCH: u64 = 0;
    const ECOIN_INFO_ALREADY_PUBLISHED: u64 = 1;
    const ECOIN_INFO_NOT_PUBLISHED: u64 = 2;
    const ECOIN_STORE_ALREADY_PUBLISHED: u64 = 3;
    const ECOIN_STORE_NOT_PUBLISHED: u64 = 4;
    const EINSUFFICIENT_BALANCE: u64 = 5;
    const ENO_BURN_CAPABILITY: u64 = 6;
    const ENO_MINT_CAPABILITY: u64 = 7;
    const EDESTRUCTION_OF_NONZERO_TOKEN: u64 = 8;

    // Core data structures

    // Represents a set amount of coin
    struct Coin<phantom CoinType> has store {
        value: u64,
    }

    // Represents ownership of coin
    struct CoinStore<phantom CoinType> has key {
        coin: Coin<CoinType>,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
    }

    // Represents resources associated with a coin
    struct CoinInfo<phantom CoinType> has key {
        name: ASCII::String,
        scaling_factor: u64,
        supply: Option<u64>,
    }

    // Set of data sent to the event stream during a receive
    struct DepositEvent has drop, store {
        amount: u64,
    }

    // Set of data sent to the event stream during a withdrawal
    struct WithdrawEvent has drop, store {
        amount: u64,
    }

    /// Capability required to mint coins.
    struct MintCapability<phantom CoinType> has copy, key, store { }

    /// Capability required to burn coins.
    struct BurnCapability<phantom CoinType> has copy, key, store { }

    //
    // Getter functions
    //

    /// Returns the balance of `owner`.
    public fun balance<CoinType>(owner: address): u64 acquires CoinStore {
        assert!(
            exists<CoinStore<CoinType>>(owner),
            Errors::not_published(ECOIN_STORE_NOT_PUBLISHED),
        );
        borrow_global<CoinStore<CoinType>>(owner).coin.value
    }

    /// Returns `true` if the type `CoinType` is a registered coin.
    /// Returns `false` otherwise.
    public fun is_registered<CoinType>(): bool {
        let type_info = TypeInfo::type_of<CoinType>();
        let coin_address = TypeInfo::account_address(&type_info);
        exists<CoinInfo<CoinType>>(coin_address)
    }

    public fun name<CoinType>(): ASCII::String acquires CoinInfo {
        let type_info = TypeInfo::type_of<CoinType>();
        let coin_address = TypeInfo::account_address(&type_info);
        borrow_global<CoinInfo<CoinType>>(coin_address).name
    }

    public fun scaling_factor<CoinType>(): u64 acquires CoinInfo {
        let type_info = TypeInfo::type_of<CoinType>();
        let coin_address = TypeInfo::account_address(&type_info);
        borrow_global<CoinInfo<CoinType>>(coin_address).scaling_factor
    }

    public fun supply<CoinType>(): Option<u64> acquires CoinInfo {
        let type_info = TypeInfo::type_of<CoinType>();
        let coin_address = TypeInfo::account_address(&type_info);
        borrow_global<CoinInfo<CoinType>>(coin_address).supply
    }

    // Public functions

    /// Burn coin with capability.
    public fun burn<CoinType>(
        account: &signer,
        coin: Coin<CoinType>,
    ) acquires BurnCapability, CoinInfo {
        let account_addr = Signer::address_of(account);
        assert!(
            exists<BurnCapability<CoinType>>(account_addr),
            Errors::not_published(ENO_BURN_CAPABILITY),
        );
        let _cap = borrow_global<BurnCapability<CoinType>>(account_addr);

        let Coin { value: amount } = coin;

        let coin_addr = TypeInfo::account_address(&TypeInfo::type_of<CoinType>());
        let supply = &mut borrow_global_mut<CoinInfo<CoinType>>(coin_addr).supply;
        if (Option::is_some(supply)) {
            let supply = Option::borrow_mut(supply);
            *supply = *supply - amount;
        }
    }

    /// Deposit the coin balance into the recipients account and emit an event.
    public fun deposit<CoinType>(account_addr: address, coin: Coin<CoinType>) acquires CoinStore {
        assert!(
            exists<CoinStore<CoinType>>(account_addr),
            Errors::not_published(ECOIN_STORE_NOT_PUBLISHED),
        );

        let coin_store = borrow_global_mut<CoinStore<CoinType>>(account_addr);
        Event::emit_event<DepositEvent>(
            &mut coin_store.deposit_events,
            DepositEvent { amount: coin.value },
        );

        merge(&mut coin_store.coin, coin);
    }

    /// Destroy a zero-value coin. Calls will fail if the `value` in the passed-in `token` is non-zero
    /// so it is impossible to "burn" any non-zero amount of `Coin` without having
    /// a `BurnCapability` for the specific `CoinType`.
    public fun destroy_zero<CoinType>(zero_coin: Coin<CoinType>) {
        let Coin { value } = zero_coin;
        assert!(value == 0, Errors::invalid_argument(EDESTRUCTION_OF_NONZERO_TOKEN))
    }

    /// Extract `amount` from the passed-in `coin`, where the original token is modified in place.
    public fun extract<CoinType>(coin: &mut Coin<CoinType>, amount: u64): Coin<CoinType> {
        assert!(coin.value >= amount, Errors::invalid_argument(EINSUFFICIENT_BALANCE));
        coin.value = coin.value - amount;
        Coin { value: amount }
    }

    public(script) fun initialize<CoinType>(
        account: &signer,
        name: vector<u8>,
        scaling_factor: u64,
        monitor_supply: bool,
    ) {
        let account_addr = Signer::address_of(account);

        let type_info = TypeInfo::type_of<CoinType>();
        assert!(
            TypeInfo::account_address(&type_info) == account_addr,
            Errors::invalid_argument(ECOIN_INFO_ADDRESS_MISMATCH),
        );

        assert!(
            !exists<CoinInfo<CoinType>>(account_addr),
            Errors::already_published(ECOIN_INFO_ALREADY_PUBLISHED),
        );

        let coin_info = CoinInfo<CoinType> {
            name: ASCII::string(name),
            scaling_factor,
            supply: if (monitor_supply) { Option::some(0) } else { Option::none() },
        };
        move_to(account, coin_info);

        move_to(account, BurnCapability<CoinType> { });
        move_to(account, MintCapability<CoinType> { });
    }

    /// "Merges" the two coins.
    /// The coin passed in as `dst_coin` will have a value equal to the sum of the two tokens (`dst_coin` and `source_coin`).
    public fun merge<CoinType>(dst_coin: &mut Coin<CoinType>, source_coin: Coin<CoinType>) {
        dst_coin.value = dst_coin.value + source_coin.value;
        let Coin { value: _ } = source_coin;
    }

    /// Create new coins and deposit them into dst_addr's account.
    public(script) fun mint<CoinType>(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) acquires CoinInfo, CoinStore, MintCapability {
        let account_addr = Signer::address_of(account);
        assert!(
            exists<MintCapability<CoinType>>(account_addr),
            Errors::not_published(ENO_MINT_CAPABILITY),
        );
        let _cap = borrow_global<MintCapability<CoinType>>(account_addr);
        deposit(dst_addr, Coin<CoinType> { value: amount });

        let coin_addr = TypeInfo::account_address(&TypeInfo::type_of<CoinType>());
        let supply = &mut borrow_global_mut<CoinInfo<CoinType>>(coin_addr).supply;
        if (Option::is_some(supply)) {
            let supply = Option::borrow_mut(supply);
            *supply = *supply + amount;
        }
    }

    public(script) fun register<CoinType>(account: &signer) {
        assert!(
            !exists<CoinStore<CoinType>>(Signer::address_of(account)),
            Errors::already_published(ECOIN_STORE_ALREADY_PUBLISHED),
        );

        let coin_store = CoinStore<CoinType> {
            coin: Coin { value: 0 },
            deposit_events: Event::new_event_handle<DepositEvent>(account),
            withdraw_events: Event::new_event_handle<WithdrawEvent>(account),
        };
        move_to(account, coin_store);
    }

    /// Transfers `amount` of coins from `from` to `to`.
    public(script) fun transfer<CoinType>(
        from: &signer,
        to: address,
        amount: u64,
    ) acquires CoinStore {
        let coin = withdraw<CoinType>(from, amount);
        deposit(to, coin);
    }

    /// Returns the `value` of the passed in `coin`.
    public fun value<CoinType>(coin: &Coin<CoinType>): u64 {
        coin.value
    }

    public fun withdraw<CoinType>(
        account: &signer,
        amount: u64,
    ): Coin<CoinType> acquires CoinStore {
        let account_addr = Signer::address_of(account);
        assert!(
            exists<CoinStore<CoinType>>(account_addr),
            Errors::not_published(ECOIN_STORE_NOT_PUBLISHED),
        );
        let coin_store = borrow_global_mut<CoinStore<CoinType>>(account_addr);

        Event::emit_event<WithdrawEvent>(
            &mut coin_store.withdraw_events,
            WithdrawEvent { amount },
        );

        extract(&mut coin_store.coin, amount)
    }

    /// Create a new `Coin<CoinType>` with a value of `0`.
    public fun zero<CoinType>(): Coin<CoinType> {
        Coin<CoinType> {
            value: 0
        }
    }

    //
    // Tests
    //
    #[test_only]
    struct FakeMoney { }

    #[test(source = @0x1, destination = @0x2)]
    public(script) fun end_to_end(
        source: signer,
        destination: signer,
    ) acquires BurnCapability, CoinStore, CoinInfo, MintCapability {
        let source_addr = Signer::address_of(&source);
        let destination_addr = Signer::address_of(&destination);

        initialize<FakeMoney>(&source, b"Fake money", 1, true);
        register<FakeMoney>(&source);
        register<FakeMoney>(&destination);
        assert!(*Option::borrow(&supply<FakeMoney>()) == 0, 0);

        mint<FakeMoney>(&source, source_addr, 100);
        transfer<FakeMoney>(&source, destination_addr, 50);

        assert!(balance<FakeMoney>(source_addr) == 50, 1);
        assert!(balance<FakeMoney>(destination_addr) == 50, 2);
        assert!(*Option::borrow(&supply<FakeMoney>()) == 100, 3);

        let coin = withdraw<FakeMoney>(&source, 10);
        assert!(value(&coin) == 10, 4);
        burn(&source, coin);
        assert!(*Option::borrow(&supply<FakeMoney>()) == 90, 5);
    }

    #[test(source = @0x1, destination = @0x2)]
    public(script) fun end_to_end_no_supply(
        source: signer,
        destination: signer,
    ) acquires BurnCapability, CoinStore, CoinInfo, MintCapability {
        let source_addr = Signer::address_of(&source);
        let destination_addr = Signer::address_of(&destination);

        initialize<FakeMoney>(&source, b"Fake money", 1, false);
        register<FakeMoney>(&source);
        register<FakeMoney>(&destination);
        assert!(Option::is_none(&supply<FakeMoney>()), 0);

        mint<FakeMoney>(&source, source_addr, 100);
        transfer<FakeMoney>(&source, destination_addr, 50);

        assert!(balance<FakeMoney>(source_addr) == 50, 1);
        assert!(balance<FakeMoney>(destination_addr) == 50, 2);
        assert!(Option::is_none(&supply<FakeMoney>()), 3);

        let coin = withdraw<FakeMoney>(&source, 10);
        burn(&source, coin);
        assert!(Option::is_none(&supply<FakeMoney>()), 4);
    }

    #[test(source = @0x2)]
    #[expected_failure(abort_code = 7)]
    public(script) fun fail_initialize(source: signer) {
        initialize<FakeMoney>(&source, b"Fake money", 1, true);
    }

    #[test(source = @0x1, destination = @0x2)]
    #[expected_failure(abort_code = 1029)]
    public(script) fun fail_transfer(
        source: signer,
        destination: signer,
    ) acquires CoinStore, CoinInfo, MintCapability {
        let source_addr = Signer::address_of(&source);
        let destination_addr = Signer::address_of(&destination);

        initialize<FakeMoney>(&source, b"Fake money", 1, true);
        register<FakeMoney>(&source);
        assert!(*Option::borrow(&supply<FakeMoney>()) == 0, 0);

        mint<FakeMoney>(&source, source_addr, 100);
        transfer<FakeMoney>(&source, destination_addr, 50);
    }

    #[test(source = @0x1, destination = @0x2)]
    #[expected_failure(abort_code = 1797)]
    public(script) fun fail_mint(
        source: signer,
        destination: signer,
    ) acquires CoinStore, CoinInfo, MintCapability {
        let source_addr = Signer::address_of(&source);

        initialize<FakeMoney>(&source, b"Fake money", 1, true);
        register<FakeMoney>(&source);
        register<FakeMoney>(&destination);
        assert!(*Option::borrow(&supply<FakeMoney>()) == 0, 0);

        mint<FakeMoney>(&destination, source_addr, 100);
    }

    #[test(source = @0x1, destination = @0x2)]
    #[expected_failure(abort_code = 1541)]
    public(script) fun fail_burn(
        source: signer,
        destination: signer,
    ) acquires BurnCapability, CoinStore, CoinInfo, MintCapability {
        let source_addr = Signer::address_of(&source);

        initialize<FakeMoney>(&source, b"Fake money", 1, true);
        register<FakeMoney>(&source);
        register<FakeMoney>(&destination);

        mint<FakeMoney>(&source, source_addr, 100);
        let coin = withdraw<FakeMoney>(&source, 10);
        burn(&destination, coin);
    }

    #[test(source = @0x1)]
    #[expected_failure(abort_code = 2055)]
    public(script) fun test_destroy_non_zero(source: signer) acquires CoinInfo, CoinStore, MintCapability {
        let source_addr = Signer::address_of(&source);

        initialize<FakeMoney>(&source, b"Fake money", 1, true);
        register<FakeMoney>(&source);

        mint<FakeMoney>(&source, source_addr, 100);
        let coin = withdraw<FakeMoney>(&source, 100);

        destroy_zero(coin);
    }

    #[test(source = @0x1)]
    public(script) fun test_extract(source: signer) acquires CoinInfo, CoinStore, MintCapability {
        let source_addr = Signer::address_of(&source);

        initialize<FakeMoney>(&source, b"Fake money", 1, true);
        register<FakeMoney>(&source);

        mint<FakeMoney>(&source, source_addr, 100);
        let coin = withdraw<FakeMoney>(&source, 100);

        let extracted = extract(&mut coin, 25);
        assert!(value(&coin) == 75, 0);
        assert!(value(&extracted) == 25, 1);

        deposit(source_addr, coin);
        deposit(source_addr, extracted);

        assert!(balance<FakeMoney>(source_addr) == 100, 4);
    }

    #[test(source = @0x1)]
    public(script) fun test_is_registered(source: signer) {
        assert!(!is_registered<FakeMoney>(), 0);
        initialize<FakeMoney>(&source, b"Fake money", 1, true);
        assert!(is_registered<FakeMoney>(), 1);
    }

    #[test]
    fun test_zero() {
        let zero = zero<FakeMoney>();
        assert!(value(&zero) == 0, 1);
        destroy_zero(zero);
    }
}
