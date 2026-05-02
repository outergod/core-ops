//! `core-ops skill install` — embeds an agent skill bundle at compile
//! time and writes it (or prints it) per the contract at
//! `specs/016-source-repository-layout/contracts/skill-cli.md`.
//!
//! The bundle is currently a single file (`SKILL.md`); the entries
//! constant is shaped as `&[(&str, &[u8])]` so adding asset files
//! later is a one-line change.
//!
//! The path standard is **agentskills.io**: bundles install to
//! `.agents/skills/<skill-name>/` (default) or
//! `~/.agents/skills/<skill-name>/` (`--global`). Vendor-specific
//! paths like `.claude/skills/` are never written by this command.

use std::io::Write;
use std::path::{Path, PathBuf};

use miette::{miette, IntoDiagnostic, Result, WrapErr};

use crate::cli::args::SkillInstallArgs;

/// The name segment under `.agents/skills/` for this bundle. Bound by
/// the agentskills.io standard and enforced by the contract test
/// `test_skill_install_vendor_neutral`.
const SKILL_NAME: &str = "core-ops-source-repo";

/// Bundle entries as `(relative-path, bytes)` pairs, sorted lex by
/// relative-path so the print stream is deterministic. Entries are
/// embedded at compile time via `include_bytes!` so the binary needs
/// no runtime access to the source repository.
const SKILL_BUNDLE: &[(&str, &[u8])] = &[(
    "SKILL.md",
    include_bytes!("../../specs/016-source-repository-layout/skill/SKILL.md"),
)];

/// Run `core-ops skill install` in the mode selected by `args`.
pub fn run(args: &SkillInstallArgs) -> Result<()> {
    if args.print {
        return print_bundle(&mut std::io::stdout().lock());
    }
    let dest = if args.global {
        global_destination()?
    } else {
        local_destination()?
    };
    install_to(&dest)
}

fn local_destination() -> Result<PathBuf> {
    let cwd = std::env::current_dir()
        .into_diagnostic()
        .wrap_err("resolving current working directory for skill install")?;
    Ok(cwd.join(".agents/skills").join(SKILL_NAME))
}

fn global_destination() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| miette!("HOME is not set; --global requires a resolvable home directory"))?;
    Ok(PathBuf::from(home)
        .join(".agents/skills")
        .join(SKILL_NAME))
}

fn install_to(dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)
        .into_diagnostic()
        .wrap_err_with(|| format!("creating skill destination {}", dest.display()))?;

    for (relative, bytes) in SKILL_BUNDLE {
        let target = dest.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .into_diagnostic()
                .wrap_err_with(|| format!("creating {}", parent.display()))?;
        }
        // Idempotent: if the file already exists with byte-identical
        // content, leave it alone (preserves mtime and avoids a
        // rewrite). Differing content is an explicit error per the
        // contract — a stale or hand-edited install is better surfaced
        // than silently clobbered.
        if target.exists() {
            let existing = std::fs::read(&target)
                .into_diagnostic()
                .wrap_err_with(|| format!("reading existing {}", target.display()))?;
            if existing.as_slice() == *bytes {
                continue;
            }
            return Err(miette!(
                "skill bundle entry at {} differs from the embedded bundle; \
                 remove or back up the existing file and re-run",
                target.display()
            ));
        }
        std::fs::write(&target, bytes)
            .into_diagnostic()
            .wrap_err_with(|| format!("writing {}", target.display()))?;
    }
    Ok(())
}

fn print_bundle(writer: &mut dyn Write) -> Result<()> {
    for (relative, bytes) in SKILL_BUNDLE {
        writeln!(writer, "==> {relative} <==")
            .into_diagnostic()
            .wrap_err("writing skill bundle header to stdout")?;
        writer
            .write_all(bytes)
            .into_diagnostic()
            .wrap_err("writing skill bundle entry to stdout")?;
    }
    writer
        .flush()
        .into_diagnostic()
        .wrap_err("flushing skill bundle stdout")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_bundle_starts_with_header_and_includes_skill_md() {
        let mut buf: Vec<u8> = Vec::new();
        print_bundle(&mut buf).expect("print bundle");
        let text = String::from_utf8(buf).expect("utf-8 stream");
        assert!(text.starts_with("==> SKILL.md <==\n"));
        assert!(text.contains("name: core-ops-source-repo"));
    }

    #[test]
    fn install_to_writes_bundle_and_is_idempotent() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        install_to(tmp.path()).expect("first install");
        let path = tmp.path().join("SKILL.md");
        assert!(path.exists());
        let first = std::fs::read(&path).expect("read");
        // Reinstall on byte-identical content must not error.
        install_to(tmp.path()).expect("idempotent install");
        let second = std::fs::read(&path).expect("read");
        assert_eq!(first, second);
    }

    #[test]
    fn install_to_refuses_to_clobber_diverged_content() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        install_to(tmp.path()).expect("first install");
        std::fs::write(tmp.path().join("SKILL.md"), b"--- divergent ---\n").expect("clobber");
        let err = install_to(tmp.path()).expect_err("must refuse divergent content");
        let msg = err.to_string();
        assert!(msg.contains("differs from the embedded bundle"), "{msg}");
    }
}
