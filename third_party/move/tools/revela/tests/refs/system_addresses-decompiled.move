module 0x1::system_addresses {
    public fun assert_aptos_framework(arg0: &signer) {
        assert!(is_aptos_framework_address(0x1::signer::address_of(arg0)), 0x1::error::permission_denied(3));
    }

    public fun assert_core_resource(arg0: &signer) {
        assert_core_resource_address(0x1::signer::address_of(arg0));
    }

    public fun assert_core_resource_address(arg0: address) {
        assert!(is_core_resource_address(arg0), 0x1::error::permission_denied(1));
    }

    public fun assert_framework_reserved(arg0: address) {
        assert!(is_framework_reserved_address(arg0), 0x1::error::permission_denied(4));
    }

    public fun assert_framework_reserved_address(arg0: &signer) {
        assert_framework_reserved(0x1::signer::address_of(arg0));
    }

    public fun assert_vm(arg0: &signer) {
        assert!(is_vm(arg0), 0x1::error::permission_denied(2));
    }

    public fun is_aptos_framework_address(arg0: address) : bool {
        arg0 == @0x1
    }

    public fun is_core_resource_address(arg0: address) : bool {
        arg0 == @0x3000
    }

    public fun is_framework_reserved_address(arg0: address) : bool {
        let v0 = is_aptos_framework_address(arg0) || arg0 == @0x2;
        if (v0) {
            v0 = true;
        } else {
            v0 = arg0 == @0x3;
        };
        if (v0) {
            v0 = true;
        } else {
            v0 = arg0 == @0x4;
        };
        if (v0) {
            v0 = true;
        } else {
            v0 = arg0 == @0x5;
        };
        if (v0) {
            v0 = true;
        } else {
            v0 = arg0 == @0x6;
        };
        if (v0) {
            v0 = true;
        } else {
            v0 = arg0 == @0x7;
        };
        if (v0) {
            v0 = true;
        } else {
            v0 = arg0 == @0x8;
        };
        if (v0) {
            v0 = true;
        } else {
            v0 = arg0 == @0x9;
        };
        if (v0) {
            v0 = true;
        } else {
            v0 = arg0 == @0xa;
        };
        v0
    }

    public fun is_reserved_address(arg0: address) : bool {
        is_aptos_framework_address(arg0) || is_vm_address(arg0)
    }

    public fun is_vm(arg0: &signer) : bool {
        is_vm_address(0x1::signer::address_of(arg0))
    }

    public fun is_vm_address(arg0: address) : bool {
        arg0 == @0x3001
    }

    // decompiled from Move bytecode v7
}
