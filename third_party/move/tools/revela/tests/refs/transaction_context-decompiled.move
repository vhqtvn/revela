module 0x1::transaction_context {
    struct AUID has drop, store {
        unique_address: address,
    }
    
    struct EntryFunctionPayload has copy, drop {
        account_address: address,
        module_name: 0x1::string::String,
        function_name: 0x1::string::String,
        ty_args_names: vector<0x1::string::String>,
        args: vector<vector<u8>>,
    }
    
    struct MultisigPayload has copy, drop {
        multisig_address: address,
        entry_function_payload: 0x1::option::Option<EntryFunctionPayload>,
    }
    
    public fun account_address(arg0: &EntryFunctionPayload) : address {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        arg0.account_address
    }
    
    public fun args(arg0: &EntryFunctionPayload) : vector<vector<u8>> {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        arg0.args
    }
    
    public fun auid_address(arg0: &AUID) : address {
        arg0.unique_address
    }
    
    public fun chain_id() : u8 {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        chain_id_internal()
    }
    
    native fun chain_id_internal() : u8;
    public fun entry_function_payload() : 0x1::option::Option<EntryFunctionPayload> {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        entry_function_payload_internal()
    }
    
    native fun entry_function_payload_internal() : 0x1::option::Option<EntryFunctionPayload>;
    public fun function_name(arg0: &EntryFunctionPayload) : 0x1::string::String {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        arg0.function_name
    }
    
    public fun gas_payer() : address {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        gas_payer_internal()
    }
    
    native fun gas_payer_internal() : address;
    public fun gas_unit_price() : u64 {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        gas_unit_price_internal()
    }
    
    native fun gas_unit_price_internal() : u64;
    public fun generate_auid() : AUID {
        AUID{unique_address: generate_unique_address()}
    }
    
    public fun generate_auid_address() : address {
        generate_unique_address()
    }
    
    native fun generate_unique_address() : address;
    native public fun get_script_hash() : vector<u8>;
    public fun get_transaction_hash() : vector<u8> {
        get_txn_hash()
    }
    
    native fun get_txn_hash() : vector<u8>;
    public fun inner_entry_function_payload(arg0: &MultisigPayload) : 0x1::option::Option<EntryFunctionPayload> {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        arg0.entry_function_payload
    }
    
    public fun max_gas_amount() : u64 {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        max_gas_amount_internal()
    }
    
    native fun max_gas_amount_internal() : u64;
    public fun module_name(arg0: &EntryFunctionPayload) : 0x1::string::String {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        arg0.module_name
    }
    
    public fun multisig_address(arg0: &MultisigPayload) : address {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        arg0.multisig_address
    }
    
    public fun multisig_payload() : 0x1::option::Option<MultisigPayload> {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        multisig_payload_internal()
    }
    
    native fun multisig_payload_internal() : 0x1::option::Option<MultisigPayload>;
    public fun secondary_signers() : vector<address> {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        secondary_signers_internal()
    }
    
    native fun secondary_signers_internal() : vector<address>;
    public fun sender() : address {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        sender_internal()
    }
    
    native fun sender_internal() : address;
    public fun type_arg_names(arg0: &EntryFunctionPayload) : vector<0x1::string::String> {
        assert!(0x1::features::transaction_context_extension_enabled(), 0x1::error::invalid_state(2));
        arg0.ty_args_names
    }
    
    // decompiled from Move bytecode v6
}
