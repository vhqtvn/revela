/// This defines the Move object model with the the following properties:
/// - Simplified storage interface that supports a heterogeneous collection of resources to be
///   stored together. This enables data types to share a common core data layer (e.g., tokens),
///   while having richer extensions (e.g., concert ticket, sword).
/// - Globally accessible data and ownership model that enables creators and developers to dictate
///   the application and lifetime of data.
/// - Extensible programming model that supports individualization of user applications that
///   leverage the core framework including tokens.
/// - Support emitting events directly, thus improving discoverability of events associated with
///   objects.
/// - Considerate of the underlying system by leveraging resource groups for gas efficiency,
///   avoiding costly deserialization and serialization costs, and supporting deletability.
///
/// TODO:
/// * There is no means to borrow an object or a reference to an object. We are exploring how to
///   make it so that a reference to a global object can be returned from a function.
module aptos_framework::object {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::signer;
    use std::vector;

    use aptos_framework::account;
    use aptos_framework::create_signer::create_signer;
    use aptos_framework::event;
    use aptos_framework::from_bcs;
    use aptos_framework::guid;

    /// An object already exists at this address
    const EOBJECT_EXISTS: u64 = 1;
    /// An object does not exist at this address
    const EOBJECT_DOES_NOT_EXIST: u64 = 2;
    /// The object does not have ungated transfers enabled
    const ENO_UNGATED_TRANSFERS: u64 = 3;
    /// The caller does not have ownership permissions
    const ENOT_OBJECT_OWNER: u64 = 4;
    /// The object does not allow for deletion
    const ECANNOT_DELETE: u64 = 5;
    /// Exceeds maximum nesting for an object transfer.
    const EMAXIMUM_NESTING: u64 = 6;
    /// The resource is not stored at the specified address.
    const ERESOURCE_DOES_NOT_EXIST: u64 = 7;

    /// Maximum nesting from one object to another. That is objects can technically have infinte
    /// nesting, but any checks such as transfer will only be evaluated this deep.
    const MAXIMUM_OBJECT_NESTING: u8 = 8;

    /// Scheme identifier used to generate an object's address `obj_addr` via a fresh GUID generated by the creator at
    /// `source_addr`. The object's address is generated as:
    /// ```
    ///     obj_addr = sha3_256(guid | 0xFD)
    /// ```
    /// where `guid = account::create_guid(create_signer(source_addr))`
    ///
    /// This 0xFD constant serves as a domain separation tag to prevent existing authentication key and resource account
    /// derivation to produce an object address.
    const OBJECT_FROM_GUID_ADDRESS_SCHEME: u8 = 0xFD;

    /// Scheme identifier used to generate an object's address `obj_addr` from the creator's `source_addr` and a `seed` as:
    ///     obj_addr = sha3_256(source_addr | seed | 0xFE).
    ///
    /// This 0xFE constant serves as a domain separation tag to prevent existing authentication key and resource account
    /// derivation to produce an object address.
    const OBJECT_FROM_SEED_ADDRESS_SCHEME: u8 = 0xFE;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The core of the object model that defines ownership, transferability, and events.
    struct ObjectCore has key {
        /// Used by guid to guarantee globally unique objects and create event streams
        guid_creation_num: u64,
        /// The address (object or account) that owns this object
        owner: address,
        /// Object transferring is a common operation, this allows for disabling and enabling
        /// transfers bypassing the use of a TransferRef.
        allow_ungated_transfer: bool,
        /// Emitted events upon transferring of ownership.
        transfer_events: event::EventHandle<TransferEvent>,
    }

    #[resource_group(scope = global)]
    /// A shared resource group for storing object resources together in storage.
    struct ObjectGroup { }

    /// A pointer to an object -- these can only provide guarantees based upon the underlying data
    /// type, that is the validity of T existing at an address is something that cannot be verified
    /// by any other module than the module that defined T. Similarly, the module that defines T
    /// can remove it from storage at any point in time.
    struct Object<phantom T> has copy, drop, store {
        inner: address,
    }

    /// This is a one time ability given to the creator to configure the object as necessary
    struct ConstructorRef has drop {
        self: address,
        /// Set to true so long as deleting the object is possible. For example, the object was
        /// created via create_named_object.
        can_delete: bool,
    }

    /// Used to remove an object from storage.
    struct DeleteRef has drop, store {
        self: address,
    }

    /// Used to create events or move additional resources into object storage.
    struct ExtendRef has drop, store {
        self: address,
    }

    /// Used to create LinearTransferRef, hence ownership transfer.
    struct TransferRef has drop, store {
        self: address,
    }

    /// Used to perform transfers. This locks transferring ability to a single time use bound to
    /// the current owner.
    struct LinearTransferRef has drop {
        self: address,
        owner: address,
    }

    /// Emitted whenever the object's owner field is changed.
    struct TransferEvent has drop, store {
        object: address,
        from: address,
        to: address,
    }

    /// Produces an ObjectId from the given address. This is not verified.
    public fun address_to_object<T: key>(object: address): Object<T> {
        assert!(exists<ObjectCore>(object), error::not_found(EOBJECT_DOES_NOT_EXIST));
        assert!(exists_at<T>(object), error::not_found(ERESOURCE_DOES_NOT_EXIST));
        Object<T>{ inner: object }
    }

    /// Derives an object address from source material: sha3_256([creator address | seed | 0xFE]).
    public fun create_object_address(source: &address, seed: vector<u8>): address {
        let bytes = bcs::to_bytes(source);
        vector::append(&mut bytes, seed);
        vector::push_back(&mut bytes, OBJECT_FROM_SEED_ADDRESS_SCHEME);
        from_bcs::to_address(hash::sha3_256(bytes))
    }

    native fun exists_at<T: key>(object: address): bool;

    /// Returns the address of within an ObjectId.
    public fun object_address<T>(object: &Object<T>): address {
        object.inner
    }

    /// Create a new named object and return the ConstructorRef. Named objects can be queried globally
    /// by knowing the user generated seed used to create them. Named objects cannot be deleted.
    public fun create_named_object(creator: &signer, seed: vector<u8>): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let obj_addr = create_object_address(&creator_address, seed);
        create_object_internal(creator_address, obj_addr, false)
    }

    /// Create a new object from a GUID generated by an account.
    public fun create_object_from_account(creator: &signer): ConstructorRef {
        let guid = account::create_guid(creator);
        create_object_from_guid(signer::address_of(creator), guid)
    }

    /// Create a new object from a GUID generated by an object.
    public fun create_object_from_object(creator: &signer): ConstructorRef acquires ObjectCore {
        let guid = create_guid(creator);
        create_object_from_guid(signer::address_of(creator), guid)
    }

    fun create_object_from_guid(creator_address: address, guid: guid::GUID): ConstructorRef {
        let bytes = bcs::to_bytes(&guid);
        vector::push_back(&mut bytes, OBJECT_FROM_GUID_ADDRESS_SCHEME);
        let obj_addr = from_bcs::to_address(hash::sha3_256(bytes));
        create_object_internal(creator_address, obj_addr, true)
    }

    fun create_object_internal(
        creator_address: address,
        object: address,
        can_delete: bool,
    ): ConstructorRef {
        assert!(!exists<ObjectCore>(object), error::already_exists(EOBJECT_EXISTS));

        let object_signer = create_signer(object);
        let guid_creation_num = 0;
        let transfer_events_guid = guid::create(object, &mut guid_creation_num);

        move_to(
            &object_signer,
            ObjectCore {
                guid_creation_num,
                owner: creator_address,
                allow_ungated_transfer: true,
                transfer_events: event::new_event_handle(transfer_events_guid),
            },
        );
        ConstructorRef { self: object, can_delete }
    }

    // Creation helpers

    /// Generates the DeleteRef, which can be used to remove ObjectCore from global storage.
    public fun generate_delete_ref(ref: &ConstructorRef): DeleteRef {
        assert!(ref.can_delete, error::permission_denied(ECANNOT_DELETE));
        DeleteRef { self: ref.self }
    }

    /// Generates the ExtendRef, which can be used to add new events and resources to the object.
    public fun generate_extend_ref(ref: &ConstructorRef): ExtendRef {
        ExtendRef { self: ref.self }
    }

    /// Generates the TransferRef, which can be used to manage object transfers.
    public fun generate_transfer_ref(ref: &ConstructorRef): TransferRef {
        TransferRef { self: ref.self }
    }

    /// Create a signer for the ConstructorRef
    public fun generate_signer(ref: &ConstructorRef): signer {
        create_signer(ref.self)
    }

    /// Returns the address of within a ConstructorRef
    public fun object_from_constructor_ref<T: key>(ref: &ConstructorRef): Object<T> {
        address_to_object<T>(ref.self)
    }

    // Signer required functions

    /// Create a guid for the object, typically used for events
    public fun create_guid(object: &signer): guid::GUID acquires ObjectCore {
        let addr = signer::address_of(object);
        let object_data = borrow_global_mut<ObjectCore>(addr);
        guid::create(addr, &mut object_data.guid_creation_num)
    }

    /// Generate a new event handle.
    public fun new_event_handle<T: drop + store>(
        object: &signer,
    ): event::EventHandle<T> acquires ObjectCore {
        event::new_event_handle(create_guid(object))
    }

    // Deletion helpers

    /// Returns the address of within a DeleteRef.
    public fun object_from_delete_ref<T: key>(ref: &DeleteRef): Object<T> {
        address_to_object<T>(ref.self)
    }

    /// Removes from the specified Object from global storage.
    public fun delete(ref: DeleteRef) acquires ObjectCore {
        let object_core = move_from<ObjectCore>(ref.self);
        let ObjectCore {
            guid_creation_num: _,
            owner: _,
            allow_ungated_transfer: _,
            transfer_events,
        } = object_core;
        event::destroy_handle(transfer_events);
    }

    // Extension helpers

    /// Create a signer for the ExtendRef
    public fun generate_signer_for_extending(ref: &ExtendRef): signer {
        create_signer(ref.self)
    }

    // Transfer functionality

    /// Disable direct transfer, transfers can only be triggered via a TransferRef
    public fun disable_ungated_transfer(ref: &TransferRef) acquires ObjectCore {
        let object = borrow_global_mut<ObjectCore>(ref.self);
        object.allow_ungated_transfer = false;
    }

    /// Enable direct transfer.
    public fun enable_ungated_transfer(ref: &TransferRef) acquires ObjectCore {
        let object = borrow_global_mut<ObjectCore>(ref.self);
        object.allow_ungated_transfer = true;
    }

    /// Create a LinearTransferRef for a one-time transfer. This requires that the owner at the
    /// time of generation is the owner at the time of transferring.
    public fun generate_linear_transfer_ref(ref: &TransferRef): LinearTransferRef acquires ObjectCore {
        let owner = owner(Object<ObjectCore> { inner: ref.self });
        LinearTransferRef {
            self: ref.self,
            owner,
        }
    }

    /// Transfer to the destination address using a LinearTransferRef.
    public fun transfer_with_ref(ref: LinearTransferRef, to: address) acquires ObjectCore {
        let object = borrow_global_mut<ObjectCore>(ref.self);
        assert!(
            object.owner == ref.owner,
            error::permission_denied(ENOT_OBJECT_OWNER),
        );
        event::emit_event(
            &mut object.transfer_events,
            TransferEvent {
                object: ref.self,
                from: object.owner,
                to,
            },
        );
        object.owner = to;
    }

    /// Entry function that can be used to transfer, if allow_ungated_transfer is set true.
    public entry fun transfer_call(
        owner: &signer,
        object: address,
        to: address,
    ) acquires ObjectCore {
        transfer_raw(owner, object, to)
    }

    public fun transfer<T: key>(
        owner: &signer,
        object: Object<T>,
        to: address,
    ) acquires ObjectCore {
        transfer_raw(owner, object.inner, to)
    }

    /// Attempts to transfer using addresses only. Transfers the given object if
    /// allow_ungated_transfer is set true. Note, that this allows the owner of a nested object to
    /// transfer that object, so long as allow_ungated_transfer is enabled at each stage in the
    /// hierarchy.
    public fun transfer_raw(
        owner: &signer,
        object: address,
        to: address,
    ) acquires ObjectCore {
        let owner_address = signer::address_of(owner);
        verify_ungated_and_descendant(owner_address, object);

        let object_core = borrow_global_mut<ObjectCore>(object);
        if (object_core.owner == to) {
            return
        };

        event::emit_event(
            &mut object_core.transfer_events,
            TransferEvent {
                object: object,
                from: object_core.owner,
                to,
            },
        );
        object_core.owner = to;
    }

    /// Transfer the given object to another object. See `transfer` for more information.
    public fun transfer_to_object<O: key, T: key>(
        owner: &signer,
        object: Object<O>,
        to: Object<T>,
    ) acquires ObjectCore {
        transfer(owner, object, to.inner)
    }

    /// This checks that the destination address is eventually owned by the owner and that each
    /// object between the two allows for ungated transfers. Note, this is limited to a depth of 8
    /// objects may have cyclic dependencies.
    fun verify_ungated_and_descendant(owner: address, destination: address) acquires ObjectCore {
        let current_address = destination;
        assert!(
            exists<ObjectCore>(current_address),
            error::not_found(EOBJECT_DOES_NOT_EXIST),
        );

        let object = borrow_global<ObjectCore>(current_address);
        assert!(
            object.allow_ungated_transfer,
            error::permission_denied(ENO_UNGATED_TRANSFERS),
        );

        let current_address = object.owner;

        let count = 0;
        while (owner != current_address) {
            let count = count + 1;
            assert!(count < MAXIMUM_OBJECT_NESTING, error::out_of_range(EMAXIMUM_NESTING));

            // At this point, the first object exists and so the more likely case is that the
            // object's owner is not an object. So we return a more sensible error.
            assert!(
                exists<ObjectCore>(current_address),
                error::permission_denied(ENOT_OBJECT_OWNER),
            );
            let object = borrow_global<ObjectCore>(current_address);
            assert!(
                object.allow_ungated_transfer,
                error::permission_denied(ENO_UNGATED_TRANSFERS),
            );

            current_address = object.owner;
        };
    }

    /// Accessors

    /// Return the current owner.
    public fun owner<T: key>(object: Object<T>): address acquires ObjectCore {
        assert!(
            exists<ObjectCore>(object.inner),
            error::not_found(EOBJECT_DOES_NOT_EXIST),
        );
        borrow_global<ObjectCore>(object.inner).owner
    }

    /// Return true if the provided address is the current owner.
    public fun is_owner<T: key>(object: Object<T>, owner: address): bool acquires ObjectCore {
        owner(object) == owner
    }

    #[test_only]
    use std::option::{Self, Option};

    #[test_only]
    const EHERO_DOES_NOT_EXIST: u64 = 0x100;
    #[test_only]
    const EWEAPON_DOES_NOT_EXIST: u64 = 0x101;

    #[test_only]
    struct HeroEquipEvent has drop, store {
        weapon_id: Option<Object<Weapon>>,
    }

    #[test_only]
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Hero has key {
        equip_events: event::EventHandle<HeroEquipEvent>,
        weapon: Option<Object<Weapon>>,
    }

    #[test_only]
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Weapon has key { }

    #[test_only]
    public fun create_hero(creator: &signer): (ConstructorRef, Object<Hero>) acquires ObjectCore {
        let hero_constructor_ref = create_named_object(creator, b"hero");
        let hero_signer = generate_signer(&hero_constructor_ref);
        let guid_for_equip_events = create_guid(&hero_signer);
        move_to(
            &hero_signer,
            Hero {
                weapon: option::none(),
                equip_events: event::new_event_handle(guid_for_equip_events),
            },
        );

        let hero = object_from_constructor_ref<Hero>(&hero_constructor_ref);
        (hero_constructor_ref, hero)
    }

    #[test_only]
    public fun create_weapon(creator: &signer): (ConstructorRef, Object<Weapon>) {
        let weapon_constructor_ref = create_named_object(creator, b"weapon");
        let weapon_signer = generate_signer(&weapon_constructor_ref);
        move_to(&weapon_signer, Weapon { });
        let weapon = object_from_constructor_ref<Weapon>(&weapon_constructor_ref);
        (weapon_constructor_ref, weapon)
    }

    #[test_only]
    public fun hero_equip(
        owner: &signer,
        hero: Object<Hero>,
        weapon: Object<Weapon>,
    ) acquires Hero, ObjectCore {
        transfer_to_object(owner, weapon, hero);
        let hero_obj = borrow_global_mut<Hero>(object_address(&hero));
        option::fill(&mut hero_obj.weapon, weapon);
        event::emit_event(
            &mut hero_obj.equip_events,
            HeroEquipEvent { weapon_id: option::some(weapon) },
        );
    }

    #[test_only]
    public fun hero_unequip(
        owner: &signer,
        hero: Object<Hero>,
        weapon: Object<Weapon>,
    ) acquires Hero, ObjectCore {
        transfer(owner, weapon, signer::address_of(owner));
        let hero = borrow_global_mut<Hero>(object_address(&hero));
        option::extract(&mut hero.weapon);
        event::emit_event(
            &mut hero.equip_events,
            HeroEquipEvent { weapon_id: option::none() },
        );
    }

    #[test(creator = @0x123)]
    fun test_object(creator: &signer) acquires Hero, ObjectCore {
        let (_, hero) = create_hero(creator);
        let (_, weapon) = create_weapon(creator);

        hero_equip(creator, hero, weapon);
        hero_unequip(creator, hero, weapon);
    }

    #[test(creator = @0x123)]
    fun test_linear_transfer(creator: &signer) acquires ObjectCore {
        let (hero_constructor, hero) = create_hero(creator);
        let transfer_ref = generate_transfer_ref(&hero_constructor);
        let linear_transfer_ref = generate_linear_transfer_ref(&transfer_ref);
        transfer_with_ref(linear_transfer_ref, @0x456);
        assert!(owner(hero) == @0x456, 0);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_bad_linear_transfer(creator: &signer) acquires ObjectCore {
        let (hero_constructor, hero) = create_hero(creator);
        let transfer_ref = generate_transfer_ref(&hero_constructor);
        let linear_transfer_ref_good = generate_linear_transfer_ref(&transfer_ref);
        // This will contain the address of the creator
        let linear_transfer_ref_bad = generate_linear_transfer_ref(&transfer_ref);
        transfer_with_ref(linear_transfer_ref_good, @0x456);
        assert!(owner(hero) == @0x456, 0);
        transfer_with_ref(linear_transfer_ref_bad, @0x789);
    }
}
