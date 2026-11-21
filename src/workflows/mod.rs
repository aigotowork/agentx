/// Bundled scripts embedded at compile time.
pub const SCRIPTS: &[(&str, &str)] = &[
    ("confess", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/workflows/bundled/confess.sh"))),
    ("debate", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/workflows/bundled/debate.sh"))),
    ("fatcow", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/workflows/bundled/fatcow.sh"))),
    ("onidle", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/workflows/bundled/onidle.sh"))),
];
