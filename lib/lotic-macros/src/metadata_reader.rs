use cargo_metadata::MetadataCommand;

pub fn read_metadata(identifier: &str) -> Option<String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let local_manifest_path = std::path::Path::new(&manifest_dir).join("Cargo.toml");

    let metadata = MetadataCommand::new()
        .manifest_path(&local_manifest_path)
        .no_deps()
        .exec()
        .expect("Failed to fetch cargo metadata for the current crate");

    let package = metadata
        .packages
        .iter()
        .find(|pkg| pkg.manifest_path == local_manifest_path)
        .unwrap_or_else(|| {
            panic!(
                "Could not find package info for manifest at {:?}",
                local_manifest_path
            )
        });

    let package_name = &package.name;

    let json_path = metadata
        .target_directory
        .join(format!("{package_name}-{identifier}"));

    std::fs::read_to_string(json_path).ok()
}
