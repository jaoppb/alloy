//! [`HostPlatform`] answers for the OS the tests run on — the three CI
//! runners (Linux, macOS, Windows) each exercise their own module.

use platform::{FontDirectories, FontLocator, GenericFamily, HostPlatform};

const FAMILIES: [GenericFamily; 3] = [
    GenericFamily::SansSerif,
    GenericFamily::Serif,
    GenericFamily::Monospace,
];

#[test]
fn every_supported_host_names_font_directories_and_absolute_candidates() {
    let host = HostPlatform::new();
    let directories = host.font_directories();
    assert!(
        cfg!(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows"
        ))) || !directories.is_empty(),
        "a supported OS always names at least its system font directory"
    );
    for family in FAMILIES {
        let candidates = host.generic_family_candidates(family);
        assert!(
            candidates.iter().all(std::path::Path::is_absolute),
            "{family:?} candidates are absolute install paths"
        );
    }
}

#[test]
fn font_directories_keep_their_preference_order() {
    let directories: FontDirectories = ["/first", "/second"]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect();
    let listed: Vec<_> = directories.iter().collect();
    assert_eq!(
        listed,
        [
            std::path::Path::new("/first"),
            std::path::Path::new("/second")
        ]
    );
}
