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

    enum Option<T0> has drop {
        None,
        Some { value: T0, },
    }

    enum Outer {
        None,
        One { i: Inner, },
        Two { i: Inner, b: Box, },
    }

    public fun inner_value(arg0: Inner) : u64 {
        let v0 = &arg0;
        if (match (v0) { Inner1{x:_} => true, _ => false }) {
            let v1 = match (arg0) { Inner1{ x } => x, _ => abort 15621340914461310977 };
            v1
        } else {
            assert!(match (v0) { Inner2{x:_,y:_} => true, _ => false }, 15621340914461310977);
            let (v2, v3) = match (arg0) { Inner2{ x, y } => (x, y), _ => abort 15621340914461310977 };
            v2 + v3
        }
    }

    public fun is_inner1(arg0: &Inner) : bool {
        match (arg0) { Inner1{x:_} => true, _ => false }
    }

    public fun is_some<T0>(arg0: &Option<T0>) : bool {
        if (match (arg0) { None => true, _ => false }) {
            false
        } else {
            assert!(match (arg0) { Some{value:_} => true, _ => false }, 15621340914461310977);
            true
        }
    }

    public fun is_some_dropped<T0: drop>(arg0: Option<T0>) : bool {
        if (match (&arg0) { None => true, _ => false }) {
            match (arg0) { None => (), _ => abort 0 };
            false
        } else {
            true
        }
    }

    public fun is_some_specialized(arg0: &Option<Option<u64>>) : bool {
        if (match (arg0) { None => true, _ => false }) {
            false
        } else {
            let v0;
            if (match (arg0) { Some{value:_} => true, _ => false }) {
                if (match (&arg0.value) { None => true, _ => false }) {
                    v0 = false;
                    return v0
                };
            };
            assert!(match (arg0) { Some{value:_} => true, _ => false }, 15621340914461310977);
            assert!(match (&arg0.value) { Some{value:_} => true, _ => false }, 15621340914461310977);
            v0 = true;
            v0
        }
    }

    public fun outer_value(arg0: Outer) : u64 {
        let v0 = &arg0;
        if (match (v0) { None => true, _ => false }) {
            match (arg0) { None => (), _ => abort 0 };
            0
        } else if (match (v0) { One{i:_} => true, _ => false }) {
            let v2 = match (arg0) { One{ i } => i, _ => abort 15621340914461310977 };
            inner_value(v2)
        } else {
            assert!(match (v0) { Two{i:_,b:_} => true, _ => false }, 15621340914461310977);
            let (v3, v4) = match (arg0) { Two{ i, b } => (i, b), _ => abort 15621340914461310977 };
            let v5 = v4;
            inner_value(v3) + v5.x
        }
    }

    public fun outer_value_nested(arg0: Outer) : u64 {
        let v0 = &arg0;
        if (match (v0) { None => true, _ => false }) {
            match (arg0) { None => (), _ => abort 0 };
            0
        } else {
            let v1;
            if (match (v0) { One{i:_} => true, _ => false }) {
                if (match (&v0.i) { Inner1{x:_} => true, _ => false }) {
                    let v2 = match (arg0) { One{ i } => i, _ => abort 15621340914461310977 };
                    let v1 = match (v2) { Inner1{ x } => x, _ => abort 15621340914461310977 };
                    return v1
                };
            };
            if (match (v0) { One{i:_} => true, _ => false }) {
                let v3 = match (arg0) { One{ i } => i, _ => abort 15621340914461310977 };
                v1 = inner_value(v3);
            } else {
                assert!(match (v0) { Two{i:_,b:_} => true, _ => false }, 15621340914461310977);
                let (v4, v5) = match (arg0) { Two{ i, b } => (i, b), _ => abort 15621340914461310977 };
                let v6 = v5;
                v1 = inner_value(v4) + v6.x;
            };
            v1
        }
    }

    public fun outer_value_with_cond(arg0: Outer) : u64 {
        let v0 = &arg0;
        if (match (v0) { None => true, _ => false }) {
            match (arg0) { None => (), _ => abort 0 };
            0
        } else {
            let v1;
            if (match (v0) { One{i:_} => true, _ => false }) {
                if (is_inner1(&v0.i)) {
                    let v2 = match (arg0) { One{ i } => i, _ => abort 15621340914461310977 };
                    v1 = inner_value(v2) % 2;
                    return v1
                };
            };
            if (match (v0) { One{i:_} => true, _ => false }) {
                let v3 = match (arg0) { One{ i } => i, _ => abort 15621340914461310977 };
                v1 = inner_value(v3);
            } else {
                assert!(match (v0) { Two{i:_,b:_} => true, _ => false }, 15621340914461310977);
                let (v4, v5) = match (arg0) { Two{ i, b } => (i, b), _ => abort 15621340914461310977 };
                let v6 = v5;
                v1 = inner_value(v4) + v6.x;
            };
            v1
        }
    }

    public fun outer_value_with_cond_ref(arg0: &Outer) : bool {
        if (match (arg0) { None => true, _ => false }) {
            false
        } else {
            let v0;
            if (match (arg0) { One{i:_} => true, _ => false }) {
                if (is_inner1(&arg0.i)) {
                    v0 = true;
                    return v0
                };
            };
            if (match (arg0) { One{i:_} => true, _ => false }) {
                v0 = is_inner1(&arg0.i);
            } else {
                assert!(match (arg0) { Two{i:_,b:_} => true, _ => false }, 15621340914461310977);
                v0 = is_inner1(&arg0.i);
            };
            v0
        }
    }

    fun select_common_fields(arg0: CommonFields) : u64 {
        let v0 = arg0.x;
        let v1 = &arg0;
        let v2 = if (match (v1) { Foo{x:_,y:_} => true, _ => false }) {
            let (_, v4) = match (arg0) { Foo{ x, y } => (x, y), _ => abort 15621340914461310977 };
            v4
        } else {
            assert!(match (v1) { Bar{x:_,z:_} => true, _ => false }, 15621340914461310977);
            let (_, v6) = match (arg0) { Bar{ x, z } => (x, z), _ => abort 15621340914461310977 };
            v6
        };
        v0 + v2
    }

    fun select_common_fields_different_offset(arg0: CommonFieldsAtDifferentOffset) : u64 {
        let v0 = &arg0;
        let v1 = if (match (v0) { Bar{x:_,z:_} => true, Balt{foo:_,z:_} => true, _ => false }) {
            &v0.z
        } else {
            &v0.z
        };
        *v1
    }

    // decompiled from Move bytecode v7
}
