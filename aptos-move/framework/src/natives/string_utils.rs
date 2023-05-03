// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeError, SafeNativeResult},
    safely_pop_arg,
};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use ark_std::iterable::Iterable;
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{GasQuantity, InternalGas},
    language_storage::TypeTag,
    u256,
    value::{MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Reference, Struct, Value, Vector, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, fmt::Write, ops::Deref, sync::Arc};

struct FormatContext<'a, 'b, 'c, 'd, 'e> {
    context: &'d mut SafeNativeContext<'a, 'b, 'c, 'e>,
    base_gas: InternalGas,
    per_byte_gas: InternalGas,
    max_depth: usize,
    max_len: usize,
    type_tag: bool,
    canonicalize: bool,
    single_line: bool,
    include_int_type: bool,
}

/// Converts a `MoveValue::Vector` of `u8`'s to a `String` by wrapping it in double quotes and
/// escaping double quotes and backslashes.
///
/// Examples:
///  - 'Hello' returns "Hello"
///  - '"Hello?" What are you saying?' returns "\"Hello?\" What are you saying?"
///  - '\ and " are escaped' returns "\\ and \" are escaped"
fn bytes_as_escaped_string(buf: &str) -> String {
    let str = String::from(buf);

    // We need to escape displayed double quotes " as \" and, as a result, also escape
    // displayed \ as \\.
    str.replace('\\', "\\\\").replace('"', "\\\"")
}

fn print_space_or_newline(newline: bool, out: &mut String, depth: usize) {
    if newline {
        out.push('\n');
        for _ in 0..depth {
            // add 2 spaces
            write!(out, "  ").unwrap();
        }
    } else {
        out.push(' ');
    }
}

fn primitive_type(ty: &MoveTypeLayout) -> bool {
    !matches!(ty, MoveTypeLayout::Vector(_) | MoveTypeLayout::Struct(_))
}

trait MoveLayout {
    fn write_name(&self, out: &mut String);
    fn get_layout(&self) -> &MoveTypeLayout;
}

impl MoveLayout for MoveFieldLayout {
    fn write_name(&self, out: &mut String) {
        write!(out, "{}: ", self.name).unwrap();
    }

    fn get_layout(&self) -> &MoveTypeLayout {
        &self.layout
    }
}

impl MoveLayout for MoveTypeLayout {
    fn write_name(&self, _out: &mut String) {}

    fn get_layout(&self) -> &MoveTypeLayout {
        self
    }
}

fn format_vector<'a>(
    context: &mut FormatContext,
    fields: impl Iterator<Item = &'a (impl MoveLayout + 'a)>,
    values: Vec<Value>,
    depth: usize,
    newline: bool,
    out: &mut String,
) -> SafeNativeResult<()> {
    if values.is_empty() {
        return Ok(());
    }
    if depth >= context.max_depth {
        write!(out, " .. ").unwrap();
        return Ok(());
    }
    print_space_or_newline(newline, out, depth + 1);
    for (i, (ty, val)) in fields.zip(values.into_iter()).enumerate() {
        if i > 0 {
            out.push(',');
            print_space_or_newline(newline, out, depth + 1);
        }
        if i >= context.max_len {
            write!(out, "..").unwrap();
            break;
        }
        ty.write_name(out);
        native_format_impl(context, ty.get_layout(), val, depth + 1, out)?;
    }
    print_space_or_newline(newline, out, depth);
    Ok(())
}

fn native_format_impl(
    context: &mut FormatContext,
    layout: &MoveTypeLayout,
    val: Value,
    depth: usize,
    out: &mut String,
) -> SafeNativeResult<()> {
    context.context.charge(context.base_gas)?;
    let mut suffix = "";
    match layout {
        MoveTypeLayout::Bool => {
            let b = val.value_as::<bool>()?;
            write!(out, "{}", b).unwrap();
        },
        MoveTypeLayout::U8 => {
            let u = val.value_as::<u8>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u8";
        },
        MoveTypeLayout::U64 => {
            let u = val.value_as::<u64>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u64";
        },
        MoveTypeLayout::U128 => {
            let u = val.value_as::<u128>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u128";
        },
        MoveTypeLayout::U16 => {
            let u = val.value_as::<u16>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u16";
        },
        MoveTypeLayout::U32 => {
            let u = val.value_as::<u32>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u32";
        },
        MoveTypeLayout::U256 => {
            let u = val.value_as::<u256::U256>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u256";
        },
        MoveTypeLayout::Address => {
            let addr = val.value_as::<move_core_types::account_address::AccountAddress>()?;
            let str = if context.canonicalize {
                addr.to_canonical_string()
            } else {
                addr.to_hex_literal()
            };
            write!(out, "@{}", str).unwrap();
        },
        MoveTypeLayout::Signer => {
            let addr = val.value_as::<move_core_types::account_address::AccountAddress>()?;
            let str = if context.canonicalize {
                addr.to_canonical_string()
            } else {
                addr.to_hex_literal()
            };
            write!(out, "signer({})", str).unwrap();
        },
        MoveTypeLayout::Vector(ty) => {
            if let MoveTypeLayout::U8 = ty.as_ref() {
                let bytes = val.value_as::<Vec<u8>>()?;
                write!(out, "0x{}", hex::encode(bytes)).unwrap();
                return Ok(());
            }
            let values = val.value_as::<Vector>()?.unpack_unchecked()?;
            out.push('[');
            format_vector(
                context,
                std::iter::repeat(ty.as_ref()).take(values.len()),
                values,
                depth,
                !context.single_line && !primitive_type(ty.as_ref()),
                out,
            )?;
            out.push(']');
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithTypes { type_, fields, .. }) => {
            let strct = val.value_as::<Struct>()?;
            if type_.name.as_str() == "String"
                && type_.module.as_str() == "string"
                && type_.address == AccountAddress::ONE
            {
                let v = strct.unpack()?.next().unwrap().value_as::<Vec<u8>>()?;
                context
                    .context
                    .charge(GasQuantity::from(v.len() as u64) * context.per_byte_gas)?;
                write!(
                    out,
                    "\"{}\"",
                    bytes_as_escaped_string(std::str::from_utf8(&v).unwrap())
                )
                .unwrap();
                return Ok(());
            } else if type_.name.as_str() == "Option"
                && type_.module.as_str() == "option"
                && type_.address == AccountAddress::ONE
            {
                let mut v = strct
                    .unpack()?
                    .next()
                    .unwrap()
                    .value_as::<Vector>()?
                    .unpack_unchecked()?;
                if v.is_empty() {
                    out.push_str("None");
                } else {
                    out.push_str("Some(");
                    let inner_ty = if let MoveTypeLayout::Vector(inner_ty) = &fields[0].layout {
                        inner_ty.deref()
                    } else {
                        unreachable!()
                    };
                    native_format_impl(context, inner_ty, v.pop().unwrap(), depth, out)?;
                    out.push(')');
                }
                return Ok(());
            }
            if context.type_tag {
                write!(out, "{} {{", TypeTag::from(type_.clone())).unwrap();
            } else {
                write!(out, "{} {{", type_.name.as_str()).unwrap();
            };
            format_vector(
                context,
                fields.iter(),
                strct.unpack()?.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push('}');
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithFields(fields)) => {
            let strct = val.value_as::<Struct>()?;
            out.push('{');
            format_vector(
                context,
                fields.iter(),
                strct.unpack()?.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push('}');
        },
        MoveTypeLayout::Struct(MoveStructLayout::Runtime(fields)) => {
            let strct = val.value_as::<Struct>()?;
            out.push('{');
            format_vector(
                context,
                fields.iter(),
                strct.unpack()?.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push('}');
        },
    };
    if context.include_int_type {
        write!(out, "{}", suffix).unwrap();
    };
    Ok(())
}

/// For old debug implementation
/// TODO: remove when old framework is completely removed
pub(crate) fn native_format_debug(
    context: &mut SafeNativeContext,
    ty: &Type,
    v: Value,
) -> SafeNativeResult<String> {
    let layout = context.deref().type_to_fully_annotated_layout(ty)?.unwrap();
    let mut format_context = FormatContext {
        context,
        base_gas: 0.into(),
        per_byte_gas: 0.into(),
        max_depth: usize::MAX,
        max_len: usize::MAX,
        type_tag: true,
        canonicalize: false,
        single_line: false,
        include_int_type: false,
    };
    let mut out = String::new();
    native_format_impl(&mut format_context, &layout, v, 0, &mut out)?;
    Ok(out)
}

fn native_format(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    let ty = context
        .deref()
        .type_to_fully_annotated_layout(&ty_args[0])?
        .unwrap();
    let include_int_type = safely_pop_arg!(arguments, bool);
    let single_line = safely_pop_arg!(arguments, bool);
    let canonicalize = safely_pop_arg!(arguments, bool);
    let type_tag = safely_pop_arg!(arguments, bool);
    let x = safely_pop_arg!(arguments, Reference);
    let v = x.read_ref().map_err(SafeNativeError::InvariantViolation)?;
    let mut out = String::new();
    let mut format_context = FormatContext {
        context,
        base_gas: gas_params.base,
        per_byte_gas: gas_params.per_byte,
        max_depth: usize::MAX,
        max_len: usize::MAX,
        type_tag,
        canonicalize,
        single_line,
        include_int_type,
    };
    native_format_impl(&mut format_context, &ty, v, 0, &mut out)?;
    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(out.into_bytes())]));
    Ok(smallvec![move_str])
}

fn native_format_list(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    let mut list_ty = &ty_args[0];

    let arg_mismatch = 1;
    let invalid_fmt = 2;

    let val = safely_pop_arg!(arguments, Reference);
    let mut val = val
        .read_ref()
        .map_err(SafeNativeError::InvariantViolation)?;

    let fmt_ref = safely_pop_arg!(arguments, VectorRef);
    let fmt_ref2 = fmt_ref.as_bytes_ref();
    // Could use unsafe here, but it's forbidden in this crate.
    let fmt = std::str::from_utf8(fmt_ref2.as_slice()).map_err(|_| SafeNativeError::Abort {
        abort_code: invalid_fmt,
    })?;

    context.charge(gas_params.per_byte * GasQuantity::from(fmt.len() as u64))?;

    let match_list_ty = |context: &mut SafeNativeContext, list_ty, name| {
        if let TypeTag::Struct(struct_tag) = context
            .type_to_type_tag(list_ty)
            .map_err(SafeNativeError::InvariantViolation)?
        {
            if !(struct_tag.address == AccountAddress::ONE
                && struct_tag.module.as_str() == "string_utils"
                && struct_tag.name.as_str() == name)
            {
                return Err(SafeNativeError::Abort {
                    abort_code: arg_mismatch,
                });
            }
            Ok(())
        } else {
            Err(SafeNativeError::Abort {
                abort_code: arg_mismatch,
            })
        }
    };

    let mut out = String::new();
    let mut in_braces = 0;
    for c in fmt.chars() {
        if in_braces == 1 {
            in_braces = 0;
            if c == '}' {
                // verify`that the type is a list
                match_list_ty(context, list_ty, "Cons")?;

                // We know that the type is a list, so we can safely unwrap
                let ty_args = if let Type::StructInstantiation(_, ty_args) = list_ty {
                    ty_args
                } else {
                    unreachable!()
                };
                let mut it = val.value_as::<Struct>()?.unpack()?;
                let car = it.next().unwrap();
                val = it.next().unwrap();
                list_ty = &ty_args[1];

                let ty = context
                    .type_to_fully_annotated_layout(&ty_args[0])?
                    .unwrap();
                let mut format_context = FormatContext {
                    context,
                    base_gas: gas_params.base,
                    per_byte_gas: gas_params.per_byte,
                    max_depth: usize::MAX,
                    max_len: usize::MAX,
                    type_tag: true,
                    canonicalize: false,
                    single_line: true,
                    include_int_type: false,
                };
                native_format_impl(&mut format_context, &ty, car, 0, &mut out)?;
                continue;
            } else if c != '{' {
                return Err(SafeNativeError::Abort {
                    abort_code: invalid_fmt,
                });
            }
        } else if in_braces == -1 {
            in_braces = 0;
            if c != '}' {
                return Err(SafeNativeError::Abort {
                    abort_code: invalid_fmt,
                });
            }
        } else if c == '{' {
            in_braces = 1;
            continue;
        } else if c == '}' {
            in_braces = -1;
            continue;
        }
        out.push(c);
    }
    if in_braces != 0 {
        return Err(SafeNativeError::Abort {
            abort_code: invalid_fmt,
        });
    }
    match_list_ty(context, list_ty, "NIL")?;

    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(out.into_bytes())]));
    Ok(smallvec![move_str])
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGas,
}

pub fn make_all(
    gas_param: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "native_format",
            make_safe_native(
                gas_param.clone(),
                timed_features.clone(),
                features.clone(),
                native_format,
            ),
        ),
        (
            "native_format_list",
            make_safe_native(gas_param, timed_features, features, native_format_list),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
