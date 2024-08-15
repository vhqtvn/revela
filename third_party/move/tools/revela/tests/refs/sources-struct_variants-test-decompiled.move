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
        // Raw stackless bytecode
        //   $t3 := borrow_local($t0)
        //   $t4 := test_variant m::Inner::Inner1($t3)
        //   if ($t4) goto L5 else goto L0
        //   label L1
        //   $t5 := move($t0)
        //   $t2 := unpack_variant m::Inner::Inner1($t5)
        //   goto L2
        //   label L0
        //   $t6 := test_variant m::Inner::Inner2($t3)
        //   if ($t6) goto L4 else goto L3
        //   label L4
        //   $t7 := move($t0)
        //   ($t8, $t9) := unpack_variant m::Inner::Inner2($t7)
        //   $t2 := +($t8, $t9)
        //   goto L2
        //   label L3
        //   $t10 := 15621340914461310977
        //   abort($t10)
        //   label L2
        //   return $t2
        //   label L5
        //   drop($t3)
        //   goto L1
        // End raw stackless bytecode
        // Bytecode
        //   $t3 := borrow_local($t0)
        //   $t4 := test_variant m::Inner::Inner1($t3)
        //   if ($t4) {
        //     drop($t3)
        //     $t5 := move($t0)
        //     $t2 := unpack_variant m::Inner::Inner1($t5)
        //   } else {
        //     $t6 := test_variant m::Inner::Inner2($t3)
        //     if ($t6) {
        //       $t7 := move($t0)
        //       ($t8, $t9) := unpack_variant m::Inner::Inner2($t7)
        //       $t2 := +($t8, $t9)
        //     } else {
        //       $t10 := 15621340914461310977
        //       abort($t10)
        //       // aborted path
        //     }
        //   }
        //   return $t2
        //   // returned path
        //
        // End Bytecode
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
        // Raw stackless bytecode
        //   $t2 := test_variant m::Inner::Inner1($t0)
        //   if ($t2) goto L1 else goto L0
        //   label L1
        //   $t3 := true
        //   $t1 := $t3
        //   goto L2
        //   label L0
        //   $t4 := false
        //   $t1 := $t4
        //   label L2
        //   return $t1
        // End raw stackless bytecode
        // Bytecode
        //   $t2 := test_variant m::Inner::Inner1($t0)
        //   if ($t2) {
        //     $t3 := true
        //     $t1 := $t3
        //   } else {
        //     $t4 := false
        //     $t1 := $t4
        //   }
        //   return $t1
        //   // returned path
        //
        // End Bytecode
        test_variant(arg0, Inner::(Inner1))
    }

    public fun is_some<T0>(arg0: &Option<T0>) : bool {
        // Raw stackless bytecode
        //   $t2 := test_variant m::Option<#0>::None($t0)
        //   if ($t2) goto L5 else goto L0
        //   label L1
        //   $t3 := false
        //   $t1 := $t3
        //   goto L2
        //   label L0
        //   $t4 := test_variant m::Option<#0>::Some($t0)
        //   if ($t4) goto L4 else goto L3
        //   label L4
        //   $t5 := true
        //   $t1 := $t5
        //   goto L2
        //   label L3
        //   $t6 := 15621340914461310977
        //   abort($t6)
        //   label L2
        //   return $t1
        //   label L5
        //   drop($t0)
        //   goto L1
        // End raw stackless bytecode
        // Bytecode
        //   $t2 := test_variant m::Option<#0>::None($t0)
        //   if ($t2) {
        //     drop($t0)
        //     $t3 := false
        //     $t1 := $t3
        //   } else {
        //     $t4 := test_variant m::Option<#0>::Some($t0)
        //     if ($t4) {
        //       $t5 := true
        //       $t1 := $t5
        //     } else {
        //       $t6 := 15621340914461310977
        //       abort($t6)
        //       // aborted path
        //     }
        //   }
        //   return $t1
        //   // returned path
        //
        // End Bytecode
        if (test_variant(arg0, Option<T0>::(None))) {
            false
        } else {
            assert!(test_variant(arg0, Option<T0>::(Some)), 15621340914461310977);
            true
        }
    }

    public fun is_some_dropped<T0: drop>(arg0: Option<T0>) : bool {
        // Raw stackless bytecode
        //   $t2 := borrow_local($t0)
        //   $t3 := test_variant m::Option<#0>::None($t2)
        //   if ($t3) goto L1 else goto L0
        //   label L1
        //   $t4 := move($t0)
        //   unpack_variant m::Option<#0>::None($t4)
        //   $t5 := false
        //   $t1 := $t5
        //   goto L2
        //   label L0
        //   $t6 := true
        //   $t1 := $t6
        //   label L2
        //   return $t1
        // End raw stackless bytecode
        // Bytecode
        //   $t2 := borrow_local($t0)
        //   $t3 := test_variant m::Option<#0>::None($t2)
        //   if ($t3) {
        //     $t4 := move($t0)
        //     unpack_variant m::Option<#0>::None($t4)
        //     $t5 := false
        //     $t1 := $t5
        //   } else {
        //     $t6 := true
        //     $t1 := $t6
        //   }
        //   return $t1
        //   // returned path
        //
        // End Bytecode
        if (test_variant(&arg0, Option<T0>::(None))) {
            let Option<T0>::None {  } = arg0;
            false
        } else {
            true
        }
    }

    public fun is_some_specialized(arg0: &Option<Option<u64>>) : bool {
        // Raw stackless bytecode
        //   $t2 := test_variant m::Option<m::Option<u64>>::None($t0)
        //   if ($t2) goto L9 else goto L0
        //   label L1
        //   $t3 := false
        //   $t1 := $t3
        //   goto L2
        //   label L0
        //   $t4 := test_variant m::Option<m::Option<u64>>::Some($t0)
        //   if ($t4) goto L4 else goto L3
        //   label L4
        //   $t5 := borrow_variant_field<m::Option<m::Option<u64>>::Some>.value($t0)
        //   $t6 := test_variant m::Option<u64>::None($t5)
        //   if ($t6) goto L10 else goto L3
        //   label L5
        //   $t7 := false
        //   $t1 := $t7
        //   goto L2
        //   label L3
        //   $t8 := test_variant m::Option<m::Option<u64>>::Some($t0)
        //   if ($t8) goto L7 else goto L11
        //   label L7
        //   $t9 := borrow_variant_field<m::Option<m::Option<u64>>::Some>.value($t0)
        //   $t10 := test_variant m::Option<u64>::Some($t9)
        //   if ($t10) goto L8 else goto L6
        //   label L8
        //   $t11 := true
        //   $t1 := $t11
        //   goto L2
        //   label L6
        //   $t12 := 15621340914461310977
        //   abort($t12)
        //   label L2
        //   return $t1
        //   label L9
        //   drop($t0)
        //   goto L1
        //   label L10
        //   drop($t0)
        //   goto L5
        //   label L11
        //   drop($t0)
        //   goto L6
        // End raw stackless bytecode
        // Bytecode
        //   $t2 := test_variant m::Option<m::Option<u64>>::None($t0)
        //   if ($t2) {
        //     drop($t0)
        //     $t3 := false
        //     $t1 := $t3
        //   } else {
        //     $t4 := test_variant m::Option<m::Option<u64>>::Some($t0)
        //     if ($t4) {
        //       $t5 := borrow_variant_field<m::Option<m::Option<u64>>::Some>.value($t0)
        //       $t6 := test_variant m::Option<u64>::None($t5)
        //       if ($t6) {
        //         drop($t0)
        //         $t7 := false
        //         $t1 := $t7
        //         return $t1
        //         // returned path
        //       } else {
        //       }
        //     } else {
        //     }
        //     $t8 := test_variant m::Option<m::Option<u64>>::Some($t0)
        //     if ($t8) {
        //       $t9 := borrow_variant_field<m::Option<m::Option<u64>>::Some>.value($t0)
        //       $t10 := test_variant m::Option<u64>::Some($t9)
        //       if ($t10) {
        //         $t11 := true
        //         $t1 := $t11
        //       } else {
        //         $t12 := 15621340914461310977
        //         abort($t12)
        //         // aborted path
        //       }
        //     } else {
        //       drop($t0)
        //       $t12 := 15621340914461310977
        //       abort($t12)
        //       // aborted path
        //     }
        //   }
        //   return $t1
        //   // returned path
        //
        // End Bytecode
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
        // Raw stackless bytecode
        //   $t4 := borrow_local($t0)
        //   $t5 := test_variant m::Outer::None($t4)
        //   if ($t5) goto L7 else goto L0
        //   label L1
        //   $t6 := move($t0)
        //   unpack_variant m::Outer::None($t6)
        //   $t7 := 0
        //   $t2 := $t7
        //   goto L2
        //   label L0
        //   $t8 := test_variant m::Outer::One($t4)
        //   if ($t8) goto L8 else goto L3
        //   label L4
        //   $t9 := move($t0)
        //   $t10 := unpack_variant m::Outer::One($t9)
        //   $t2 := m::inner_value($t10)
        //   goto L2
        //   label L3
        //   $t11 := test_variant m::Outer::Two($t4)
        //   if ($t11) goto L6 else goto L5
        //   label L6
        //   $t12 := move($t0)
        //   ($t13, $t14) := unpack_variant m::Outer::Two($t12)
        //   $t3 := $t14
        //   $t15 := m::inner_value($t13)
        //   $t16 := borrow_local($t3)
        //   $t17 := borrow_field<m::Box>.x($t16)
        //   $t18 := read_ref($t17)
        //   $t2 := +($t15, $t18)
        //   goto L2
        //   label L5
        //   $t19 := 15621340914461310977
        //   abort($t19)
        //   label L2
        //   return $t2
        //   label L7
        //   drop($t4)
        //   goto L1
        //   label L8
        //   drop($t4)
        //   goto L4
        // End raw stackless bytecode
        // Bytecode
        //   $t4 := borrow_local($t0)
        //   $t5 := test_variant m::Outer::None($t4)
        //   if ($t5) {
        //     drop($t4)
        //     $t6 := move($t0)
        //     unpack_variant m::Outer::None($t6)
        //     $t7 := 0
        //     $t2 := $t7
        //   } else {
        //     $t8 := test_variant m::Outer::One($t4)
        //     if ($t8) {
        //       drop($t4)
        //       $t9 := move($t0)
        //       $t10 := unpack_variant m::Outer::One($t9)
        //       $t2 := m::inner_value($t10)
        //     } else {
        //       $t11 := test_variant m::Outer::Two($t4)
        //       if ($t11) {
        //         $t12 := move($t0)
        //         ($t13, $t14) := unpack_variant m::Outer::Two($t12)
        //         $t3 := $t14
        //         $t15 := m::inner_value($t13)
        //         $t16 := borrow_local($t3)
        //         $t17 := borrow_field<m::Box>.x($t16)
        //         $t18 := read_ref($t17)
        //         $t2 := +($t15, $t18)
        //       } else {
        //         $t19 := 15621340914461310977
        //         abort($t19)
        //         // aborted path
        //       }
        //     }
        //   }
        //   return $t2
        //   // returned path
        //
        // End Bytecode
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
        // Raw stackless bytecode
        //   $t4 := borrow_local($t0)
        //   $t5 := test_variant m::Outer::None($t4)
        //   if ($t5) goto L10 else goto L0
        //   label L1
        //   $t6 := move($t0)
        //   unpack_variant m::Outer::None($t6)
        //   $t7 := 0
        //   $t2 := $t7
        //   goto L2
        //   label L0
        //   $t8 := test_variant m::Outer::One($t4)
        //   if ($t8) goto L4 else goto L3
        //   label L4
        //   $t9 := borrow_variant_field<m::Outer::One>.i($t4)
        //   $t10 := test_variant m::Inner::Inner1($t9)
        //   if ($t10) goto L11 else goto L3
        //   label L5
        //   $t11 := move($t0)
        //   $t12 := unpack_variant m::Outer::One($t11)
        //   $t2 := unpack_variant m::Inner::Inner1($t12)
        //   goto L2
        //   label L3
        //   $t13 := test_variant m::Outer::One($t4)
        //   if ($t13) goto L12 else goto L6
        //   label L7
        //   $t14 := move($t0)
        //   $t15 := unpack_variant m::Outer::One($t14)
        //   $t2 := m::inner_value($t15)
        //   goto L2
        //   label L6
        //   $t16 := test_variant m::Outer::Two($t4)
        //   if ($t16) goto L9 else goto L8
        //   label L9
        //   $t17 := move($t0)
        //   ($t18, $t19) := unpack_variant m::Outer::Two($t17)
        //   $t3 := $t19
        //   $t20 := m::inner_value($t18)
        //   $t21 := borrow_local($t3)
        //   $t22 := borrow_field<m::Box>.x($t21)
        //   $t23 := read_ref($t22)
        //   $t2 := +($t20, $t23)
        //   goto L2
        //   label L8
        //   $t24 := 15621340914461310977
        //   abort($t24)
        //   label L2
        //   return $t2
        //   label L10
        //   drop($t4)
        //   goto L1
        //   label L11
        //   drop($t4)
        //   goto L5
        //   label L12
        //   drop($t4)
        //   goto L7
        // End raw stackless bytecode
        // Bytecode
        //   $t4 := borrow_local($t0)
        //   $t5 := test_variant m::Outer::None($t4)
        //   if ($t5) {
        //     drop($t4)
        //     $t6 := move($t0)
        //     unpack_variant m::Outer::None($t6)
        //     $t7 := 0
        //     $t2 := $t7
        //   } else {
        //     $t8 := test_variant m::Outer::One($t4)
        //     if ($t8) {
        //       $t9 := borrow_variant_field<m::Outer::One>.i($t4)
        //       $t10 := test_variant m::Inner::Inner1($t9)
        //       if ($t10) {
        //         drop($t4)
        //         $t11 := move($t0)
        //         $t12 := unpack_variant m::Outer::One($t11)
        //         $t2 := unpack_variant m::Inner::Inner1($t12)
        //         return $t2
        //         // returned path
        //       } else {
        //       }
        //     } else {
        //     }
        //     $t13 := test_variant m::Outer::One($t4)
        //     if ($t13) {
        //       drop($t4)
        //       $t14 := move($t0)
        //       $t15 := unpack_variant m::Outer::One($t14)
        //       $t2 := m::inner_value($t15)
        //     } else {
        //       $t16 := test_variant m::Outer::Two($t4)
        //       if ($t16) {
        //         $t17 := move($t0)
        //         ($t18, $t19) := unpack_variant m::Outer::Two($t17)
        //         $t3 := $t19
        //         $t20 := m::inner_value($t18)
        //         $t21 := borrow_local($t3)
        //         $t22 := borrow_field<m::Box>.x($t21)
        //         $t23 := read_ref($t22)
        //         $t2 := +($t20, $t23)
        //       } else {
        //         $t24 := 15621340914461310977
        //         abort($t24)
        //         // aborted path
        //       }
        //     }
        //   }
        //   return $t2
        //   // returned path
        //
        // End Bytecode
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
        // Raw stackless bytecode
        //   $t4 := borrow_local($t0)
        //   $t5 := test_variant m::Outer::None($t4)
        //   if ($t5) goto L10 else goto L0
        //   label L1
        //   $t6 := move($t0)
        //   unpack_variant m::Outer::None($t6)
        //   $t7 := 0
        //   $t2 := $t7
        //   goto L2
        //   label L0
        //   $t8 := test_variant m::Outer::One($t4)
        //   if ($t8) goto L4 else goto L3
        //   label L4
        //   $t9 := borrow_variant_field<m::Outer::One>.i($t4)
        //   $t10 := m::is_inner1($t9)
        //   if ($t10) goto L11 else goto L3
        //   label L5
        //   $t11 := move($t0)
        //   $t12 := unpack_variant m::Outer::One($t11)
        //   $t13 := m::inner_value($t12)
        //   $t14 := 2
        //   $t2 := %($t13, $t14)
        //   goto L2
        //   label L3
        //   $t15 := test_variant m::Outer::One($t4)
        //   if ($t15) goto L12 else goto L6
        //   label L7
        //   $t16 := move($t0)
        //   $t17 := unpack_variant m::Outer::One($t16)
        //   $t2 := m::inner_value($t17)
        //   goto L2
        //   label L6
        //   $t18 := test_variant m::Outer::Two($t4)
        //   if ($t18) goto L9 else goto L8
        //   label L9
        //   $t19 := move($t0)
        //   ($t20, $t21) := unpack_variant m::Outer::Two($t19)
        //   $t3 := $t21
        //   $t22 := m::inner_value($t20)
        //   $t23 := borrow_local($t3)
        //   $t24 := borrow_field<m::Box>.x($t23)
        //   $t25 := read_ref($t24)
        //   $t2 := +($t22, $t25)
        //   goto L2
        //   label L8
        //   $t26 := 15621340914461310977
        //   abort($t26)
        //   label L2
        //   return $t2
        //   label L10
        //   drop($t4)
        //   goto L1
        //   label L11
        //   drop($t4)
        //   goto L5
        //   label L12
        //   drop($t4)
        //   goto L7
        // End raw stackless bytecode
        // Bytecode
        //   $t4 := borrow_local($t0)
        //   $t5 := test_variant m::Outer::None($t4)
        //   if ($t5) {
        //     drop($t4)
        //     $t6 := move($t0)
        //     unpack_variant m::Outer::None($t6)
        //     $t7 := 0
        //     $t2 := $t7
        //   } else {
        //     $t8 := test_variant m::Outer::One($t4)
        //     if ($t8) {
        //       $t9 := borrow_variant_field<m::Outer::One>.i($t4)
        //       $t10 := m::is_inner1($t9)
        //       if ($t10) {
        //         drop($t4)
        //         $t11 := move($t0)
        //         $t12 := unpack_variant m::Outer::One($t11)
        //         $t13 := m::inner_value($t12)
        //         $t14 := 2
        //         $t2 := %($t13, $t14)
        //         return $t2
        //         // returned path
        //       } else {
        //       }
        //     } else {
        //     }
        //     $t15 := test_variant m::Outer::One($t4)
        //     if ($t15) {
        //       drop($t4)
        //       $t16 := move($t0)
        //       $t17 := unpack_variant m::Outer::One($t16)
        //       $t2 := m::inner_value($t17)
        //     } else {
        //       $t18 := test_variant m::Outer::Two($t4)
        //       if ($t18) {
        //         $t19 := move($t0)
        //         ($t20, $t21) := unpack_variant m::Outer::Two($t19)
        //         $t3 := $t21
        //         $t22 := m::inner_value($t20)
        //         $t23 := borrow_local($t3)
        //         $t24 := borrow_field<m::Box>.x($t23)
        //         $t25 := read_ref($t24)
        //         $t2 := +($t22, $t25)
        //       } else {
        //         $t26 := 15621340914461310977
        //         abort($t26)
        //         // aborted path
        //       }
        //     }
        //   }
        //   return $t2
        //   // returned path
        //
        // End Bytecode
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
        // Raw stackless bytecode
        //   $t2 := test_variant m::Outer::None($t0)
        //   if ($t2) goto L10 else goto L0
        //   label L1
        //   $t3 := false
        //   $t1 := $t3
        //   goto L2
        //   label L0
        //   $t4 := test_variant m::Outer::One($t0)
        //   if ($t4) goto L4 else goto L3
        //   label L4
        //   $t5 := borrow_variant_field<m::Outer::One>.i($t0)
        //   $t6 := m::is_inner1($t5)
        //   if ($t6) goto L11 else goto L3
        //   label L5
        //   $t7 := true
        //   $t1 := $t7
        //   goto L2
        //   label L3
        //   $t8 := test_variant m::Outer::One($t0)
        //   if ($t8) goto L7 else goto L6
        //   label L7
        //   $t9 := borrow_variant_field<m::Outer::One>.i($t0)
        //   $t1 := m::is_inner1($t9)
        //   goto L2
        //   label L6
        //   $t10 := test_variant m::Outer::Two($t0)
        //   if ($t10) goto L9 else goto L12
        //   label L9
        //   $t11 := borrow_variant_field<m::Outer::Two>.i($t0)
        //   $t1 := m::is_inner1($t11)
        //   goto L2
        //   label L8
        //   $t12 := 15621340914461310977
        //   abort($t12)
        //   label L2
        //   return $t1
        //   label L10
        //   drop($t0)
        //   goto L1
        //   label L11
        //   drop($t0)
        //   goto L5
        //   label L12
        //   drop($t0)
        //   goto L8
        // End raw stackless bytecode
        // Bytecode
        //   $t2 := test_variant m::Outer::None($t0)
        //   if ($t2) {
        //     drop($t0)
        //     $t3 := false
        //     $t1 := $t3
        //   } else {
        //     $t4 := test_variant m::Outer::One($t0)
        //     if ($t4) {
        //       $t5 := borrow_variant_field<m::Outer::One>.i($t0)
        //       $t6 := m::is_inner1($t5)
        //       if ($t6) {
        //         drop($t0)
        //         $t7 := true
        //         $t1 := $t7
        //         return $t1
        //         // returned path
        //       } else {
        //       }
        //     } else {
        //     }
        //     $t8 := test_variant m::Outer::One($t0)
        //     if ($t8) {
        //       $t9 := borrow_variant_field<m::Outer::One>.i($t0)
        //       $t1 := m::is_inner1($t9)
        //     } else {
        //       $t10 := test_variant m::Outer::Two($t0)
        //       if ($t10) {
        //         $t11 := borrow_variant_field<m::Outer::Two>.i($t0)
        //         $t1 := m::is_inner1($t11)
        //       } else {
        //         drop($t0)
        //         $t12 := 15621340914461310977
        //         abort($t12)
        //         // aborted path
        //       }
        //     }
        //   }
        //   return $t1
        //   // returned path
        //
        // End Bytecode
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
        // Raw stackless bytecode
        //   $t5 := borrow_local($t0)
        //   $t6 := borrow_variant_field<m::CommonFields::Foo|Bar>.x($t5)
        //   $t7 := read_ref($t6)
        //   $t8 := borrow_local($t0)
        //   $t9 := test_variant m::CommonFields::Foo($t8)
        //   if ($t9) goto L5 else goto L0
        //   label L1
        //   $t10 := move($t0)
        //   ($t11, $t12) := unpack_variant m::CommonFields::Foo($t10)
        //   $t4 := $t12
        //   drop($t11)
        //   goto L2
        //   label L0
        //   $t13 := test_variant m::CommonFields::Bar($t8)
        //   if ($t13) goto L4 else goto L3
        //   label L4
        //   $t14 := move($t0)
        //   ($t15, $t16) := unpack_variant m::CommonFields::Bar($t14)
        //   $t4 := $t16
        //   drop($t15)
        //   goto L2
        //   label L3
        //   $t17 := 15621340914461310977
        //   abort($t17)
        //   label L2
        //   $t18 := +($t7, $t4)
        //   return $t18
        //   label L5
        //   drop($t8)
        //   goto L1
        // End raw stackless bytecode
        // Bytecode
        //   $t5 := borrow_local($t0)
        //   $t6 := borrow_variant_field<m::CommonFields::Foo|Bar>.x($t5)
        //   $t7 := read_ref($t6)
        //   $t8 := borrow_local($t0)
        //   $t9 := test_variant m::CommonFields::Foo($t8)
        //   if ($t9) {
        //     drop($t8)
        //     $t10 := move($t0)
        //     ($t11, $t12) := unpack_variant m::CommonFields::Foo($t10)
        //     $t4 := $t12
        //     drop($t11)
        //   } else {
        //     $t13 := test_variant m::CommonFields::Bar($t8)
        //     if ($t13) {
        //       $t14 := move($t0)
        //       ($t15, $t16) := unpack_variant m::CommonFields::Bar($t14)
        //       $t4 := $t16
        //       drop($t15)
        //     } else {
        //       $t17 := 15621340914461310977
        //       abort($t17)
        //       // aborted path
        //     }
        //   }
        //   $t18 := +($t7, $t4)
        //   return $t18
        //   // returned path
        //
        // End Bytecode
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
        // Raw stackless bytecode
        //   $t3 := borrow_local($t0)
        //   $t4 := test_variant m::CommonFieldsAtDifferentOffset::Bar($t3)
        //   if ($t4) goto L0 else goto L1
        //   label L1
        //   $t5 := test_variant m::CommonFieldsAtDifferentOffset::Balt($t3)
        //   if ($t5) goto L0 else goto L3
        //   label L0
        //   $t2 := borrow_variant_field<m::CommonFieldsAtDifferentOffset::Bar|Balt>.z($t3)
        //   goto L4
        //   label L3
        //   $t2 := borrow_variant_field<m::CommonFieldsAtDifferentOffset::Baz>.z($t3)
        //   label L4
        //   $t6 := read_ref($t2)
        //   return $t6
        // End raw stackless bytecode
        // Bytecode
        //   $t3 := borrow_local($t0)
        //   $t4 := test_variant m::CommonFieldsAtDifferentOffset::Bar($t3)
        //   if ($t4) {
        //     $t2 := borrow_variant_field<m::CommonFieldsAtDifferentOffset::Bar|Balt>.z($t3)
        //   } else {
        //     $t2 := borrow_variant_field<m::CommonFieldsAtDifferentOffset::Baz>.z($t3)
        //   }
        //   $t6 := read_ref($t2)
        //   return $t6
        //   // returned path
        //
        // End Bytecode
        let v0 = &arg0;
        let v1 = if (test_variant(v0, CommonFieldsAtDifferentOffset::(Bar|Balt))) {
            &v0.z
        } else {
            &v0.z
        };
        *v1
    }

    // decompiled from Move bytecode v7
}
