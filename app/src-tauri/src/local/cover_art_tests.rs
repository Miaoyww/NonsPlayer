use super::*;

fn png(color: u8) -> Vec<u8> {
    let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        2,
        2,
        image::Rgb([color, 0, 0]),
    ));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[test]
fn candidates_follow_priority_and_only_one_named_subdirectory() {
    let root = tempfile::tempdir().unwrap();
    for name in [
        "random.JPG",
        "cover.jpg",
        "Folder.PNG",
        "unsupported.webp",
        "x.tif",
        "x.avif",
        "last.JXL",
        "x.GIF",
        "x.bmp",
        "x.jpeg",
    ] {
        std::fs::write(root.path().join(name), []).unwrap();
    }
    for name in ["Scans", "art", "arbitrary"] {
        std::fs::create_dir(root.path().join(name)).unwrap();
        std::fs::write(root.path().join(name).join("front.png"), []).unwrap();
    }
    std::fs::create_dir(root.path().join("art/deeper")).unwrap();
    std::fs::write(root.path().join("art/deeper/ignored.png"), []).unwrap();
    let paths = candidates(root.path())
        .into_iter()
        .map(|path| normalized(path.strip_prefix(root.path()).unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            "Folder.PNG",
            "cover.jpg",
            "last.JXL",
            "random.JPG",
            "x.GIF",
            "x.bmp",
            "x.jpeg",
            "Scans/front.png",
            "art/front.png"
        ]
    );
}

#[test]
fn assets_deduplicate_and_repair_missing_or_changed_content() {
    let root = tempfile::tempdir().unwrap();
    let bytes = png(3);
    let path = asset(&bytes, root.path()).unwrap();
    assert!(valid(Path::new(&path)));
    assert_eq!(asset(&bytes, root.path()).unwrap(), path);
    std::fs::write(&path, png(4)).unwrap();
    assert!(!valid(Path::new(&path)));
    assert_eq!(asset(&bytes, root.path()).unwrap(), path);
    assert!(valid(Path::new(&path)));
    std::fs::remove_file(&path).unwrap();
    assert_eq!(asset(&bytes, root.path()).unwrap(), path);
    assert!(valid(Path::new(&path)));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn external_skips_invalid_candidates_and_writes_beyond_the_former_total_limit() {
    let root = tempfile::tempdir().unwrap();
    let covers = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Folder.PNG"), b"not an image").unwrap();
    std::fs::write(root.path().join("cover.jpg"), vec![0; MAX_LYRIC_BYTES + 1]).unwrap();
    let bytes = png(1);
    std::fs::write(root.path().join("a.png"), &bytes).unwrap();
    std::fs::File::create(covers.path().join("existing.img"))
        .unwrap()
        .set_len(32 * 1024 * 1024)
        .unwrap();
    let found = external(root.path(), covers.path()).unwrap();
    assert_eq!(std::fs::read(found).unwrap(), bytes);
    assert!(asset(&png(2), covers.path()).is_some());
}

#[test]
fn signatures_change_on_add_replace_delete_and_ignore_unsupported_files() {
    let root = tempfile::tempdir().unwrap();
    let empty = signature(root.path());
    std::fs::write(root.path().join("ignored.webp"), png(1)).unwrap();
    assert_eq!(signature(root.path()), empty);
    let path = root.path().join("any.PNG");
    std::fs::write(&path, png(1)).unwrap();
    let added = signature(root.path());
    assert_ne!(empty, added);
    std::fs::write(&path, b"changed-size").unwrap();
    assert_ne!(signature(root.path()), added);
    std::fs::remove_file(path).unwrap();
    assert_eq!(signature(root.path()), empty);
    for folder in SUBDIRECTORIES {
        let folder = root.path().join(folder);
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("any.png"), png(2)).unwrap();
    }
    assert_eq!(candidates(root.path()).len(), SUBDIRECTORIES.len());
}

#[test]
fn dimensions_and_truncated_images_are_rejected() {
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(8193, 1)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let covers = tempfile::tempdir().unwrap();
    assert!(asset(encoded.get_ref(), covers.path()).is_none());
    let mut truncated = png(0);
    truncated.truncate(40);
    assert!(asset(&truncated, covers.path()).is_none());
    assert!(asset(&[0xff, 0x0a, 0], covers.path()).is_none());
}

#[test]
fn validation_cache_is_bounded_to_one_scan_and_asset_writes_refresh_it() {
    let root = tempfile::tempdir().unwrap();
    let bytes = png(1);
    let path = asset(&bytes, root.path()).unwrap();
    {
        let _session = ValidationSession::new();
        assert!(valid(Path::new(&path)));
        std::fs::write(&path, b"broken").unwrap();
        assert!(valid(Path::new(&path)));
    }
    {
        let _session = ValidationSession::new();
        assert!(!valid(Path::new(&path)));
        asset(&bytes, root.path()).unwrap();
        assert!(valid(Path::new(&path)));
    }
    std::fs::remove_file(&path).unwrap();
    assert!(!valid(Path::new(&path)));
}

#[test]
fn jpeg_xl_is_decoded_and_published_as_browser_readable_png() {
    // A 2×2 red RGB image encoded with zune-jpegxl 0.5.2.
    let jxl = [
        255, 10, 8, 0, 2, 128, 72, 8, 2, 1, 0, 160, 2, 75, 24, 155, 156, 113, 132, 3, 56, 128, 3,
        56, 32, 74, 192, 57, 5, 1, 0, 32, 68, 128, 8, 16, 1, 34, 64, 132, 255, 247, 239, 249, 239,
        161, 49, 231, 156, 107, 237, 115, 111, 146, 36, 9, 1, 85, 85, 85, 85, 85, 213, 255, 255,
        255, 115, 239, 235, 238, 238, 238, 134, 255, 247, 239, 249, 239, 161, 49, 231, 156, 107,
        237, 115, 111, 146, 36, 9, 1, 85, 85, 85, 85, 85, 213, 255, 255, 255, 115, 239, 235, 238,
        238, 238, 134, 255, 247, 239, 249, 239, 161, 49, 231, 156, 107, 237, 115, 111, 146, 36, 9,
        1, 85, 85, 85, 85, 85, 213, 255, 255, 255, 115, 239, 235, 238, 238, 238, 134, 255, 247,
        239, 249, 239, 161, 49, 231, 156, 107, 237, 115, 111, 146, 36, 9, 1, 85, 85, 85, 85, 85,
        213, 255, 255, 255, 115, 239, 235, 238, 238, 238, 62, 0, 199, 7, 30, 254, 192, 213, 7, 0,
    ];
    let root = tempfile::tempdir().unwrap();
    let covers = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("cover.JXL"), jxl).unwrap();
    let found = external(root.path(), covers.path()).unwrap();
    let bytes = std::fs::read(&found).unwrap();
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        image::ImageFormat::Png
    );
    let image = image::load_from_memory(&bytes).unwrap().to_rgb8();
    assert_eq!(image.dimensions(), (2, 2));
    assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0]);
    assert!(valid(Path::new(&found)));
    assert_eq!(asset(&jxl, covers.path()).unwrap(), found);
}
