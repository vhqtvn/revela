module 0x1::math64 {
    public fun average(arg0: u64, arg1: u64) : u64 {
        if (arg0 < arg1) {
            arg0 + (arg1 - arg0) / 2
        } else {
            arg1 + (arg0 - arg1) / 2
        }
    }
    
    public fun clamp(arg0: u64, arg1: u64, arg2: u64) : u64 {
        min(arg2, max(arg1, arg0))
    }
    
    public fun floor_log2(arg0: u64) : u8 {
        let v0 = 0;
        assert!(arg0 != 0, 0x1::error::invalid_argument(1));
        let v1 = 32;
        while (v1 > 0) {
            if (arg0 >= 1 << v1) {
                arg0 = arg0 >> v1;
                v0 = v0 + v1;
            };
            v1 = v1 >> 1;
        };
        v0
    }
    
    public fun log2(arg0: u64) : 0x1::fixed_point32::FixedPoint32 {
        let v0 = floor_log2(arg0);
        let v1 = if (arg0 >= 4294967296) {
            arg0 >> v0 - 32
        } else {
            arg0 << 32 - v0
        };
        let v2 = (v1 as u128);
        v1 = 2147483648;
        arg0 = 0;
        let v3 = v2;
        while (v1 != 0) {
            let v4 = v3 * v3 >> 32;
            v3 = v4;
            if (v4 >= 8589934592) {
                arg0 = arg0 + v1;
                v3 = v4 >> 1;
            };
            v1 = v1 >> 1;
        };
        0x1::fixed_point32::create_from_raw_value(((v0 as u64) << 32) + arg0)
    }
    
    public fun max(arg0: u64, arg1: u64) : u64 {
        if (arg0 >= arg1) {
            arg0
        } else {
            arg1
        }
    }
    
    public fun min(arg0: u64, arg1: u64) : u64 {
        if (arg0 < arg1) {
            arg0
        } else {
            arg1
        }
    }
    
    public fun pow(arg0: u64, arg1: u64) : u64 {
        if (arg1 == 0) {
            1
        } else {
            let v1 = 1;
            while (arg1 > 1) {
                if (arg1 % 2 == 1) {
                    v1 = v1 * arg0;
                };
                arg1 = arg1 / 2;
                arg0 = arg0 * arg0;
            };
            v1 * arg0
        }
    }
    
    public fun sqrt(arg0: u64) : u64 {
        if (arg0 == 0) {
            return 0
        };
        let v0 = 1 << floor_log2(arg0) + 1 >> 1;
        let v1 = v0 + arg0 / v0 >> 1;
        let v2 = v1 + arg0 / v1 >> 1;
        let v3 = v2 + arg0 / v2 >> 1;
        let v4 = v3 + arg0 / v3 >> 1;
        min(v4, arg0 / v4)
    }
    
    // decompiled from Move bytecode v7
}
