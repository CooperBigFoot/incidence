//! Verify initialization preserves the self-contained doctrine sync command.
//!
//! `initialize : RustTemplate × PackageName → InitializedRustProject`

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const PROJECT_NAME: &str = "acme_widget";
const PACKAGE_NAME: &str = "acme-widget";
const SOURCE_BLOCK: &str = "<!-- BEGIN SYNCED DOCTRINE; source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d -->\nFour rules. They are one design stance seen four ways: a module means one thing, receives exactly what it needs, in types that cannot lie, and dies rather than guess.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n<!-- END SYNCED DOCTRINE -->";
const ROOT_MANIFEST: &str = "[workspace]\nmembers = [\"crates/*\"]\n\n# Denied only in library crates: each crates/* manifest opts in via `[lints] workspace = true`.\n# The root binary crate deliberately does not inherit these (`.expect(\"reason\")` allowed in main.rs).\n[workspace.lints.clippy]\nunwrap_used = \"deny\"\nexpect_used = \"deny\"\nprint_stdout = \"deny\"\nprint_stderr = \"deny\"\n\n[package]\nname = \"acme-widget\"\nversion = \"0.1.4\"\nedition = \"2024\"\n\n[dependencies]\nacme-widget-core = { path = \"crates/core\" }\nanyhow = \"1\"\ntracing = \"0.1\"\ntracing-subscriber = { version = \"0.3\", features = [\"env-filter\"] }\n";
const CORE_MANIFEST: &str = "[package]\nname = \"acme-widget-core\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nthiserror = \"2\"\ntracing = \"0.1\"\n\n[lints]\nworkspace = true\n";
const INITIALIZED_ENTRIES: &[&str] = &[
    ".claude",
    ".claude/settings.json",
    ".gitignore",
    ".pce",
    ".pce/repository-contract.json",
    "AGENTS.md",
    "CLAUDE.md",
    "CONTEXT.md",
    "Cargo.lock",
    "Cargo.toml",
    "README.md",
    "clippy.toml",
    "crates",
    "crates/core",
    "crates/core/Cargo.toml",
    "crates/core/src",
    "crates/core/src/lib.rs",
    "src",
    "src/bin",
    "src/bin/sync_doctrine.rs",
    "src/main.rs",
    "tests",
    "tests/init_template.rs",
    "tests/sync_doctrine.rs",
];
const TRACKED_FILES: &[&str] = &[
    ".claude/settings.json",
    ".gitignore",
    ".pce/repository-contract.json",
    "AGENTS.md",
    "CLAUDE.md",
    "CONTEXT.md",
    "Cargo.lock",
    "Cargo.toml",
    "README.md",
    "clippy.toml",
    "crates/core/Cargo.toml",
    "crates/core/src/lib.rs",
    "init.sh",
    "src/bin/sync_doctrine.rs",
    "src/main.rs",
    "tests/init_template.rs",
    "tests/sync_doctrine.rs",
];

static SERIAL: AtomicU64 = AtomicU64::new(0);

struct TemporaryTree {
    path: PathBuf,
}

impl TemporaryTree {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rustplate-init-{}-{nanos}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary tree must be created");
        Self { path }
    }
}

impl Drop for TemporaryTree {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path)
            && !std::thread::panicking()
        {
            panic!("temporary tree must be removed: {error}");
        }
    }
}

fn copy_tracked_template(source: &Path, target: &Path) {
    for relative in TRACKED_FILES {
        let source_path = source.join(relative);
        let target_path = target.join(relative);
        fs::create_dir_all(
            target_path
                .parent()
                .expect("every tracked path must have a parent"),
        )
        .expect("tracked parent directory must be created");
        fs::copy(source_path, target_path).expect("tracked file must be copied");
    }
}

fn collect_entries(root: &Path) -> Vec<String> {
    fn visit(root: &Path, directory: &Path, entries: &mut Vec<String>) {
        for item in fs::read_dir(directory).expect("template directory must be readable") {
            let path = item.expect("template entry must be readable").path();
            entries.push(
                path.strip_prefix(root)
                    .expect("entry must be below template root")
                    .to_str()
                    .expect("fixture paths must be UTF-8")
                    .to_owned(),
            );
            if path.is_dir() {
                visit(root, &path, entries);
            }
        }
    }

    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort();
    entries
}

fn run_without_home(
    program: &OsStr,
    current_dir: &Path,
    arguments: &[&str],
    home: &Path,
) -> Output {
    Command::new(program)
        .args(arguments)
        .current_dir(current_dir)
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .expect("isolated command must run")
}

#[test]
fn initialization_preserves_doctrine_sync_and_only_changes_template_owned_files() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tree = TemporaryTree::new();
    let template = tree.path.join("template");
    let empty_home = tree.path.join("empty-home");
    fs::create_dir(&template).expect("template root must be created");
    fs::create_dir(&empty_home).expect("empty HOME must be created");
    copy_tracked_template(source_root, &template);

    let original_agents = fs::read_to_string(template.join("AGENTS.md"))
        .expect("template AGENTS.md must be readable");
    let original_sync = fs::read(template.join("src/bin/sync_doctrine.rs"))
        .expect("template sync command must be readable");
    let untouched = TRACKED_FILES
        .iter()
        .filter(|path| {
            !matches!(
                **path,
                "AGENTS.md" | "Cargo.toml" | "README.md" | "crates/core/Cargo.toml" | "init.sh"
            )
        })
        .map(|path| {
            (
                *path,
                fs::read(template.join(path)).expect("untouched tracked file must be readable"),
            )
        })
        .collect::<Vec<_>>();

    let initialization = run_without_home(
        OsStr::new("/bin/bash"),
        &template,
        &["init.sh", PROJECT_NAME],
        &empty_home,
    );
    assert_eq!(initialization.status.code(), Some(0));
    assert!(initialization.stderr.is_empty());
    assert_eq!(
        std::str::from_utf8(&initialization.stdout).expect("stdout must be UTF-8"),
        "Initializing project: acme-widget\nDone! Project 'acme-widget' is ready.\nRun 'cargo check' to verify.\n"
    );
    assert!(
        fs::read_dir(&empty_home)
            .expect("empty HOME must remain readable")
            .next()
            .is_none()
    );
    assert!(!template.join("init.sh").exists());
    assert_eq!(collect_entries(&template), INITIALIZED_ENTRIES);
    assert_eq!(
        fs::read_to_string(template.join("Cargo.toml")).expect("root manifest must be readable"),
        ROOT_MANIFEST
    );
    assert_eq!(
        fs::read_to_string(template.join("crates/core/Cargo.toml"))
            .expect("core manifest must be readable"),
        CORE_MANIFEST
    );
    assert_eq!(
        fs::read_to_string(template.join("README.md")).expect("README must be readable"),
        "# acme-widget\n"
    );
    for (path, expected) in untouched {
        assert_eq!(
            fs::read(template.join(path)).expect("untouched tracked file must remain readable"),
            expected,
            "initialization changed {path}"
        );
    }
    assert_eq!(
        fs::read(template.join("src/bin/sync_doctrine.rs"))
            .expect("preserved sync command must be readable"),
        original_sync
    );

    let initialized_agents = fs::read_to_string(template.join("AGENTS.md"))
        .expect("initialized AGENTS.md must be readable");
    assert_eq!(
        original_agents
            .matches("DESCRIBE THE PROJECT BRIEFLY")
            .count(),
        1
    );
    assert_eq!(
        initialized_agents,
        original_agents.replacen("DESCRIBE THE PROJECT BRIEFLY", PACKAGE_NAME, 1)
    );
    assert_eq!(initialized_agents.matches(SOURCE_BLOCK).count(), 1);
    let end_marker = "<!-- END SYNCED DOCTRINE -->";
    let original_suffix = original_agents
        .split_once(end_marker)
        .expect("original doctrine end marker must exist")
        .1;
    let initialized_suffix = initialized_agents
        .split_once(end_marker)
        .expect("initialized doctrine end marker must exist")
        .1;
    assert_eq!(initialized_suffix, original_suffix);

    let before_sync =
        fs::read(template.join("AGENTS.md")).expect("initialized AGENTS.md bytes must be readable");
    for _ in 0..2 {
        let sync = run_without_home(
            OsStr::new(env!("CARGO_BIN_EXE_sync_doctrine")),
            &template,
            &[],
            &empty_home,
        );
        assert_eq!(sync.status.code(), Some(0));
        assert!(sync.stderr.is_empty());
        assert_eq!(
            std::str::from_utf8(&sync.stdout).expect("stdout must be UTF-8"),
            "UNCHANGED: AGENTS.md: source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d\n"
        );
        assert_eq!(
            fs::read(template.join("AGENTS.md"))
                .expect("unchanged AGENTS.md bytes must be readable"),
            before_sync
        );
        assert!(
            fs::read_dir(&empty_home)
                .expect("empty HOME must remain readable")
                .next()
                .is_none()
        );
    }
}
