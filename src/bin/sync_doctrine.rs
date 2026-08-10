//! Synchronize or classify a content-stamped doctrine block without overwriting local edits.
//!
//! `sync_doctrine : StampedDoctrine × InstructionFile → SyncOutcome`
//! `check_doctrine : StampedDoctrine × InstructionFile → DoctrineStatus`

use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const BEGIN_TOKEN: &str = "<!-- BEGIN SYNCED DOCTRINE;";
const BEGIN_PREFIX: &str = "<!-- BEGIN SYNCED DOCTRINE; source-sha256=";
const END_MARKER: &str = "<!-- END SYNCED DOCTRINE -->";
const USAGE: &str = "Usage: sync_doctrine [--check] [AGENTS.md]";

#[derive(Debug, Eq, PartialEq)]
enum SyncOutcome {
    Updated,
    Unchanged,
}

impl fmt::Display for SyncOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Updated => formatter.write_str("UPDATED"),
            Self::Unchanged => formatter.write_str("UNCHANGED"),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum DoctrineStatus {
    Current {
        source_stamp: String,
    },
    Stale {
        installed_source_stamp: String,
        available_source_stamp: String,
    },
    LocallyEdited {
        marker_source_stamp: String,
        content_source_stamp: String,
        available_source_stamp: String,
    },
    Unstamped {
        available_source_stamp: String,
    },
}

#[derive(Debug)]
struct ParsedBlock<'a> {
    start: usize,
    end: usize,
    body: &'a str,
    marker_stamp: &'a str,
    rendered: &'a str,
}

#[derive(Debug)]
enum SyncError {
    Missing {
        path: PathBuf,
    },
    Malformed {
        path: PathBuf,
    },
    Conflict {
        path: PathBuf,
        marker_stamp: String,
        actual_stamp: String,
    },
    Io {
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for SyncError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { path } => write!(
                formatter,
                "{}: instruction file does not exist",
                path.display()
            ),
            Self::Malformed { path } => write!(
                formatter,
                "{}: expected exactly one well-formed synced doctrine block",
                path.display()
            ),
            Self::Conflict {
                path,
                marker_stamp,
                actual_stamp,
            } => write!(
                formatter,
                "{}: doctrine block was edited locally: marker source-sha256={marker_stamp}, content source-sha256={actual_stamp}",
                path.display()
            ),
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
        }
    }
}

fn read_instruction(path: &Path) -> Result<String, SyncError> {
    fs::read_to_string(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            SyncError::Missing {
                path: path.to_path_buf(),
            }
        } else {
            SyncError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
    })
}

fn is_line_start(text: &str, index: usize) -> bool {
    index == 0 || text.as_bytes().get(index.wrapping_sub(1)) == Some(&b'\n')
}

fn parse_block<'a>(text: &'a str, path: &Path) -> Result<ParsedBlock<'a>, SyncError> {
    let malformed = || SyncError::Malformed {
        path: path.to_path_buf(),
    };

    let mut begins = text.match_indices(BEGIN_TOKEN);
    let (start, _) = begins.next().ok_or_else(&malformed)?;
    if begins.next().is_some() || !is_line_start(text, start) {
        return Err(malformed());
    }

    let mut ends = text.match_indices(END_MARKER);
    let (end_start, _) = ends.next().ok_or_else(&malformed)?;
    if ends.next().is_some() || !is_line_start(text, end_start) {
        return Err(malformed());
    }

    let marker_newline = text[start..]
        .find('\n')
        .map(|offset| start + offset)
        .ok_or_else(&malformed)?;
    let marker = &text[start..marker_newline];
    let stamp = marker
        .strip_prefix(BEGIN_PREFIX)
        .and_then(|remainder| remainder.strip_suffix(" -->"))
        .ok_or_else(&malformed)?;
    if stamp.len() != 64
        || !stamp
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        || end_start <= marker_newline
        || text
            .as_bytes()
            .get(end_start + END_MARKER.len())
            .is_some_and(|byte| *byte != b'\n')
    {
        return Err(malformed());
    }

    let end = end_start + END_MARKER.len();
    Ok(ParsedBlock {
        start,
        end,
        body: &text[marker_newline + 1..end_start],
        marker_stamp: stamp,
        rendered: &text[start..end],
    })
}

fn verified_block<'a>(text: &'a str, path: &Path) -> Result<(ParsedBlock<'a>, String), SyncError> {
    let block = parse_block(text, path)?;
    let actual_stamp = sha256_hex(block.body.as_bytes());
    if actual_stamp != block.marker_stamp {
        return Err(SyncError::Conflict {
            path: path.to_path_buf(),
            marker_stamp: block.marker_stamp.to_owned(),
            actual_stamp,
        });
    }
    Ok((block, actual_stamp))
}

fn synchronize(source_path: &Path, target_path: &Path) -> Result<(SyncOutcome, String), SyncError> {
    let source_text = read_instruction(source_path)?;
    let (source, source_stamp) = verified_block(&source_text, source_path)?;

    let target_text = read_instruction(target_path)?;
    let (target, _) = verified_block(&target_text, target_path)?;

    if source.rendered == target.rendered {
        return Ok((SyncOutcome::Unchanged, source_stamp));
    }

    let mut updated = String::with_capacity(
        target_text.len() - (target.end - target.start) + source.rendered.len(),
    );
    updated.push_str(&target_text[..target.start]);
    updated.push_str(source.rendered);
    updated.push_str(&target_text[target.end..]);
    fs::write(target_path, updated).map_err(|source| SyncError::Io {
        path: target_path.to_path_buf(),
        source,
    })?;

    Ok((SyncOutcome::Updated, source_stamp))
}

fn check_doctrine(source_path: &Path, target_path: &Path) -> Result<DoctrineStatus, SyncError> {
    let source_text = read_instruction(source_path)?;
    let (_, source_stamp) = verified_block(&source_text, source_path)?;

    let target_text = read_instruction(target_path)?;
    if !target_text.contains(BEGIN_TOKEN) && !target_text.contains(END_MARKER) {
        return Ok(DoctrineStatus::Unstamped {
            available_source_stamp: source_stamp,
        });
    }

    let target = parse_block(&target_text, target_path)?;
    let actual_stamp = sha256_hex(target.body.as_bytes());
    if actual_stamp != target.marker_stamp {
        return Ok(DoctrineStatus::LocallyEdited {
            marker_source_stamp: target.marker_stamp.to_owned(),
            content_source_stamp: actual_stamp,
            available_source_stamp: source_stamp,
        });
    }
    if target.marker_stamp == source_stamp {
        return Ok(DoctrineStatus::Current { source_stamp });
    }
    Ok(DoctrineStatus::Stale {
        installed_source_stamp: target.marker_stamp.to_owned(),
        available_source_stamp: source_stamp,
    })
}

fn print_check_result(status: DoctrineStatus, target_path: &Path) -> Result<(), ExitCode> {
    match status {
        DoctrineStatus::Current { source_stamp } => {
            println!(
                "CURRENT: {}: installed source-sha256={source_stamp}, available source-sha256={source_stamp}",
                target_path.display()
            );
            Ok(())
        }
        DoctrineStatus::Stale {
            installed_source_stamp,
            available_source_stamp,
        } => {
            println!(
                "STALE: {}: installed source-sha256={installed_source_stamp}, available source-sha256={available_source_stamp}",
                target_path.display()
            );
            Err(ExitCode::from(1))
        }
        DoctrineStatus::LocallyEdited {
            marker_source_stamp,
            content_source_stamp,
            available_source_stamp,
        } => {
            println!(
                "LOCALLY_EDITED: {}: marker source-sha256={marker_source_stamp}, content source-sha256={content_source_stamp}, available source-sha256={available_source_stamp}",
                target_path.display()
            );
            Err(ExitCode::from(3))
        }
        DoctrineStatus::Unstamped {
            available_source_stamp,
        } => {
            println!(
                "UNSTAMPED: {}: target carries no source stamp; available source-sha256={available_source_stamp}",
                target_path.display()
            );
            Err(ExitCode::from(1))
        }
    }
}

fn print_error(error: SyncError) -> Result<(), ExitCode> {
    match error {
        error @ SyncError::Conflict { .. } => {
            eprintln!("CONFLICT: {error}; no changes written");
            Err(ExitCode::from(3))
        }
        error
        @ (SyncError::Missing { .. } | SyncError::Malformed { .. } | SyncError::Io { .. }) => {
            eprintln!("ERROR: {error}; no changes written");
            Err(ExitCode::from(2))
        }
    }
}

fn sha256_hex(input: &[u8]) -> String {
    const INITIAL_STATE: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const ROUND_CONSTANTS: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut padded = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    let mut state = INITIAL_STATE;
    for chunk in padded.chunks_exact(64) {
        let mut schedule = [0_u32; 64];
        for (index, word) in schedule[..16].iter_mut().enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..64 {
            let sigma0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let sigma1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(sigma0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(sigma1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let temporary1 = h
                .wrapping_add(sum1)
                .wrapping_add(choice)
                .wrapping_add(ROUND_CONSTANTS[index])
                .wrapping_add(schedule[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temporary2 = sum0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temporary1);
            d = c;
            c = b;
            b = a;
            a = temporary1.wrapping_add(temporary2);
        }

        for (value, addition) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *value = value.wrapping_add(addition);
        }
    }

    let mut hexadecimal = String::with_capacity(64);
    for word in state {
        use fmt::Write as _;
        write!(&mut hexadecimal, "{word:08x}").expect("writing to a String cannot fail");
    }
    hexadecimal
}

fn run() -> Result<(), ExitCode> {
    let mut arguments = env::args_os().skip(1).peekable();
    let check_only = arguments
        .peek()
        .is_some_and(|argument| argument == "--check");
    if check_only {
        arguments.next();
    }
    let target_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("AGENTS.md"));
    if arguments.next().is_some() {
        eprintln!("{USAGE}");
        return Err(ExitCode::from(64));
    }

    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md");
    if check_only {
        return match check_doctrine(&source_path, &target_path) {
            Ok(status) => print_check_result(status, &target_path),
            Err(error) => print_error(error),
        };
    }
    match synchronize(&source_path, &target_path) {
        Ok((outcome, source_stamp)) => {
            println!(
                "{outcome}: {}: source-sha256={source_stamp}",
                target_path.display()
            );
            Ok(())
        }
        Err(error) => print_error(error),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

#[cfg(test)]
mod tests {
    use super::{SyncError, sha256_hex, synchronize};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SOURCE_STAMP: &str = "59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d";
    const SOURCE_BODY: &str = "Four rules. They are one design stance seen four ways: a module means one thing, receives exactly what it needs, in types that cannot lie, and dies rather than guess.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n";
    const STALE_STAMP: &str = "276add6ad3059d4de66db2250890ce61c7cda02a9c4da8622abf51afd2d7054a";
    const EDITED_STAMP: &str = "e7107a175f6aa7daf3d1ab9953bdfc5fa01b7bde885324f060bd73951e75abbb";

    struct TemporaryTree {
        path: PathBuf,
    }

    impl TemporaryTree {
        fn source_conflict() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must follow the Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "rustplate-source-conflict-{}-{nanos}",
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

    #[test]
    fn hashes_literal_doctrine_body() {
        assert_eq!(sha256_hex(SOURCE_BODY.as_bytes()), SOURCE_STAMP);
        assert_eq!(
            sha256_hex(
                b"Four rules. This fixture is a clean older source.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n"
            ),
            STALE_STAMP
        );
        assert_eq!(
            sha256_hex(
                b"Four rules. This fixture is a locally edited older source.\n\n1. **A module means one thing.**\n2. **It receives exactly what it needs.**\n3. **Its types cannot lie.**\n4. **It dies rather than guess.**\n"
            ),
            EDITED_STAMP
        );
    }

    #[test]
    fn rejects_an_edited_source_before_touching_target() {
        let tree = TemporaryTree::source_conflict();
        let source_path = tree.path.join("source-AGENTS.md");
        let target_path = tree.path.join("target-AGENTS.md");
        fs::write(
            &source_path,
            "<!-- BEGIN SYNCED DOCTRINE; source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d -->\nlocally edited source\n<!-- END SYNCED DOCTRINE -->\n",
        )
        .expect("edited source fixture must be written");
        fs::write(&target_path, "target sentinel\n").expect("target sentinel must be written");

        let error = synchronize(&source_path, &target_path)
            .expect_err("an edited source must cause a conflict");
        assert!(matches!(error, SyncError::Conflict { .. }));
        assert_eq!(
            error.to_string(),
            format!(
                "{}: doctrine block was edited locally: marker source-sha256=59e37fd6b3dbab27530822e6956da51bb7ae76b637e3638530f99a8b4db9038d, content source-sha256=75b7eb278c10633eb122be2ae5a77fdf033152210016d5bfc335c16e0ff9a0f1",
                source_path.display()
            )
        );
        assert_eq!(
            fs::read_to_string(&target_path).expect("target sentinel must remain readable"),
            "target sentinel\n"
        );
    }
}
