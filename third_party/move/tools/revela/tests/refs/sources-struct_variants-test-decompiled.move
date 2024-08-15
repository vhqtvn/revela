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
        if (test_variant(v0, Inner::(Inner1))) {
            let Inner::Inner1 { x: v1 } = arg0;
            v1
        } else {
            assert!(test_variant(v0, Inner::(Inner2)), 15621340914461310977);
            let Inner::Inner2 {
                x : v2,
                y : v3,
            } = arg0;
            v2 + v3
        }
    }

    public fun is_inner1(arg0: &Inner) : bool {
        test_variant(arg0, Inner::(Inner1))
    }

    public fun is_some<T0>(arg0: &Option<T0>) : bool {
        if (test_variant(arg0, Option<T0>::(None))) {
            false
        } else {
            assert!(test_variant(arg0, Option<T0>::(Some)), 15621340914461310977);
            true
        }
    }

    public fun is_some_dropped<T0: drop>(arg0: Option<T0>) : bool {
        if (test_variant(&arg0, Option<T0>::(None))) {
            let Option<T0>::None {  } = arg0;
            false
        } else {
            true
        }
    }

    public fun is_some_specialized(arg0: &Option<Option<u64>>) : bool {
        if (test_variant(arg0, Option<Option<u64>>::(None))) {
            false
        } else {
            let v0;
            if (test_variant(arg0, Option<Option<u64>>::(Some))) {
                if (test_variant(&arg0.value, Option<u64>::(None))) {
                    v0 = false;
                    return v0
                };
            };
            assert!(test_variant(arg0, Option<Option<u64>>::(Some)), 15621340914461310977);
            assert!(test_variant(&arg0.value, Option<u64>::(Some)), 15621340914461310977);
            v0 = true;
            v0
        }
    }

    public fun outer_value(arg0: Outer) : u64 {
        let v0 = &arg0;
        if (test_variant(v0, Outer::(None))) {
            let Outer::None {  } = arg0;
            0
        } else if (test_variant(v0, Outer::(One))) {
            let Outer::One { i: v2 } = arg0;
            inner_value(v2)
        } else {
            assert!(test_variant(v0, Outer::(Two)), 15621340914461310977);
            let Outer::Two {
                i : v3,
                b : v4,
            } = arg0;
            let v5 = v4;
            inner_value(v3) + v5.x
        }
    }

    public fun outer_value_nested(arg0: Outer) : u64 {
        let v0 = &arg0;
        if (test_variant(v0, Outer::(None))) {
            let Outer::None {  } = arg0;
            0
        } else {
            let v1;
            if (test_variant(v0, Outer::(One))) {
                if (test_variant(&v0.i, Inner::(Inner1))) {
                    let Outer::One { i: v2 } = arg0;
                    let Inner::Inner1 { x: v1 } = v2;
                    return v1
                };
            };
            if (test_variant(v0, Outer::(One))) {
                let Outer::One { i: v3 } = arg0;
                v1 = inner_value(v3);
            } else {
                assert!(test_variant(v0, Outer::(Two)), 15621340914461310977);
                let Outer::Two {
                    i : v4,
                    b : v5,
                } = arg0;
                let v6 = v5;
                v1 = inner_value(v4) + v6.x;
            };
            v1
        }
    }

    public fun outer_value_with_cond(arg0: Outer) : u64 {
        let v0 = &arg0;
        if (test_variant(v0, Outer::(None))) {
            let Outer::None {  } = arg0;
            0
        } else {
            let v1;
            if (test_variant(v0, Outer::(One))) {
                if (is_inner1(&v0.i)) {
                    let Outer::One { i: v2 } = arg0;
                    v1 = inner_value(v2) % 2;
                    return v1
                };
            };
            if (test_variant(v0, Outer::(One))) {
                let Outer::One { i: v3 } = arg0;
                v1 = inner_value(v3);
            } else {
                assert!(test_variant(v0, Outer::(Two)), 15621340914461310977);
                let Outer::Two {
                    i : v4,
                    b : v5,
                } = arg0;
                let v6 = v5;
                v1 = inner_value(v4) + v6.x;
            };
            v1
        }
    }

    public fun outer_value_with_cond_ref(arg0: &Outer) : bool {
        if (test_variant(arg0, Outer::(None))) {
            false
        } else {
            let v0;
            if (test_variant(arg0, Outer::(One))) {
                if (is_inner1(&arg0.i)) {
                    v0 = true;
                    return v0
                };
            };
            if (test_variant(arg0, Outer::(One))) {
                v0 = is_inner1(&arg0.i);
            } else {
                assert!(test_variant(arg0, Outer::(Two)), 15621340914461310977);
                v0 = is_inner1(&arg0.i);
            };
            v0
        }
    }

    fun select_common_fields(arg0: CommonFields) : u64 {
        let v0 = &arg0;
        let v1 = if (test_variant(v0, CommonFields::(Foo))) {
            let CommonFields::Foo {
                x : _,
                y : v3,
            } = arg0;
            v3
        } else {
            assert!(test_variant(v0, CommonFields::(Bar)), 15621340914461310977);
            let CommonFields::Bar {
                x : _,
                z : v5,
            } = arg0;
            v5
        };
        arg0.x + v1
    }

    fun select_common_fields_different_offset(arg0: CommonFieldsAtDifferentOffset) : u64 {
        let v0 = &arg0;
        let v1 = if (test_variant(v0, CommonFieldsAtDifferentOffset::(Balt|Bar))) {
            &v0.z
        } else {
            &v0.z
        };
        *v1
    }

    // decompiled from Move bytecode v7
}
