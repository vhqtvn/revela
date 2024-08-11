module 0x1::multisig_account {
    struct AddOwners has drop, store {
        multisig_account: address,
        owners_added: vector<address>,
    }
    
    struct AddOwnersEvent has drop, store {
        owners_added: vector<address>,
    }
    
    struct CreateTransaction has drop, store {
        multisig_account: address,
        creator: address,
        sequence_number: u64,
        transaction: MultisigTransaction,
    }
    
    struct CreateTransactionEvent has drop, store {
        creator: address,
        sequence_number: u64,
        transaction: MultisigTransaction,
    }
    
    struct ExecuteRejectedTransaction has drop, store {
        multisig_account: address,
        sequence_number: u64,
        num_rejections: u64,
        executor: address,
    }
    
    struct ExecuteRejectedTransactionEvent has drop, store {
        sequence_number: u64,
        num_rejections: u64,
        executor: address,
    }
    
    struct ExecutionError has copy, drop, store {
        abort_location: 0x1::string::String,
        error_type: 0x1::string::String,
        error_code: u64,
    }
    
    struct MetadataUpdated has drop, store {
        multisig_account: address,
        old_metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
        new_metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
    }
    
    struct MetadataUpdatedEvent has drop, store {
        old_metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
        new_metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
    }
    
    struct MultisigAccount has key {
        owners: vector<address>,
        num_signatures_required: u64,
        transactions: 0x1::table::Table<u64, MultisigTransaction>,
        last_executed_sequence_number: u64,
        next_sequence_number: u64,
        signer_cap: 0x1::option::Option<0x1::account::SignerCapability>,
        metadata: 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>>,
        add_owners_events: 0x1::event::EventHandle<AddOwnersEvent>,
        remove_owners_events: 0x1::event::EventHandle<RemoveOwnersEvent>,
        update_signature_required_events: 0x1::event::EventHandle<UpdateSignaturesRequiredEvent>,
        create_transaction_events: 0x1::event::EventHandle<CreateTransactionEvent>,
        vote_events: 0x1::event::EventHandle<VoteEvent>,
        execute_rejected_transaction_events: 0x1::event::EventHandle<ExecuteRejectedTransactionEvent>,
        execute_transaction_events: 0x1::event::EventHandle<TransactionExecutionSucceededEvent>,
        transaction_execution_failed_events: 0x1::event::EventHandle<TransactionExecutionFailedEvent>,
        metadata_updated_events: 0x1::event::EventHandle<MetadataUpdatedEvent>,
    }
    
    struct MultisigAccountCreationMessage has copy, drop {
        chain_id: u8,
        account_address: address,
        sequence_number: u64,
        owners: vector<address>,
        num_signatures_required: u64,
    }
    
    struct MultisigAccountCreationWithAuthKeyRevocationMessage has copy, drop {
        chain_id: u8,
        account_address: address,
        sequence_number: u64,
        owners: vector<address>,
        num_signatures_required: u64,
    }
    
    struct MultisigTransaction has copy, drop, store {
        payload: 0x1::option::Option<vector<u8>>,
        payload_hash: 0x1::option::Option<vector<u8>>,
        votes: 0x1::simple_map::SimpleMap<address, bool>,
        creator: address,
        creation_time_secs: u64,
    }
    
    struct RemoveOwners has drop, store {
        multisig_account: address,
        owners_removed: vector<address>,
    }
    
    struct RemoveOwnersEvent has drop, store {
        owners_removed: vector<address>,
    }
    
    struct TransactionExecutionFailed has drop, store {
        multisig_account: address,
        executor: address,
        sequence_number: u64,
        transaction_payload: vector<u8>,
        num_approvals: u64,
        execution_error: ExecutionError,
    }
    
    struct TransactionExecutionFailedEvent has drop, store {
        executor: address,
        sequence_number: u64,
        transaction_payload: vector<u8>,
        num_approvals: u64,
        execution_error: ExecutionError,
    }
    
    struct TransactionExecutionSucceeded has drop, store {
        multisig_account: address,
        executor: address,
        sequence_number: u64,
        transaction_payload: vector<u8>,
        num_approvals: u64,
    }
    
    struct TransactionExecutionSucceededEvent has drop, store {
        executor: address,
        sequence_number: u64,
        transaction_payload: vector<u8>,
        num_approvals: u64,
    }
    
    struct UpdateSignaturesRequired has drop, store {
        multisig_account: address,
        old_num_signatures_required: u64,
        new_num_signatures_required: u64,
    }
    
    struct UpdateSignaturesRequiredEvent has drop, store {
        old_num_signatures_required: u64,
        new_num_signatures_required: u64,
    }
    
    struct Vote has drop, store {
        multisig_account: address,
        owner: address,
        sequence_number: u64,
        approved: bool,
    }
    
    struct VoteEvent has drop, store {
        owner: address,
        sequence_number: u64,
        approved: bool,
    }
    
    public entry fun create(arg0: &signer, arg1: u64, arg2: vector<0x1::string::String>, arg3: vector<vector<u8>>) acquires MultisigAccount {
        create_with_owners(arg0, vector[], arg1, arg2, arg3);
    }
    
    entry fun add_owner(arg0: &signer, arg1: address) acquires MultisigAccount {
        let v0 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v0, arg1);
        add_owners(arg0, v0);
    }
    
    entry fun add_owners(arg0: &signer, arg1: vector<address>) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), arg1, vector[], 0x1::option::none<u64>());
    }
    
    entry fun add_owners_and_update_signatures_required(arg0: &signer, arg1: vector<address>, arg2: u64) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), arg1, vector[], 0x1::option::some<u64>(arg2));
    }
    
    public entry fun approve_transaction(arg0: &signer, arg1: address, arg2: u64) acquires MultisigAccount {
        vote_transanction(arg0, arg1, arg2, true);
    }
    
    public fun available_transaction_queue_capacity(arg0: address) : u64 acquires MultisigAccount {
        let v0 = borrow_global_mut<MultisigAccount>(arg0);
        let v1 = v0.next_sequence_number - v0.last_executed_sequence_number - 1;
        if (v1 > 20) {
            0
        } else {
            20 - v1
        }
    }
    
    public fun can_be_executed(arg0: address, arg1: u64) : bool acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg0);
        assert!(arg1 > 0 && arg1 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        let v1 = borrow_global<MultisigAccount>(arg0);
        let v2 = &v1.owners;
        let v3 = 0;
        let v4 = 0;
        let v5 = &0x1::table::borrow<u64, MultisigTransaction>(&v1.transactions, arg1).votes;
        let v6 = 0;
        while (v6 < 0x1::vector::length<address>(v2)) {
            let v7 = 0x1::vector::borrow<address>(v2, v6);
            if (0x1::simple_map::contains_key<address, bool>(v5, v7)) {
                if (*0x1::simple_map::borrow<address, bool>(v5, v7)) {
                    v3 = v3 + 1;
                } else {
                    v4 = v4 + 1;
                };
            };
            v6 = v6 + 1;
        };
        let v8 = last_resolved_sequence_number(arg0);
        if (arg1 == v8 + 1) {
            let v10 = num_signatures_required(arg0);
            v3 >= v10
        } else {
            false
        }
    }
    
    public fun can_be_rejected(arg0: address, arg1: u64) : bool acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg0);
        assert!(arg1 > 0 && arg1 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        let v1 = borrow_global<MultisigAccount>(arg0);
        let v2 = &v1.owners;
        let v3 = 0;
        let v4 = 0;
        let v5 = &0x1::table::borrow<u64, MultisigTransaction>(&v1.transactions, arg1).votes;
        let v6 = 0;
        while (v6 < 0x1::vector::length<address>(v2)) {
            let v7 = 0x1::vector::borrow<address>(v2, v6);
            if (0x1::simple_map::contains_key<address, bool>(v5, v7)) {
                if (*0x1::simple_map::borrow<address, bool>(v5, v7)) {
                    v3 = v3 + 1;
                } else {
                    v4 = v4 + 1;
                };
            };
            v6 = v6 + 1;
        };
        let v8 = last_resolved_sequence_number(arg0);
        if (arg1 == v8 + 1) {
            let v10 = num_signatures_required(arg0);
            v4 >= v10
        } else {
            false
        }
    }
    
    public fun can_execute(arg0: address, arg1: address, arg2: u64) : bool acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg1);
        assert!(arg2 > 0 && arg2 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        let v1 = borrow_global<MultisigAccount>(arg1);
        let v2 = &v1.owners;
        let v3 = 0;
        let v4 = 0;
        let v5 = &0x1::table::borrow<u64, MultisigTransaction>(&v1.transactions, arg2).votes;
        let v6 = 0;
        while (v6 < 0x1::vector::length<address>(v2)) {
            let v7 = 0x1::vector::borrow<address>(v2, v6);
            if (0x1::simple_map::contains_key<address, bool>(v5, v7)) {
                if (*0x1::simple_map::borrow<address, bool>(v5, v7)) {
                    v3 = v3 + 1;
                } else {
                    v4 = v4 + 1;
                };
            };
            v6 = v6 + 1;
        };
        let v8 = v3;
        let (v9, v10) = vote(arg1, arg2, arg0);
        if (!(v9 && v10)) {
            v8 = v3 + 1;
        };
        let v11 = if (is_owner(arg0, arg1)) {
            let v12 = last_resolved_sequence_number(arg1);
            arg2 == v12 + 1
        } else {
            false
        };
        if (v11) {
            let v14 = num_signatures_required(arg1);
            v8 >= v14
        } else {
            false
        }
    }
    
    public fun can_reject(arg0: address, arg1: address, arg2: u64) : bool acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg1);
        assert!(arg2 > 0 && arg2 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        let v1 = borrow_global<MultisigAccount>(arg1);
        let v2 = &v1.owners;
        let v3 = 0;
        let v4 = 0;
        let v5 = &0x1::table::borrow<u64, MultisigTransaction>(&v1.transactions, arg2).votes;
        let v6 = 0;
        while (v6 < 0x1::vector::length<address>(v2)) {
            let v7 = 0x1::vector::borrow<address>(v2, v6);
            if (0x1::simple_map::contains_key<address, bool>(v5, v7)) {
                if (*0x1::simple_map::borrow<address, bool>(v5, v7)) {
                    v3 = v3 + 1;
                } else {
                    v4 = v4 + 1;
                };
            };
            v6 = v6 + 1;
        };
        let v8 = v4;
        let (v9, v10) = vote(arg1, arg2, arg0);
        let v11 = v9 && !v10;
        if (!v11) {
            v8 = v4 + 1;
        };
        let v12 = if (is_owner(arg0, arg1)) {
            let v13 = last_resolved_sequence_number(arg1);
            arg2 == v13 + 1
        } else {
            false
        };
        if (v12) {
            let v15 = num_signatures_required(arg1);
            v8 >= v15
        } else {
            false
        }
    }
    
    fun create_multisig_account(arg0: &signer) : (signer, 0x1::account::SignerCapability) {
        let v0 = 0x1::account::get_sequence_number(0x1::signer::address_of(arg0));
        let v1 = create_multisig_account_seed(0x1::bcs::to_bytes<u64>(&v0));
        let (v2, v3) = 0x1::account::create_resource_account(arg0, v1);
        let v4 = v2;
        if (!0x1::coin::is_account_registered<0x1::aptos_coin::AptosCoin>(0x1::signer::address_of(&v4))) {
            0x1::coin::register<0x1::aptos_coin::AptosCoin>(&v4);
        };
        (v4, v3)
    }
    
    fun create_multisig_account_seed(arg0: vector<u8>) : vector<u8> {
        let v0 = 0x1::vector::empty<u8>();
        0x1::vector::append<u8>(&mut v0, b"aptos_framework::multisig_account");
        0x1::vector::append<u8>(&mut v0, arg0);
        v0
    }
    
    public entry fun create_transaction(arg0: &signer, arg1: address, arg2: vector<u8>) acquires MultisigAccount {
        assert!(0x1::vector::length<u8>(&arg2) > 0, 0x1::error::invalid_argument(4));
        assert!(exists<MultisigAccount>(arg1), 0x1::error::invalid_state(2002));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = 0x1::vector::contains<address>(&borrow_global<MultisigAccount>(arg1).owners, &v0);
        assert!(v1, 0x1::error::permission_denied(2003));
        let v2 = 0x1::signer::address_of(arg0);
        let v3 = 0x1::option::some<vector<u8>>(arg2);
        let v4 = 0x1::option::none<vector<u8>>();
        let v5 = 0x1::simple_map::create<address, bool>();
        let v6 = 0x1::timestamp::now_seconds();
        let v7 = MultisigTransaction{
            payload            : v3, 
            payload_hash       : v4, 
            votes              : v5, 
            creator            : v2, 
            creation_time_secs : v6,
        };
        if (0x1::features::multisig_v2_enhancement_feature_enabled()) {
            let v8 = available_transaction_queue_capacity(arg1);
            assert!(v8 > 0, 0x1::error::invalid_state(19));
        };
        let v9 = borrow_global_mut<MultisigAccount>(arg1);
        0x1::simple_map::add<address, bool>(&mut v7.votes, v2, true);
        let v10 = v9.next_sequence_number;
        v9.next_sequence_number = v10 + 1;
        0x1::table::add<u64, MultisigTransaction>(&mut v9.transactions, v10, v7);
        if (0x1::features::module_event_migration_enabled()) {
            let v11 = CreateTransaction{
                multisig_account : arg1, 
                creator          : v2, 
                sequence_number  : v10, 
                transaction      : v7,
            };
            0x1::event::emit<CreateTransaction>(v11);
        };
        let v12 = CreateTransactionEvent{
            creator         : v2, 
            sequence_number : v10, 
            transaction     : v7,
        };
        0x1::event::emit_event<CreateTransactionEvent>(&mut v9.create_transaction_events, v12);
    }
    
    public entry fun create_transaction_with_hash(arg0: &signer, arg1: address, arg2: vector<u8>) acquires MultisigAccount {
        assert!(0x1::vector::length<u8>(&arg2) == 32, 0x1::error::invalid_argument(12));
        assert!(exists<MultisigAccount>(arg1), 0x1::error::invalid_state(2002));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = 0x1::vector::contains<address>(&borrow_global<MultisigAccount>(arg1).owners, &v0);
        assert!(v1, 0x1::error::permission_denied(2003));
        let v2 = 0x1::signer::address_of(arg0);
        let v3 = 0x1::option::none<vector<u8>>();
        let v4 = 0x1::option::some<vector<u8>>(arg2);
        let v5 = 0x1::simple_map::create<address, bool>();
        let v6 = 0x1::timestamp::now_seconds();
        let v7 = MultisigTransaction{
            payload            : v3, 
            payload_hash       : v4, 
            votes              : v5, 
            creator            : v2, 
            creation_time_secs : v6,
        };
        if (0x1::features::multisig_v2_enhancement_feature_enabled()) {
            let v8 = available_transaction_queue_capacity(arg1);
            assert!(v8 > 0, 0x1::error::invalid_state(19));
        };
        let v9 = borrow_global_mut<MultisigAccount>(arg1);
        0x1::simple_map::add<address, bool>(&mut v7.votes, v2, true);
        let v10 = v9.next_sequence_number;
        v9.next_sequence_number = v10 + 1;
        0x1::table::add<u64, MultisigTransaction>(&mut v9.transactions, v10, v7);
        if (0x1::features::module_event_migration_enabled()) {
            let v11 = CreateTransaction{
                multisig_account : arg1, 
                creator          : v2, 
                sequence_number  : v10, 
                transaction      : v7,
            };
            0x1::event::emit<CreateTransaction>(v11);
        };
        let v12 = CreateTransactionEvent{
            creator         : v2, 
            sequence_number : v10, 
            transaction     : v7,
        };
        0x1::event::emit_event<CreateTransactionEvent>(&mut v9.create_transaction_events, v12);
    }
    
    public entry fun create_with_existing_account(arg0: address, arg1: vector<address>, arg2: u64, arg3: u8, arg4: vector<u8>, arg5: vector<u8>, arg6: vector<0x1::string::String>, arg7: vector<vector<u8>>) acquires MultisigAccount {
        let v0 = 0x1::chain_id::get();
        let v1 = 0x1::account::get_sequence_number(arg0);
        let v2 = MultisigAccountCreationMessage{
            chain_id                : v0, 
            account_address         : arg0, 
            sequence_number         : v1, 
            owners                  : arg1, 
            num_signatures_required : arg2,
        };
        0x1::account::verify_signed_message<MultisigAccountCreationMessage>(arg0, arg3, arg4, arg5, v2);
        let v3 = 0x1::create_signer::create_signer(arg0);
        create_with_owners_internal(&v3, arg1, arg2, 0x1::option::none<0x1::account::SignerCapability>(), arg6, arg7);
    }
    
    public entry fun create_with_existing_account_and_revoke_auth_key(arg0: address, arg1: vector<address>, arg2: u64, arg3: u8, arg4: vector<u8>, arg5: vector<u8>, arg6: vector<0x1::string::String>, arg7: vector<vector<u8>>) acquires MultisigAccount {
        let v0 = 0x1::chain_id::get();
        let v1 = 0x1::account::get_sequence_number(arg0);
        let v2 = MultisigAccountCreationWithAuthKeyRevocationMessage{
            chain_id                : v0, 
            account_address         : arg0, 
            sequence_number         : v1, 
            owners                  : arg1, 
            num_signatures_required : arg2,
        };
        0x1::account::verify_signed_message<MultisigAccountCreationWithAuthKeyRevocationMessage>(arg0, arg3, arg4, arg5, v2);
        let v3 = 0x1::create_signer::create_signer(arg0);
        let v4 = &v3;
        create_with_owners_internal(v4, arg1, arg2, 0x1::option::none<0x1::account::SignerCapability>(), arg6, arg7);
        let v5 = 0x1::signer::address_of(v4);
        0x1::account::rotate_authentication_key_internal(v4, x"0000000000000000000000000000000000000000000000000000000000000000");
        if (0x1::account::is_signer_capability_offered(v5)) {
            0x1::account::revoke_any_signer_capability(v4);
        };
        if (0x1::account::is_rotation_capability_offered(v5)) {
            0x1::account::revoke_any_rotation_capability(v4);
        };
    }
    
    public entry fun create_with_owners(arg0: &signer, arg1: vector<address>, arg2: u64, arg3: vector<0x1::string::String>, arg4: vector<vector<u8>>) acquires MultisigAccount {
        let (v0, v1) = create_multisig_account(arg0);
        let v2 = v0;
        0x1::vector::push_back<address>(&mut arg1, 0x1::signer::address_of(arg0));
        let v3 = 0x1::option::some<0x1::account::SignerCapability>(v1);
        create_with_owners_internal(&v2, arg1, arg2, v3, arg3, arg4);
    }
    
    fun create_with_owners_internal(arg0: &signer, arg1: vector<address>, arg2: u64, arg3: 0x1::option::Option<0x1::account::SignerCapability>, arg4: vector<0x1::string::String>, arg5: vector<vector<u8>>) acquires MultisigAccount {
        assert!(0x1::features::multisig_accounts_enabled(), 0x1::error::unavailable(14));
        assert!(arg2 > 0 && arg2 <= 0x1::vector::length<address>(&arg1), 0x1::error::invalid_argument(11));
        validate_owners(&arg1, 0x1::signer::address_of(arg0));
        let v0 = arg1;
        let v1 = 0x1::table::new<u64, MultisigTransaction>();
        let v2 = 0x1::simple_map::create<0x1::string::String, vector<u8>>();
        let v3 = 0x1::account::new_event_handle<AddOwnersEvent>(arg0);
        let v4 = 0x1::account::new_event_handle<RemoveOwnersEvent>(arg0);
        let v5 = 0x1::account::new_event_handle<UpdateSignaturesRequiredEvent>(arg0);
        let v6 = 0x1::account::new_event_handle<CreateTransactionEvent>(arg0);
        let v7 = 0x1::account::new_event_handle<VoteEvent>(arg0);
        let v8 = 0x1::account::new_event_handle<ExecuteRejectedTransactionEvent>(arg0);
        let v9 = 0x1::account::new_event_handle<TransactionExecutionSucceededEvent>(arg0);
        let v10 = 0x1::account::new_event_handle<TransactionExecutionFailedEvent>(arg0);
        let v11 = 0x1::account::new_event_handle<MetadataUpdatedEvent>(arg0);
        let v12 = MultisigAccount{
            owners                              : v0, 
            num_signatures_required             : arg2, 
            transactions                        : v1, 
            last_executed_sequence_number       : 0, 
            next_sequence_number                : 1, 
            signer_cap                          : arg3, 
            metadata                            : v2, 
            add_owners_events                   : v3, 
            remove_owners_events                : v4, 
            update_signature_required_events    : v5, 
            create_transaction_events           : v6, 
            vote_events                         : v7, 
            execute_rejected_transaction_events : v8, 
            execute_transaction_events          : v9, 
            transaction_execution_failed_events : v10, 
            metadata_updated_events             : v11,
        };
        move_to<MultisigAccount>(arg0, v12);
        update_metadata_internal(arg0, arg4, arg5, false);
    }
    
    public entry fun create_with_owners_then_remove_bootstrapper(arg0: &signer, arg1: vector<address>, arg2: u64, arg3: vector<0x1::string::String>, arg4: vector<vector<u8>>) acquires MultisigAccount {
        let v0 = 0x1::signer::address_of(arg0);
        create_with_owners(arg0, arg1, arg2, arg3, arg4);
        let v1 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v1, v0);
        update_owner_schema(get_next_multisig_account_address(v0), vector[], v1, 0x1::option::none<u64>());
    }
    
    public entry fun execute_rejected_transaction(arg0: &signer, arg1: address) acquires MultisigAccount {
        assert!(exists<MultisigAccount>(arg1), 0x1::error::invalid_state(2002));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = 0x1::vector::contains<address>(&borrow_global<MultisigAccount>(arg1).owners, &v0);
        assert!(v1, 0x1::error::permission_denied(2003));
        let v2 = last_resolved_sequence_number(arg1);
        let v3 = v2 + 1;
        let v4 = 0x1::signer::address_of(arg0);
        if (0x1::features::multisig_v2_enhancement_feature_enabled()) {
            let (v5, v6) = vote(arg1, v3, v4);
            let v7 = v5 && !v6;
            if (!v7) {
                reject_transaction(arg0, arg1, v3);
            };
        };
        let v8 = borrow_global_mut<MultisigAccount>(arg1);
        let (_, v10) = remove_executed_transaction(v8);
        assert!(v10 >= v8.num_signatures_required, 0x1::error::invalid_state(10));
        if (0x1::features::module_event_migration_enabled()) {
            let v11 = 0x1::signer::address_of(arg0);
            let v12 = ExecuteRejectedTransaction{
                multisig_account : arg1, 
                sequence_number  : v3, 
                num_rejections   : v10, 
                executor         : v11,
            };
            0x1::event::emit<ExecuteRejectedTransaction>(v12);
        };
        let v13 = ExecuteRejectedTransactionEvent{
            sequence_number : v3, 
            num_rejections  : v10, 
            executor        : v4,
        };
        0x1::event::emit_event<ExecuteRejectedTransactionEvent>(&mut v8.execute_rejected_transaction_events, v13);
    }
    
    public entry fun execute_rejected_transactions(arg0: &signer, arg1: address, arg2: u64) acquires MultisigAccount {
        assert!(0x1::features::multisig_v2_enhancement_feature_enabled(), 0x1::error::invalid_state(20));
        let v0 = last_resolved_sequence_number(arg1);
        assert!(v0 < arg2, 0x1::error::invalid_argument(17));
        let v1 = next_sequence_number(arg1);
        assert!(arg2 < v1, 0x1::error::invalid_argument(17));
        let v2 = last_resolved_sequence_number(arg1);
        while (v2 < arg2) {
            execute_rejected_transaction(arg0, arg1);
        };
    }
    
    fun failed_transaction_execution_cleanup(arg0: address, arg1: address, arg2: vector<u8>, arg3: ExecutionError) acquires MultisigAccount {
        let v0 = last_resolved_sequence_number(arg1);
        let v1 = v0 + 1;
        let (v2, v3) = vote(arg1, v1, arg0);
        let v4 = borrow_global_mut<MultisigAccount>(arg1);
        let (v5, _) = remove_executed_transaction(v4);
        let v7 = v5;
        if (0x1::features::multisig_v2_enhancement_feature_enabled() && !(v2 && v3)) {
            if (0x1::features::module_event_migration_enabled()) {
                let v8 = Vote{
                    multisig_account : arg1, 
                    owner            : arg0, 
                    sequence_number  : v1, 
                    approved         : true,
                };
                0x1::event::emit<Vote>(v8);
            };
            v7 = v5 + 1;
            let v9 = VoteEvent{
                owner           : arg0, 
                sequence_number : v1, 
                approved        : true,
            };
            0x1::event::emit_event<VoteEvent>(&mut v4.vote_events, v9);
        };
        let v10 = borrow_global_mut<MultisigAccount>(arg1);
        if (0x1::features::module_event_migration_enabled()) {
            let v11 = v10.last_executed_sequence_number;
            let v12 = TransactionExecutionFailed{
                multisig_account    : arg1, 
                executor            : arg0, 
                sequence_number     : v11, 
                transaction_payload : arg2, 
                num_approvals       : v7, 
                execution_error     : arg3,
            };
            0x1::event::emit<TransactionExecutionFailed>(v12);
        };
        let v13 = v10.last_executed_sequence_number;
        let v14 = TransactionExecutionFailedEvent{
            executor            : arg0, 
            sequence_number     : v13, 
            transaction_payload : arg2, 
            num_approvals       : v7, 
            execution_error     : arg3,
        };
        0x1::event::emit_event<TransactionExecutionFailedEvent>(&mut v10.transaction_execution_failed_events, v14);
    }
    
    public fun get_next_multisig_account_address(arg0: address) : address {
        let v0 = 0x1::account::get_sequence_number(arg0);
        let v1 = create_multisig_account_seed(0x1::bcs::to_bytes<u64>(&v0));
        0x1::account::create_resource_address(&arg0, v1)
    }
    
    public fun get_next_transaction_payload(arg0: address, arg1: vector<u8>) : vector<u8> acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg0);
        let v1 = 0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, v0.last_executed_sequence_number + 1);
        if (0x1::option::is_some<vector<u8>>(&v1.payload)) {
            *0x1::option::borrow<vector<u8>>(&v1.payload)
        } else {
            arg1
        }
    }
    
    public fun get_pending_transactions(arg0: address) : vector<MultisigTransaction> acquires MultisigAccount {
        let v0 = 0x1::vector::empty<MultisigTransaction>();
        let v1 = borrow_global<MultisigAccount>(arg0);
        let v2 = v1.last_executed_sequence_number + 1;
        while (v2 < v1.next_sequence_number) {
            let v3 = *0x1::table::borrow<u64, MultisigTransaction>(&v1.transactions, v2);
            0x1::vector::push_back<MultisigTransaction>(&mut v0, v3);
            v2 = v2 + 1;
        };
        v0
    }
    
    public fun get_transaction(arg0: address, arg1: u64) : MultisigTransaction acquires MultisigAccount {
        let v0 = borrow_global<MultisigAccount>(arg0);
        assert!(arg1 > 0 && arg1 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        *0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, arg1)
    }
    
    public fun is_owner(arg0: address, arg1: address) : bool acquires MultisigAccount {
        0x1::vector::contains<address>(&borrow_global<MultisigAccount>(arg1).owners, &arg0)
    }
    
    public fun last_resolved_sequence_number(arg0: address) : u64 acquires MultisigAccount {
        borrow_global_mut<MultisigAccount>(arg0).last_executed_sequence_number
    }
    
    public fun metadata(arg0: address) : 0x1::simple_map::SimpleMap<0x1::string::String, vector<u8>> acquires MultisigAccount {
        borrow_global<MultisigAccount>(arg0).metadata
    }
    
    public fun next_sequence_number(arg0: address) : u64 acquires MultisigAccount {
        borrow_global_mut<MultisigAccount>(arg0).next_sequence_number
    }
    
    public fun num_signatures_required(arg0: address) : u64 acquires MultisigAccount {
        borrow_global<MultisigAccount>(arg0).num_signatures_required
    }
    
    public fun owners(arg0: address) : vector<address> acquires MultisigAccount {
        borrow_global<MultisigAccount>(arg0).owners
    }
    
    public entry fun reject_transaction(arg0: &signer, arg1: address, arg2: u64) acquires MultisigAccount {
        vote_transanction(arg0, arg1, arg2, false);
    }
    
    fun remove_executed_transaction(arg0: &mut MultisigAccount) : (u64, u64) {
        let v0 = arg0.last_executed_sequence_number + 1;
        let v1 = 0x1::table::remove<u64, MultisigTransaction>(&mut arg0.transactions, v0);
        arg0.last_executed_sequence_number = v0;
        let v2 = &arg0.owners;
        let v3 = 0;
        let v4 = 0;
        let v5 = &v1.votes;
        let v6 = 0;
        while (v6 < 0x1::vector::length<address>(v2)) {
            let v7 = 0x1::vector::borrow<address>(v2, v6);
            if (0x1::simple_map::contains_key<address, bool>(v5, v7)) {
                if (*0x1::simple_map::borrow<address, bool>(v5, v7)) {
                    v3 = v3 + 1;
                } else {
                    v4 = v4 + 1;
                };
            };
            v6 = v6 + 1;
        };
        (v3, v4)
    }
    
    entry fun remove_owner(arg0: &signer, arg1: address) acquires MultisigAccount {
        let v0 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v0, arg1);
        remove_owners(arg0, v0);
    }
    
    entry fun remove_owners(arg0: &signer, arg1: vector<address>) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), vector[], arg1, 0x1::option::none<u64>());
    }
    
    fun successful_transaction_execution_cleanup(arg0: address, arg1: address, arg2: vector<u8>) acquires MultisigAccount {
        let v0 = last_resolved_sequence_number(arg1);
        let v1 = v0 + 1;
        let (v2, v3) = vote(arg1, v1, arg0);
        let v4 = borrow_global_mut<MultisigAccount>(arg1);
        let (v5, _) = remove_executed_transaction(v4);
        let v7 = v5;
        if (0x1::features::multisig_v2_enhancement_feature_enabled() && !(v2 && v3)) {
            if (0x1::features::module_event_migration_enabled()) {
                let v8 = Vote{
                    multisig_account : arg1, 
                    owner            : arg0, 
                    sequence_number  : v1, 
                    approved         : true,
                };
                0x1::event::emit<Vote>(v8);
            };
            v7 = v5 + 1;
            let v9 = VoteEvent{
                owner           : arg0, 
                sequence_number : v1, 
                approved        : true,
            };
            0x1::event::emit_event<VoteEvent>(&mut v4.vote_events, v9);
        };
        let v10 = borrow_global_mut<MultisigAccount>(arg1);
        if (0x1::features::module_event_migration_enabled()) {
            let v11 = v10.last_executed_sequence_number;
            let v12 = TransactionExecutionSucceeded{
                multisig_account    : arg1, 
                executor            : arg0, 
                sequence_number     : v11, 
                transaction_payload : arg2, 
                num_approvals       : v7,
            };
            0x1::event::emit<TransactionExecutionSucceeded>(v12);
        };
        let v13 = v10.last_executed_sequence_number;
        let v14 = TransactionExecutionSucceededEvent{
            executor            : arg0, 
            sequence_number     : v13, 
            transaction_payload : arg2, 
            num_approvals       : v7,
        };
        0x1::event::emit_event<TransactionExecutionSucceededEvent>(&mut v10.execute_transaction_events, v14);
    }
    
    entry fun swap_owner(arg0: &signer, arg1: address, arg2: address) acquires MultisigAccount {
        let v0 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v0, arg1);
        let v1 = 0x1::vector::empty<address>();
        0x1::vector::push_back<address>(&mut v1, arg2);
        update_owner_schema(0x1::signer::address_of(arg0), v0, v1, 0x1::option::none<u64>());
    }
    
    entry fun swap_owners(arg0: &signer, arg1: vector<address>, arg2: vector<address>) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), arg1, arg2, 0x1::option::none<u64>());
    }
    
    entry fun swap_owners_and_update_signatures_required(arg0: &signer, arg1: vector<address>, arg2: vector<address>, arg3: u64) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), arg1, arg2, 0x1::option::some<u64>(arg3));
    }
    
    entry fun update_metadata(arg0: &signer, arg1: vector<0x1::string::String>, arg2: vector<vector<u8>>) acquires MultisigAccount {
        update_metadata_internal(arg0, arg1, arg2, true);
    }
    
    fun update_metadata_internal(arg0: &signer, arg1: vector<0x1::string::String>, arg2: vector<vector<u8>>, arg3: bool) acquires MultisigAccount {
        let v0 = 0x1::vector::length<0x1::string::String>(&arg1);
        assert!(v0 == 0x1::vector::length<vector<u8>>(&arg2), 0x1::error::invalid_argument(15));
        let v1 = 0x1::signer::address_of(arg0);
        assert!(exists<MultisigAccount>(v1), 0x1::error::invalid_state(2002));
        let v2 = borrow_global_mut<MultisigAccount>(v1);
        let v3 = v2.metadata;
        v2.metadata = 0x1::simple_map::create<0x1::string::String, vector<u8>>();
        let v4 = &mut v2.metadata;
        let v5 = 0;
        while (v5 < v0) {
            let v6 = *0x1::vector::borrow<0x1::string::String>(&arg1, v5);
            let v7 = *0x1::vector::borrow<vector<u8>>(&arg2, v5);
            let v8 = !0x1::simple_map::contains_key<0x1::string::String, vector<u8>>(v4, &v6);
            assert!(v8, 0x1::error::invalid_argument(16));
            0x1::simple_map::add<0x1::string::String, vector<u8>>(v4, v6, v7);
            v5 = v5 + 1;
        };
        if (arg3) {
            if (0x1::features::module_event_migration_enabled()) {
                let v9 = MetadataUpdated{
                    multisig_account : v1, 
                    old_metadata     : v3, 
                    new_metadata     : v2.metadata,
                };
                0x1::event::emit<MetadataUpdated>(v9);
            };
            let v10 = MetadataUpdatedEvent{
                old_metadata : v3, 
                new_metadata : v2.metadata,
            };
            0x1::event::emit_event<MetadataUpdatedEvent>(&mut v2.metadata_updated_events, v10);
        };
    }
    
    fun update_owner_schema(arg0: address, arg1: vector<address>, arg2: vector<address>, arg3: 0x1::option::Option<u64>) acquires MultisigAccount {
        assert!(exists<MultisigAccount>(arg0), 0x1::error::invalid_state(2002));
        let v0 = borrow_global_mut<MultisigAccount>(arg0);
        let v1 = &arg1;
        let v2 = 0;
        while (v2 < 0x1::vector::length<address>(v1)) {
            let v3 = !0x1::vector::contains<address>(&arg2, 0x1::vector::borrow<address>(v1, v2));
            assert!(v3, 0x1::error::invalid_argument(18));
            v2 = v2 + 1;
        };
        if (0x1::vector::length<address>(&arg1) > 0) {
            0x1::vector::append<address>(&mut v0.owners, arg1);
            validate_owners(&v0.owners, arg0);
            if (0x1::features::module_event_migration_enabled()) {
                let v4 = AddOwners{
                    multisig_account : arg0, 
                    owners_added     : arg1,
                };
                0x1::event::emit<AddOwners>(v4);
            };
            let v5 = AddOwnersEvent{owners_added: arg1};
            0x1::event::emit_event<AddOwnersEvent>(&mut v0.add_owners_events, v5);
        };
        if (0x1::vector::length<address>(&arg2) > 0) {
            let v6 = &mut v0.owners;
            let v7 = vector[];
            let v8 = &arg2;
            let v9 = 0;
            while (v9 < 0x1::vector::length<address>(v8)) {
                let (v10, v11) = 0x1::vector::index_of<address>(v6, 0x1::vector::borrow<address>(v8, v9));
                if (v10) {
                    0x1::vector::push_back<address>(&mut v7, 0x1::vector::swap_remove<address>(v6, v11));
                };
                v9 = v9 + 1;
            };
            if (0x1::vector::length<address>(&v7) > 0) {
                if (0x1::features::module_event_migration_enabled()) {
                    let v12 = RemoveOwners{
                        multisig_account : arg0, 
                        owners_removed   : v7,
                    };
                    0x1::event::emit<RemoveOwners>(v12);
                };
                let v13 = RemoveOwnersEvent{owners_removed: v7};
                0x1::event::emit_event<RemoveOwnersEvent>(&mut v0.remove_owners_events, v13);
            };
        };
        if (0x1::option::is_some<u64>(&arg3)) {
            let v14 = 0x1::option::extract<u64>(&mut arg3);
            assert!(v14 > 0, 0x1::error::invalid_argument(11));
            let v15 = v0.num_signatures_required;
            if (v14 != v15) {
                v0.num_signatures_required = v14;
                if (0x1::features::module_event_migration_enabled()) {
                    let v16 = UpdateSignaturesRequired{
                        multisig_account            : arg0, 
                        old_num_signatures_required : v15, 
                        new_num_signatures_required : v14,
                    };
                    0x1::event::emit<UpdateSignaturesRequired>(v16);
                };
                let v17 = UpdateSignaturesRequiredEvent{
                    old_num_signatures_required : v15, 
                    new_num_signatures_required : v14,
                };
                0x1::event::emit_event<UpdateSignaturesRequiredEvent>(&mut v0.update_signature_required_events, v17);
            };
        };
        assert!(0x1::vector::length<address>(&v0.owners) >= v0.num_signatures_required, 0x1::error::invalid_state(5));
    }
    
    entry fun update_signatures_required(arg0: &signer, arg1: u64) acquires MultisigAccount {
        update_owner_schema(0x1::signer::address_of(arg0), vector[], vector[], 0x1::option::some<u64>(arg1));
    }
    
    fun validate_multisig_transaction(arg0: &signer, arg1: address, arg2: vector<u8>) acquires MultisigAccount {
        assert!(exists<MultisigAccount>(arg1), 0x1::error::invalid_state(2002));
        let v0 = 0x1::signer::address_of(arg0);
        let v1 = 0x1::vector::contains<address>(&borrow_global<MultisigAccount>(arg1).owners, &v0);
        assert!(v1, 0x1::error::permission_denied(2003));
        let v2 = last_resolved_sequence_number(arg1);
        let v3 = v2 + 1;
        let v4 = borrow_global<MultisigAccount>(arg1);
        let v5 = 0x1::table::contains<u64, MultisigTransaction>(&v4.transactions, v3);
        assert!(v5, 0x1::error::not_found(2006));
        if (0x1::features::multisig_v2_enhancement_feature_enabled()) {
            assert!(can_execute(0x1::signer::address_of(arg0), arg1, v3), 0x1::error::invalid_argument(2009));
        } else {
            assert!(can_be_executed(arg1, v3), 0x1::error::invalid_argument(2009));
        };
        let v6 = 0x1::table::borrow<u64, MultisigTransaction>(&borrow_global<MultisigAccount>(arg1).transactions, v3);
        if (0x1::option::is_some<vector<u8>>(&v6.payload_hash)) {
            let v7 = 0x1::hash::sha3_256(arg2) == *0x1::option::borrow<vector<u8>>(&v6.payload_hash);
            assert!(v7, 0x1::error::invalid_argument(2008));
        };
        let v8 = 0x1::features::abort_if_multisig_payload_mismatch_enabled();
        if (v8 && 0x1::option::is_some<vector<u8>>(&v6.payload) && !0x1::vector::is_empty<u8>(&arg2)) {
            assert!(arg2 == *0x1::option::borrow<vector<u8>>(&v6.payload), 0x1::error::invalid_argument(2010));
        };
    }
    
    fun validate_owners(arg0: &vector<address>, arg1: address) {
        let v0 = vector[];
        let v1 = 0;
        while (v1 < 0x1::vector::length<address>(arg0)) {
            let v2 = *0x1::vector::borrow<address>(arg0, v1);
            assert!(v2 != arg1, 0x1::error::invalid_argument(13));
            let (v3, _) = 0x1::vector::index_of<address>(&v0, &v2);
            assert!(!v3, 0x1::error::invalid_argument(1));
            0x1::vector::push_back<address>(&mut v0, v2);
            v1 = v1 + 1;
        };
    }
    
    public fun vote(arg0: address, arg1: u64, arg2: address) : (bool, bool) acquires MultisigAccount {
        let v0 = borrow_global_mut<MultisigAccount>(arg0);
        assert!(arg1 > 0 && arg1 < v0.next_sequence_number, 0x1::error::invalid_argument(17));
        let v1 = &0x1::table::borrow<u64, MultisigTransaction>(&v0.transactions, arg1).votes;
        let v2 = &arg2;
        let v3 = 0x1::simple_map::contains_key<address, bool>(v1, v2) && *0x1::simple_map::borrow<address, bool>(v1, &arg2);
        (v4, v3)
    }
    
    public entry fun vote_transaction(arg0: &signer, arg1: address, arg2: u64, arg3: bool) acquires MultisigAccount {
        assert!(0x1::features::multisig_v2_enhancement_feature_enabled(), 0x1::error::invalid_state(20));
        vote_transanction(arg0, arg1, arg2, arg3);
    }
    
    public entry fun vote_transactions(arg0: &signer, arg1: address, arg2: u64, arg3: u64, arg4: bool) acquires MultisigAccount {
        assert!(0x1::features::multisig_v2_enhancement_feature_enabled(), 0x1::error::invalid_state(20));
        let v0 = arg2;
        while (v0 <= arg3) {
            vote_transanction(arg0, arg1, v0, arg4);
            v0 = v0 + 1;
        };
    }
    
    public entry fun vote_transanction(arg0: &signer, arg1: address, arg2: u64, arg3: bool) acquires MultisigAccount {
        assert!(exists<MultisigAccount>(arg1), 0x1::error::invalid_state(2002));
        let v0 = borrow_global_mut<MultisigAccount>(arg1);
        let v1 = 0x1::signer::address_of(arg0);
        assert!(0x1::vector::contains<address>(&v0.owners, &v1), 0x1::error::permission_denied(2003));
        let v2 = 0x1::table::contains<u64, MultisigTransaction>(&v0.transactions, arg2);
        assert!(v2, 0x1::error::not_found(2006));
        let v3 = &mut 0x1::table::borrow_mut<u64, MultisigTransaction>(&mut v0.transactions, arg2).votes;
        let v4 = 0x1::signer::address_of(arg0);
        if (0x1::simple_map::contains_key<address, bool>(v3, &v4)) {
            *0x1::simple_map::borrow_mut<address, bool>(v3, &v4) = arg3;
        } else {
            0x1::simple_map::add<address, bool>(v3, v4, arg3);
        };
        if (0x1::features::module_event_migration_enabled()) {
            let v5 = Vote{
                multisig_account : arg1, 
                owner            : v4, 
                sequence_number  : arg2, 
                approved         : arg3,
            };
            0x1::event::emit<Vote>(v5);
        };
        let v6 = VoteEvent{
            owner           : v4, 
            sequence_number : arg2, 
            approved        : arg3,
        };
        0x1::event::emit_event<VoteEvent>(&mut v0.vote_events, v6);
    }
    
    // decompiled from Move bytecode v6
}
