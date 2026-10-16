fn main() {
    println!("cargo:rerun-if-env-changed=NONS_TELEMETRY_INGEST_KEY");
    tauri_build::build()
}
