use super::*;

const EMPTY_COMPONENT: &[u8] = b"\0asm\x0d\0\x01\0";

fn cached_engine(cache: &CodeCache, optimize: bool) -> Engine {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .epoch_interruption(true)
        .parallel_compilation(true)
        .cranelift_opt_level(if optimize {
            wasmtime::OptLevel::Speed
        } else {
            wasmtime::OptLevel::None
        })
        .cache(Some(cache.cache.clone()));
    Engine::new(&config).unwrap()
}

fn artifacts(directory: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(directory)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file() && entry.path().extension().is_none())
        .map(|entry| entry.into_path())
        .collect()
}

#[test]
fn clearing_compiled_files_preserves_loaded_components_and_rebuilds_on_next_load() {
    let directory = tempfile::tempdir().unwrap();
    let cache = CodeCache::new(directory.path().join("compiled")).unwrap();
    let engine = cached_engine(&cache, true);
    let component = compile_with_cache(&engine, EMPTY_COMPONENT, Some(&cache))
        .unwrap()
        .component;
    assert!(cache.status().unwrap().entries > 0);
    assert!(cache.status().unwrap().used_bytes > 0);
    let unrelated = directory.path().join("song.flac");
    std::fs::write(&unrelated, "music").unwrap();
    {
        let _guard = COMPILE.lock().unwrap();
        cache.clear().unwrap();
    }
    assert_eq!(cache.status().unwrap().used_bytes, 0);
    // The already compiled in-memory component still instantiates without a disk artifact.
    let linker = wasmtime::component::Linker::<()>::new(&engine);
    let mut store = wasmtime::Store::new(&engine, ());
    linker.instantiate(&mut store, &component).unwrap();
    assert_eq!(
        compile_with_cache(&engine, EMPTY_COMPONENT, Some(&cache))
            .unwrap()
            .cache,
        "miss"
    );
    assert_eq!(std::fs::read_to_string(unrelated).unwrap(), "music");
}

#[test]
fn compiled_code_survives_cache_and_engine_recreation() {
    let directory = tempfile::tempdir().unwrap();
    {
        let cache = CodeCache::new(directory.path().to_path_buf()).unwrap();
        assert_eq!(
            compile_with_cache(&cached_engine(&cache, true), EMPTY_COMPONENT, Some(&cache))
                .unwrap()
                .cache,
            "miss"
        );
    }
    let cache = CodeCache::new(directory.path().to_path_buf()).unwrap();
    assert_eq!(
        compile_with_cache(&cached_engine(&cache, true), EMPTY_COMPONENT, Some(&cache))
            .unwrap()
            .cache,
        "hit"
    );
}

#[test]
fn source_and_compiler_settings_invalidate_compiled_code() {
    let directory = tempfile::tempdir().unwrap();
    let cache = CodeCache::new(directory.path().to_path_buf()).unwrap();
    let first = cached_engine(&cache, true);
    assert_eq!(
        compile_with_cache(&first, EMPTY_COMPONENT, Some(&cache))
            .unwrap()
            .cache,
        "miss"
    );
    // A valid component with a different custom section has different source bytes.
    let mut changed = EMPTY_COMPONENT.to_vec();
    changed.extend_from_slice(&[0, 3, 1, b'x', 1]);
    assert_eq!(
        compile_with_cache(&first, &changed, Some(&cache))
            .unwrap()
            .cache,
        "miss"
    );
    assert_eq!(
        compile_with_cache(&cached_engine(&cache, false), EMPTY_COMPONENT, Some(&cache))
            .unwrap()
            .cache,
        "miss"
    );
}

#[test]
fn corrupt_compiled_code_is_rebuilt_and_then_reused() {
    let directory = tempfile::tempdir().unwrap();
    let cache = CodeCache::new(directory.path().to_path_buf()).unwrap();
    let engine = cached_engine(&cache, true);
    compile_with_cache(&engine, EMPTY_COMPONENT, Some(&cache)).unwrap();
    let files = artifacts(directory.path());
    assert!(!files.is_empty());
    for file in files {
        std::fs::write(file, b"broken cache").unwrap();
    }
    assert_eq!(
        compile_with_cache(&engine, EMPTY_COMPONENT, Some(&cache))
            .unwrap()
            .cache,
        "miss"
    );
    assert_eq!(
        compile_with_cache(&engine, EMPTY_COMPONENT, Some(&cache))
            .unwrap()
            .cache,
        "hit"
    );
}

#[test]
fn unavailable_cache_does_not_prevent_uncached_compilation() {
    let file = tempfile::NamedTempFile::new().unwrap();
    assert!(CodeCache::new(file.path().to_path_buf()).is_err());
    assert_eq!(
        compile_with_cache(&Engine::default(), EMPTY_COMPONENT, None)
            .unwrap()
            .cache,
        "disabled"
    );
}

#[test]
#[ignore = "Requires built QQ Component; set NONS_WASM_BENCH_CACHE to reuse across processes"]
fn qq_component_compilation_cache_benchmark() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temporary = tempfile::tempdir().unwrap();
    let directory = std::env::var_os("NONS_WASM_BENCH_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_path_buf());
    let bytes = std::fs::read(root.join("adapters/qqmusic/dist/adapter.component.wasm")).unwrap();
    let cache = CodeCache::new(directory).unwrap();
    let engine = cached_engine(&cache, true);
    let compiled = compile_with_cache(&engine, &bytes, Some(&cache)).unwrap();
    eprintln!(
        "QQ compilation cache={} elapsed_ms={} wasm_bytes={}",
        compiled.cache,
        compiled.elapsed.as_millis(),
        bytes.len()
    );
    if let Ok(expected) = std::env::var("NONS_WASM_EXPECT_CACHE") {
        assert_eq!(compiled.cache, expected);
    }
}
