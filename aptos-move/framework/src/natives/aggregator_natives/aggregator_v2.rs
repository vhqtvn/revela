// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::{
    aggregator_natives::helpers_v2::{
        aggregator_snapshot_field_value, get_aggregator_fields_u128, get_aggregator_fields_u64,
        set_aggregator_value_field, string_to_bytes, to_utf8_bytes, u128_to_u64,
    },
    AccountAddress,
};
use aptos_gas_algebra::NumBytes;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, ops::Deref};

/// The generic type supplied to aggregator snapshots is not supported.
pub const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 0x03_0005;

/// The aggregator api feature is not enabled.
pub const EAGGREGATOR_API_NOT_ENABLED: u64 = 0x03_0006;

/// The generic type supplied to the aggregators is not supported.
pub const EUNSUPPORTED_AGGREGATOR_TYPE: u64 = 0x03_0007;

/// Arguments passed to concat exceed max limit of 256 bytes (for prefix and suffix together).
pub const ECONCAT_STRING_LENGTH_TOO_LARGE: u64 = 0x03_0008;

/// The native aggregator function, that is in the move file, is not yet supported.
/// and any calls will raise this error.
pub const EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED: u64 = 0x03_0009;

pub const CONCAT_PREFIX_AND_SUFFIX_MAX_LENGTH: usize = 256;

/// Checks if the type argument `type_arg` is a string type.
fn is_string_type(context: &SafeNativeContext, type_arg: &Type) -> SafeNativeResult<bool> {
    let ty = context.deref().type_to_fully_annotated_layout(type_arg)?;
    if let MoveTypeLayout::Struct(MoveStructLayout::WithTypes { type_, .. }) = ty {
        return Ok(type_.name.as_str() == "String"
            && type_.module.as_str() == "string"
            && type_.address == AccountAddress::ONE);
    }
    Ok(false)
}

/// Given the native function argument and a type, returns a tuple of its
/// fields: (`aggregator id`, `max_value`).
pub fn get_aggregator_fields_by_type(
    ty_arg: &Type,
    agg: &StructRef,
) -> SafeNativeResult<(u128, u128)> {
    match ty_arg {
        Type::U128 => {
            let (id, max_value) = get_aggregator_fields_u128(agg)?;
            Ok((id, max_value))
        },
        Type::U64 => {
            let (id, max_value) = get_aggregator_fields_u64(agg)?;
            Ok((id as u128, max_value as u128))
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

/// Given the list of native function arguments and a type, pop the next argument if it is of given type.
pub fn pop_value_by_type(ty_arg: &Type, args: &mut VecDeque<Value>) -> SafeNativeResult<u128> {
    match ty_arg {
        Type::U128 => Ok(safely_pop_arg!(args, u128)),
        Type::U64 => Ok(safely_pop_arg!(args, u64) as u128),
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

pub fn create_value_by_type(ty_arg: &Type, value: u128) -> SafeNativeResult<Value> {
    match ty_arg {
        Type::U128 => Ok(Value::u128(value)),
        Type::U64 => Ok(Value::u64(u128_to_u64(value)?)),
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

// To avoid checking is_string_type multiple times, check type_arg only once, and convert into this enum
enum SnapshotType {
    U128,
    U64,
    String,
}

impl SnapshotType {
    fn from_ty_arg(context: &SafeNativeContext, ty_arg: &Type) -> SafeNativeResult<Self> {
        match ty_arg {
            Type::U128 => Ok(Self::U128),
            Type::U64 => Ok(Self::U64),
            _ => {
                // Check if the type is a string
                if is_string_type(context, ty_arg)? {
                    Ok(Self::String)
                } else {
                    // If not a string, return an error
                    Err(SafeNativeError::Abort {
                        abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
                    })
                }
            },
        }
    }

    pub fn pop_snapshot_field_by_type(
        &self,
        args: &mut VecDeque<Value>,
    ) -> SafeNativeResult<SnapshotValue> {
        self.parse_snapshot_value_by_type(aggregator_snapshot_field_value(&safely_pop_arg!(
            args, StructRef
        ))?)
    }

    pub fn pop_snapshot_value_by_type(
        &self,
        args: &mut VecDeque<Value>,
    ) -> SafeNativeResult<SnapshotValue> {
        match self {
            SnapshotType::U128 => Ok(SnapshotValue::Integer(safely_pop_arg!(args, u128))),
            SnapshotType::U64 => Ok(SnapshotValue::Integer(safely_pop_arg!(args, u64) as u128)),
            SnapshotType::String => {
                let input = string_to_bytes(safely_pop_arg!(args, Struct))?;
                Ok(SnapshotValue::String(input))
            },
        }
    }

    pub fn parse_snapshot_value_by_type(&self, value: Value) -> SafeNativeResult<SnapshotValue> {
        // Simpler to wrap to be able to reuse safely_pop_arg functions
        self.pop_snapshot_value_by_type(&mut VecDeque::from([value]))
    }

    pub fn create_snapshot_value_by_type(&self, value: SnapshotValue) -> SafeNativeResult<Value> {
        match (self, value) {
            (SnapshotType::U128, SnapshotValue::Integer(v)) => Ok(Value::u128(v)),
            (SnapshotType::U64, SnapshotValue::Integer(v)) => Ok(Value::u64(u128_to_u64(v)?)),
            (SnapshotType::String, value) => {
                Ok(Value::struct_(Struct::pack(vec![Value::vector_u8(
                    match value {
                        SnapshotValue::String(v) => v,
                        SnapshotValue::Integer(v) => to_utf8_bytes(v),
                    },
                )])))
            },
            // Type cannot be Integer, if value is String
            _ => Err(SafeNativeError::Abort {
                abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
            }),
        }
    }
}

// ================= START TEMPORARY CODE =================
// TODO: aggregator_v2 branch will introduce these in different places in code

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotValue {
    Integer(u128),
    String(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotToStringFormula {
    Concat { prefix: Vec<u8>, suffix: Vec<u8> },
}

impl SnapshotToStringFormula {
    pub fn apply_to(&self, base: u128) -> Vec<u8> {
        match self {
            SnapshotToStringFormula::Concat { prefix, suffix } => {
                let middle_string = base.to_string();
                let middle = middle_string.as_bytes();
                let mut result = Vec::with_capacity(prefix.len() + middle.len() + suffix.len());
                result.extend(prefix);
                result.extend(middle);
                result.extend(suffix);
                result
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BoundedMathError {
    Overflow,
    Underflow,
}

// Unsigned operations operate on [0, max_value] range.
// Signed operations operate on [-max_value, max_value] range.
pub struct BoundedMath {
    max_value: u128,
}

impl BoundedMath {
    pub fn new(max_value: u128) -> Self {
        Self { max_value }
    }

    pub fn get_max_value(&self) -> u128 {
        self.max_value
    }

    pub fn unsigned_add(&self, base: u128, value: u128) -> Result<u128, BoundedMathError> {
        if self.max_value < base || value > (self.max_value - base) {
            Err(BoundedMathError::Overflow)
        } else {
            Ok(base + value)
        }
    }

    pub fn unsigned_subtract(&self, base: u128, value: u128) -> Result<u128, BoundedMathError> {
        if value > base {
            Err(BoundedMathError::Underflow)
        } else {
            Ok(base - value)
        }
    }
}

// ================= END TEMPORARY CODE =================

macro_rules! abort_if_not_enabled {
    ($context:expr) => {
        if !$context.aggregator_v2_api_enabled() {
            return Err(SafeNativeError::Abort {
                abort_code: EAGGREGATOR_API_NOT_ENABLED,
            });
        }
    };
}

/***************************************************************************************************
 * native fun create_aggregator<IntElement>(max_value: IntElement): Aggregator<IntElement>;
 **************************************************************************************************/

fn native_create_aggregator(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);

    context.charge(AGGREGATOR_V2_CREATE_AGGREGATOR_BASE)?;
    let max_value = pop_value_by_type(&ty_args[0], &mut args)?;

    let value_field_value = 0;

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        create_value_by_type(&ty_args[0], value_field_value)?,
        create_value_by_type(&ty_args[0], max_value)?,
    ]))])
}

/***************************************************************************************************
 * native fun create_unbounded_aggregator<IntElement: copy + drop>(): Aggregator<IntElement>;
 **************************************************************************************************/

fn native_create_unbounded_aggregator(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 0);
    debug_assert_eq!(ty_args.len(), 1);

    context.charge(AGGREGATOR_V2_CREATE_AGGREGATOR_BASE)?;
    let max_value = {
        match &ty_args[0] {
            Type::U128 => u128::MAX,
            Type::U64 => u64::MAX as u128,
            _ => {
                return Err(SafeNativeError::Abort {
                    abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
                })
            },
        }
    };

    let value_field_value = 0;

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        create_value_by_type(&ty_args[0], value_field_value)?,
        create_value_by_type(&ty_args[0], max_value)?,
    ]))])
}

/***************************************************************************************************
 * native fun try_add<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;
 **************************************************************************************************/
fn native_try_add(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 2);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_TRY_ADD_BASE)?;

    let input = pop_value_by_type(&ty_args[0], &mut args)?;
    let agg_struct = safely_pop_arg!(args, StructRef);
    let (agg_value, agg_max_value) = get_aggregator_fields_by_type(&ty_args[0], &agg_struct)?;

    let result_value = {
        let math = BoundedMath::new(agg_max_value);
        match math.unsigned_add(agg_value, input) {
            Ok(sum) => {
                set_aggregator_value_field(&agg_struct, create_value_by_type(&ty_args[0], sum)?)?;
                true
            },
            Err(_) => false,
        }
    };

    Ok(smallvec![Value::bool(result_value)])
}

/***************************************************************************************************
 * native fun try_sub<IntElement>(aggregator: &mut Aggregator<IntElement>, value: IntElement): bool;
 **************************************************************************************************/
fn native_try_sub(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 2);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_TRY_SUB_BASE)?;

    let input = pop_value_by_type(&ty_args[0], &mut args)?;
    let agg_struct = safely_pop_arg!(args, StructRef);
    let (agg_value, agg_max_value) = get_aggregator_fields_by_type(&ty_args[0], &agg_struct)?;

    let result_value = {
        let math = BoundedMath::new(agg_max_value);
        match math.unsigned_subtract(agg_value, input) {
            Ok(sum) => {
                set_aggregator_value_field(&agg_struct, create_value_by_type(&ty_args[0], sum)?)?;
                true
            },
            Err(_) => false,
        }
    };
    Ok(smallvec![Value::bool(result_value)])
}

/***************************************************************************************************
 * native fun read<IntElement>(aggregator: &Aggregator<IntElement>): IntElement;
 **************************************************************************************************/

fn native_read(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_READ_BASE)?;

    let (agg_value, agg_max_value) =
        get_aggregator_fields_by_type(&ty_args[0], &safely_pop_arg!(args, StructRef))?;

    let result_value = agg_value;

    if result_value > agg_max_value {
        return Err(SafeNativeError::InvariantViolation(PartialVMError::new(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        )));
    };
    Ok(smallvec![create_value_by_type(&ty_args[0], result_value)?])
}

/***************************************************************************************************
 * native fun snapshot<IntElement>(aggregator: &Aggregator<IntElement>): AggregatorSnapshot<IntElement>;
 **************************************************************************************************/

fn native_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(args.len(), 1);
    debug_assert_eq!(ty_args.len(), 1);
    context.charge(AGGREGATOR_V2_SNAPSHOT_BASE)?;

    let (agg_value, _agg_max_value) =
        get_aggregator_fields_by_type(&ty_args[0], &safely_pop_arg!(args, StructRef))?;

    let result_value = agg_value;

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        create_value_by_type(&ty_args[0], result_value)?
    ]))])
}

/***************************************************************************************************
 * native fun create_snapshot<Element>(value: Element): AggregatorSnapshot<Element>
 **************************************************************************************************/

fn native_create_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_CREATE_SNAPSHOT_BASE)?;

    let snapshot_type = SnapshotType::from_ty_arg(context, &ty_args[0])?;
    let input = snapshot_type.pop_snapshot_value_by_type(&mut args)?;

    let result_value = input;

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        snapshot_type.create_snapshot_value_by_type(result_value)?
    ]))])
}

/***************************************************************************************************
 * native fun copy_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): AggregatorSnapshot<Element>
 **************************************************************************************************/

fn native_copy_snapshot(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);
    Err(SafeNativeError::Abort {
        abort_code: EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED,
    })

    // debug_assert_eq!(ty_args.len(), 1);
    // debug_assert_eq!(args.len(), 1);
    // context.charge(AGGREGATOR_V2_COPY_SNAPSHOT_BASE)?;

    // let snapshot_type = SnapshotType::from_ty_arg(context, &ty_args[0])?;
    // let snapshot_value = snapshot_type.pop_snapshot_field_by_type(&mut args)?;

    // let result_value = snapshot_value;

    // Ok(smallvec![Value::struct_(Struct::pack(vec![
    //     snapshot_type.create_snapshot_value_by_type(result_value)?
    // ]))])
}

/***************************************************************************************************
 * native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;
 **************************************************************************************************/

fn native_read_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_READ_SNAPSHOT_BASE)?;

    let snapshot_type = SnapshotType::from_ty_arg(context, &ty_args[0])?;
    let snapshot_value = snapshot_type.pop_snapshot_field_by_type(&mut args)?;

    let result_value = snapshot_value;

    Ok(smallvec![
        snapshot_type.create_snapshot_value_by_type(result_value)?
    ])
}

/***************************************************************************************************
 * native fun string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): AggregatorSnapshot<String>;
 **************************************************************************************************/

fn native_string_concat(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    abort_if_not_enabled!(context);

    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 3);
    context.charge(AGGREGATOR_V2_STRING_CONCAT_BASE)?;

    let snapshot_input_type = SnapshotType::from_ty_arg(context, &ty_args[0])?;

    // Concat works only with integer snapshot types
    // This is to avoid unnecessary recursive snapshot dependencies
    if !matches!(snapshot_input_type, SnapshotType::U128 | SnapshotType::U64) {
        return Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
        });
    }

    // popping arguments from the end
    let suffix = string_to_bytes(safely_pop_arg!(args, Struct))?;
    let snapshot_value = match snapshot_input_type.pop_snapshot_field_by_type(&mut args)? {
        SnapshotValue::Integer(v) => v,
        SnapshotValue::String(_) => {
            return Err(SafeNativeError::Abort {
                abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
            })
        },
    };

    let prefix = string_to_bytes(safely_pop_arg!(args, Struct))?;

    if prefix
        .len()
        .checked_add(suffix.len())
        .map_or(false, |v| v > CONCAT_PREFIX_AND_SUFFIX_MAX_LENGTH)
    {
        return Err(SafeNativeError::Abort {
            abort_code: ECONCAT_STRING_LENGTH_TOO_LARGE,
        });
    }

    context.charge(STRING_UTILS_PER_BYTE * NumBytes::new((prefix.len() + suffix.len()) as u64))?;

    let result_value = SnapshotValue::String(
        SnapshotToStringFormula::Concat { prefix, suffix }.apply_to(snapshot_value),
    );

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        SnapshotType::String.create_snapshot_value_by_type(result_value)?
    ]))])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "create_aggregator",
            native_create_aggregator as RawSafeNative,
        ),
        (
            "create_unbounded_aggregator",
            native_create_unbounded_aggregator,
        ),
        ("try_add", native_try_add),
        ("read", native_read),
        ("try_sub", native_try_sub),
        ("snapshot", native_snapshot),
        ("create_snapshot", native_create_snapshot),
        ("copy_snapshot", native_copy_snapshot),
        ("read_snapshot", native_read_snapshot),
        ("string_concat", native_string_concat),
    ];
    builder.make_named_natives(natives)
}
