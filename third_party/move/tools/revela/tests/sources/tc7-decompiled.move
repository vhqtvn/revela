module 0x12::tc7 {
    public fun foo(arg0: u64) : u64 {
        let v0 = arg0 + 1;
        let v1 = arg0 + 2;
        let v2 = 1;
        let v3 = v2;
        let v4 = v1;
        let v5 = v0;
        if (v0 == 2) {
            let v6 = arg0 + 2;
            let v7 = v6;
            if (v6 > 3) {
                let v8 = arg0 + 3;
                if (v8 > 10) {
                    v7 = v6 + 4 - v8;
                } else {
                    v7 = v6 - 6 - v8;
                };
                v3 = v0 + v8 + 1;
            } else {
                v5 = v2 - 5;
            };
            v4 = v1 + 7 - v7;
        } else {
            if (arg0 + 5 < 3) {
                let v9 = arg0 + 3;
                if (v9 < 10) {
                    v3 = v1 + 4;
                } else {
                    v4 = v2 + 6;
                };
                v5 = v0 + v9 + 2;
            };
            v5 = arg0 + 1 - v5;
        };
        v3 + v4 + v5 + 11
    }
    
    // decompiled from Move bytecode v7
}
