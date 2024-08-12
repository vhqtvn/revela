module 0x12::tc11 {
    public fun foo() : u64 {
        let v0 = 1;
        while (v0 < 5) {
            let v1 = v0 + 1;
            while (v1 >= 0) {
                let v2 = v1 + 2;
                v1 = v1 + 1;
                let v3 = v2;
                while (v3 != 7) {
                    let v4 = v3 + 1;
                    v3 = v4;
                    v1 = v1 - v4;
                };
                let v5 = v1 + 3;
                v1 = v5 - v3;
            };
            v0 = v0 + v1;
        };
        0 + v0 + 99
    }
    
    // decompiled from Move bytecode v7
}
