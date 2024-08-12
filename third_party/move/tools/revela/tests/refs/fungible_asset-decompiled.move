module 0x1::fungible_asset {
    struct TransferRef has drop, store {
        metadata: 0x1::object::Object<Metadata>,
    }
    
    struct Untransferable has key {
        dummy_field: bool,
    }
    
    struct BurnRef has drop, store {
        metadata: 0x1::object::Object<Metadata>,
    }
    
    struct ConcurrentFungibleBalance has key {
        balance: 0x1::aggregator_v2::Aggregator<u64>,
    }
    
    struct ConcurrentSupply has key {
        current: 0x1::aggregator_v2::Aggregator<u128>,
    }
    
    struct Deposit has drop, store {
        store: address,
        amount: u64,
    }
    
    struct DepositEvent has drop, store {
        amount: u64,
    }
    
    struct DeriveSupply has key {
        dispatch_function: 0x1::option::Option<0x1::function_info::FunctionInfo>,
    }
    
    struct DispatchFunctionStore has key {
        withdraw_function: 0x1::option::Option<0x1::function_info::FunctionInfo>,
        deposit_function: 0x1::option::Option<0x1::function_info::FunctionInfo>,
        derived_balance_function: 0x1::option::Option<0x1::function_info::FunctionInfo>,
    }
    
    struct Frozen has drop, store {
        store: address,
        frozen: bool,
    }
    
    struct FrozenEvent has drop, store {
        frozen: bool,
    }
    
    struct FungibleAsset {
        metadata: 0x1::object::Object<Metadata>,
        amount: u64,
    }
    
    struct FungibleAssetEvents has key {
        deposit_events: 0x1::event::EventHandle<DepositEvent>,
        withdraw_events: 0x1::event::EventHandle<WithdrawEvent>,
        frozen_events: 0x1::event::EventHandle<FrozenEvent>,
    }
    
    struct FungibleStore has key {
        metadata: 0x1::object::Object<Metadata>,
        balance: u64,
        frozen: bool,
    }
    
    struct Metadata has copy, drop, key {
        name: 0x1::string::String,
        symbol: 0x1::string::String,
        decimals: u8,
        icon_uri: 0x1::string::String,
        project_uri: 0x1::string::String,
    }
    
    struct MintRef has drop, store {
        metadata: 0x1::object::Object<Metadata>,
    }
    
    struct MutateMetadataRef has drop, store {
        metadata: 0x1::object::Object<Metadata>,
    }
    
    struct Supply has key {
        current: u128,
        maximum: 0x1::option::Option<u128>,
    }
    
    struct Withdraw has drop, store {
        store: address,
        amount: u64,
    }
    
    struct WithdrawEvent has drop, store {
        amount: u64,
    }
    
    public fun set_untransferable(arg0: &0x1::object::ConstructorRef) {
        let v0 = exists<Metadata>(0x1::object::address_from_constructor_ref(arg0));
        assert!(v0, 0x1::error::not_found(30));
        let v1 = 0x1::object::generate_signer(arg0);
        let v2 = Untransferable{dummy_field: false};
        move_to<Untransferable>(&v1, v2);
    }
    
    public fun extract(arg0: &mut FungibleAsset, arg1: u64) : FungibleAsset {
        assert!(arg0.amount >= arg1, 0x1::error::invalid_argument(4));
        arg0.amount = arg0.amount - arg1;
        FungibleAsset{
            metadata : arg0.metadata, 
            amount   : arg1,
        }
    }
    
    public fun add_fungibility(arg0: &0x1::object::ConstructorRef, arg1: 0x1::option::Option<u128>, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u8, arg5: 0x1::string::String, arg6: 0x1::string::String) : 0x1::object::Object<Metadata> {
        if (0x1::object::can_generate_delete_ref(arg0)) {
            abort 0x1::error::invalid_argument(18)
        };
        let v0 = 0x1::object::generate_signer(arg0);
        let v1 = &v0;
        assert!(0x1::string::length(&arg2) <= 32, 0x1::error::out_of_range(15));
        assert!(0x1::string::length(&arg3) <= 10, 0x1::error::out_of_range(16));
        assert!(arg4 <= 32, 0x1::error::out_of_range(17));
        assert!(0x1::string::length(&arg5) <= 512, 0x1::error::out_of_range(19));
        assert!(0x1::string::length(&arg6) <= 512, 0x1::error::out_of_range(19));
        let v2 = Metadata{
            name        : arg2, 
            symbol      : arg3, 
            decimals    : arg4, 
            icon_uri    : arg5, 
            project_uri : arg6,
        };
        move_to<Metadata>(v1, v2);
        if (0x1::features::concurrent_fungible_assets_enabled()) {
            let v3 = if (0x1::option::is_none<u128>(&arg1)) {
                0x1::aggregator_v2::create_unbounded_aggregator<u128>()
            } else {
                0x1::aggregator_v2::create_aggregator<u128>(0x1::option::extract<u128>(&mut arg1))
            };
            let v4 = ConcurrentSupply{current: v3};
            move_to<ConcurrentSupply>(v1, v4);
        } else {
            let v5 = Supply{
                current : 0, 
                maximum : arg1,
            };
            move_to<Supply>(v1, v5);
        };
        0x1::object::object_from_constructor_ref<Metadata>(arg0)
    }
    
    public(friend) fun address_burn_from(arg0: &BurnRef, arg1: address, arg2: u64) acquires ConcurrentFungibleBalance, ConcurrentSupply, FungibleStore, Supply {
        let v0 = withdraw_internal(arg1, arg2);
        burn(arg0, v0);
    }
    
    public fun amount(arg0: &FungibleAsset) : u64 {
        arg0.amount
    }
    
    public fun asset_metadata(arg0: &FungibleAsset) : 0x1::object::Object<Metadata> {
        arg0.metadata
    }
    
    public fun balance<T0: key>(arg0: 0x1::object::Object<T0>) : u64 acquires ConcurrentFungibleBalance, FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        if (exists<FungibleStore>(v0)) {
            let v2 = 0x1::object::object_address<T0>(&arg0);
            assert!(exists<FungibleStore>(v2), 0x1::error::not_found(23));
            let v3 = borrow_global<FungibleStore>(v2).balance;
            if (v3 == 0 && exists<ConcurrentFungibleBalance>(v0)) {
                0x1::aggregator_v2::read<u64>(&borrow_global<ConcurrentFungibleBalance>(v0).balance)
            } else {
                v3
            }
        } else {
            0
        }
    }
    
    public fun burn(arg0: &BurnRef, arg1: FungibleAsset) acquires ConcurrentSupply, Supply {
        assert!(arg0.metadata == metadata_from_asset(&arg1), 0x1::error::invalid_argument(13));
        burn_internal(arg1);
    }
    
    public fun burn_from<T0: key>(arg0: &BurnRef, arg1: 0x1::object::Object<T0>, arg2: u64) acquires ConcurrentFungibleBalance, ConcurrentSupply, FungibleStore, Supply {
        let v0 = withdraw_internal(0x1::object::object_address<T0>(&arg1), arg2);
        burn(arg0, v0);
    }
    
    public(friend) fun burn_internal(arg0: FungibleAsset) : u64 acquires ConcurrentSupply, Supply {
        let FungibleAsset {
            metadata : v0,
            amount   : v1,
        } = arg0;
        let v2 = v0;
        decrease_supply<Metadata>(&v2, v1);
        v1
    }
    
    public fun burn_ref_metadata(arg0: &BurnRef) : 0x1::object::Object<Metadata> {
        arg0.metadata
    }
    
    public fun create_store<T0: key>(arg0: &0x1::object::ConstructorRef, arg1: 0x1::object::Object<T0>) : 0x1::object::Object<FungibleStore> {
        let v0 = 0x1::object::generate_signer(arg0);
        let v1 = &v0;
        let v2 = FungibleStore{
            metadata : 0x1::object::convert<T0, Metadata>(arg1), 
            balance  : 0, 
            frozen   : false,
        };
        move_to<FungibleStore>(v1, v2);
        if (is_untransferable<T0>(arg1)) {
            0x1::object::set_untransferable(arg0);
        };
        if (0x1::features::default_to_concurrent_fungible_balance_enabled()) {
            let v3 = ConcurrentFungibleBalance{balance: 0x1::aggregator_v2::create_unbounded_aggregator<u64>()};
            move_to<ConcurrentFungibleBalance>(v1, v3);
        };
        0x1::object::object_from_constructor_ref<FungibleStore>(arg0)
    }
    
    public fun decimals<T0: key>(arg0: 0x1::object::Object<T0>) : u8 acquires Metadata {
        borrow_global<Metadata>(0x1::object::object_address<T0>(&arg0)).decimals
    }
    
    fun decrease_supply<T0: key>(arg0: &0x1::object::Object<T0>, arg1: u64) acquires ConcurrentSupply, Supply {
        if (arg1 == 0) {
            return
        };
        let v0 = 0x1::object::object_address<T0>(arg0);
        if (exists<ConcurrentSupply>(v0)) {
            let v1 = &mut borrow_global_mut<ConcurrentSupply>(v0).current;
            assert!(0x1::aggregator_v2::try_sub<u128>(v1, (arg1 as u128)), 0x1::error::out_of_range(20));
        } else {
            assert!(exists<Supply>(v0), 0x1::error::not_found(21));
            assert!(exists<Supply>(v0), 0x1::error::not_found(21));
            let v2 = borrow_global_mut<Supply>(v0);
            assert!(v2.current >= (arg1 as u128), 0x1::error::invalid_state(20));
            v2.current = v2.current - (arg1 as u128);
        };
    }
    
    public fun deposit<T0: key>(arg0: 0x1::object::Object<T0>, arg1: FungibleAsset) acquires ConcurrentFungibleBalance, DispatchFunctionStore, FungibleStore {
        deposit_sanity_check<T0>(arg0, true);
        deposit_internal(0x1::object::object_address<T0>(&arg0), arg1);
    }
    
    public fun deposit_dispatch_function<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::option::Option<0x1::function_info::FunctionInfo> acquires DispatchFunctionStore, FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        assert!(exists<FungibleStore>(v0), 0x1::error::not_found(23));
        let v1 = 0x1::object::object_address<Metadata>(&borrow_global<FungibleStore>(v0).metadata);
        if (exists<DispatchFunctionStore>(v1)) {
            borrow_global<DispatchFunctionStore>(v1).deposit_function
        } else {
            0x1::option::none<0x1::function_info::FunctionInfo>()
        }
    }
    
    public(friend) fun deposit_internal(arg0: address, arg1: FungibleAsset) acquires ConcurrentFungibleBalance, FungibleStore {
        let FungibleAsset {
            metadata : v0,
            amount   : v1,
        } = arg1;
        assert!(exists<FungibleStore>(arg0), 0x1::error::not_found(23));
        let v2 = borrow_global_mut<FungibleStore>(arg0);
        assert!(v0 == v2.metadata, 0x1::error::invalid_argument(11));
        if (v1 == 0) {
            return
        };
        if (v2.balance == 0 && exists<ConcurrentFungibleBalance>(arg0)) {
            0x1::aggregator_v2::add<u64>(&mut borrow_global_mut<ConcurrentFungibleBalance>(arg0).balance, v1);
        } else {
            v2.balance = v2.balance + v1;
        };
        let v3 = Deposit{
            store  : arg0, 
            amount : v1,
        };
        0x1::event::emit<Deposit>(v3);
    }
    
    public fun deposit_sanity_check<T0: key>(arg0: 0x1::object::Object<T0>, arg1: bool) acquires DispatchFunctionStore, FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        assert!(exists<FungibleStore>(v0), 0x1::error::not_found(23));
        let v1 = borrow_global<FungibleStore>(v0);
        if (arg1) {
            let v2 = has_deposit_dispatch_function(v1.metadata);
            arg1 = !v2;
        } else {
            arg1 = true;
        };
        assert!(arg1, 0x1::error::invalid_argument(28));
        if (v1.frozen) {
            abort 0x1::error::permission_denied(3)
        };
    }
    
    public fun deposit_with_ref<T0: key>(arg0: &TransferRef, arg1: 0x1::object::Object<T0>, arg2: FungibleAsset) acquires ConcurrentFungibleBalance, FungibleStore {
        assert!(arg0.metadata == arg2.metadata, 0x1::error::invalid_argument(2));
        deposit_internal(0x1::object::object_address<T0>(&arg1), arg2);
    }
    
    public(friend) fun derived_balance_dispatch_function<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::option::Option<0x1::function_info::FunctionInfo> acquires DispatchFunctionStore, FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        assert!(exists<FungibleStore>(v0), 0x1::error::not_found(23));
        let v1 = 0x1::object::object_address<Metadata>(&borrow_global<FungibleStore>(v0).metadata);
        if (exists<DispatchFunctionStore>(v1)) {
            borrow_global<DispatchFunctionStore>(v1).derived_balance_function
        } else {
            0x1::option::none<0x1::function_info::FunctionInfo>()
        }
    }
    
    public(friend) fun derived_supply_dispatch_function<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::option::Option<0x1::function_info::FunctionInfo> acquires DeriveSupply {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        if (exists<DeriveSupply>(v0)) {
            borrow_global<DeriveSupply>(v0).dispatch_function
        } else {
            0x1::option::none<0x1::function_info::FunctionInfo>()
        }
    }
    
    public fun destroy_zero(arg0: FungibleAsset) {
        let FungibleAsset {
            metadata : _,
            amount   : v1,
        } = arg0;
        assert!(v1 == 0, 0x1::error::invalid_argument(12));
    }
    
    fun ensure_store_upgraded_to_concurrent_internal(arg0: address) acquires FungibleStore {
        if (exists<ConcurrentFungibleBalance>(arg0)) {
            return
        };
        let v0 = borrow_global_mut<FungibleStore>(arg0);
        let v1 = 0x1::aggregator_v2::create_unbounded_aggregator_with_value<u64>(v0.balance);
        v0.balance = 0;
        let v2 = 0x1::create_signer::create_signer(arg0);
        let v3 = ConcurrentFungibleBalance{balance: v1};
        move_to<ConcurrentFungibleBalance>(&v2, v3);
    }
    
    public fun generate_burn_ref(arg0: &0x1::object::ConstructorRef) : BurnRef {
        BurnRef{metadata: 0x1::object::object_from_constructor_ref<Metadata>(arg0)}
    }
    
    public fun generate_mint_ref(arg0: &0x1::object::ConstructorRef) : MintRef {
        MintRef{metadata: 0x1::object::object_from_constructor_ref<Metadata>(arg0)}
    }
    
    public fun generate_mutate_metadata_ref(arg0: &0x1::object::ConstructorRef) : MutateMetadataRef {
        MutateMetadataRef{metadata: 0x1::object::object_from_constructor_ref<Metadata>(arg0)}
    }
    
    public fun generate_transfer_ref(arg0: &0x1::object::ConstructorRef) : TransferRef {
        TransferRef{metadata: 0x1::object::object_from_constructor_ref<Metadata>(arg0)}
    }
    
    fun has_deposit_dispatch_function(arg0: 0x1::object::Object<Metadata>) : bool acquires DispatchFunctionStore {
        let v0 = 0x1::object::object_address<Metadata>(&arg0);
        if (v0 != @0xa && exists<DispatchFunctionStore>(v0)) {
            let v1 = &borrow_global<DispatchFunctionStore>(v0).deposit_function;
            v2 = 0x1::option::is_some<0x1::function_info::FunctionInfo>(v1);
        } else {
            v2 = false;
        };
        v2
    }
    
    fun has_withdraw_dispatch_function(arg0: 0x1::object::Object<Metadata>) : bool acquires DispatchFunctionStore {
        let v0 = 0x1::object::object_address<Metadata>(&arg0);
        if (v0 != @0xa && exists<DispatchFunctionStore>(v0)) {
            let v1 = &borrow_global<DispatchFunctionStore>(v0).withdraw_function;
            v2 = 0x1::option::is_some<0x1::function_info::FunctionInfo>(v1);
        } else {
            v2 = false;
        };
        v2
    }
    
    public fun icon_uri<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::string::String acquires Metadata {
        borrow_global<Metadata>(0x1::object::object_address<T0>(&arg0)).icon_uri
    }
    
    fun increase_supply<T0: key>(arg0: &0x1::object::Object<T0>, arg1: u64) acquires ConcurrentSupply, Supply {
        if (arg1 == 0) {
            return
        };
        let v0 = 0x1::object::object_address<T0>(arg0);
        if (exists<ConcurrentSupply>(v0)) {
            let v1 = (arg1 as u128);
            let v2 = 0x1::aggregator_v2::try_add<u128>(&mut borrow_global_mut<ConcurrentSupply>(v0).current, v1);
            assert!(v2, 0x1::error::out_of_range(5));
        } else {
            assert!(exists<Supply>(v0), 0x1::error::not_found(21));
            let v3 = borrow_global_mut<Supply>(v0);
            if (0x1::option::is_some<u128>(&v3.maximum)) {
                let v4 = *0x1::option::borrow_mut<u128>(&mut v3.maximum) - v3.current >= (arg1 as u128);
                assert!(v4, 0x1::error::out_of_range(5));
            };
            v3.current = v3.current + (arg1 as u128);
        };
    }
    
    public(friend) fun is_address_balance_at_least(arg0: address, arg1: u64) : bool acquires ConcurrentFungibleBalance, FungibleStore {
        if (exists<FungibleStore>(arg0)) {
            let v1 = borrow_global<FungibleStore>(arg0).balance;
            if (v1 == 0 && exists<ConcurrentFungibleBalance>(arg0)) {
                v0 = 0x1::aggregator_v2::is_at_least<u64>(&borrow_global<ConcurrentFungibleBalance>(arg0).balance, arg1);
            } else {
                v0 = v1 >= arg1;
            };
            v0
        } else {
            arg1 == 0
        }
    }
    
    public fun is_balance_at_least<T0: key>(arg0: 0x1::object::Object<T0>, arg1: u64) : bool acquires ConcurrentFungibleBalance, FungibleStore {
        is_address_balance_at_least(0x1::object::object_address<T0>(&arg0), arg1)
    }
    
    public fun is_frozen<T0: key>(arg0: 0x1::object::Object<T0>) : bool acquires FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        exists<FungibleStore>(v0) && borrow_global<FungibleStore>(v0).frozen
    }
    
    public fun is_store_dispatchable<T0: key>(arg0: 0x1::object::Object<T0>) : bool acquires FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        assert!(exists<FungibleStore>(v0), 0x1::error::not_found(23));
        let v1 = 0x1::object::object_address<Metadata>(&borrow_global<FungibleStore>(v0).metadata);
        exists<DispatchFunctionStore>(v1)
    }
    
    public fun is_untransferable<T0: key>(arg0: 0x1::object::Object<T0>) : bool {
        exists<Untransferable>(0x1::object::object_address<T0>(&arg0))
    }
    
    public fun maximum<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::option::Option<u128> acquires ConcurrentSupply, Supply {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        if (exists<ConcurrentSupply>(v0)) {
            let v2 = 0x1::aggregator_v2::max_value<u128>(&borrow_global<ConcurrentSupply>(v0).current);
            if (v2 == 340282366920938463463374607431768211455) {
                0x1::option::none<u128>()
            } else {
                0x1::option::some<u128>(v2)
            }
        } else {
            if (exists<Supply>(v0)) {
                borrow_global<Supply>(v0).maximum
            } else {
                0x1::option::none<u128>()
            }
        }
    }
    
    public fun merge(arg0: &mut FungibleAsset, arg1: FungibleAsset) {
        let FungibleAsset {
            metadata : v0,
            amount   : v1,
        } = arg1;
        assert!(v0 == arg0.metadata, 0x1::error::invalid_argument(6));
        arg0.amount = arg0.amount + v1;
    }
    
    public fun metadata<T0: key>(arg0: 0x1::object::Object<T0>) : Metadata acquires Metadata {
        *borrow_global<Metadata>(0x1::object::object_address<T0>(&arg0))
    }
    
    public fun metadata_from_asset(arg0: &FungibleAsset) : 0x1::object::Object<Metadata> {
        arg0.metadata
    }
    
    public fun mint(arg0: &MintRef, arg1: u64) : FungibleAsset acquires ConcurrentSupply, Supply {
        mint_internal(arg0.metadata, arg1)
    }
    
    public(friend) fun mint_internal(arg0: 0x1::object::Object<Metadata>, arg1: u64) : FungibleAsset acquires ConcurrentSupply, Supply {
        increase_supply<Metadata>(&arg0, arg1);
        FungibleAsset{
            metadata : arg0, 
            amount   : arg1,
        }
    }
    
    public fun mint_ref_metadata(arg0: &MintRef) : 0x1::object::Object<Metadata> {
        arg0.metadata
    }
    
    public fun mint_to<T0: key>(arg0: &MintRef, arg1: 0x1::object::Object<T0>, arg2: u64) acquires ConcurrentFungibleBalance, ConcurrentSupply, DispatchFunctionStore, FungibleStore, Supply {
        deposit_sanity_check<T0>(arg1, false);
        let v0 = mint(arg0, arg2);
        deposit_internal(0x1::object::object_address<T0>(&arg1), v0);
    }
    
    public fun mutate_metadata(arg0: &MutateMetadataRef, arg1: 0x1::option::Option<0x1::string::String>, arg2: 0x1::option::Option<0x1::string::String>, arg3: 0x1::option::Option<u8>, arg4: 0x1::option::Option<0x1::string::String>, arg5: 0x1::option::Option<0x1::string::String>) acquires Metadata {
        let v0 = borrow_global_mut<Metadata>(0x1::object::object_address<Metadata>(&arg0.metadata));
        if (0x1::option::is_some<0x1::string::String>(&arg1)) {
            v0.name = 0x1::option::extract<0x1::string::String>(&mut arg1);
        };
        if (0x1::option::is_some<0x1::string::String>(&arg2)) {
            v0.symbol = 0x1::option::extract<0x1::string::String>(&mut arg2);
        };
        if (0x1::option::is_some<u8>(&arg3)) {
            v0.decimals = 0x1::option::extract<u8>(&mut arg3);
        };
        if (0x1::option::is_some<0x1::string::String>(&arg4)) {
            v0.icon_uri = 0x1::option::extract<0x1::string::String>(&mut arg4);
        };
        if (0x1::option::is_some<0x1::string::String>(&arg5)) {
            v0.project_uri = 0x1::option::extract<0x1::string::String>(&mut arg5);
        };
    }
    
    public fun name<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::string::String acquires Metadata {
        borrow_global<Metadata>(0x1::object::object_address<T0>(&arg0)).name
    }
    
    public fun object_from_metadata_ref(arg0: &MutateMetadataRef) : 0x1::object::Object<Metadata> {
        arg0.metadata
    }
    
    public fun project_uri<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::string::String acquires Metadata {
        borrow_global<Metadata>(0x1::object::object_address<T0>(&arg0)).project_uri
    }
    
    public(friend) fun register_derive_supply_dispatch_function(arg0: &0x1::object::ConstructorRef, arg1: 0x1::option::Option<0x1::function_info::FunctionInfo>) {
        let v0 = &arg1;
        if (0x1::option::is_some<0x1::function_info::FunctionInfo>(v0)) {
            let v1 = 0x1::option::borrow<0x1::function_info::FunctionInfo>(v0);
            let v2 = 0x1::string::utf8(b"dispatchable_fungible_asset");
            let v3 = 0x1::string::utf8(b"dispatchable_derived_supply");
            let v4 = 0x1::function_info::new_function_info_from_address(@0x1, v2, v3);
            let v5 = 0x1::function_info::check_dispatch_type_compatibility(&v4, v1);
            assert!(v5, 0x1::error::invalid_argument(33));
        };
        assert!(0x1::object::address_from_constructor_ref(arg0) != @0xa, 0x1::error::permission_denied(31));
        if (0x1::object::can_generate_delete_ref(arg0)) {
            abort 0x1::error::invalid_argument(18)
        };
        let v6 = exists<Metadata>(0x1::object::address_from_constructor_ref(arg0));
        assert!(v6, 0x1::error::not_found(30));
        if (exists<DeriveSupply>(0x1::object::address_from_constructor_ref(arg0))) {
            abort 0x1::error::already_exists(29)
        };
        let v7 = 0x1::object::generate_signer(arg0);
        let v8 = DeriveSupply{dispatch_function: arg1};
        move_to<DeriveSupply>(&v7, v8);
    }
    
    public(friend) fun register_dispatch_functions(arg0: &0x1::object::ConstructorRef, arg1: 0x1::option::Option<0x1::function_info::FunctionInfo>, arg2: 0x1::option::Option<0x1::function_info::FunctionInfo>, arg3: 0x1::option::Option<0x1::function_info::FunctionInfo>) {
        let v0 = &arg1;
        if (0x1::option::is_some<0x1::function_info::FunctionInfo>(v0)) {
            let v1 = 0x1::function_info::new_function_info_from_address(@0x1, 0x1::string::utf8(b"dispatchable_fungible_asset"), 0x1::string::utf8(b"dispatchable_withdraw"));
            assert!(0x1::function_info::check_dispatch_type_compatibility(&v1, 0x1::option::borrow<0x1::function_info::FunctionInfo>(v0)), 0x1::error::invalid_argument(25));
        };
        let v2 = &arg2;
        if (0x1::option::is_some<0x1::function_info::FunctionInfo>(v2)) {
            let v3 = 0x1::function_info::new_function_info_from_address(@0x1, 0x1::string::utf8(b"dispatchable_fungible_asset"), 0x1::string::utf8(b"dispatchable_deposit"));
            assert!(0x1::function_info::check_dispatch_type_compatibility(&v3, 0x1::option::borrow<0x1::function_info::FunctionInfo>(v2)), 0x1::error::invalid_argument(26));
        };
        let v4 = &arg3;
        if (0x1::option::is_some<0x1::function_info::FunctionInfo>(v4)) {
            let v5 = 0x1::function_info::new_function_info_from_address(@0x1, 0x1::string::utf8(b"dispatchable_fungible_asset"), 0x1::string::utf8(b"dispatchable_derived_balance"));
            assert!(0x1::function_info::check_dispatch_type_compatibility(&v5, 0x1::option::borrow<0x1::function_info::FunctionInfo>(v4)), 0x1::error::invalid_argument(27));
        };
        assert!(0x1::object::address_from_constructor_ref(arg0) != @0xa, 0x1::error::permission_denied(31));
        if (0x1::object::can_generate_delete_ref(arg0)) {
            abort 0x1::error::invalid_argument(18)
        };
        assert!(exists<Metadata>(0x1::object::address_from_constructor_ref(arg0)), 0x1::error::not_found(30));
        if (exists<DispatchFunctionStore>(0x1::object::address_from_constructor_ref(arg0))) {
            abort 0x1::error::already_exists(29)
        };
        let v6 = 0x1::object::generate_signer(arg0);
        let v7 = DispatchFunctionStore{
            withdraw_function        : arg1, 
            deposit_function         : arg2, 
            derived_balance_function : arg3,
        };
        move_to<DispatchFunctionStore>(&v6, v7);
    }
    
    public fun remove_store(arg0: &0x1::object::DeleteRef) acquires ConcurrentFungibleBalance, FungibleAssetEvents, FungibleStore {
        let v0 = 0x1::object::object_from_delete_ref<FungibleStore>(arg0);
        let v1 = 0x1::object::object_address<FungibleStore>(&v0);
        let FungibleStore {
            metadata : _,
            balance  : v3,
            frozen   : _,
        } = move_from<FungibleStore>(v1);
        assert!(v3 == 0, 0x1::error::permission_denied(14));
        if (exists<ConcurrentFungibleBalance>(v1)) {
            let ConcurrentFungibleBalance { balance: v5 } = move_from<ConcurrentFungibleBalance>(v1);
            assert!(0x1::aggregator_v2::read<u64>(&v5) == 0, 0x1::error::permission_denied(14));
        };
        if (exists<FungibleAssetEvents>(v1)) {
            let FungibleAssetEvents {
                deposit_events  : v6,
                withdraw_events : v7,
                frozen_events   : v8,
            } = move_from<FungibleAssetEvents>(v1);
            0x1::event::destroy_handle<DepositEvent>(v6);
            0x1::event::destroy_handle<WithdrawEvent>(v7);
            0x1::event::destroy_handle<FrozenEvent>(v8);
        };
    }
    
    public fun set_frozen_flag<T0: key>(arg0: &TransferRef, arg1: 0x1::object::Object<T0>, arg2: bool) acquires FungibleStore {
        let v0 = store_metadata<T0>(arg1);
        assert!(arg0.metadata == v0, 0x1::error::invalid_argument(9));
        set_frozen_flag_internal<T0>(arg1, arg2);
    }
    
    public(friend) fun set_frozen_flag_internal<T0: key>(arg0: 0x1::object::Object<T0>, arg1: bool) acquires FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        borrow_global_mut<FungibleStore>(v0).frozen = arg1;
        let v1 = Frozen{
            store  : v0, 
            frozen : arg1,
        };
        0x1::event::emit<Frozen>(v1);
    }
    
    public fun store_exists(arg0: address) : bool {
        exists<FungibleStore>(arg0)
    }
    
    public fun store_metadata<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::object::Object<Metadata> acquires FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        assert!(exists<FungibleStore>(v0), 0x1::error::not_found(23));
        borrow_global<FungibleStore>(v0).metadata
    }
    
    public fun supply<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::option::Option<u128> acquires ConcurrentSupply, Supply {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        if (exists<ConcurrentSupply>(v0)) {
            0x1::option::some<u128>(0x1::aggregator_v2::read<u128>(&borrow_global<ConcurrentSupply>(v0).current))
        } else {
            if (exists<Supply>(v0)) {
                0x1::option::some<u128>(borrow_global<Supply>(v0).current)
            } else {
                0x1::option::none<u128>()
            }
        }
    }
    
    public fun symbol<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::string::String acquires Metadata {
        borrow_global<Metadata>(0x1::object::object_address<T0>(&arg0)).symbol
    }
    
    public entry fun transfer<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>, arg2: 0x1::object::Object<T0>, arg3: u64) acquires ConcurrentFungibleBalance, DispatchFunctionStore, FungibleStore {
        let v0 = withdraw<T0>(arg0, arg1, arg3);
        deposit<T0>(arg2, v0);
    }
    
    public fun transfer_ref_metadata(arg0: &TransferRef) : 0x1::object::Object<Metadata> {
        arg0.metadata
    }
    
    public fun transfer_with_ref<T0: key>(arg0: &TransferRef, arg1: 0x1::object::Object<T0>, arg2: 0x1::object::Object<T0>, arg3: u64) acquires ConcurrentFungibleBalance, FungibleStore {
        let v0 = withdraw_with_ref<T0>(arg0, arg1, arg3);
        deposit_with_ref<T0>(arg0, arg2, v0);
    }
    
    public entry fun upgrade_store_to_concurrent<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>) acquires FungibleStore {
        let v0 = 0x1::object::owns<T0>(arg1, 0x1::signer::address_of(arg0));
        assert!(v0, 0x1::error::permission_denied(8));
        if (is_frozen<T0>(arg1)) {
            abort 0x1::error::invalid_argument(3)
        };
        assert!(0x1::features::concurrent_fungible_balance_enabled(), 0x1::error::invalid_argument(32));
        ensure_store_upgraded_to_concurrent_internal(0x1::object::object_address<T0>(&arg1));
    }
    
    public fun upgrade_to_concurrent(arg0: &0x1::object::ExtendRef) acquires Supply {
        let v0 = 0x1::object::address_from_extend_ref(arg0);
        let v1 = 0x1::object::generate_signer_for_extending(arg0);
        assert!(0x1::features::concurrent_fungible_assets_enabled(), 0x1::error::invalid_argument(22));
        assert!(exists<Supply>(v0), 0x1::error::not_found(21));
        let Supply {
            current : v2,
            maximum : v3,
        } = move_from<Supply>(v0);
        let v4 = v3;
        let v5 = if (0x1::option::is_none<u128>(&v4)) {
            0x1::aggregator_v2::create_unbounded_aggregator_with_value<u128>(v2)
        } else {
            0x1::aggregator_v2::create_aggregator_with_value<u128>(v2, 0x1::option::extract<u128>(&mut v4))
        };
        let v6 = ConcurrentSupply{current: v5};
        move_to<ConcurrentSupply>(&v1, v6);
    }
    
    public fun withdraw<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>, arg2: u64) : FungibleAsset acquires ConcurrentFungibleBalance, DispatchFunctionStore, FungibleStore {
        withdraw_sanity_check<T0>(arg0, arg1, true);
        withdraw_internal(0x1::object::object_address<T0>(&arg1), arg2)
    }
    
    public fun withdraw_dispatch_function<T0: key>(arg0: 0x1::object::Object<T0>) : 0x1::option::Option<0x1::function_info::FunctionInfo> acquires DispatchFunctionStore, FungibleStore {
        let v0 = 0x1::object::object_address<T0>(&arg0);
        assert!(exists<FungibleStore>(v0), 0x1::error::not_found(23));
        let v1 = 0x1::object::object_address<Metadata>(&borrow_global<FungibleStore>(v0).metadata);
        if (exists<DispatchFunctionStore>(v1)) {
            borrow_global<DispatchFunctionStore>(v1).withdraw_function
        } else {
            0x1::option::none<0x1::function_info::FunctionInfo>()
        }
    }
    
    public(friend) fun withdraw_internal(arg0: address, arg1: u64) : FungibleAsset acquires ConcurrentFungibleBalance, FungibleStore {
        assert!(exists<FungibleStore>(arg0), 0x1::error::not_found(23));
        let v0 = borrow_global_mut<FungibleStore>(arg0);
        if (arg1 != 0) {
            if (v0.balance == 0 && exists<ConcurrentFungibleBalance>(arg0)) {
                let v1 = &mut borrow_global_mut<ConcurrentFungibleBalance>(arg0).balance;
                assert!(0x1::aggregator_v2::try_sub<u64>(v1, arg1), 0x1::error::invalid_argument(4));
            } else {
                assert!(v0.balance >= arg1, 0x1::error::invalid_argument(4));
                v0.balance = v0.balance - arg1;
            };
            let v2 = Withdraw{
                store  : arg0, 
                amount : arg1,
            };
            0x1::event::emit<Withdraw>(v2);
        };
        FungibleAsset{
            metadata : v0.metadata, 
            amount   : arg1,
        }
    }
    
    public(friend) fun withdraw_sanity_check<T0: key>(arg0: &signer, arg1: 0x1::object::Object<T0>, arg2: bool) acquires DispatchFunctionStore, FungibleStore {
        let v0 = 0x1::object::owns<T0>(arg1, 0x1::signer::address_of(arg0));
        assert!(v0, 0x1::error::permission_denied(8));
        let v1 = 0x1::object::object_address<T0>(&arg1);
        assert!(exists<FungibleStore>(v1), 0x1::error::not_found(23));
        let v2 = borrow_global<FungibleStore>(v1);
        if (arg2) {
            let v3 = has_withdraw_dispatch_function(v2.metadata);
            arg2 = !v3;
        } else {
            arg2 = true;
        };
        assert!(arg2, 0x1::error::invalid_argument(28));
        if (v2.frozen) {
            abort 0x1::error::permission_denied(3)
        };
    }
    
    public fun withdraw_with_ref<T0: key>(arg0: &TransferRef, arg1: 0x1::object::Object<T0>, arg2: u64) : FungibleAsset acquires ConcurrentFungibleBalance, FungibleStore {
        let v0 = store_metadata<T0>(arg1);
        assert!(arg0.metadata == v0, 0x1::error::invalid_argument(9));
        withdraw_internal(0x1::object::object_address<T0>(&arg1), arg2)
    }
    
    public fun zero<T0: key>(arg0: 0x1::object::Object<T0>) : FungibleAsset {
        FungibleAsset{
            metadata : 0x1::object::convert<T0, Metadata>(arg0), 
            amount   : 0,
        }
    }
    
    // decompiled from Move bytecode v7
}
