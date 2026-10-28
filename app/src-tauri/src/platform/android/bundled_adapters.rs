use std::{fs, path::Path};

struct AdapterResource {
    name: &'static str,
    manifest: &'static [u8],
    wasm: &'static [u8],
    licenses: &'static [(&'static str, &'static [u8])],
}

const ADAPTERS: &[AdapterResource] = &[
    AdapterResource {
        name: "qqmusic",
        manifest: include_bytes!("../../../bundled-adapters/qqmusic/manifest.json"),
        wasm: include_bytes!("../../../bundled-adapters/qqmusic/adapter.component.wasm"),
        licenses: &[
            (
                "LICENSE.dependencies",
                include_bytes!("../../../bundled-adapters/qqmusic/LICENSE.dependencies"),
            ),
            (
                "LICENSE.qq-music-api",
                include_bytes!("../../../bundled-adapters/qqmusic/LICENSE.qq-music-api"),
            ),
        ],
    },
    AdapterResource {
        name: "gdstudio",
        manifest: include_bytes!("../../../bundled-adapters/gdstudio/manifest.json"),
        wasm: include_bytes!("../../../bundled-adapters/gdstudio/adapter.component.wasm"),
        licenses: &[(
            "LICENSE.dependencies",
            include_bytes!("../../../bundled-adapters/gdstudio/LICENSE.dependencies"),
        )],
    },
    AdapterResource {
        name: "xiaobai",
        manifest: include_bytes!("../../../bundled-adapters/xiaobai/manifest.json"),
        wasm: include_bytes!("../../../bundled-adapters/xiaobai/adapter.component.wasm"),
        licenses: &[(
            "LICENSE.dependencies",
            include_bytes!("../../../bundled-adapters/xiaobai/LICENSE.dependencies"),
        )],
    },
];

pub(crate) fn materialize(root: &Path) -> std::io::Result<()> {
    fs::create_dir_all(root)?;
    for adapter in ADAPTERS {
        let directory = root.join(adapter.name);
        fs::create_dir_all(&directory)?;
        fs::write(directory.join("manifest.json"), adapter.manifest)?;
        fs::write(directory.join("adapter.component.wasm"), adapter.wasm)?;
        for (name, contents) in adapter.licenses {
            fs::write(directory.join(name), contents)?;
        }
    }
    Ok(())
}
