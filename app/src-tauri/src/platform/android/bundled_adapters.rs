use std::path::Path;

/// External adapters are installed by the adapter store at runtime.
pub(crate) fn materialize(root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(root)
}
