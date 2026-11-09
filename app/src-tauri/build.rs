fn main() {
    println!("cargo:rerun-if-env-changed=NONS_TELEMETRY_INGEST_KEY");
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=tests/windows-test.rc");
        println!("cargo:rerun-if-changed=tests/windows-test.manifest");
        let result =
            embed_resource::compile_for_tests("tests/windows-test.rc", embed_resource::NONE);
        if !matches!(
            result,
            embed_resource::CompilationResult::Ok | embed_resource::CompilationResult::NotWindows
        ) {
            panic!("failed to build Windows test manifest resource: {result}");
        }
        println!(
            "cargo:rustc-link-search=native={}",
            std::env::var("OUT_DIR").unwrap()
        );
    }
    tauri_build::build()
}
