//! Diagnostic native build identities follow Cargo's supported build graph.
//! See `.docs/model/state-ownership.md#native-build-compatibility` and the
//! daemon Task for invalidation cases and unsupported final-crate overrides.

use std::{env, fs, path::Path, process::Command};

fn main() {
    track_build_inputs();
    build_browser_assets();
    // Never consume an incoming AGENTX_NATIVE_BUILD_ID. A failed entropy
    // source fails this build; no version/time/default identity is substituted.
    println!("cargo::rustc-env=AGENTX_NATIVE_BUILD_ID=native-build-v1:{}", uuid::Uuid::new_v4());
}

fn track_build_inputs() {
    // Cargo recursively scans directory watches, including new embedded assets.
    for path in ["src", "tests/support", "Cargo.toml", "Cargo.lock", "build.rs", "web/observability", "web/workbench", "web/management", "packages/agentx-client/src", "packages/agentx-client/scripts", "packages/agentx-client/tsconfig.json", "packages/agentx-client/package.json", "packages/agentx-client/package-lock.json", "package.json", "package-lock.json"] {
        println!("cargo::rerun-if-changed={path}");
    }
    // Changes to an installed checkout compiler also invalidate the bundle.
    for path in [
        "node_modules/typescript",
        "packages/agentx-client/node_modules/lossless-json",
        "packages/agentx-client/node_modules/typescript",
        "node_modules/lossless-json",
        "node_modules/vite",
        "node_modules/rolldown",
        "node_modules/@rolldown/binding-wasm32-wasi",
        "node_modules/@vitejs/plugin-react",
        "node_modules/react",
        "node_modules/react-dom",
        "node_modules/@types/react",
        "node_modules/@types/react-dom",
    ] {
        if Path::new(path).exists() {
            println!("cargo::rerun-if-changed={path}");
        }
    }
    println!("cargo::rerun-if-env-changed=PATH");
    // Missing watches would make every cached build dirty. Cargo itself owns
    // toolchain/config discovery; explicitly watch the inputs present here.
    for path in ["rust-toolchain", "rust-toolchain.toml", ".cargo"] {
        if Path::new(path).try_exists().expect("inspect native build configuration") {
            println!("cargo::rerun-if-changed={path}");
        }
    }
    for name in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTC_BOOTSTRAP",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        "RUSTUP_TOOLCHAIN",
        "RUSTUP_HOME",
        "CARGO_HOME",
        "CARGO_BUILD_TARGET",
    ] {
        println!("cargo::rerun-if-env-changed={name}");
    }
    // TARGET/HOST, PROFILE/OPT_LEVEL/DEBUG, CARGO_FEATURE_* and CARGO_CFG_*
    // belong to Cargo's unit fingerprint, not global-env watcher semantics.
    // Both implicit and explicit target builds must preserve that separation.
    for unit in ["HOST", "TARGET"] {
        let target = env::var(unit).expect("Cargo supplies native build target/host");
        let key = target.to_ascii_uppercase().replace(['-', '.'], "_");
        for input in ["RUSTFLAGS", "LINKER"] {
            println!("cargo::rerun-if-env-changed=CARGO_TARGET_{key}_{input}");
        }
    }
}

/// Compile current browser sources in Cargo-owned output, never checkout/.bin bytes.
fn build_browser_assets() {
    // Public package dist is a derived input to every actual browser consumer.
    // Compile once per Cargo asset batch; no npm installation or old dist fallback.
    let root = std::path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let status = Command::new("node").arg(root.join("packages/agentx-client/scripts/build.mjs")).current_dir(&root).status().expect("SDK build requires locked package dependencies; see CONTRIBUTING.md");
    assert!(status.success(), "Public SDK compilation failed; refusing stale assets");
    build_browser_app("observability");
    build_browser_app("workbench");
    build_browser_app("management");
    build_migrated_ui("agent-space", "space");
    build_migrated_ui("inspector", "inspector");
}

/// Migrated applications retain nested scene/assets and license files. Compile
/// them into fresh output rather than embedding a running preview's dist.
fn build_migrated_ui(app: &str, prefix: &str) {
    let root = std::path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let source = root.join("web").join(app);
    for entry in fs::read_dir(&source).expect("read UI source inputs") {
        let entry = entry.expect("UI source input");
        let name = entry.file_name();
        if !matches!(name.to_str(), Some("node_modules" | "dist" | "tests")) {
            println!("cargo::rerun-if-changed={}", entry.path().display());
        }
    }
    let out = std::path::PathBuf::from(env::var_os("OUT_DIR").expect("build output"));
    let bundle = out.join(format!("{app}-web"));
    if bundle.exists() {
        fs::remove_dir_all(&bundle).expect("remove previous UI bundle");
    }
    let status = Command::new("node").arg(source.join("build.mjs")).arg(&bundle).current_dir(&root).status().expect("UI build requires locked module dependencies");
    assert!(status.success(), "UI compilation failed; refusing stale assets");
    let mut files = Vec::new();
    collect_ui_files(&bundle, &bundle, &mut files);
    files.sort();
    assert!(files.iter().any(|file| file == "index.html"), "UI needs index.html");
    assert!(files.len() <= 1024, "UI asset count exceeds bounded bundle");
    let mut generated = String::from("// Freshly compiled finite UI assets.\n&[\n");
    let mut total = 0;
    for file in files {
        let content_type = match Path::new(&file).extension().and_then(|ext| ext.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("js" | "mjs") => "text/javascript; charset=utf-8",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("svg") => "image/svg+xml",
            Some("gif") => "image/gif",
            Some("woff2") => "font/woff2",
            Some("json") => "application/json",
            Some("md" | "txt") | None => "text/plain; charset=utf-8",
            _ => panic!("unsupported UI asset: {file}"),
        };
        let size = fs::metadata(bundle.join(&file)).expect("UI asset metadata").len();
        assert!(size <= 8 * 1024 * 1024, "UI asset exceeds bound");
        total += size;
        assert!(total <= 32 * 1024 * 1024, "UI bundle exceeds bound");
        let url = if file == "index.html" { format!("/{prefix}/") } else { format!("/{prefix}/{file}") };
        generated.push_str(&format!("    ({url:?}, {content_type:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{app}-web/{file}\"))),\n"));
    }
    generated.push_str("]\n");
    fs::write(out.join(format!("{app}_assets.rs")), generated).expect("write UI resource table");
}

fn collect_ui_files(root: &Path, directory: &Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(directory).expect("read UI bundle") {
        let entry = entry.expect("UI bundle entry");
        let kind = entry.file_type().expect("UI bundle entry type");
        assert!(!kind.is_symlink(), "UI bundle cannot contain symlinks");
        let path = entry.path();
        if kind.is_dir() {
            collect_ui_files(root, &path, files);
        } else {
            assert!(kind.is_file(), "UI bundle requires regular files");
            let relative = path.strip_prefix(root).expect("UI bundle relative path").to_str().expect("UTF-8 UI asset").replace('\\', "/");
            assert!(relative.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte)), "invalid UI asset path");
            assert!(files.len() < 1024, "UI asset count exceeds bound");
            files.push(relative);
        }
    }
}

fn build_browser_app(app: &str) {
    let root = env::var_os("CARGO_MANIFEST_DIR").expect("Cargo supplies manifest directory");
    let out = env::var_os("OUT_DIR").expect("Cargo supplies build output directory");
    let out = Path::new(&out);
    let bundle = out.join(format!("{app}-web"));
    if bundle.exists() {
        fs::remove_dir_all(&bundle).expect("remove previous Cargo browser bundle");
    }
    let status = Command::new("node").arg(Path::new(&root).join(format!("web/{app}/build.mjs"))).arg(&bundle).current_dir(&root).status().expect("browser bundle requires Node.js and TypeScript; see CONTRIBUTING.md#standalone-binary-packaging");
    assert!(status.success(), "browser bundle compilation failed; refusing to embed stale assets");

    let mut files: Vec<_> = fs::read_dir(&bundle).expect("read generated browser bundle").map(|entry| entry.expect("read browser asset entry").file_name().into_string().expect("UTF-8 browser asset name")).collect();
    files.sort();
    let mut generated = String::from("// Generated from freshly compiled browser assets by build.rs.\n&[\n");
    let mut count = 0;
    let mut total = 0;
    let mut has_index = false;
    for file in files {
        if file == "manifest.json" {
            continue;
        }
        assert!(file.bytes().all(|byte| byte.is_ascii_alphanumeric() || b".-".contains(&byte)), "invalid browser asset filename");
        let content_type = match Path::new(&file).extension().and_then(|ext| ext.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("js") => "text/javascript; charset=utf-8",
            _ => panic!("unsupported generated browser asset: {file}"),
        };
        let size = fs::metadata(bundle.join(&file)).expect("read browser asset metadata").len();
        assert!(size <= 4 * 1024 * 1024, "browser asset exceeds size limit");
        total += size;
        count += 1;
        let url = if file == "index.html" {
            has_index = true;
            "/".to_owned()
        } else {
            format!("/{file}")
        };
        generated.push_str(&format!("    ({url:?}, {content_type:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{app}-web/{file}\"))),\n"));
    }
    assert!(has_index && count <= 128 && total <= 16 * 1024 * 1024, "invalid browser bundle bounds or missing index.html");
    generated.push_str("]\n");
    fs::write(out.join(format!("{app}_assets.rs")), generated).expect("write embedded browser asset table");
}
