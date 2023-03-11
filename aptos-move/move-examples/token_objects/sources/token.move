/// This defines an object-based Token. The key differentiating features from the Aptos standard
/// token are:
/// * Decouple token ownership from token data.
/// * Explicit data model for token metadata via adjacent resources
/// * Extensible framework for tokens
///
/// TODO:
/// * Update Object<T> to be a viable input as a transaction arg and then update all readers as view.
module token_objects::token {
    use std::error;
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::signer;
    use std::vector;

    use aptos_framework::event;
    use aptos_framework::object::{Self, ConstructorRef, Object};

    use token_objects::collection;
    use token_objects::royalty::{Self, Royalty};

    // The token does not exist
    const ETOKEN_DOES_NOT_EXIST: u64 = 1;
    /// The provided signer is not the creator
    const ENOT_CREATOR: u64 = 2;
    /// Attempted to mutate an immutable field
    const EFIELD_NOT_MUTABLE: u64 = 3;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields to all tokens.
    struct Token has key {
        /// An optional categorization of similar token, there are no constraints on collections.
        collection: String,
        /// Unique identifier within the collection, optional, 0 means unassigned
        collection_id: u64,
        /// The original creator of this token.
        creator: address,
        /// A brief description of the token.
        description: String,
        /// The name of the token, which should be unique within the collection; the length of name
        /// should be smaller than 128, characters, eg: "Aptos Animal #1234"
        name: String,
        /// The creation name of the token. Since tokens are created with the name as part of the
        /// object id generation.
        creation_name: Option<String>,
        /// The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
        /// storage; the URL length will likely need a maximum any suggestions?
        uri: String,
        /// Emitted upon any mutation of the token.
        mutation_events: event::EventHandle<MutationEvent>,
    }

    /// This enables burning an NFT, if possible, it will also delete the object. Note, the data
    /// in inner and self occupies 32-bytes each, rather than have both, this data structure makes
    /// a small optimization to support either and take a fixed amount of 34-bytes.
    struct BurnRef has drop, store {
        inner: Option<object::DeleteRef>,
        self: Option<address>,
    }

    /// Contains the mutated fields name. This makes the life of indexers easier, so that they can
    /// directly understand the behavior in a writeset.
    struct MutationEvent has drop, store {
        mutated_field_name: String,
    }

    /// This enables mutating descritpion and URI by higher level services.
    struct MutatorRef has drop, store {
        self: address,
    }

    public fun create(
        creator: &signer,
        collection: String,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let seed = create_token_seed(&collection, &name);
        let constructor_ref = object::create_named_object(creator, seed);
        let object_signer = object::generate_signer(&constructor_ref);

        let id = collection::increment_supply(&creator_address, &collection);
        let token = Token {
            collection,
            collection_id: option::get_with_default(&mut id, 0),
            creator: creator_address,
            description,
            name,
            creation_name: option::none(),
            uri,
            mutation_events: object::new_event_handle(&object_signer),
        };
        move_to(&object_signer, token);

        if (option::is_some(&royalty)) {
            royalty::init(&constructor_ref, option::extract(&mut royalty))
        };
        constructor_ref
    }

    public fun create_token_address(creator: &address, collection: &String, name: &String): address {
        object::create_object_address(creator, create_token_seed(collection, name))
    }

    public fun create_token_seed(collection: &String, name: &String): vector<u8> {
        let seed = *string::bytes(collection);
        vector::append(&mut seed, b"::");
        vector::append(&mut seed, *string::bytes(name));
        seed
    }

    /// Simple token creation that generates a token and deposits it into the creators object store.
    public entry fun mint(
        creator: &signer,
        collection: String,
        description: String,
        name: String,
        uri: String,
        enable_royalty: bool,
        royalty_numerator: u64,
        royalty_denominator: u64,
        royalty_payee_address: address,
    ) {
        let royalty = if (enable_royalty) {
            option::some(royalty::create(
                royalty_numerator,
                royalty_denominator,
                royalty_payee_address,
            ))
        } else {
            option::none()
        };

        create(
            creator,
            collection,
            description,
            name,
            royalty,
            uri,
        );
    }

    public fun generate_mutator_ref(ref: &ConstructorRef): MutatorRef {
        let object = object::object_from_constructor_ref<Token>(ref);
        MutatorRef { self: object::object_address(&object) }
    }

    public fun generate_burn_ref(ref: &ConstructorRef): BurnRef {
        let (inner, self) = if (object::can_generate_delete_ref(ref)) {
            let delete_ref = object::generate_delete_ref(ref);
            (option::some(delete_ref), option::none())
        } else {
            let addr = object::address_from_constructor_ref(ref);
            (option::none(), option::some(addr))
        };
        BurnRef { self, inner }
    }

    public fun address_from_burn_ref(ref: &BurnRef): address {
        if (option::is_some(&ref.inner)) {
            object::address_from_delete_ref(option::borrow(&ref.inner))
        } else {
            *option::borrow(&ref.self)
        }
    }

    // Accessors
    inline fun verify<T: key>(token: &Object<T>): address {
        let token_address = object::object_address(token);
        assert!(
            exists<Token>(token_address),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        token_address
    }

    public fun creator<T: key>(token: Object<T>): address acquires Token {
        let token_address = verify(&token);
        borrow_global<Token>(token_address).creator
    }

    public fun collection<T: key>(token: Object<T>): String acquires Token {
        let token_address = verify(&token);
        borrow_global<Token>(token_address).collection
    }

    public fun creation_name<T: key>(token: Object<T>): String acquires Token {
        let token_address = verify(&token);
        let token = borrow_global<Token>(token_address);
        if (option::is_some(&token.creation_name)) {
            *option::borrow(&token.creation_name)
        } else {
            token.name
        }
    }

    public fun description<T: key>(token: Object<T>): String acquires Token {
        let token_address = verify(&token);
        borrow_global<Token>(token_address).description
    }

    public fun name<T: key>(token: Object<T>): String acquires Token {
        let token_address = verify(&token);
        borrow_global<Token>(token_address).name
    }

    public fun uri<T: key>(token: Object<T>): String acquires Token {
        let token_address = verify(&token);
        borrow_global<Token>(token_address).uri
    }

    public fun royalty<T: key>(token: Object<T>): Option<Royalty> acquires Token {
        verify(&token);
        let royalty = royalty::get(token);
        if (option::is_some(&royalty)) {
            royalty
        } else {
            let creator = creator(token);
            let collection_name = collection(token);
            let collection_address = collection::create_collection_address(&creator, &collection_name);
            let collection = object::address_to_object<collection::Collection>(collection_address);
            royalty::get(collection)
        }
    }

    // Mutators

    inline fun mutator_ref_to_address(mutator_ref: &MutatorRef): address {
        assert!(
            exists<Token>(mutator_ref.self),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        mutator_ref.self
    }

    public fun burn(burn_ref: BurnRef) acquires Token {
        let addr = if (option::is_some(&burn_ref.inner)) {
            let delete_ref = option::extract(&mut burn_ref.inner);
            let addr = object::address_from_delete_ref(&delete_ref);
            object::delete(delete_ref);
            addr
        } else {
            option::extract(&mut burn_ref.self)
        };

        if (royalty::exists_at(addr)) {
            royalty::delete(addr)
        };

        let Token {
            collection: _,
            collection_id: _,
            creator: _,
            description: _,
            name: _,
            creation_name: _,
            uri: _,
            mutation_events,
        } = move_from<Token>(addr);
        event::destroy_handle(mutation_events);
    }

    public fun set_description(
        mutator_ref: &MutatorRef,
        description: String,
    ) acquires Token {
        let token_address = mutator_ref_to_address(mutator_ref);
        let token = borrow_global_mut<Token>(token_address);

        token.description = description;
        event::emit_event(
            &mut token.mutation_events,
            MutationEvent { mutated_field_name: string::utf8(b"description") },
        );
    }

    public fun set_name(
        mutator_ref: &MutatorRef,
        name: String,
    ) acquires Token {
        let token_address = mutator_ref_to_address(mutator_ref);
        let token = borrow_global_mut<Token>(token_address);

        if (option::is_none(&token.creation_name)) {
            option::fill(&mut token.creation_name, token.name)
        };
        token.name = name;
        event::emit_event(
            &mut token.mutation_events,
            MutationEvent { mutated_field_name: string::utf8(b"name") },
        );
    }

    public fun set_uri(
        mutator_ref: &MutatorRef,
        uri: String,
    ) acquires Token {
        let token_address = mutator_ref_to_address(mutator_ref);
        let token = borrow_global_mut<Token>(token_address);

        token.uri = uri;
        event::emit_event(
            &mut token.mutation_events,
            MutationEvent { mutated_field_name: string::utf8(b"uri") },
        );
    }

    #[test(creator = @0x123, trader = @0x456)]
    entry fun test_create_and_transfer(creator: &signer, trader: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        create_token_helper(creator, *&collection_name, *&token_name);

        let creator_address = signer::address_of(creator);
        let token_addr = create_token_address(&creator_address, &collection_name, &token_name);
        let token = object::address_to_object<Token>(token_addr);
        assert!(object::owner(token) == creator_address, 1);
        object::transfer(creator, token, signer::address_of(trader));
        assert!(object::owner(token) == signer::address_of(trader), 1);

        let expected_royalty = royalty::create(25, 10000, creator_address);
        assert!(option::some(expected_royalty) == royalty(token), 2);
    }

    #[test(creator = @0x123)]
    entry fun test_collection_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        collection::create_collection(
            creator,
            string::utf8(b"collection description"),
            collection_name,
            string::utf8(b"collection uri"),
            5,
            true,
            10,
            1000,
            signer::address_of(creator),
        );

        create(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );

        let creator_address = signer::address_of(creator);
        let token_addr = create_token_address(&creator_address, &collection_name, &token_name);
        let token = object::address_to_object<Token>(token_addr);
        let expected_royalty = royalty::create(10, 1000, creator_address);
        assert!(option::some(expected_royalty) == royalty(token), 0);
    }

    #[test(creator = @0x123)]
    entry fun test_no_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        collection::create_untracked_collection(
            creator,
            string::utf8(b"collection description"),
            collection_name,
            option::none(),
            string::utf8(b"collection uri"),
        );

        create(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );

        let creator_address = signer::address_of(creator);
        let token_addr = create_token_address(&creator_address, &collection_name, &token_name);
        let token = object::address_to_object<Token>(token_addr);
        assert!(option::none() == royalty(token), 0);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x20001, location = token_objects::collection)]
    entry fun test_too_many_tokens(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        create_token_helper(creator, *&collection_name, token_name);
        create_token_helper(creator, collection_name, string::utf8(b"bad"));
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x80001, location = aptos_framework::object)]
    entry fun test_duplicate_tokens(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        create_token_helper(creator, *&collection_name, *&token_name);
        create_token_helper(creator, collection_name, token_name);
    }

    #[test(creator = @0x123)]
    entry fun test_set_description(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        let mutator_ref = create_token_with_mutation_ref(creator, collection_name, token_name);
        let token = object::address_to_object<Token>(
            create_token_address(&signer::address_of(creator), &collection_name, &token_name),
        );

        let description = string::utf8(b"no fail");
        assert!(description != description(token), 0);
        set_description(&mutator_ref, *&description);
        assert!(description == description(token), 1);
    }

    #[test(creator = @0x123)]
    entry fun test_set_name(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        let mutator_ref = create_token_with_mutation_ref(creator, collection_name, token_name);
        let token = object::address_to_object<Token>(
            create_token_address(&signer::address_of(creator), &collection_name, &token_name),
        );

        let name = string::utf8(b"no fail");
        assert!(name != name(token), 0);
        {
            let token = borrow_global<Token>(object::object_address(&token));
            assert!(option::is_none(&token.creation_name), 1);
        };
        set_name(&mutator_ref, *&name);
        assert!(name == name(token), 2);
        assert!(token_name == creation_name(token), 3);
    }

    #[test(creator = @0x123)]
    entry fun test_set_uri(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        let mutator_ref = create_token_with_mutation_ref(creator, collection_name, token_name);
        let token = object::address_to_object<Token>(
            create_token_address(&signer::address_of(creator), &collection_name, &token_name),
        );

        let uri = string::utf8(b"no fail");
        assert!(uri != uri(token), 0);
        set_uri(&mutator_ref, *&uri);
        assert!(uri == uri(token), 1);
    }

    #[test(creator = @0x123)]
    entry fun test_burn_without_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        let constructor_ref = create(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );
        let burn_ref = generate_burn_ref(&constructor_ref);
        let token_addr = object::address_from_constructor_ref(&constructor_ref);
        assert!(exists<Token>(token_addr), 0);
        assert!(!royalty::exists_at(token_addr), 3);
        burn(burn_ref);
        assert!(!exists<Token>(token_addr), 2);
        assert!(!royalty::exists_at(token_addr), 3);
    }

    #[test(creator = @0x123)]
    entry fun test_burn_with_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        let constructor_ref = create(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::some(royalty::create(1, 1, signer::address_of(creator))),
            string::utf8(b"token uri"),
        );
        let burn_ref = generate_burn_ref(&constructor_ref);
        let token_addr = object::address_from_constructor_ref(&constructor_ref);
        assert!(exists<Token>(token_addr), 0);
        assert!(royalty::exists_at(token_addr), 1);
        burn(burn_ref);
        assert!(!exists<Token>(token_addr), 2);
        assert!(!royalty::exists_at(token_addr), 3);
    }

    #[test_only]
    entry fun create_collection_helper(creator: &signer, collection_name: String, max_supply: u64) {
        collection::create_collection(
            creator,
            string::utf8(b"collection description"),
            collection_name,
            string::utf8(b"collection uri"),
            max_supply,
            false,
            0,
            0,
            signer::address_of(creator),
        );
    }

    #[test_only]
    entry fun create_token_helper(creator: &signer, collection_name: String, token_name: String) {
        mint(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            string::utf8(b"token uri"),
            true,
            25,
            10000,
            signer::address_of(creator),
        );
    }

    #[test_only]
    entry fun create_token_with_mutation_ref(
        creator: &signer,
        collection_name: String,
        token_name: String,
    ): MutatorRef {
        let constructor_ref = create(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );
        generate_mutator_ref(&constructor_ref)
    }
}
