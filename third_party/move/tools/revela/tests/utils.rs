use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
};

use move_binary_format::{
    binary_views::BinaryIndexedView, file_format::CompiledScript, CompiledModule,
};
use move_command_line_common::{address::NumericalAddress, files::FileHash};
use move_compiler::compiled_unit::CompiledUnit;

#[allow(dead_code)]
fn default_testing_addresses() -> BTreeMap<String, NumericalAddress> {
    let mapping = [
        ("std", "0x1"),
        ("NamedAddr", "0xbadbadbad"),
        ("aptos_framework", "0x1"),
        ("aptos_std", "0x1"),
        ("aptos_token", "0x1337"),
        ("Extensions", "0x1"),
        ("admin_addr", "0xbeef"),
        ("mint_nft", "0x1234"),
        ("source_addr", "0x2345"),
        ("core_resources", "0x3000"),
        ("vm_reserved", "0x3001"),
        ("aptos_fungible_asset", "0xA"),
        ("vm", "0x0"),
    ];
    mapping
        .iter()
        .map(|(name, addr)| (name.to_string(), NumericalAddress::parse_str(addr).unwrap()))
        .collect()
}

#[allow(dead_code)]
pub(crate) fn into_binary_indexed_view<'a>(
    scripts: &'a Vec<CompiledScript>,
    modules: &'a Vec<CompiledModule>,
) -> Vec<BinaryIndexedView<'a>> {
    let mut binaries: Vec<BinaryIndexedView<'a>> = Vec::new();

    binaries.extend(modules.iter().map(BinaryIndexedView::Module));
    binaries.extend(scripts.iter().map(BinaryIndexedView::Script));

    binaries
}

#[allow(dead_code)]
pub(crate) fn run_compiler(
    output_dir: &str,
    sources: Vec<&str>,
    stdlib_as_sources: bool,
) -> (Vec<CompiledScript>, Vec<CompiledModule>) {
    let stdlib_files = move_command_line_common::files::find_filenames(
        &[
            aptos_framework::path_in_crate("aptos-stdlib/sources"),
            aptos_framework::path_in_crate("move-stdlib/sources"),
            aptos_framework::path_in_crate("aptos-framework/sources"),
            aptos_framework::path_in_crate("aptos-token/sources"),
        ],
        |p| {
            move_command_line_common::files::extension_equals(
                p,
                move_command_line_common::files::MOVE_EXTENSION,
            ) //&& !p.file_name().unwrap().to_str().unwrap().contains(".spec.")
        },
    )
    .unwrap();

    let stdlib_files_str = stdlib_files.iter().map(|f| f.as_str()).collect::<Vec<_>>();
    let (compiler_sources, compiler_stdlibs) = if stdlib_as_sources {
        (
            sources
                .iter()
                .chain(stdlib_files_str.iter())
                .cloned()
                .collect::<Vec<_>>(),
            Vec::<&str>::new(),
        )
    } else {
        (sources, stdlib_files_str)
    };

    let source_hashes: HashSet<_> = compiler_sources
        .iter()
        .map(|f| {
            let content = std::fs::read_to_string(f).expect("Unable to read file");
            FileHash::new(&content)
        })
        .collect();

    let options = move_compiler_v2::options::Options {
        dependencies: Vec::new(),
        named_address_mapping: default_testing_addresses()
            .into_iter()
            .map(|(k, v)| format!("{}={:#X}", k, v))
            .collect(),
        output_dir: String::from(output_dir),
        language_version: Some(move_model::metadata::LanguageVersion::V2_0),
        skip_attribute_checks: true,
        known_attributes: Default::default(),
        testing: false,
        experiments: Vec::new(),
        experiment_cache: Default::default(),
        sources: compiler_sources.into_iter().map(String::from).collect(),
        sources_deps: compiler_stdlibs.into_iter().map(String::from).collect(),
        warn_deprecated: false,
        warn_of_deprecation_use_in_aptos_libs: false,
        warn_unused: false,
        whole_program: false,
        compile_test_code: false,
        compile_verify_code: false,
    };

    let (_, compiled_units) =
        move_compiler_v2::run_move_compiler_to_stderr(options).expect("compilation failed");

    let (compiled_modules, compiled_scripts): (Vec<_>, Vec<_>) = compiled_units
        .into_iter()
        .filter(|m| source_hashes.contains(&m.loc().file_hash()))
        .map(|x| x.into_compiled_unit())
        .partition(|x| matches!(x, CompiledUnit::Module(_)));

    let modules: Vec<_> = compiled_modules
        .into_iter()
        .map(|x| match x {
            CompiledUnit::Module(m) => m.module,
            _ => unreachable!(),
        })
        .collect();

    let scripts: Vec<_> = compiled_scripts
        .into_iter()
        .map(|x| match x {
            CompiledUnit::Script(s) => s.script,
            _ => unreachable!(),
        })
        .collect();

    (scripts, modules)
}

#[allow(dead_code)]
pub(crate) fn tmp_project(tmp_files: Vec<(&str, &str)>, mut runner: impl FnMut(&str, Vec<&str>)) {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let tmp_dir = std::env::temp_dir();
    let project_root = tmp_dir.join(format!("revela--test-project-{}", uuid::Uuid::new_v4()));

    std::fs::create_dir(&project_root).unwrap();
    let tmp_files: Vec<_> = tmp_files
        .iter()
        .map(|(name, content)| {
            let path = project_root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
            path
        })
        .collect();

    if !tmp_files
        .iter()
        .any(|x| x.file_name() == Some(std::ffi::OsStr::new("Move.toml")))
    {
        // copy "$MANIFEST/tests/Move.toml"  to project root
        let move_toml = PathBuf::from(manifest).join("tests/Move.toml");
        // copy the file
        let path = project_root.join("Move.toml");
        std::fs::copy(&move_toml, &path).unwrap();
    }

    runner(
        project_root.to_str().unwrap(),
        tmp_files
            .iter()
            .map(|x| x.to_str().unwrap())
            .collect::<Vec<_>>(),
    );

    // only remove the project root if the test passed
    std::fs::remove_dir_all(&project_root).unwrap();
}

#[allow(dead_code)]
// Compare output and output2 which has variables may be renamed
// all variables are in the form v\d+
pub(crate) fn assert_same_source(output: &String, output2: &String) {
    let s1 = output.as_bytes();
    let s2 = output2.as_bytes();

    let mut rename_map = HashMap::new();
    let (mut i, n) = (0, s1.len());
    let (mut j, m) = (0, s2.len());

    while i < n && j < m {
        if s1[i] == s2[j] {
            i += 1;
            j += 1;
        } else if i > 0 && j > 0 && s1[i - 1] == b'v' && s2[j - 1] == b'v' {
            let i0 = i;
            let j0 = j;
            let mut n1 = String::new();
            while i < n && (s1[i] as char).is_numeric() {
                n1.push(s1[i] as char);
                i += 1;
            }
            let mut n2 = String::new();
            while j < m && (s2[j] as char).is_numeric() {
                n2.push(s2[j] as char);
                j += 1;
            }
            if let Some(old_remap) = rename_map.get(&n1) {
                if &n2 != old_remap {
                    panic!(
                        "output and output2 are not the same\nOutput=====\n{}\n\nOutput2=====\n{}",
                        &output[i0..],
                        &output2[j0..]
                    );
                }
            } else {
                rename_map.insert(n1, n2);
            }
        } else {
            panic!(
                "output and output2 are not the same\nOutput=====\n{}\n\nOutput2=====\n{}",
                &output[i..],
                &output2[j..]
            );
        }
    }

    if i < n || j < m {
        panic!(
            "output and output2 are not the same\nOutput=====\n{}\n\nOutput2=====\n{}",
            &output[i..],
            &output2[j..]
        );
    }
}

#[allow(dead_code)]
pub(crate) fn assert_same_source_ignore_assign_order(output: &String, output2: &String) {
    fn normalize_source(output: &String) -> String {
        let re = regex::Regex::new(r"v\d+").unwrap();
        re.replace_all(output, "v0").to_string()
    }
    let normalized_output = normalize_source(output);
    let normalized_output2 = normalize_source(output2);

    println!("Output=====\n{}\n\nOutput2=====\n{}", normalized_output, normalized_output2);

    assert_eq!(normalized_output.len(), normalized_output2.len());
}

#[allow(dead_code)]
pub(crate) fn should_same_script_bytecode(
    src_scripts: &[CompiledScript],
    scripts: &[CompiledScript],
) {
    assert_eq!(src_scripts.len(), scripts.len());

    for (src_script, script) in src_scripts.iter().zip(scripts.iter()) {
        let mut binary = vec![];
        let mut binary2 = vec![];
        src_script.serialize(&mut binary).unwrap();
        script.serialize(&mut binary2).unwrap();
        assert_eq!(binary, binary2);
        // assert_eq!(src_script.as_inner(), script.as_inner());
    }
}

#[allow(dead_code)]
pub(crate) fn should_same_module_bytecode(
    src_modules: &[CompiledModule],
    modules: &[CompiledModule],
) {
    assert_eq!(src_modules.len(), modules.len());

    for (src_module, module) in src_modules.iter().zip(modules.iter()) {
        let mut binary = vec![];
        let mut binary2 = vec![];
        src_module.serialize(&mut binary).unwrap();
        module.serialize(&mut binary2).unwrap();
        assert_eq!(binary, binary2);
        // assert_eq!(src_module.as_inner(), module.as_inner());
    }
}
