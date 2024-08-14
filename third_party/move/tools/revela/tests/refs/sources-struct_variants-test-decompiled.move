module 0xc0ffee::m {
    struct Box has drop {
        x: u64,
    }
    
    enum CommonFields {
        Foo { x: u64, y: u64, },
        Bar { x: u64, z: u64, },
    }
    
    enum CommonFieldsAtDifferentOffset has drop {
        Foo { x: u64, y: u64, },
        Bar { x: u64, z: u64, },
        Baz { z: u64, },
        Balt { foo: u8, z: u64, },
    }
    
    enum Inner {
        Inner1 { x: u64, },
        Inner2 { x: u64, y: u64, },
    }
    
    enum Outer {
        None,
        One { i: Inner, },
        Two { i: Inner, b: Box, },
    }
    
    // decompiled from Move bytecode v7
}
