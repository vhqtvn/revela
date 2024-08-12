module 0x1::coin {
    struct Deposit<phantom T0> has drop, store {
        account: address,
        amount: u64,
    }
    
    struct DepositEvent has drop, store {
        amount: u64,
    }
    
    struct Withdraw<phantom T0> has drop, store {
        account: address,
        amount: u64,
    }
    
    struct WithdrawEvent has drop, store {
        amount: u64,
    }
    
    struct AggregatableCoin<phantom T0> has store {
        value: 0x1::aggregator::Aggregator,
    }
    
    struct BurnCapability<phantom T0> has copy, store {
        dummy_field: bool,
    }
    
    struct BurnRefReceipt {
        metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>,
    }
    
    struct Coin<phantom T0> has store {
        value: u64,
    }
    
    struct CoinConversionMap has key {
        coin_to_fungible_asset_map: 0x1::table::Table<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>,
    }
    
    struct CoinDeposit has drop, store {
        coin_type: 0x1::string::String,
        account: address,
        amount: u64,
    }
    
    struct CoinEventHandleDeletion has drop, store {
        event_handle_creation_address: address,
        deleted_deposit_event_handle_creation_number: u64,
        deleted_withdraw_event_handle_creation_number: u64,
    }
    
    struct CoinInfo<phantom T0> has key {
        name: 0x1::string::String,
        symbol: 0x1::string::String,
        decimals: u8,
        supply: 0x1::option::Option<0x1::optional_aggregator::OptionalAggregator>,
    }
    
    struct CoinStore<phantom T0> has key {
        coin: Coin<T0>,
        frozen: bool,
        deposit_events: 0x1::event::EventHandle<DepositEvent>,
        withdraw_events: 0x1::event::EventHandle<WithdrawEvent>,
    }
    
    struct CoinWithdraw has drop, store {
        coin_type: 0x1::string::String,
        account: address,
        amount: u64,
    }
    
    struct FreezeCapability<phantom T0> has copy, store {
        dummy_field: bool,
    }
    
    struct MigrationFlag has key {
        dummy_field: bool,
    }
    
    struct MintCapability<phantom T0> has copy, store {
        dummy_field: bool,
    }
    
    struct MintRefReceipt {
        metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>,
    }
    
    struct PairCreation has drop, store {
        coin_type: 0x1::type_info::TypeInfo,
        fungible_asset_metadata_address: address,
    }
    
    struct PairedCoinType has key {
        type: 0x1::type_info::TypeInfo,
    }
    
    struct PairedFungibleAssetRefs has key {
        mint_ref_opt: 0x1::option::Option<0x1::fungible_asset::MintRef>,
        transfer_ref_opt: 0x1::option::Option<0x1::fungible_asset::TransferRef>,
        burn_ref_opt: 0x1::option::Option<0x1::fungible_asset::BurnRef>,
    }
    
    struct SupplyConfig has key {
        allow_upgrades: bool,
    }
    
    struct TransferRefReceipt {
        metadata: 0x1::object::Object<0x1::fungible_asset::Metadata>,
    }
    
    public fun burn_from<T0>(arg0: address, arg1: u64, arg2: &BurnCapability<T0>) acquires CoinConversionMap, CoinInfo, CoinStore, PairedFungibleAssetRefs {
        if (arg1 == 0) {
            return
        };
        let v0 = if (exists<CoinStore<T0>>(arg0)) {
            borrow_global<CoinStore<T0>>(arg0).coin.value
        } else {
            0
        };
        let (v1, v2) = if (v0 >= arg1) {
            (arg1, 0)
        } else {
            let v3 = paired_metadata<T0>();
            let v4 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v3);
            assert!(v4 && 0x1::primary_fungible_store::primary_store_exists<0x1::fungible_asset::Metadata>(arg0, 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v3)), 0x1::error::invalid_argument(6));
            (v0, arg1 - v0)
        };
        if (v1 > 0) {
            burn<T0>(extract<T0>(&mut borrow_global_mut<CoinStore<T0>>(arg0).coin, v1), arg2);
        };
        if (v2 > 0) {
            let v5 = paired_metadata<T0>();
            assert!(0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v5), 0x1::error::not_found(16));
            let v6 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v5);
            let v7 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v6);
            assert!(exists<PairedFungibleAssetRefs>(v7), 0x1::error::internal(19));
            let v8 = &mut borrow_global_mut<PairedFungibleAssetRefs>(v7).burn_ref_opt;
            assert!(0x1::option::is_some<0x1::fungible_asset::BurnRef>(v8), 0x1::error::not_found(25));
            let v9 = paired_metadata<T0>();
            let v10 = 0x1::primary_fungible_store::primary_store<0x1::fungible_asset::Metadata>(arg0, 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v9));
            0x1::fungible_asset::burn_from<0x1::fungible_asset::FungibleStore>(0x1::option::borrow<0x1::fungible_asset::BurnRef>(v8), v10, v2);
        };
    }
    
    fun burn_internal<T0>(arg0: Coin<T0>) : u64 acquires CoinInfo {
        let Coin { value: v0 } = arg0;
        if (v0 != 0) {
            let v1 = &mut borrow_global_mut<CoinInfo<T0>>(coin_address<T0>()).supply;
            if (0x1::option::is_some<0x1::optional_aggregator::OptionalAggregator>(v1)) {
                let v2 = 0x1::option::borrow_mut<0x1::optional_aggregator::OptionalAggregator>(v1);
                0x1::optional_aggregator::sub(v2, (v0 as u128));
            };
        };
        v0
    }
    
    public fun deposit<T0>(arg0: address, arg1: Coin<T0>) acquires CoinConversionMap, CoinInfo, CoinStore {
        if (exists<CoinStore<T0>>(arg0)) {
            let v0 = borrow_global_mut<CoinStore<T0>>(arg0);
            if (v0.frozen) {
                abort 0x1::error::permission_denied(10)
            };
            if (0x1::features::module_event_migration_enabled()) {
                let v1 = CoinDeposit{
                    coin_type : 0x1::type_info::type_name<T0>(), 
                    account   : arg0, 
                    amount    : arg1.value,
                };
                0x1::event::emit<CoinDeposit>(v1);
            };
            let v2 = DepositEvent{amount: arg1.value};
            0x1::event::emit_event<DepositEvent>(&mut v0.deposit_events, v2);
            merge<T0>(&mut v0.coin, arg1);
        } else {
            let v3 = paired_metadata<T0>();
            let v4 = if (0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v3)) {
                let v5 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v3);
                let v6 = 0x1::primary_fungible_store::primary_store_address<0x1::fungible_asset::Metadata>(arg0, v5);
                if (0x1::fungible_asset::store_exists(v6)) {
                    if (0x1::features::new_accounts_default_to_fa_apt_store_enabled()) {
                        true
                    } else {
                        exists<MigrationFlag>(v6)
                    }
                } else {
                    false
                }
            } else {
                false
            };
            assert!(v4, 0x1::error::not_found(5));
            let v7 = coin_to_fungible_asset<T0>(arg1);
            0x1::primary_fungible_store::deposit(arg0, v7);
        };
        return
        abort 0x1::error::permission_denied(10)
    }
    
    fun mint_internal<T0>(arg0: u64) : Coin<T0> acquires CoinInfo {
        if (arg0 == 0) {
            return Coin<T0>{value: 0}
        };
        let v0 = &mut borrow_global_mut<CoinInfo<T0>>(coin_address<T0>()).supply;
        if (0x1::option::is_some<0x1::optional_aggregator::OptionalAggregator>(v0)) {
            let v1 = 0x1::option::borrow_mut<0x1::optional_aggregator::OptionalAggregator>(v0);
            0x1::optional_aggregator::add(v1, (arg0 as u128));
        };
        Coin<T0>{value: arg0}
    }
    
    public fun supply<T0>() : 0x1::option::Option<u128> acquires CoinConversionMap, CoinInfo {
        let v0 = coin_supply<T0>();
        let v1 = paired_metadata<T0>();
        if (0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v1)) {
            let v2 = 0x1::option::extract<0x1::object::Object<0x1::fungible_asset::Metadata>>(&mut v1);
            if (0x1::option::is_some<u128>(&v0)) {
                let v3 = 0x1::option::borrow_mut<u128>(&mut v0);
                let v4 = 0x1::option::destroy_some<u128>(0x1::fungible_asset::supply<0x1::fungible_asset::Metadata>(v2));
                *v3 = *v3 + v4;
            };
        };
        v0
    }
    
    public fun extract<T0>(arg0: &mut Coin<T0>, arg1: u64) : Coin<T0> {
        assert!(arg0.value >= arg1, 0x1::error::invalid_argument(6));
        arg0.value = arg0.value - arg1;
        Coin<T0>{value: arg1}
    }
    
    public fun balance<T0>(arg0: address) : u64 acquires CoinConversionMap, CoinStore {
        let v0 = paired_metadata<T0>();
        let v1 = if (exists<CoinStore<T0>>(arg0)) {
            borrow_global<CoinStore<T0>>(arg0).coin.value
        } else {
            0
        };
        let v2 = if (0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0)) {
            0x1::primary_fungible_store::balance<0x1::fungible_asset::Metadata>(arg0, 0x1::option::extract<0x1::object::Object<0x1::fungible_asset::Metadata>>(&mut v0))
        } else {
            0
        };
        v1 + v2
    }
    
    public fun is_balance_at_least<T0>(arg0: address, arg1: u64) : bool acquires CoinConversionMap, CoinStore {
        let v0 = if (exists<CoinStore<T0>>(arg0)) {
            borrow_global<CoinStore<T0>>(arg0).coin.value
        } else {
            0
        };
        if (v0 >= arg1) {
            return true
        };
        let v1 = paired_metadata<T0>();
        let v2 = arg1 - v0;
        let v3 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v1);
        v3 && 0x1::primary_fungible_store::is_balance_at_least<0x1::fungible_asset::Metadata>(arg0, 0x1::option::extract<0x1::object::Object<0x1::fungible_asset::Metadata>>(&mut v1), v2)
    }
    
    public fun withdraw<T0>(arg0: &signer, arg1: u64) : Coin<T0> acquires CoinConversionMap, CoinInfo, CoinStore, PairedCoinType {
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = if (exists<CoinStore<T0>>(v0)) {
            borrow_global<CoinStore<T0>>(v0).coin.value
        } else {
            0
        };
        let (v2, v3) = if (v1 >= arg1) {
            (arg1, 0)
        } else {
            let v4 = paired_metadata<T0>();
            let v5 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v4);
            assert!(v5 && 0x1::primary_fungible_store::primary_store_exists<0x1::fungible_asset::Metadata>(v0, 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v4)), 0x1::error::invalid_argument(6));
            (v1, arg1 - v1)
        };
        let v6 = if (v2 > 0) {
            let v7 = borrow_global_mut<CoinStore<T0>>(v0);
            if (v7.frozen) {
                abort 0x1::error::permission_denied(10)
            };
            if (0x1::features::module_event_migration_enabled()) {
                let v8 = CoinWithdraw{
                    coin_type : 0x1::type_info::type_name<T0>(), 
                    account   : v0, 
                    amount    : v2,
                };
                0x1::event::emit<CoinWithdraw>(v8);
            };
            let v9 = WithdrawEvent{amount: v2};
            0x1::event::emit_event<WithdrawEvent>(&mut v7.withdraw_events, v9);
            extract<T0>(&mut v7.coin, v2)
        } else {
            zero<T0>()
        };
        if (v3 > 0) {
            let v10 = paired_metadata<T0>();
            let v11 = fungible_asset_to_coin<T0>(0x1::primary_fungible_store::withdraw<0x1::fungible_asset::Metadata>(arg0, 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v10), v3));
            merge<T0>(&mut v6, v11);
        };
        v6
    }
    
    public fun allow_supply_upgrades(arg0: &signer, arg1: bool) acquires SupplyConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        borrow_global_mut<SupplyConfig>(@0x1).allow_upgrades = arg1;
    }
    
    public fun burn<T0>(arg0: Coin<T0>, arg1: &BurnCapability<T0>) acquires CoinInfo {
        burn_internal<T0>(arg0);
    }
    
    fun coin_address<T0>() : address {
        let v0 = 0x1::type_info::type_of<T0>();
        0x1::type_info::account_address(&v0)
    }
    
    public fun coin_supply<T0>() : 0x1::option::Option<u128> acquires CoinInfo {
        let v0 = &borrow_global<CoinInfo<T0>>(coin_address<T0>()).supply;
        if (0x1::option::is_some<0x1::optional_aggregator::OptionalAggregator>(v0)) {
            0x1::option::some<u128>(0x1::optional_aggregator::read(0x1::option::borrow<0x1::optional_aggregator::OptionalAggregator>(v0)))
        } else {
            0x1::option::none<u128>()
        }
    }
    
    public fun coin_to_fungible_asset<T0>(arg0: Coin<T0>) : 0x1::fungible_asset::FungibleAsset acquires CoinConversionMap, CoinInfo {
        let v0 = ensure_paired_metadata<T0>();
        let v1 = burn_internal<T0>(arg0);
        0x1::fungible_asset::mint_internal(v0, v1)
    }
    
    public(friend) fun collect_into_aggregatable_coin<T0>(arg0: address, arg1: u64, arg2: &mut AggregatableCoin<T0>) acquires CoinConversionMap, CoinInfo, CoinStore, PairedCoinType {
        if (arg1 == 0) {
            return
        } else {
            let v0 = if (exists<CoinStore<T0>>(arg0)) {
                borrow_global<CoinStore<T0>>(arg0).coin.value
            } else {
                0
            };
            let (v1, v2) = if (v0 >= arg1) {
                (arg1, 0)
            } else {
                let v3 = paired_metadata<T0>();
                let v4 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v3);
                assert!(v4 && 0x1::primary_fungible_store::primary_store_exists<0x1::fungible_asset::Metadata>(arg0, 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v3)), v10);
                (v0, arg1 - v0)
            };
            let v5 = if (v1 > 0) {
                extract<T0>(&mut borrow_global_mut<CoinStore<T0>>(arg0).coin, v1)
            } else {
                zero<T0>()
            };
            if (v2 > 0) {
                let v6 = paired_metadata<T0>();
                let v7 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v6);
                let v8 = 0x1::fungible_asset::withdraw_internal(0x1::primary_fungible_store::primary_store_address<0x1::fungible_asset::Metadata>(arg0, v7), v2);
                let v9 = fungible_asset_to_coin<T0>(v8);
                merge<T0>(&mut v5, v9);
            };
            merge_aggregatable_coin<T0>(arg2, v5);
            return
            let v10 = 0x1::error::invalid_argument(6);
            abort v10
        };
    }
    
    public fun convert_and_take_paired_burn_ref<T0>(arg0: BurnCapability<T0>) : 0x1::fungible_asset::BurnRef acquires CoinConversionMap, PairedFungibleAssetRefs {
        destroy_burn_cap<T0>(arg0);
        let v0 = paired_metadata<T0>();
        let v1 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0);
        assert!(v1, 0x1::error::not_found(16));
        let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v0);
        let v3 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v2);
        assert!(exists<PairedFungibleAssetRefs>(v3), 0x1::error::internal(19));
        let v4 = &mut borrow_global_mut<PairedFungibleAssetRefs>(v3).burn_ref_opt;
        assert!(0x1::option::is_some<0x1::fungible_asset::BurnRef>(v4), 0x1::error::not_found(25));
        0x1::option::extract<0x1::fungible_asset::BurnRef>(v4)
    }
    
    public entry fun create_coin_conversion_map(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (exists<CoinConversionMap>(@0x1)) {
        } else {
            let v0 = 0x1::table::new<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>();
            let v1 = CoinConversionMap{coin_to_fungible_asset_map: v0};
            move_to<CoinConversionMap>(arg0, v1);
        };
    }
    
    public entry fun create_pairing<T0>(arg0: &signer) acquires CoinConversionMap, CoinInfo {
        0x1::system_addresses::assert_aptos_framework(arg0);
        assert!(0x1::features::coin_to_fungible_asset_migration_feature_enabled(), 0x1::error::invalid_state(26));
        assert!(exists<CoinConversionMap>(@0x1), 0x1::error::not_found(27));
        let v0 = borrow_global_mut<CoinConversionMap>(@0x1);
        let v1 = 0x1::type_info::type_of<T0>();
        let v2 = &v0.coin_to_fungible_asset_map;
        if (0x1::table::contains<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>(v2, v1)) {
        } else {
            let v3 = 0x1::string::utf8(b"0x1::aptos_coin::AptosCoin");
            assert!(0x1::type_info::type_name<T0>() == v3 || true, 0x1::error::invalid_state(28));
            let v4 = if (v25) {
                0x1::object::create_sticky_object_at_address(@0x1, @0xa)
            } else {
                let v5 = 0x1::create_signer::create_signer(@0xa);
                let v6 = 0x1::type_info::type_name<T0>();
                0x1::object::create_named_object(&v5, *0x1::string::bytes(&v6))
            };
            let v7 = 0x1::option::none<u128>();
            let v8 = name<T0>();
            let v9 = symbol<T0>();
            let v10 = decimals<T0>();
            let v11 = 0x1::string::utf8(b"");
            let v12 = 0x1::string::utf8(b"");
            0x1::primary_fungible_store::create_primary_store_enabled_fungible_asset(&v4, v7, v8, v9, v10, v11, v12);
            let v13 = 0x1::object::generate_signer(&v4);
            let v14 = &v13;
            let v15 = 0x1::type_info::type_of<T0>();
            let v16 = PairedCoinType{type: v15};
            move_to<PairedCoinType>(v14, v16);
            let v17 = 0x1::object::object_from_constructor_ref<0x1::fungible_asset::Metadata>(&v4);
            let v18 = &mut v0.coin_to_fungible_asset_map;
            0x1::table::add<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>(v18, v15, v17);
            let v19 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v17);
            let v20 = PairCreation{
                coin_type                       : v15, 
                fungible_asset_metadata_address : v19,
            };
            0x1::event::emit<PairCreation>(v20);
            let v21 = 0x1::option::some<0x1::fungible_asset::MintRef>(0x1::fungible_asset::generate_mint_ref(&v4));
            let v22 = 0x1::option::some<0x1::fungible_asset::TransferRef>(0x1::fungible_asset::generate_transfer_ref(&v4));
            let v23 = 0x1::option::some<0x1::fungible_asset::BurnRef>(0x1::fungible_asset::generate_burn_ref(&v4));
            let v24 = PairedFungibleAssetRefs{
                mint_ref_opt     : v21, 
                transfer_ref_opt : v22, 
                burn_ref_opt     : v23,
            };
            move_to<PairedFungibleAssetRefs>(v14, v24);
        };
    }
    
    public fun decimals<T0>() : u8 acquires CoinInfo {
        borrow_global<CoinInfo<T0>>(coin_address<T0>()).decimals
    }
    
    public fun destroy_burn_cap<T0>(arg0: BurnCapability<T0>) {
        let BurnCapability {  } = arg0;
    }
    
    public fun destroy_freeze_cap<T0>(arg0: FreezeCapability<T0>) {
        let FreezeCapability {  } = arg0;
    }
    
    public fun destroy_mint_cap<T0>(arg0: MintCapability<T0>) {
        let MintCapability {  } = arg0;
    }
    
    public fun destroy_zero<T0>(arg0: Coin<T0>) {
        let Coin { value: v0 } = arg0;
        assert!(v0 == 0, 0x1::error::invalid_argument(7));
    }
    
    public(friend) fun drain_aggregatable_coin<T0>(arg0: &mut AggregatableCoin<T0>) : Coin<T0> {
        let v0 = 0x1::aggregator::read(&arg0.value);
        assert!(v0 <= 18446744073709551615, 0x1::error::out_of_range(14));
        0x1::aggregator::sub(&mut arg0.value, v0);
        Coin<T0>{value: (v0 as u64)}
    }
    
    public(friend) fun ensure_paired_metadata<T0>() : 0x1::object::Object<0x1::fungible_asset::Metadata> acquires CoinConversionMap, CoinInfo {
        assert!(0x1::features::coin_to_fungible_asset_migration_feature_enabled(), 0x1::error::invalid_state(26));
        assert!(exists<CoinConversionMap>(@0x1), 0x1::error::not_found(27));
        let v0 = borrow_global_mut<CoinConversionMap>(@0x1);
        let v1 = 0x1::type_info::type_of<T0>();
        let v2 = &v0.coin_to_fungible_asset_map;
        if (0x1::table::contains<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>(v2, v1)) {
        } else {
            let v3 = 0x1::string::utf8(b"0x1::aptos_coin::AptosCoin");
            assert!(0x1::type_info::type_name<T0>() == v3 && false || true, 0x1::error::invalid_state(28));
            let v4 = if (v26) {
                0x1::object::create_sticky_object_at_address(@0x1, @0xa)
            } else {
                let v5 = 0x1::create_signer::create_signer(@0xa);
                let v6 = 0x1::type_info::type_name<T0>();
                0x1::object::create_named_object(&v5, *0x1::string::bytes(&v6))
            };
            let v7 = 0x1::option::none<u128>();
            let v8 = name<T0>();
            let v9 = symbol<T0>();
            let v10 = decimals<T0>();
            let v11 = 0x1::string::utf8(b"");
            let v12 = 0x1::string::utf8(b"");
            0x1::primary_fungible_store::create_primary_store_enabled_fungible_asset(&v4, v7, v8, v9, v10, v11, v12);
            let v13 = 0x1::object::generate_signer(&v4);
            let v14 = &v13;
            let v15 = 0x1::type_info::type_of<T0>();
            let v16 = PairedCoinType{type: v15};
            move_to<PairedCoinType>(v14, v16);
            let v17 = 0x1::object::object_from_constructor_ref<0x1::fungible_asset::Metadata>(&v4);
            let v18 = &mut v0.coin_to_fungible_asset_map;
            0x1::table::add<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>(v18, v15, v17);
            let v19 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v17);
            let v20 = PairCreation{
                coin_type                       : v15, 
                fungible_asset_metadata_address : v19,
            };
            0x1::event::emit<PairCreation>(v20);
            let v21 = 0x1::option::some<0x1::fungible_asset::MintRef>(0x1::fungible_asset::generate_mint_ref(&v4));
            let v22 = 0x1::option::some<0x1::fungible_asset::TransferRef>(0x1::fungible_asset::generate_transfer_ref(&v4));
            let v23 = 0x1::option::some<0x1::fungible_asset::BurnRef>(0x1::fungible_asset::generate_burn_ref(&v4));
            let v24 = PairedFungibleAssetRefs{
                mint_ref_opt     : v21, 
                transfer_ref_opt : v22, 
                burn_ref_opt     : v23,
            };
            move_to<PairedFungibleAssetRefs>(v14, v24);
        };
        let v25 = &v0.coin_to_fungible_asset_map;
        *0x1::table::borrow<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>(v25, v1)
    }
    
    public fun extract_all<T0>(arg0: &mut Coin<T0>) : Coin<T0> {
        arg0.value = 0;
        Coin<T0>{value: arg0.value}
    }
    
    public(friend) fun force_deposit<T0>(arg0: address, arg1: Coin<T0>) acquires CoinConversionMap, CoinInfo, CoinStore {
        if (exists<CoinStore<T0>>(arg0)) {
            merge<T0>(&mut borrow_global_mut<CoinStore<T0>>(arg0).coin, arg1);
        } else {
            let v0 = paired_metadata<T0>();
            let v1 = if (0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0)) {
                let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v0);
                let v3 = 0x1::primary_fungible_store::primary_store_address<0x1::fungible_asset::Metadata>(arg0, v2);
                if (0x1::fungible_asset::store_exists(v3)) {
                    if (0x1::features::new_accounts_default_to_fa_apt_store_enabled()) {
                        true
                    } else {
                        exists<MigrationFlag>(v3)
                    }
                } else {
                    false
                }
            } else {
                false
            };
            assert!(v1, 0x1::error::not_found(5));
            let v4 = coin_to_fungible_asset<T0>(arg1);
            let v5 = 0x1::fungible_asset::asset_metadata(&v4);
            let v6 = 0x1::primary_fungible_store::primary_store<0x1::fungible_asset::Metadata>(arg0, v5);
            let v7 = 0x1::object::object_address<0x1::fungible_asset::FungibleStore>(&v6);
            0x1::fungible_asset::deposit_internal(v7, v4);
        };
    }
    
    public entry fun freeze_coin_store<T0>(arg0: address, arg1: &FreezeCapability<T0>) acquires CoinStore {
        borrow_global_mut<CoinStore<T0>>(arg0).frozen = true;
    }
    
    fun fungible_asset_to_coin<T0>(arg0: 0x1::fungible_asset::FungibleAsset) : Coin<T0> acquires CoinInfo, PairedCoinType {
        let v0 = 0x1::fungible_asset::metadata_from_asset(&arg0);
        let v1 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v0);
        assert!(0x1::object::object_exists<PairedCoinType>(v1), 0x1::error::not_found(15));
        let v2 = borrow_global<PairedCoinType>(v1).type == 0x1::type_info::type_of<T0>();
        assert!(v2, 0x1::error::invalid_argument(17));
        mint_internal<T0>(0x1::fungible_asset::burn_internal(arg0))
    }
    
    public fun get_paired_burn_ref<T0>(arg0: &BurnCapability<T0>) : (0x1::fungible_asset::BurnRef, BurnRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs {
        let v0 = paired_metadata<T0>();
        let v1 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0);
        assert!(v1, 0x1::error::not_found(16));
        let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v0);
        let v3 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v2);
        assert!(exists<PairedFungibleAssetRefs>(v3), 0x1::error::internal(19));
        let v4 = &mut borrow_global_mut<PairedFungibleAssetRefs>(v3).burn_ref_opt;
        assert!(0x1::option::is_some<0x1::fungible_asset::BurnRef>(v4), 0x1::error::not_found(25));
        let v5 = BurnRefReceipt{metadata: v2};
        (0x1::option::extract<0x1::fungible_asset::BurnRef>(v4), v5)
    }
    
    public fun get_paired_mint_ref<T0>(arg0: &MintCapability<T0>) : (0x1::fungible_asset::MintRef, MintRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs {
        let v0 = paired_metadata<T0>();
        let v1 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0);
        assert!(v1, 0x1::error::not_found(16));
        let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v0);
        let v3 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v2);
        assert!(exists<PairedFungibleAssetRefs>(v3), 0x1::error::internal(19));
        let v4 = &mut borrow_global_mut<PairedFungibleAssetRefs>(v3).mint_ref_opt;
        assert!(0x1::option::is_some<0x1::fungible_asset::MintRef>(v4), 0x1::error::not_found(21));
        let v5 = MintRefReceipt{metadata: v2};
        (0x1::option::extract<0x1::fungible_asset::MintRef>(v4), v5)
    }
    
    public fun get_paired_transfer_ref<T0>(arg0: &FreezeCapability<T0>) : (0x1::fungible_asset::TransferRef, TransferRefReceipt) acquires CoinConversionMap, PairedFungibleAssetRefs {
        let v0 = paired_metadata<T0>();
        let v1 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0);
        assert!(v1, 0x1::error::not_found(16));
        let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v0);
        let v3 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v2);
        assert!(exists<PairedFungibleAssetRefs>(v3), 0x1::error::internal(19));
        let v4 = &mut borrow_global_mut<PairedFungibleAssetRefs>(v3).transfer_ref_opt;
        assert!(0x1::option::is_some<0x1::fungible_asset::TransferRef>(v4), 0x1::error::not_found(23));
        let v5 = TransferRefReceipt{metadata: v2};
        (0x1::option::extract<0x1::fungible_asset::TransferRef>(v4), v5)
    }
    
    public fun initialize<T0>(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u8, arg4: bool) : (BurnCapability<T0>, FreezeCapability<T0>, MintCapability<T0>) {
        initialize_internal<T0>(arg0, arg1, arg2, arg3, arg4, false)
    }
    
    public(friend) fun initialize_aggregatable_coin<T0>(arg0: &signer) : AggregatableCoin<T0> {
        AggregatableCoin<T0>{value: 0x1::aggregator_factory::create_aggregator(arg0, 18446744073709551615)}
    }
    
    fun initialize_internal<T0>(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u8, arg4: bool, arg5: bool) : (BurnCapability<T0>, FreezeCapability<T0>, MintCapability<T0>) {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(coin_address<T0>() == v0, 0x1::error::invalid_argument(1));
        if (exists<CoinInfo<T0>>(v0)) {
            abort 0x1::error::already_exists(2)
        };
        assert!(0x1::string::length(&arg1) <= 32, 0x1::error::invalid_argument(12));
        assert!(0x1::string::length(&arg2) <= 10, 0x1::error::invalid_argument(13));
        let v1 = if (arg4) {
            0x1::option::some<0x1::optional_aggregator::OptionalAggregator>(0x1::optional_aggregator::new(340282366920938463463374607431768211455, arg5))
        } else {
            0x1::option::none<0x1::optional_aggregator::OptionalAggregator>()
        };
        let v2 = CoinInfo<T0>{
            name     : arg1, 
            symbol   : arg2, 
            decimals : arg3, 
            supply   : v1,
        };
        move_to<CoinInfo<T0>>(arg0, v2);
        let v3 = BurnCapability<T0>{dummy_field: false};
        let v4 = FreezeCapability<T0>{dummy_field: false};
        let v5 = MintCapability<T0>{dummy_field: false};
        (v3, v4, v5)
    }
    
    public(friend) fun initialize_supply_config(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = SupplyConfig{allow_upgrades: false};
        move_to<SupplyConfig>(arg0, v0);
    }
    
    public(friend) fun initialize_with_parallelizable_supply<T0>(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u8, arg4: bool) : (BurnCapability<T0>, FreezeCapability<T0>, MintCapability<T0>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        initialize_internal<T0>(arg0, arg1, arg2, arg3, arg4, true)
    }
    
    public fun is_account_registered<T0>(arg0: address) : bool acquires CoinConversionMap {
        assert!(is_coin_initialized<T0>(), 0x1::error::invalid_argument(3));
        if (exists<CoinStore<T0>>(arg0)) {
            true
        } else {
            let v1 = paired_metadata<T0>();
            if (0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v1)) {
                let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v1);
                let v3 = 0x1::primary_fungible_store::primary_store_address<0x1::fungible_asset::Metadata>(arg0, v2);
                if (0x1::fungible_asset::store_exists(v3)) {
                    if (0x1::features::new_accounts_default_to_fa_apt_store_enabled()) {
                        true
                    } else {
                        exists<MigrationFlag>(v3)
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
    }
    
    public(friend) fun is_aggregatable_coin_zero<T0>(arg0: &AggregatableCoin<T0>) : bool {
        0x1::aggregator::read(&arg0.value) == 0
    }
    
    public fun is_coin_initialized<T0>() : bool {
        exists<CoinInfo<T0>>(coin_address<T0>())
    }
    
    public fun is_coin_store_frozen<T0>(arg0: address) : bool acquires CoinConversionMap, CoinStore {
        if (is_account_registered<T0>(arg0)) {
            return borrow_global<CoinStore<T0>>(arg0).frozen
        };
        true
    }
    
    fun maybe_convert_to_fungible_store<T0>(arg0: address) acquires CoinConversionMap, CoinInfo, CoinStore {
        assert!(0x1::features::coin_to_fungible_asset_migration_feature_enabled(), 0x1::error::unavailable(18));
        assert!(is_coin_initialized<T0>(), 0x1::error::invalid_argument(3));
        let v0 = ensure_paired_metadata<T0>();
        let v1 = 0x1::primary_fungible_store::ensure_primary_store_exists<0x1::fungible_asset::Metadata>(arg0, v0);
        let v2 = 0x1::object::object_address<0x1::fungible_asset::FungibleStore>(&v1);
        if (exists<CoinStore<T0>>(arg0)) {
            let CoinStore {
                coin            : v3,
                frozen          : v4,
                deposit_events  : v5,
                withdraw_events : v6,
            } = move_from<CoinStore<T0>>(arg0);
            let v7 = v6;
            let v8 = v5;
            let v9 = v3;
            let v10 = 0x1::guid::creator_address(0x1::event::guid<DepositEvent>(&v8));
            let v11 = 0x1::guid::creation_num(0x1::event::guid<DepositEvent>(&v8));
            let v12 = 0x1::guid::creation_num(0x1::event::guid<WithdrawEvent>(&v7));
            let v13 = CoinEventHandleDeletion{
                event_handle_creation_address                 : v10, 
                deleted_deposit_event_handle_creation_number  : v11, 
                deleted_withdraw_event_handle_creation_number : v12,
            };
            0x1::event::emit<CoinEventHandleDeletion>(v13);
            0x1::event::destroy_handle<DepositEvent>(v8);
            0x1::event::destroy_handle<WithdrawEvent>(v7);
            if (v9.value == 0) {
                destroy_zero<T0>(v9);
            } else {
                let v14 = coin_to_fungible_asset<T0>(v9);
                0x1::fungible_asset::deposit<0x1::fungible_asset::FungibleStore>(v1, v14);
            };
            if (v4 != 0x1::fungible_asset::is_frozen<0x1::fungible_asset::FungibleStore>(v1)) {
                0x1::fungible_asset::set_frozen_flag_internal<0x1::fungible_asset::FungibleStore>(v1, v4);
            };
        };
        if (exists<MigrationFlag>(v2)) {
        } else {
            let v15 = 0x1::create_signer::create_signer(v2);
            let v16 = MigrationFlag{dummy_field: false};
            move_to<MigrationFlag>(&v15, v16);
        };
    }
    
    public fun merge<T0>(arg0: &mut Coin<T0>, arg1: Coin<T0>) {
        let Coin { value: v0 } = arg1;
        arg0.value = arg0.value + v0;
    }
    
    public(friend) fun merge_aggregatable_coin<T0>(arg0: &mut AggregatableCoin<T0>, arg1: Coin<T0>) {
        let Coin { value: v0 } = arg1;
        0x1::aggregator::add(&mut arg0.value, (v0 as u128));
    }
    
    public entry fun migrate_to_fungible_store<T0>(arg0: &signer) acquires CoinConversionMap, CoinInfo, CoinStore {
        maybe_convert_to_fungible_store<T0>(0x1::signer::address_of(arg0));
    }
    
    public fun mint<T0>(arg0: u64, arg1: &MintCapability<T0>) : Coin<T0> acquires CoinInfo {
        mint_internal<T0>(arg0)
    }
    
    public fun name<T0>() : 0x1::string::String acquires CoinInfo {
        borrow_global<CoinInfo<T0>>(coin_address<T0>()).name
    }
    
    public fun paired_burn_ref_exists<T0>() : bool acquires CoinConversionMap, PairedFungibleAssetRefs {
        let v0 = paired_metadata<T0>();
        let v1 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0);
        assert!(v1, 0x1::error::not_found(16));
        let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v0);
        let v3 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v2);
        assert!(exists<PairedFungibleAssetRefs>(v3), 0x1::error::internal(19));
        let v4 = &borrow_global<PairedFungibleAssetRefs>(v3).burn_ref_opt;
        0x1::option::is_some<0x1::fungible_asset::BurnRef>(v4)
    }
    
    public fun paired_coin(arg0: 0x1::object::Object<0x1::fungible_asset::Metadata>) : 0x1::option::Option<0x1::type_info::TypeInfo> acquires PairedCoinType {
        let v0 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&arg0);
        if (exists<PairedCoinType>(v0)) {
            0x1::option::some<0x1::type_info::TypeInfo>(borrow_global<PairedCoinType>(v0).type)
        } else {
            0x1::option::none<0x1::type_info::TypeInfo>()
        }
    }
    
    public fun paired_metadata<T0>() : 0x1::option::Option<0x1::object::Object<0x1::fungible_asset::Metadata>> acquires CoinConversionMap {
        if (exists<CoinConversionMap>(@0x1) && 0x1::features::coin_to_fungible_asset_migration_feature_enabled()) {
            let v0 = &borrow_global<CoinConversionMap>(@0x1).coin_to_fungible_asset_map;
            let v1 = 0x1::type_info::type_of<T0>();
            if (0x1::table::contains<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>(v0, v1)) {
                let v2 = *0x1::table::borrow<0x1::type_info::TypeInfo, 0x1::object::Object<0x1::fungible_asset::Metadata>>(v0, v1);
                return 0x1::option::some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v2)
            };
        };
        0x1::option::none<0x1::object::Object<0x1::fungible_asset::Metadata>>()
    }
    
    public fun paired_mint_ref_exists<T0>() : bool acquires CoinConversionMap, PairedFungibleAssetRefs {
        let v0 = paired_metadata<T0>();
        let v1 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0);
        assert!(v1, 0x1::error::not_found(16));
        let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v0);
        let v3 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v2);
        assert!(exists<PairedFungibleAssetRefs>(v3), 0x1::error::internal(19));
        let v4 = &borrow_global<PairedFungibleAssetRefs>(v3).mint_ref_opt;
        0x1::option::is_some<0x1::fungible_asset::MintRef>(v4)
    }
    
    public fun paired_transfer_ref_exists<T0>() : bool acquires CoinConversionMap, PairedFungibleAssetRefs {
        let v0 = paired_metadata<T0>();
        let v1 = 0x1::option::is_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(&v0);
        assert!(v1, 0x1::error::not_found(16));
        let v2 = 0x1::option::destroy_some<0x1::object::Object<0x1::fungible_asset::Metadata>>(v0);
        let v3 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v2);
        assert!(exists<PairedFungibleAssetRefs>(v3), 0x1::error::internal(19));
        let v4 = &borrow_global<PairedFungibleAssetRefs>(v3).transfer_ref_opt;
        0x1::option::is_some<0x1::fungible_asset::TransferRef>(v4)
    }
    
    public fun register<T0>(arg0: &signer) acquires CoinConversionMap {
        let v0 = 0x1::signer::address_of(arg0);
        if (is_account_registered<T0>(v0)) {
            return
        };
        0x1::account::register_coin<T0>(v0);
        let v1 = Coin<T0>{value: 0};
        let v2 = 0x1::account::new_event_handle<DepositEvent>(arg0);
        let v3 = 0x1::account::new_event_handle<WithdrawEvent>(arg0);
        let v4 = CoinStore<T0>{
            coin            : v1, 
            frozen          : false, 
            deposit_events  : v2, 
            withdraw_events : v3,
        };
        move_to<CoinStore<T0>>(arg0, v4);
    }
    
    public fun return_paired_burn_ref(arg0: 0x1::fungible_asset::BurnRef, arg1: BurnRefReceipt) acquires PairedFungibleAssetRefs {
        let BurnRefReceipt { metadata: v0 } = arg1;
        assert!(0x1::fungible_asset::burn_ref_metadata(&arg0) == v0, 0x1::error::invalid_argument(24));
        let v1 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v0);
        let v2 = &mut borrow_global_mut<PairedFungibleAssetRefs>(v1).burn_ref_opt;
        0x1::option::fill<0x1::fungible_asset::BurnRef>(v2, arg0);
    }
    
    public fun return_paired_mint_ref(arg0: 0x1::fungible_asset::MintRef, arg1: MintRefReceipt) acquires PairedFungibleAssetRefs {
        let MintRefReceipt { metadata: v0 } = arg1;
        assert!(0x1::fungible_asset::mint_ref_metadata(&arg0) == v0, 0x1::error::invalid_argument(20));
        let v1 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v0);
        let v2 = &mut borrow_global_mut<PairedFungibleAssetRefs>(v1).mint_ref_opt;
        0x1::option::fill<0x1::fungible_asset::MintRef>(v2, arg0);
    }
    
    public fun return_paired_transfer_ref(arg0: 0x1::fungible_asset::TransferRef, arg1: TransferRefReceipt) acquires PairedFungibleAssetRefs {
        let TransferRefReceipt { metadata: v0 } = arg1;
        assert!(0x1::fungible_asset::transfer_ref_metadata(&arg0) == v0, 0x1::error::invalid_argument(22));
        let v1 = 0x1::object::object_address<0x1::fungible_asset::Metadata>(&v0);
        let v2 = &mut borrow_global_mut<PairedFungibleAssetRefs>(v1).transfer_ref_opt;
        0x1::option::fill<0x1::fungible_asset::TransferRef>(v2, arg0);
    }
    
    public fun symbol<T0>() : 0x1::string::String acquires CoinInfo {
        borrow_global<CoinInfo<T0>>(coin_address<T0>()).symbol
    }
    
    public entry fun transfer<T0>(arg0: &signer, arg1: address, arg2: u64) acquires CoinConversionMap, CoinInfo, CoinStore, PairedCoinType {
        let v0 = withdraw<T0>(arg0, arg2);
        deposit<T0>(arg1, v0);
    }
    
    public entry fun unfreeze_coin_store<T0>(arg0: address, arg1: &FreezeCapability<T0>) acquires CoinStore {
        borrow_global_mut<CoinStore<T0>>(arg0).frozen = false;
    }
    
    public entry fun upgrade_supply<T0>(arg0: &signer) acquires CoinInfo, SupplyConfig {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(coin_address<T0>() == v0, 0x1::error::invalid_argument(1));
        assert!(borrow_global_mut<SupplyConfig>(@0x1).allow_upgrades, 0x1::error::permission_denied(11));
        let v1 = &mut borrow_global_mut<CoinInfo<T0>>(v0).supply;
        if (0x1::option::is_some<0x1::optional_aggregator::OptionalAggregator>(v1)) {
            let v2 = 0x1::option::borrow_mut<0x1::optional_aggregator::OptionalAggregator>(v1);
            if (0x1::optional_aggregator::is_parallelizable(v2)) {
            } else {
                0x1::optional_aggregator::switch(v2);
            };
        };
    }
    
    public fun value<T0>(arg0: &Coin<T0>) : u64 {
        arg0.value
    }
    
    public fun zero<T0>() : Coin<T0> {
        Coin<T0>{value: 0}
    }
    
    // decompiled from Move bytecode v7
}
