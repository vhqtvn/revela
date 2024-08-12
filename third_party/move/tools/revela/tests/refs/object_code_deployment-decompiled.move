module 0x1::object_code_deployment {
    struct Freeze has drop, store {
        object_address: address,
    }
    
    struct ManagingRefs has key {
        extend_ref: 0x1::object::ExtendRef,
    }
    
    struct Publish has drop, store {
        object_address: address,
    }
    
    struct Upgrade has drop, store {
        object_address: address,
    }
    
    public entry fun freeze_code_object(arg0: &signer, arg1: 0x1::object::Object<0x1::code::PackageRegistry>) {
        0x1::code::freeze_code_object(arg0, arg1);
        let v0 = Freeze{object_address: 0x1::object::object_address<0x1::code::PackageRegistry>(&arg1)};
        0x1::event::emit<Freeze>(v0);
    }
    
    public entry fun publish(arg0: &signer, arg1: vector<u8>, arg2: vector<vector<u8>>) {
        assert!(0x1::features::is_object_code_deployment_enabled(), 0x1::error::unavailable(1));
        let v0 = 0x1::account::get_sequence_number(0x1::signer::address_of(arg0)) + 1;
        let v1 = 0x1::vector::empty<u8>();
        let v2 = b"aptos_framework::object_code_deployment";
        0x1::vector::append<u8>(&mut v1, 0x1::bcs::to_bytes<vector<u8>>(&v2));
        0x1::vector::append<u8>(&mut v1, 0x1::bcs::to_bytes<u64>(&v0));
        let v3 = 0x1::object::create_named_object(arg0, v1);
        let v4 = &v3;
        let v5 = 0x1::object::generate_signer(v4);
        let v6 = &v5;
        0x1::code::publish_package_txn(v6, arg1, arg2);
        let v7 = Publish{object_address: 0x1::signer::address_of(v6)};
        0x1::event::emit<Publish>(v7);
        let v8 = ManagingRefs{extend_ref: 0x1::object::generate_extend_ref(v4)};
        move_to<ManagingRefs>(v6, v8);
    }
    
    public entry fun upgrade(arg0: &signer, arg1: vector<u8>, arg2: vector<vector<u8>>, arg3: 0x1::object::Object<0x1::code::PackageRegistry>) acquires ManagingRefs {
        let v0 = 0x1::object::is_owner<0x1::code::PackageRegistry>(arg3, 0x1::signer::address_of(arg0));
        assert!(v0, 0x1::error::permission_denied(2));
        let v1 = 0x1::object::object_address<0x1::code::PackageRegistry>(&arg3);
        assert!(exists<ManagingRefs>(v1), 0x1::error::not_found(3));
        let v2 = 0x1::object::generate_signer_for_extending(&borrow_global<ManagingRefs>(v1).extend_ref);
        let v3 = &v2;
        0x1::code::publish_package_txn(v3, arg1, arg2);
        let v4 = Upgrade{object_address: 0x1::signer::address_of(v3)};
        0x1::event::emit<Upgrade>(v4);
    }
    
    // decompiled from Move bytecode v7
}
