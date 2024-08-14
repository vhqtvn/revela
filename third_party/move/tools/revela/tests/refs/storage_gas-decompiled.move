module 0x1::storage_gas {
    struct GasCurve has copy, drop, store {
        min_gas: u64,
        max_gas: u64,
        points: vector<Point>,
    }

    struct Point has copy, drop, store {
        x: u64,
        y: u64,
    }

    struct StorageGas has key {
        per_item_read: u64,
        per_item_create: u64,
        per_item_write: u64,
        per_byte_read: u64,
        per_byte_create: u64,
        per_byte_write: u64,
    }

    struct StorageGasConfig has copy, drop, key {
        item_config: UsageGasConfig,
        byte_config: UsageGasConfig,
    }

    struct UsageGasConfig has copy, drop, store {
        target_usage: u64,
        read_curve: GasCurve,
        create_curve: GasCurve,
        write_curve: GasCurve,
    }

    public fun base_8192_exponential_curve(arg0: u64, arg1: u64) : GasCurve {
        let v0 = 0x1::vector::empty<Point>();
        let v1 = &mut v0;
        0x1::vector::push_back<Point>(v1, new_point(1000, 2));
        0x1::vector::push_back<Point>(v1, new_point(2000, 6));
        0x1::vector::push_back<Point>(v1, new_point(3000, 17));
        0x1::vector::push_back<Point>(v1, new_point(4000, 44));
        0x1::vector::push_back<Point>(v1, new_point(5000, 109));
        0x1::vector::push_back<Point>(v1, new_point(6000, 271));
        0x1::vector::push_back<Point>(v1, new_point(7000, 669));
        0x1::vector::push_back<Point>(v1, new_point(8000, 1648));
        0x1::vector::push_back<Point>(v1, new_point(9000, 4061));
        0x1::vector::push_back<Point>(v1, new_point(9500, 6372));
        0x1::vector::push_back<Point>(v1, new_point(9900, 9138));
        new_gas_curve(arg0, arg1, v0)
    }

    fun calculate_create_gas(arg0: &UsageGasConfig, arg1: u64) : u64 {
        calculate_gas(arg0.target_usage, arg1, &arg0.create_curve)
    }

    fun calculate_gas(arg0: u64, arg1: u64, arg2: &GasCurve) : u64 {
        let v0 = if (arg1 > arg0) {
            arg0
        } else {
            arg1
        };
        let v1 = &arg2.points;
        let v2 = 0x1::vector::length<Point>(v1);
        let v3 = v0 * 10000 / arg0;
        let (v4, v5) = if (v2 == 0) {
            let v6 = Point{
                x : 0,
                y : 0,
            };
            let v7 = Point{
                x : 10000,
                y : 10000,
            };
            (&v7, &v6)
        } else if (v3 < 0x1::vector::borrow<Point>(v1, 0).x) {
            let v8 = Point{
                x : 0,
                y : 0,
            };
            (0x1::vector::borrow<Point>(v1, 0), &v8)
        } else if (0x1::vector::borrow<Point>(v1, v2 - 1).x <= v3) {
            let v9 = Point{
                x : 10000,
                y : 10000,
            };
            (&v9, 0x1::vector::borrow<Point>(v1, v2 - 1))
        } else {
            let v10 = 0;
            let v11 = v2 - 2;
            while (v10 < v11) {
                let v12 = v11 - (v11 - v10) / 2;
                if (v3 < 0x1::vector::borrow<Point>(v1, v12).x) {
                    v11 = v12 - 1;
                    continue
                };
                v10 = v12;
            };
            (0x1::vector::borrow<Point>(v1, v10 + 1), 0x1::vector::borrow<Point>(v1, v10))
        };
        interpolate(0, 10000, arg2.min_gas, arg2.max_gas, interpolate(v5.x, v4.x, v5.y, v4.y, v3))
    }

    fun calculate_read_gas(arg0: &UsageGasConfig, arg1: u64) : u64 {
        calculate_gas(arg0.target_usage, arg1, &arg0.read_curve)
    }

    fun calculate_write_gas(arg0: &UsageGasConfig, arg1: u64) : u64 {
        calculate_gas(arg0.target_usage, arg1, &arg0.write_curve)
    }

    public fun initialize(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        if (exists<StorageGasConfig>(@0x1)) {
            abort 0x1::error::already_exists(0)
        };
        let v0 = base_8192_exponential_curve(300000, 30000000);
        let v1 = base_8192_exponential_curve(300000, 30000000);
        let v2 = UsageGasConfig{
            target_usage : 2000000000,
            read_curve   : base_8192_exponential_curve(300000, 30000000),
            create_curve : v0,
            write_curve  : v1,
        };
        let v3 = base_8192_exponential_curve(5000, 500000);
        let v4 = base_8192_exponential_curve(5000, 500000);
        let v5 = UsageGasConfig{
            target_usage : 1000000000000,
            read_curve   : base_8192_exponential_curve(300, 30000),
            create_curve : v3,
            write_curve  : v4,
        };
        let v6 = StorageGasConfig{
            item_config : v2,
            byte_config : v5,
        };
        move_to<StorageGasConfig>(arg0, v6);
        if (exists<StorageGas>(@0x1)) {
            abort 0x1::error::already_exists(1)
        };
        let v7 = StorageGas{
            per_item_read   : 300000,
            per_item_create : 5000000,
            per_item_write  : 300000,
            per_byte_read   : 300,
            per_byte_create : 5000,
            per_byte_write  : 5000,
        };
        move_to<StorageGas>(arg0, v7);
    }

    fun interpolate(arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) : u64 {
        arg2 + (arg4 - arg0) * (arg3 - arg2) / (arg1 - arg0)
    }

    public fun new_gas_curve(arg0: u64, arg1: u64, arg2: vector<Point>) : GasCurve {
        assert!(arg1 >= arg0, 0x1::error::invalid_argument(2));
        assert!(arg1 <= 1844674407370955, 0x1::error::invalid_argument(2));
        validate_points(&arg2);
        GasCurve{
            min_gas : arg0,
            max_gas : arg1,
            points  : arg2,
        }
    }

    public fun new_point(arg0: u64, arg1: u64) : Point {
        assert!(arg0 <= 10000 && arg1 <= 10000, 0x1::error::invalid_argument(6));
        Point{
            x : arg0,
            y : arg1,
        }
    }

    public fun new_storage_gas_config(arg0: UsageGasConfig, arg1: UsageGasConfig) : StorageGasConfig {
        StorageGasConfig{
            item_config : arg0,
            byte_config : arg1,
        }
    }

    public fun new_usage_gas_config(arg0: u64, arg1: GasCurve, arg2: GasCurve, arg3: GasCurve) : UsageGasConfig {
        assert!(arg0 > 0, 0x1::error::invalid_argument(3));
        assert!(arg0 <= 1844674407370955, 0x1::error::invalid_argument(4));
        UsageGasConfig{
            target_usage : arg0,
            read_curve   : arg1,
            create_curve : arg2,
            write_curve  : arg3,
        }
    }

    public(friend) fun on_reconfig() acquires StorageGas, StorageGasConfig {
        assert!(exists<StorageGasConfig>(@0x1), 0x1::error::not_found(0));
        assert!(exists<StorageGas>(@0x1), 0x1::error::not_found(1));
        let (v0, v1) = 0x1::state_storage::current_items_and_bytes();
        let v2 = borrow_global<StorageGasConfig>(@0x1);
        let v3 = borrow_global_mut<StorageGas>(@0x1);
        v3.per_item_read = calculate_read_gas(&v2.item_config, v0);
        v3.per_item_create = calculate_create_gas(&v2.item_config, v0);
        v3.per_item_write = calculate_write_gas(&v2.item_config, v0);
        v3.per_byte_read = calculate_read_gas(&v2.byte_config, v1);
        v3.per_byte_create = calculate_create_gas(&v2.byte_config, v1);
        v3.per_byte_write = calculate_write_gas(&v2.byte_config, v1);
    }

    public(friend) fun set_config(arg0: &signer, arg1: StorageGasConfig) acquires StorageGasConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        *borrow_global_mut<StorageGasConfig>(@0x1) = arg1;
    }

    fun validate_points(arg0: &vector<Point>) {
        let v0 = 0x1::vector::length<Point>(arg0);
        let v1 = 0;
        while (v1 <= v0) {
            let v2 = if (v1 == 0) {
                let v3 = Point{
                    x : 0,
                    y : 0,
                };
                &v3
            } else {
                0x1::vector::borrow<Point>(arg0, v1 - 1)
            };
            let v4 = if (v1 == v0) {
                let v5 = Point{
                    x : 10000,
                    y : 10000,
                };
                &v5
            } else {
                0x1::vector::borrow<Point>(arg0, v1)
            };
            assert!(v2.x < v4.x && v2.y <= v4.y, 0x1::error::invalid_argument(5));
            v1 = v1 + 1;
        };
    }

    // decompiled from Move bytecode v7
}
