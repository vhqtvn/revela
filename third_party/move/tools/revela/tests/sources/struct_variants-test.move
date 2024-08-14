module 0xc0ffee::m {

    enum Inner {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }

    struct Box has drop {
        x: u64
    }

    enum Outer {
        None,
        One{i: Inner},
        Two{i: Inner, b: Box},
    }

    // Common fields
    enum CommonFields {
        Foo{x: u64, y: u64},
        Bar{x: u64, z: u64}
    }

    enum CommonFieldsAtDifferentOffset has drop {
       Foo{x: u64, y: u64},
       Bar{x: u64, z: u64},
       Baz{z: u64} // `z` at different offset
       Balt{foo: u8, z: u64}
    }
}
