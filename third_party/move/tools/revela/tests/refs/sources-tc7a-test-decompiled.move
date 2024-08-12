module 0x12::tc7 {
    public fun foo(arg0: u64) : u64 {
        // Raw stackless bytecode
        //   $t8 := 1
        //   $t9 := +($t0, $t8)
        //   $t10 := 2
        //   $t11 := +($t0, $t10)
        //   $t12 := 1
        //   $t3 := $t12
        //   $t4 := $t11
        //   $t5 := $t9
        //   $t13 := 5
        //   $t14 := +($t0, $t13)
        //   $t15 := 133
        //   $t16 := <($t14, $t15)
        //   if ($t16) goto L1 else goto L0
        //   label L1
        //   $t17 := 3
        //   $t18 := +($t0, $t17)
        //   $t19 := 10
        //   $t20 := <($t18, $t19)
        //   if ($t20) goto L3 else goto L2
        //   label L3
        //   $t21 := 4
        //   $t3 := +($t11, $t21)
        //   goto L4
        //   label L2
        //   $t22 := 9
        //   $t4 := $t22
        //   label L4
        //   $t23 := +($t9, $t18)
        //   $t24 := 2
        //   $t5 := +($t23, $t24)
        //   label L0
        //   $t25 := +($t3, $t4)
        //   $t26 := +($t25, $t5)
        //   $t27 := 11
        //   $t28 := +($t26, $t27)
        //   return $t28
        // End raw stackless bytecode
        // Bytecode
        //   $t8 := 1
        //   $t9 := +($t0, $t8)
        //   $t10 := 2
        //   $t11 := +($t0, $t10)
        //   $t12 := 1
        //   $t3 := $t12
        //   $t4 := $t11
        //   $t5 := $t9
        //   $t13 := 5
        //   $t14 := +($t0, $t13)
        //   $t15 := 133
        //   $t16 := <($t14, $t15)
        //   if ($t16) {
        //     $t17 := 3
        //     $t18 := +($t0, $t17)
        //     $t19 := 10
        //     $t20 := <($t18, $t19)
        //     if ($t20) {
        //       $t21 := 4
        //       $t3 := +($t11, $t21)
        //     } else {
        //       $t22 := 9
        //       $t4 := $t22
        //     }
        //     $t23 := +($t9, $t18)
        //     $t24 := 2
        //     $t5 := +($t23, $t24)
        //   } else {
        //   }
        //   $t25 := +($t3, $t4)
        //   $t26 := +($t25, $t5)
        //   $t27 := 11
        //   $t28 := +($t26, $t27)
        //   return $t28
        //   // returned path
        //   
        // End Bytecode
        let v0 = arg0 + 1;
        let v1 = arg0 + 2;
        let v2 = 1;
        let v3 = v1;
        let v4 = v0;
        if (arg0 + 5 < 133) {
            let v5 = arg0 + 3;
            if (v5 < 10) {
                v2 = v1 + 4;
            } else {
                v3 = 9;
            };
            v4 = v0 + v5 + 2;
        };
        v2 + v3 + v4 + 11
    }
    
    // decompiled from Move bytecode v7
}
