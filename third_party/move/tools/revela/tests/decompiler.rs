mod utils;

#[cfg(test)]
mod test {

    use std::path::Path;
    use std::{env, fs};

    use super::utils;
    use revela::decompiler::{Decompiler, OptimizerSettings};

    pub fn decompile_compile_decompile_match_single_file(
        path: &Path,
    ) -> datatest_stable::Result<()> {
        let module_name = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".move", "");
        let source = fs::read_to_string(path).expect("Unable to read file");

        let corresponding_output_file = path.parent().unwrap().join(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace("-test.move", "-decompiled.move"),
        );

        let expected_result = fs::read_to_string(&corresponding_output_file);
        let mut src_scripts = vec![];
        let mut src_modules = vec![];
        let mut output = String::new();

        let ref_output_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("refs");

        utils::tmp_project(
            vec![("tmp.move", source.as_str())],
            |project_root, tmp_files| {
                (src_scripts, src_modules) = utils::run_compiler(project_root, tmp_files, false);

                {
                    let binaries = utils::into_binary_indexed_view(&src_scripts, &src_modules);
                    let mut decompiler = Decompiler::new(binaries, Default::default());
                    let default_output = decompiler.decompile().expect("Unable to decompile");

                    let ref_output_path =
                        ref_output_dir.join(format!("sources-{}-decompiled.move", module_name));
                    std::fs::write(&ref_output_path, default_output).unwrap();
                }
                {
                    let binaries = utils::into_binary_indexed_view(&src_scripts, &src_modules);
                    let mut decompiler = Decompiler::new(
                        binaries,
                        OptimizerSettings {
                            // this settings may cause the output to be different
                            disable_optimize_variables_declaration: true,
                        },
                    );
                    output = decompiler.decompile().expect("Unable to decompile");
                }
            },
        );

        if std::env::var("DECOMPILER_TEST_OUTPUT_ONLY").is_ok() {
            println!("{}", output);
            return Ok(());
        } else if env::var("FORCE_UPDATE_EXPECTED_OUTPUT").is_ok() {
            fs::write(&corresponding_output_file, &output).unwrap();
        } else if let Ok(expected_result) = expected_result {
            utils::assert_same_source(&output, &expected_result);
        } else if env::var("UPDATE_EXPECTED_OUTPUT").is_ok() {
            fs::write(&corresponding_output_file, &output).unwrap();
        } else {
            panic!("Unable to read expected output file");
        }

        utils::tmp_project(
            vec![("tmp.move", output.as_str())],
            |project_root, tmp_files| {
                let (scripts, modules) = utils::run_compiler(project_root, tmp_files, false);

                let binaries = utils::into_binary_indexed_view(&scripts, &modules);

                let mut decompiler = Decompiler::new(
                    binaries,
                    OptimizerSettings {
                        disable_optimize_variables_declaration: true,
                    },
                );

                let _output2 = decompiler.decompile().expect("Unable to decompile");
                // the output2 may be different because of assignment reorder and constant propagation
            },
        );

        Ok(())
    }
}

datatest_stable::harness!(
    test::decompile_compile_decompile_match_single_file,
    "tests/sources",
    r"-test\.move$"
);
