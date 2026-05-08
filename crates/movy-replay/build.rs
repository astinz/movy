use movy_sui::compile::SuiCompiledPackage;
use std::path::PathBuf;

macro_rules! cargo_print {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

fn build_std(test: bool) -> Vec<SuiCompiledPackage> {
    let flag = if test { "testing" } else { "non-testing" };
    let mut deps = vec![];
    for (package_name, package_path) in local_sui_framework_packages() {
        cargo_print!(
            "Building {} std {} at {}",
            flag,
            package_name,
            package_path.display()
        );

        let out = SuiCompiledPackage::build_all_unpublished_from_folder(&package_path, test)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to build {} std package at {}: {}",
                    package_name,
                    package_path.display(),
                    error
                )
            });
        let build_directory = package_path.join("build");
        if build_directory.exists() {
            std::fs::remove_dir_all(&build_directory).unwrap();
        }
        deps.push(out);
    }
    deps
}

fn local_sui_framework_packages() -> Vec<(&'static str, PathBuf)> {
    let sui_root = std::env::var_os("MOVY_SUI_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../..")
                .join("sui")
        });
    let packages = sui_root.join("crates/sui-framework/packages");
    vec![
        ("Bridge", packages.join("bridge")),
        ("MoveStdlib", packages.join("move-stdlib")),
        ("Sui", packages.join("sui-framework")),
        ("SuiSystem", packages.join("sui-system")),
    ]
}

fn main() {
    println!("cargo::rerun-if-env-changed=STD_BUILD_KEEP");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let move_home = out_dir.join("move-home");
    std::fs::create_dir_all(&move_home).unwrap();
    // Keep framework package resolution inside Cargo's build output instead of
    // depending on the user's global ~/.move cache.
    unsafe {
        std::env::set_var("MOVE_HOME", &move_home);
    }
    let testing_stds = build_std(true);
    let non_testing_std = build_std(false);

    let fp = std::fs::File::create(out_dir.join("std.testing")).unwrap();
    serde_json::to_writer(fp, &testing_stds).unwrap();

    let fp = std::fs::File::create(out_dir.join("std")).unwrap();
    serde_json::to_writer(fp, &non_testing_std).unwrap();
}
