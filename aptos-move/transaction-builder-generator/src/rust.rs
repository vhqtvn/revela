// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common;
use aptos_types::transaction::{
    ArgumentABI, ScriptABI, ScriptFunctionABI, TransactionScriptABI, TypeArgumentABI,
};
use move_deps::move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, TypeTag},
};
use serde_generate::{
    indent::{IndentConfig, IndentedWriter},
    rust, CodeGeneratorConfig,
};

use heck::{CamelCase, ShoutySnakeCase, SnakeCase};
use serde_reflection::ContainerFormat;
use std::{
    collections::BTreeMap,
    io::{Result, Write},
    path::PathBuf,
};

/// Output transaction builders in Rust for the given ABIs.
/// If `local_types` is true, we generate a file suitable for the Aptos codebase itself
/// rather than using serde-generated, standalone definitions.
pub fn output(out: &mut dyn Write, abis: &[ScriptABI], local_types: bool) -> Result<()> {
    if abis.is_empty() {
        return Ok(());
    }
    let mut emitter = RustEmitter {
        out: IndentedWriter::new(out, IndentConfig::Space(4)),
        local_types,
    };

    emitter.output_preamble()?;
    emitter.output_script_call_enum_with_imports(abis)?;

    let tx_script_abis = common::transaction_script_abis(abis);
    let script_function_abis = common::script_function_abis(abis);

    if !tx_script_abis.is_empty() {
        emitter.output_transaction_script_impl(&tx_script_abis)?;
    }
    if !script_function_abis.is_empty() {
        emitter.output_script_function_impl(&script_function_abis)?;
    }

    for abi in abis {
        emitter.output_script_encoder_function(abi)?;
    }

    for abi in abis {
        emitter.output_script_decoder_function(abi)?;
    }

    if !tx_script_abis.is_empty() {
        emitter.output_transaction_script_decoder_map(&tx_script_abis)?;
    }
    if !script_function_abis.is_empty() {
        emitter.output_script_function_decoder_map(&script_function_abis)?;
    }
    emitter.output_decoding_helpers(&common::filter_transaction_scripts(abis))?;

    for abi in &tx_script_abis {
        emitter.output_code_constant(abi)?;
    }
    Ok(())
}

/// Shared state for the Rust code generator.
struct RustEmitter<T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Whether we are targetting the Aptos repository itself (as opposed to generated Aptos types).
    local_types: bool,
}

impl<T> RustEmitter<T>
where
    T: Write,
{
    fn output_transaction_script_impl(
        &mut self,
        transaction_script_abis: &[TransactionScriptABI],
    ) -> Result<()> {
        writeln!(self.out, "\nimpl ScriptCall {{")?;
        self.out.indent();
        self.output_transaction_script_encode_method(transaction_script_abis)?;
        self.output_transaction_script_decode_method()?;
        self.output_transaction_script_name_method(transaction_script_abis)?;
        self.out.unindent();
        writeln!(self.out, "\n}}")
    }

    fn output_script_function_impl(
        &mut self,
        script_function_abis: &[ScriptFunctionABI],
    ) -> Result<()> {
        writeln!(self.out, "\nimpl ScriptFunctionCall {{")?;
        self.out.indent();
        self.output_script_function_encode_method(script_function_abis)?;
        self.output_script_function_decode_method()?;
        self.out.unindent();
        writeln!(self.out, "\n}}")
    }

    fn output_preamble(&mut self) -> Result<()> {
        if self.local_types {
            writeln!(
                self.out,
                r#"
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This file was generated. Do not modify!
//
// To update this code, run: `cargo run --release -p framework`.
"#
            )?;
        }
        writeln!(
            self.out,
            r#"// Conversion library between a structured representation of a Move script call (`ScriptCall`) and the
// standard BCS-compatible representation used in Aptos transactions (`Script`).
//
// This code was generated by compiling known Script interfaces ("ABIs") with the tool `transaction-builder-generator`.
"#
        )
    }

    fn output_script_call_enum_with_imports(&mut self, abis: &[ScriptABI]) -> Result<()> {
        let external_definitions = Self::get_external_definitions(self.local_types);
        let (transaction_script_abis, script_fun_abis): (Vec<_>, Vec<_>) = abis
            .iter()
            .cloned()
            .partition(|abi| abi.is_transaction_script_abi());
        let has_script = !transaction_script_abis.is_empty();
        let mut script_registry: BTreeMap<_, _> = if has_script {
            vec![(
                "ScriptCall".to_string(),
                common::make_abi_enum_container(transaction_script_abis.as_slice()),
            )]
            .into_iter()
            .collect()
        } else {
            BTreeMap::new()
        };
        let mut script_function_registry: BTreeMap<_, _> = vec![(
            "ScriptFunctionCall".to_string(),
            common::make_namespaced_abi_enum_container(script_fun_abis.as_slice()),
        )]
        .into_iter()
        .collect();
        script_registry.append(&mut script_function_registry);
        let mut comments: BTreeMap<_, _> = abis
            .iter()
            .map(|abi| {
                (
                    vec![
                        "crate".to_string(),
                        if abi.is_transaction_script_abi() {
                            "ScriptCall"
                        } else {
                            "ScriptFunctionCall"
                        }
                        .to_string(),
                        match abi {
                            ScriptABI::ScriptFunction(sf) => {
                                format!("{}{}", sf.module_name().name(), abi.name().to_camel_case())
                            }
                            _ => abi.name().to_camel_case(),
                        },
                    ],
                    common::prepare_doc_string(abi.doc()),
                )
            })
            .collect();
        if has_script {
            comments.insert(
                vec!["crate".to_string(), "ScriptCall".to_string()],
                r#"Structured representation of a call into a known Move script.
```ignore
impl ScriptCall {
    pub fn encode(self) -> Script { .. }
    pub fn decode(&Script) -> Option<ScriptCall> { .. }
}
```
"#
                .into(),
            );
        }
        comments.insert(
            vec!["crate".to_string(), "ScriptFunctionCall".to_string()],
            r#"Structured representation of a call into a known Move script function.
```ignore
impl ScriptFunctionCall {
    pub fn encode(self) -> TransactionPayload { .. }
    pub fn decode(&TransactionPayload) -> Option<ScriptFunctionCall> { .. }
}
```
"#
            .into(),
        );
        let custom_derive_block = if self.local_types {
            Some(
                r#"#[cfg_attr(feature = "fuzzing", derive(proptest_derive::Arbitrary))]
#[cfg_attr(feature = "fuzzing", proptest(no_params))]"#
                    .to_string(),
            )
        } else {
            None
        };
        // Deactivate serialization for local types to force `Bytes = Vec<u8>`.
        let config = CodeGeneratorConfig::new("crate".to_string())
            .with_comments(comments)
            .with_external_definitions(external_definitions)
            .with_serialization(!self.local_types);
        rust::CodeGenerator::new(&config)
            .with_custom_derive_block(custom_derive_block)
            .output(&mut self.out, &script_registry)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err)))?;
        Ok(())
    }

    fn get_external_definitions(local_types: bool) -> serde_generate::ExternalDefinitions {
        let definitions = if local_types {
            vec![
                (
                    "move_deps::move_core_types::language_storage",
                    vec!["ModuleId"],
                ),
                ("move_deps::move_core_types", vec!["ident_str"]),
                (
                    "aptos_types::transaction",
                    vec![
                        "Script",
                        "TransactionArgument",
                        "TransactionPayload",
                        "ScriptFunction",
                        "VecBytes",
                    ],
                ),
                ("aptos_types::account_address", vec!["AccountAddress"]),
            ]
        } else {
            vec![(
                "aptos_types",
                vec![
                    "AccountAddress",
                    "TypeTag",
                    "Script",
                    "ScriptFunction",
                    "TransactionArgument",
                    "VecBytes",
                    "TransactionPayload",
                    "ModuleId",
                    "Identifier",
                ],
            )]
        };
        definitions
            .into_iter()
            .map(|(module, defs)| {
                (
                    module.to_string(),
                    defs.into_iter().map(String::from).collect(),
                )
            })
            .collect()
    }

    fn output_transaction_script_encode_method(
        &mut self,
        abis: &[TransactionScriptABI],
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
/// Build an Aptos `Script` from a structured object `ScriptCall`.
pub fn encode(self) -> Script {{"#
        )?;
        self.out.indent();
        writeln!(self.out, "use ScriptCall::*;\nmatch self {{")?;
        self.out.indent();
        for abi in abis {
            self.output_variant_encoder(&ScriptABI::TransactionScript(abi.clone()))?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")?;
        self.out.unindent();
        writeln!(self.out, "}}\n")
    }

    fn output_script_function_encode_method(&mut self, abis: &[ScriptFunctionABI]) -> Result<()> {
        writeln!(
            self.out,
            r#"
/// Build an Aptos `TransactionPayload` from a structured object `ScriptFunctionCall`.
pub fn encode(self) -> TransactionPayload {{"#
        )?;
        self.out.indent();
        writeln!(self.out, "use ScriptFunctionCall::*;\nmatch self {{")?;
        self.out.indent();
        for abi in abis {
            self.output_variant_encoder(&ScriptABI::ScriptFunction(abi.clone()))?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")?;
        self.out.unindent();
        writeln!(self.out, "}}\n")
    }

    fn output_variant_encoder(&mut self, abi: &ScriptABI) -> Result<()> {
        let params = std::iter::empty()
            .chain(abi.ty_args().iter().map(TypeArgumentABI::name))
            .chain(abi.args().iter().map(ArgumentABI::name))
            .collect::<Vec<_>>()
            .join(", ");

        let prefix = if let ScriptABI::ScriptFunction(sf) = abi {
            sf.module_name().name().to_string()
        } else {
            String::new()
        };
        writeln!(
            self.out,
            "{5}{0}{{{2}}} => encode_{3}{4}{1}({2}),",
            abi.name().to_camel_case(),
            abi.name(),
            params,
            prefix.to_snake_case(),
            if prefix.is_empty() { "" } else { "_" },
            prefix,
        )
    }

    fn output_transaction_script_decode_method(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"
/// Try to recognize an Aptos `Script` and convert it into a structured object `ScriptCall`.
pub fn decode(script: &Script) -> Option<ScriptCall> {{
    match TRANSACTION_SCRIPT_DECODER_MAP.get({}) {{
        Some(decoder) => decoder(script),
        None => None,
    }}
}}"#,
            if self.local_types {
                "script.code()"
            } else {
                "&script.code.clone().into_vec()"
            }
        )
    }

    fn output_script_function_decode_method(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"
/// Try to recognize an Aptos `TransactionPayload` and convert it into a structured object `ScriptFunctionCall`.
pub fn decode(payload: &TransactionPayload) -> Option<ScriptFunctionCall> {{
    if let TransactionPayload::ScriptFunction(script) = payload {{
        match SCRIPT_FUNCTION_DECODER_MAP.get(&format!("{{}}{{}}", {}, {})) {{
            Some(decoder) => decoder(payload),
            None => None,
        }}
    }} else {{
        None
    }}
}}"#,
            if self.local_types {
                "script.module().name()"
            } else {
                "script.module.name.0"
            },
            if self.local_types {
                "script.function()"
            } else {
                "script.function.0"
            }
        )
    }

    fn output_transaction_script_name_method(
        &mut self,
        abis: &[TransactionScriptABI],
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
/// Return the name of an Aptos `Script` from a structured object `ScriptCall`.
pub fn name(&self) -> &'static str {{"#
        )?;
        self.out.indent();
        writeln!(self.out, "use ScriptCall::*;\nmatch self {{")?;
        self.out.indent();
        for abi in abis {
            writeln!(
                self.out,
                "{} {{ .. }} => \"{}\",",
                abi.name().to_camel_case(),
                abi.name(),
            )?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")?;
        self.out.unindent();
        writeln!(self.out, "}}\n")
    }

    fn output_comment(&mut self, indentation: usize, doc: &str) -> std::io::Result<()> {
        let prefix = " ".repeat(indentation) + "/// ";
        let empty_line = "\n".to_string() + &" ".repeat(indentation) + "///\n";
        let text = textwrap::indent(doc, &prefix).replace("\n\n", &empty_line);
        write!(self.out, "\n{}\n", text)
    }

    fn emit_transaction_script_encoder_function(
        &mut self,
        abi: &TransactionScriptABI,
    ) -> Result<()> {
        write!(
            self.out,
            "pub fn encode_{}_script({}) -> Script {{",
            abi.name(),
            [
                Self::quote_type_parameters(abi.ty_args()),
                Self::quote_parameters(abi.args(), self.local_types),
            ]
            .concat()
            .join(", ")
        )?;
        self.out.indent();
        if self.local_types {
            writeln!(
                self.out,
                r#"
Script::new(
    {}_CODE.to_vec(),
    vec![{}],
    vec![{}],
)"#,
                abi.name().to_shouty_snake_case(),
                Self::quote_type_arguments(abi.ty_args()),
                Self::quote_arguments_for_script(abi.args()),
            )?;
        } else {
            writeln!(
                self.out,
                r#"
Script {{
    code: Bytes::from({}_CODE.to_vec()),
    ty_args: vec![{}],
    args: vec![{}],
}}"#,
                abi.name().to_shouty_snake_case(),
                Self::quote_type_arguments(abi.ty_args()),
                Self::quote_arguments_for_script(abi.args()),
            )?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn emit_script_function_encoder_function(&mut self, abi: &ScriptFunctionABI) -> Result<()> {
        write!(
            self.out,
            "pub fn encode_{}_{}({}) -> TransactionPayload {{",
            abi.module_name().name().to_string().to_snake_case(),
            abi.name(),
            [
                Self::quote_type_parameters(abi.ty_args()),
                Self::quote_parameters(abi.args(), self.local_types),
            ]
            .concat()
            .join(", ")
        )?;
        self.out.indent();
        if self.local_types {
            writeln!(
                self.out,
                r#"
TransactionPayload::ScriptFunction(ScriptFunction::new(
    {},
    {},
    vec![{}],
    vec![{}],
))"#,
                self.quote_module_id(abi.module_name()),
                self.quote_identifier(abi.name()),
                Self::quote_type_arguments(abi.ty_args()),
                Self::quote_arguments(abi.args(), /* local_types */ true),
            )?;
        } else {
            writeln!(
                self.out,
                r#"
TransactionPayload::ScriptFunction(ScriptFunction {{
    module: {},
    function: {},
    ty_args: vec![{}],
    args: vec![{}],
}})"#,
                self.quote_module_id(abi.module_name()),
                self.quote_identifier(abi.name()),
                Self::quote_type_arguments(abi.ty_args()),
                Self::quote_arguments(abi.args(), /* local_types */ false),
            )?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_script_encoder_function(&mut self, abi: &ScriptABI) -> Result<()> {
        self.output_comment(0, &common::prepare_doc_string(abi.doc()))?;
        match abi {
            ScriptABI::TransactionScript(abi) => self.emit_transaction_script_encoder_function(abi),
            ScriptABI::ScriptFunction(abi) => self.emit_script_function_encoder_function(abi),
        }
    }

    fn output_script_decoder_function(&mut self, abi: &ScriptABI) -> Result<()> {
        match abi {
            ScriptABI::TransactionScript(abi) => self.emit_transaction_script_decoder_function(abi),
            ScriptABI::ScriptFunction(abi) => self.emit_script_function_decoder_function(abi),
        }
    }

    fn emit_script_function_decoder_function(&mut self, abi: &ScriptFunctionABI) -> Result<()> {
        // `payload` is always used, so don't need to fix warning "unused variable" by prefixing with "_"
        //
        writeln!(
            self.out,
            "\nfn decode_{}_{}(payload: &TransactionPayload) -> Option<ScriptFunctionCall> {{",
            abi.module_name().name().to_string().to_snake_case(),
            abi.name(),
        )?;
        self.out.indent();
        writeln!(
            self.out,
            "if let TransactionPayload::ScriptFunction({}script) = payload {{",
            // fix warning "unused variable"
            if abi.ty_args().is_empty() && abi.args().is_empty() {
                "_"
            } else {
                ""
            }
        )?;
        self.out.indent();
        writeln!(
            self.out,
            "Some(ScriptFunctionCall::{}{} {{",
            abi.module_name().name(),
            abi.name().to_camel_case(),
        )?;
        self.out.indent();
        for (index, ty_arg) in abi.ty_args().iter().enumerate() {
            writeln!(
                self.out,
                "{} : script.ty_args{}.get({})?.clone(),",
                ty_arg.name(),
                if self.local_types { "()" } else { "" },
                index,
            )?;
        }
        for (index, arg) in abi.args().iter().enumerate() {
            writeln!(
                self.out,
                "{} : bcs::from_bytes(script.args{}.get({})?).ok()?,",
                arg.name(),
                if self.local_types { "()" } else { "" },
                index,
            )?;
        }
        self.out.unindent();
        writeln!(self.out, "}})")?;
        self.out.unindent();
        writeln!(self.out, "}} else {{")?;
        self.out.indent();
        writeln!(self.out, "None")?;
        self.out.unindent();
        writeln!(self.out, "}}")?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn emit_transaction_script_decoder_function(
        &mut self,
        abi: &TransactionScriptABI,
    ) -> Result<()> {
        writeln!(
            self.out,
            "\nfn decode_{}_script({}script: &Script) -> Option<ScriptCall> {{",
            abi.name(),
            // fix warning "unused variable"
            if abi.ty_args().is_empty() && abi.args().is_empty() {
                "_"
            } else {
                ""
            }
        )?;
        self.out.indent();
        writeln!(
            self.out,
            "Some(ScriptCall::{} {{",
            abi.name().to_camel_case(),
        )?;
        self.out.indent();
        for (index, ty_arg) in abi.ty_args().iter().enumerate() {
            writeln!(
                self.out,
                "{} : script.ty_args{}.get({})?.clone(),",
                ty_arg.name(),
                if self.local_types { "()" } else { "" },
                index,
            )?;
        }
        for (index, arg) in abi.args().iter().enumerate() {
            writeln!(
                self.out,
                "{} : decode_{}_argument(script.args{}.get({})?.clone())?,",
                arg.name(),
                common::mangle_type(arg.type_tag()),
                if self.local_types { "()" } else { "" },
                index,
            )?;
        }
        self.out.unindent();
        writeln!(self.out, "}})")?;
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_transaction_script_decoder_map(
        &mut self,
        abis: &[TransactionScriptABI],
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
type TransactionScriptDecoderMap = std::collections::HashMap<Vec<u8>, Box<dyn Fn(&Script) -> Option<ScriptCall> + std::marker::Sync + std::marker::Send>>;

static TRANSACTION_SCRIPT_DECODER_MAP: once_cell::sync::Lazy<TransactionScriptDecoderMap> = once_cell::sync::Lazy::new(|| {{"#
        )?;
        self.out.indent();
        writeln!(
            self.out,
            "let mut map : TransactionScriptDecoderMap = std::collections::HashMap::new();"
        )?;
        for abi in abis {
            writeln!(
                self.out,
                "map.insert({}_CODE.to_vec(), Box::new(decode_{}_script));",
                abi.name().to_shouty_snake_case(),
                abi.name()
            )?;
        }
        writeln!(self.out, "map")?;
        self.out.unindent();
        writeln!(self.out, "}});")
    }

    fn output_script_function_decoder_map(&mut self, abis: &[ScriptFunctionABI]) -> Result<()> {
        writeln!(
            self.out,
            r#"
type ScriptFunctionDecoderMap = std::collections::HashMap<String, Box<dyn Fn(&TransactionPayload) -> Option<ScriptFunctionCall> + std::marker::Sync + std::marker::Send>>;

static SCRIPT_FUNCTION_DECODER_MAP: once_cell::sync::Lazy<ScriptFunctionDecoderMap> = once_cell::sync::Lazy::new(|| {{"#
        )?;
        self.out.indent();
        writeln!(
            self.out,
            "let mut map : ScriptFunctionDecoderMap = std::collections::HashMap::new();"
        )?;
        for abi in abis {
            writeln!(
                self.out,
                "map.insert(\"{}{}\".to_string(), Box::new(decode_{}_{}));",
                abi.module_name().name(),
                abi.name(),
                abi.module_name().name().to_string().to_snake_case(),
                abi.name()
            )?;
        }
        writeln!(self.out, "map")?;
        self.out.unindent();
        writeln!(self.out, "}});")
    }

    fn output_decoding_helpers(&mut self, abis: &[ScriptABI]) -> Result<()> {
        let required_types = common::get_required_helper_types(abis);
        for required_type in required_types {
            self.output_decoding_helper(required_type)?;
        }
        Ok(())
    }

    fn output_decoding_helper(&mut self, type_tag: &TypeTag) -> Result<()> {
        use TypeTag::*;
        let (constructor, expr) = match type_tag {
            Bool => ("Bool", "Some(value)".to_string()),
            U8 => ("U8", "Some(value)".to_string()),
            U64 => ("U64", "Some(value)".to_string()),
            U128 => ("U128", "Some(value)".to_string()),
            Address => ("Address", "Some(value)".to_string()),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => ("U8Vector", "Some(value)".to_string()),
                _ => common::type_not_allowed(type_tag),
            },
            Struct(_) | Signer => common::type_not_allowed(type_tag),
        };
        writeln!(
            self.out,
            r#"
fn decode_{}_argument(arg: TransactionArgument) -> Option<{}> {{
    match arg {{
        TransactionArgument::{}(value) => {},
        _ => None,
    }}
}}
"#,
            common::mangle_type(type_tag),
            Self::quote_type(type_tag, self.local_types),
            constructor,
            expr,
        )
    }

    fn output_code_constant(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        writeln!(
            self.out,
            "\nconst {}_CODE: &[u8] = &[{}];",
            abi.name().to_shouty_snake_case(),
            abi.code()
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        Ok(())
    }

    fn quote_identifier(&self, ident: &str) -> String {
        if self.local_types {
            format!("ident_str!(\"{}\").to_owned()", ident)
        } else {
            format!("Identifier(\"{}\".to_string())", ident)
        }
    }

    fn quote_address(&self, address: &AccountAddress) -> String {
        let u8_array = format!(
            "[{}]",
            address
                .to_vec()
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if self.local_types {
            format!("AccountAddress::new({})", u8_array)
        } else {
            format!("AccountAddress({})", u8_array)
        }
    }

    fn quote_module_id(&self, module_id: &ModuleId) -> String {
        if self.local_types {
            format!(
                "ModuleId::new(
                {},
                {},
            )",
                self.quote_address(module_id.address()),
                self.quote_identifier(module_id.name().as_str())
            )
        } else {
            format!(
                "ModuleId {{
                address: {},
                name: {},
            }}",
                self.quote_address(module_id.address()),
                self.quote_identifier(module_id.name().as_str())
            )
        }
    }

    fn quote_type_parameters(ty_args: &[TypeArgumentABI]) -> Vec<String> {
        ty_args
            .iter()
            .map(|ty_arg| format!("{}: TypeTag", ty_arg.name()))
            .collect()
    }

    fn quote_parameters(args: &[ArgumentABI], local_types: bool) -> Vec<String> {
        args.iter()
            .map(|arg| {
                format!(
                    "{}: {}",
                    arg.name(),
                    Self::quote_type(arg.type_tag(), local_types)
                )
            })
            .collect()
    }

    fn quote_type_arguments(ty_args: &[TypeArgumentABI]) -> String {
        ty_args
            .iter()
            .map(|ty_arg| ty_arg.name().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_arguments(args: &[ArgumentABI], local_types: bool) -> String {
        args.iter()
            .map(|arg| Self::quote_transaction_argument(arg.type_tag(), arg.name(), local_types))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_arguments_for_script(args: &[ArgumentABI]) -> String {
        args.iter()
            .map(|arg| Self::quote_transaction_argument_for_script(arg.type_tag(), arg.name()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_type(type_tag: &TypeTag, local_types: bool) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => "bool".into(),
            U8 => "u8".into(),
            U64 => "u64".into(),
            U128 => "u128".into(),
            Address => "AccountAddress".into(),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => {
                    if local_types {
                        "Vec<u8>".into()
                    } else {
                        "Bytes".into()
                    }
                }
                Vector(type_tag) if type_tag.as_ref() == &U8 => "VecBytes".into(),
                _ => common::type_not_allowed(type_tag),
            },

            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }

    fn quote_transaction_argument(type_tag: &TypeTag, name: &str, local_types: bool) -> String {
        // NOTE: this check is not necessary as with BCS-encoding, argument of
        // any valid Move type is possible, including Struct and Vector of types
        // other than U8. However, to be consistent with the restrictions on
        // transaction script arguments, we still check the TypeTag here.
        use TypeTag::*;
        match type_tag {
            Bool | U8 | U64 | U128 | Address => {}
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => {}
                Vector(type_tag) => {
                    if type_tag.as_ref() != &U8 {
                        common::type_not_allowed(type_tag)
                    }
                }
                _ => common::type_not_allowed(type_tag),
            },
            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
        let conversion = format!("bcs::to_bytes(&{}).unwrap()", name);
        if local_types {
            conversion
        } else {
            format!("serde_bytes::ByteBuf::from({})", conversion)
        }
    }

    fn quote_transaction_argument_for_script(type_tag: &TypeTag, name: &str) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => format!("TransactionArgument::Bool({})", name),
            U8 => format!("TransactionArgument::U8({})", name),
            U64 => format!("TransactionArgument::U64({})", name),
            U128 => format!("TransactionArgument::U128({})", name),
            Address => format!("TransactionArgument::Address({})", name),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => format!("TransactionArgument::U8Vector({})", name),
                _ => common::type_not_allowed(type_tag),
            },

            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }
}

pub struct Installer {
    install_dir: PathBuf,
    aptos_types_version: String,
}

impl Installer {
    pub fn new(install_dir: PathBuf, aptos_types_version: String) -> Self {
        Installer {
            install_dir,
            aptos_types_version,
        }
    }
}

impl crate::SourceInstaller for Installer {
    type Error = Box<dyn std::error::Error>;

    fn install_transaction_builders(
        &self,
        public_name: &str,
        abis: &[ScriptABI],
    ) -> std::result::Result<(), Self::Error> {
        let (name, version) = {
            let parts = public_name.splitn(2, ':').collect::<Vec<_>>();
            if parts.len() >= 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (parts[0].to_string(), "0.1.0".to_string())
            }
        };
        let dir_path = self.install_dir.join(&name);
        std::fs::create_dir_all(&dir_path)?;
        let mut cargo = std::fs::File::create(&dir_path.join("Cargo.toml"))?;
        write!(
            cargo,
            r#"[package]
name = "{}"
version = "{}"
edition = "2018"

[dependencies]
once_cell = "1.4.0"
serde = {{ version = "1.0", features = ["derive"] }}
serde_bytes = "0.11"
aptos-types = {{ path = "../aptos-types", version = "{}" }}
"#,
            name, version, self.aptos_types_version,
        )?;
        std::fs::create_dir(dir_path.join("src"))?;
        let source_path = dir_path.join("src/lib.rs");
        let mut source = std::fs::File::create(&source_path)?;
        output(&mut source, abis, /* local_types */ false)?;
        Ok(())
    }
}

/// Walks through the registry replacing variables known to be named as a
/// rust keyword, making the resulting codegen invalid.
/// ie: public function: Identifier => public function_name: Identifier
pub fn replace_keywords(registry: &mut BTreeMap<String, ContainerFormat>) {
    swap_keyworded_fields(registry.get_mut("TypeTag"));
    swap_keyworded_fields(registry.get_mut("StructTag"));
}

fn swap_keyworded_fields(fields: Option<&mut ContainerFormat>) {
    match fields {
        Some(ContainerFormat::Enum(fields)) => {
            for (_, val) in fields.iter_mut() {
                match val.name.as_str() {
                    "struct" => val.name = String::from("Struct"),
                    "bool" => val.name = String::from("Bool"),
                    "u8" => val.name = String::from("U8"),
                    "u64" => val.name = String::from("U64"),
                    "u128" => val.name = String::from("U128"),
                    "address" => val.name = String::from("Address"),
                    "signer" => val.name = String::from("Signer"),
                    "vector" => val.name = String::from("Vector"),
                    _ => (),
                }
            }
        }
        Some(ContainerFormat::Struct(fields)) => {
            for entry in fields.iter_mut() {
                if entry.name.as_str() == "type_args" {
                    entry.name = String::from("type_params")
                }
            }
        }
        _ => (),
    }
}
