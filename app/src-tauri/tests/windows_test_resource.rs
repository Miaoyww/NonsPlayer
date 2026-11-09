#[test]
fn test_resource_activates_common_controls_v6() {
    let manifest = include_str!("windows-test.manifest");
    assert!(manifest.contains("Microsoft.Windows.Common-Controls"));
    assert!(manifest.contains("version=\"6.0.0.0\""));
    assert!(manifest.contains("publicKeyToken=\"6595b64144ccf1df\""));
}
