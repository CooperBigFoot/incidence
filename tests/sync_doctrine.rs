use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SOURCE_STAMP: &str = "59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d";
const SOURCE_BODY: &str = "Four rules. They are one design stance seen four ways: a module means one thing, receives exactly what it needs, in types that cannot lie, and dies rather than guess.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n";
const STALE_STAMP: &str = "276add6ad3059d4de66db2250890ce61c7cda02a9c4da8622abf51afd2d7054a";
const EDITED_STAMP: &str = "e7107a175f6aa7daf3d1ab9953bdfc5fa01b7bde885324f060bd73951e75abbb";

const SOURCE_BLOCK: &str = "<!-- BEGIN SYNCED DOCTRINE; source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d -->\nFour rules. They are one design stance seen four ways: a module means one thing, receives exactly what it needs, in types that cannot lie, and dies rather than guess.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n<!-- END SYNCED DOCTRINE -->";
const STALE_TARGET: &str = "# Local instructions\n\n<!-- BEGIN SYNCED DOCTRINE; source-sha256=276add6ad3059d4de66db2250890ce61c7cda02a9c4da8622abf51afd2d7054a -->\nFour rules. This fixture is a clean older source.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n<!-- END SYNCED DOCTRINE -->\n\nProject-local note.\n";
const UPDATED_TARGET: &str = "# Local instructions\n\n<!-- BEGIN SYNCED DOCTRINE; source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d -->\nFour rules. They are one design stance seen four ways: a module means one thing, receives exactly what it needs, in types that cannot lie, and dies rather than guess.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n<!-- END SYNCED DOCTRINE -->\n\nProject-local note.\n";
const EDITED_TARGET: &str = "# Local instructions\n\n<!-- BEGIN SYNCED DOCTRINE; source-sha256=276add6ad3059d4de66db2250890ce61c7cda02a9c4da8622abf51afd2d7054a -->\nFour rules. This fixture is a locally edited older source.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n<!-- END SYNCED DOCTRINE -->\n\nProject-local note.\n";
const MARKERLESS: &str = "Project-local instructions only.\n";
const MALFORMED: &str =
    "<!-- BEGIN SYNCED DOCTRINE; source-sha256=nope -->\nbody\n<!-- END SYNCED DOCTRINE -->\n";
const DUPLICATE: &str = "<!-- BEGIN SYNCED DOCTRINE; source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d -->\nbody\n<!-- BEGIN SYNCED DOCTRINE; source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d -->\nbody\n<!-- END SYNCED DOCTRINE -->\n";
const REVERSED: &str = "<!-- END SYNCED DOCTRINE -->\n<!-- BEGIN SYNCED DOCTRINE; source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d -->\nbody\n";
const UNSTAMPED: &str = "# Project Instructions\n\n## 2. Design Doctrine\n\nFour rules. This markerless fixture carries no source stamp.\n";
const PARTIAL_BEGIN: &str = "# Project Instructions\n\n<!-- BEGIN SYNCED DOCTRINE; source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d -->\nbody\n";
const PARTIAL_END: &str = "# Project Instructions\n\nbody\n<!-- END SYNCED DOCTRINE -->\n";

static SERIAL: AtomicU64 = AtomicU64::new(0);

struct TemporaryTree {
    path: PathBuf,
}

impl TemporaryTree {
    fn new(case: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rustplate-sync-{case}-{}-{nanos}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary tree must be created");
        Self { path }
    }
}

impl Drop for TemporaryTree {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path) {
            if !std::thread::panicking() {
                panic!("temporary tree must be removed: {error}");
            }
        }
    }
}

fn invoke(tree: &TemporaryTree, target: &Path) -> Output {
    assert!(!tree.path.join("empty-home").exists());
    let output = Command::new(env!("CARGO_BIN_EXE_sync_doctrine"))
        .env_clear()
        .env("HOME", tree.path.join("empty-home"))
        .arg(target)
        .output()
        .expect("sync command must run");
    assert!(!tree.path.join("empty-home").exists());
    output
}

fn invoke_check_without_writes(tree: &TemporaryTree, target: &Path) -> Output {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md");
    let source_before = fs::read(&source).expect("source bytes must be readable");
    let target_before = fs::read(target).expect("target bytes must be readable");
    let modified_before = fs::metadata(target)
        .expect("target metadata must be readable")
        .modified()
        .expect("target modification time must be readable");
    assert!(!tree.path.join("empty-home").exists());
    let output = Command::new(env!("CARGO_BIN_EXE_sync_doctrine"))
        .env_clear()
        .env("HOME", tree.path.join("empty-home"))
        .arg("--check")
        .arg(target)
        .output()
        .expect("check command must run");
    assert!(!tree.path.join("empty-home").exists());
    assert_eq!(
        fs::read(&source).expect("source bytes must remain readable"),
        source_before
    );
    assert_eq!(
        fs::read(target).expect("target bytes must remain readable"),
        target_before
    );
    assert_eq!(
        fs::metadata(target)
            .expect("target metadata must remain readable")
            .modified()
            .expect("target modification time must remain readable"),
        modified_before
    );
    output
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout must be UTF-8")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr must be UTF-8")
}

#[test]
fn repository_contains_the_exact_language_neutral_spine() {
    let agents = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md"))
        .expect("repository AGENTS.md must be readable");
    assert_eq!(agents.matches("<!-- BEGIN SYNCED DOCTRINE;").count(), 1);
    assert_eq!(agents.matches("<!-- END SYNCED DOCTRINE -->").count(), 1);
    assert_eq!(
        SOURCE_BLOCK,
        format!(
            "<!-- BEGIN SYNCED DOCTRINE; source-sha256={SOURCE_STAMP} -->\n{SOURCE_BODY}<!-- END SYNCED DOCTRINE -->"
        )
    );
    assert!(agents.contains(SOURCE_BLOCK));
    let end = agents
        .find("<!-- END SYNCED DOCTRINE -->")
        .expect("end marker must exist");
    for prefix in [
        "Before implementing a module, state in one line in its `//!` doc",
        "`src/main.rs` is the composition root",
        "Encode domain invariants in the type system",
        "An error is either propagated with `?`",
        "Library crates use `thiserror`",
        "Documentation is for agents landing in the code",
    ] {
        assert!(
            agents[end..].contains(prefix),
            "Rust-specific prefix must follow the block: {prefix}"
        );
    }
}

#[test]
fn updates_a_clean_stale_target_then_performs_no_idempotent_write() {
    let tree = TemporaryTree::new("idempotence");
    let target = tree.path.join("AGENTS.md");
    fs::write(&target, STALE_TARGET).expect("stale fixture must be written");

    let updated = invoke(&tree, &target);
    assert_eq!(updated.status.code(), Some(0));
    assert!(updated.stderr.is_empty());
    assert_eq!(
        stdout(&updated),
        format!(
            "UPDATED: {}: source-sha256={SOURCE_STAMP}\n",
            target.display()
        )
    );
    assert_eq!(
        fs::read_to_string(&target).expect("updated target must be readable"),
        UPDATED_TARGET
    );

    let before = fs::metadata(&target)
        .expect("updated target metadata must be readable")
        .modified()
        .expect("updated target modification time must be readable");
    let unchanged = invoke(&tree, &target);
    let after = fs::metadata(&target)
        .expect("unchanged target metadata must be readable")
        .modified()
        .expect("unchanged target modification time must be readable");
    assert_eq!(unchanged.status.code(), Some(0));
    assert!(unchanged.stderr.is_empty());
    assert_eq!(
        stdout(&unchanged),
        format!(
            "UNCHANGED: {}: source-sha256={SOURCE_STAMP}\n",
            target.display()
        )
    );
    assert_eq!(
        fs::read_to_string(&target).expect("unchanged target must be readable"),
        UPDATED_TARGET
    );
    assert_eq!(before, after);
}

#[test]
fn refuses_a_locally_edited_target_without_writing() {
    let tree = TemporaryTree::new("conflict");
    let target = tree.path.join("AGENTS.md");
    fs::write(&target, EDITED_TARGET).expect("edited fixture must be written");

    let output = invoke(&tree, &target);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr(&output),
        format!(
            "CONFLICT: {}: doctrine block was edited locally: marker source-sha256={STALE_STAMP}, content source-sha256={EDITED_STAMP}; no changes written\n",
            target.display()
        )
    );
    assert_eq!(
        fs::read_to_string(&target).expect("edited target must remain readable"),
        EDITED_TARGET
    );
}

fn assert_malformed(case: &str, contents: &str) {
    let tree = TemporaryTree::new(case);
    let target = tree.path.join("AGENTS.md");
    fs::write(&target, contents).expect("invalid fixture must be written");

    let output = invoke(&tree, &target);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr(&output),
        format!(
            "ERROR: {}: expected exactly one well-formed synced doctrine block; no changes written\n",
            target.display()
        )
    );
    assert_eq!(
        fs::read_to_string(&target).expect("invalid target must remain readable"),
        contents
    );
}

#[test]
fn refuses_markerless_target_without_writing() {
    assert_malformed("markerless", MARKERLESS);
}

#[test]
fn refuses_malformed_stamp_without_writing() {
    assert_malformed("malformed", MALFORMED);
}

#[test]
fn refuses_duplicate_begin_marker_without_writing() {
    assert_malformed("duplicate", DUPLICATE);
}

#[test]
fn refuses_reversed_markers_without_writing() {
    assert_malformed("reversed", REVERSED);
}

#[test]
fn refuses_a_nonexistent_target_without_creating_it() {
    let tree = TemporaryTree::new("nonexistent");
    let target = tree.path.join("AGENTS.md");

    let output = invoke(&tree, &target);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr(&output),
        format!(
            "ERROR: {}: instruction file does not exist; no changes written\n",
            target.display()
        )
    );
    assert!(!target.exists());
}

#[test]
fn check_classifies_current_without_writing() {
    let tree = TemporaryTree::new("check-current");
    let target = tree.path.join("AGENTS.md");
    fs::write(&target, UPDATED_TARGET).expect("current fixture must be written");

    let output = invoke_check_without_writes(&tree, &target);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        stdout(&output),
        format!(
            "CURRENT: {}: installed source-sha256={SOURCE_STAMP}, available source-sha256={SOURCE_STAMP}\n",
            target.display()
        )
    );
}

#[test]
fn check_classifies_stale_and_names_stamp_gap_without_writing() {
    let tree = TemporaryTree::new("check-stale");
    let target = tree.path.join("AGENTS.md");
    fs::write(&target, STALE_TARGET).expect("stale fixture must be written");

    let output = invoke_check_without_writes(&tree, &target);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(
        stdout(&output),
        format!(
            "STALE: {}: installed source-sha256={STALE_STAMP}, available source-sha256={SOURCE_STAMP}\n",
            target.display()
        )
    );
}

#[test]
fn check_classifies_local_edit_and_names_all_stamps_without_writing() {
    let tree = TemporaryTree::new("check-locally-edited");
    let target = tree.path.join("AGENTS.md");
    fs::write(&target, EDITED_TARGET).expect("edited fixture must be written");

    let output = invoke_check_without_writes(&tree, &target);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    assert_eq!(
        stdout(&output),
        format!(
            "LOCALLY_EDITED: {}: marker source-sha256={STALE_STAMP}, content source-sha256={EDITED_STAMP}, available source-sha256={SOURCE_STAMP}\n",
            target.display()
        )
    );
}

#[test]
fn check_classifies_unstamped_offline_with_empty_home_without_writing() {
    let tree = TemporaryTree::new("check-unstamped");
    let target = tree.path.join("metis/AGENTS.md");
    fs::create_dir(tree.path.join("metis")).expect("target parent must be created");
    fs::write(&target, UNSTAMPED).expect("unstamped fixture must be written");

    let output = invoke_check_without_writes(&tree, &target);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(
        stdout(&output),
        format!(
            "UNSTAMPED: {}: target carries no source stamp; available source-sha256={SOURCE_STAMP}\n",
            target.display()
        )
    );
    assert!(!stdout(&output).contains("installed source-sha256="));
}

fn assert_check_malformed(case: &str, contents: &str) {
    let tree = TemporaryTree::new(case);
    let target = tree.path.join("AGENTS.md");
    fs::write(&target, contents).expect("partial marker fixture must be written");

    let output = invoke_check_without_writes(&tree, &target);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr(&output),
        format!(
            "ERROR: {}: expected exactly one well-formed synced doctrine block; no changes written\n",
            target.display()
        )
    );
}

#[test]
fn check_rejects_partial_begin_marker_without_writing() {
    assert_check_malformed("check-partial-begin", PARTIAL_BEGIN);
}

#[test]
fn check_rejects_partial_end_marker_without_writing() {
    assert_check_malformed("check-partial-end", PARTIAL_END);
}

#[test]
fn check_refuses_a_nonexistent_target_without_creating_it() {
    let tree = TemporaryTree::new("check-nonexistent");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md");
    let target = tree.path.join("AGENTS.md");
    let source_before = fs::read(&source).expect("source bytes must be readable");
    assert!(!tree.path.join("empty-home").exists());

    let output = Command::new(env!("CARGO_BIN_EXE_sync_doctrine"))
        .env_clear()
        .env("HOME", tree.path.join("empty-home"))
        .arg("--check")
        .arg(&target)
        .output()
        .expect("check command must run");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr(&output),
        format!(
            "ERROR: {}: instruction file does not exist; no changes written\n",
            target.display()
        )
    );
    assert_eq!(
        fs::read(&source).expect("source bytes must remain readable"),
        source_before
    );
    assert!(!target.exists());
    assert!(!tree.path.join("empty-home").exists());
}

#[test]
fn check_rejects_extra_arguments_with_exact_usage() {
    let tree = TemporaryTree::new("check-usage");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md");
    let target = tree.path.join("AGENTS.md");
    fs::write(&target, UPDATED_TARGET).expect("current fixture must be written");
    let source_before = fs::read(&source).expect("source bytes must be readable");
    let target_before = fs::read(&target).expect("target bytes must be readable");
    assert!(!tree.path.join("empty-home").exists());

    let output = Command::new(env!("CARGO_BIN_EXE_sync_doctrine"))
        .env_clear()
        .env("HOME", tree.path.join("empty-home"))
        .arg("--check")
        .arg(&target)
        .arg("extra")
        .output()
        .expect("check command must run");
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr(&output),
        "Usage: sync_doctrine [--check] [AGENTS.md]\n"
    );
    assert_eq!(
        fs::read(&source).expect("source bytes must remain readable"),
        source_before
    );
    assert_eq!(
        fs::read(&target).expect("target bytes must remain readable"),
        target_before
    );
    assert!(!tree.path.join("empty-home").exists());
}

#[test]
fn temporary_tree_guard_removes_the_complete_tree() {
    let tree = TemporaryTree::new("idempotence");
    let path = tree.path.clone();
    fs::write(path.join("AGENTS.md"), "sentinel").expect("sentinel must be written");
    drop(tree);
    assert!(!path.exists());
}
