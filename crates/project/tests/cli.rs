//! The command line `eh3d-project info|verify|extract-branch` on the synthetic project.

mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::*;
use tempfile::tempdir;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_eh3d-project"))
        .args(args)
        .output()
        .unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn project(dir: &Path) -> String {
    synthetic(dir).path().to_string_lossy().into_owned()
}

#[test]
fn info_lists_the_branches_in_text_and_in_json() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let text = run(&["info", &file]);
    assert_eq!(text.status.code(), Some(0));
    let text = out(&text);
    assert!(text.contains("Synthetic test project"));
    assert!(text.contains("branches  8"));
    assert!(text.contains("manufacturing"));

    let json = run(&["info", &file, "--json"]);
    assert_eq!(json.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_str(&out(&json)).unwrap();
    assert_eq!(value["branches"].as_array().unwrap().len(), 8);
    assert_eq!(value["format_version"], "1.0.0");
}

#[test]
fn verify_passes_a_sound_project_and_fails_a_damaged_one() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let ok = run(&["verify", &file]);
    assert_eq!(ok.status.code(), Some(0));
    assert!(out(&ok).contains("VERIFY=PASS"));

    let damaged = dir.path().join("damaged.eh3d");
    std::fs::copy(&file, &damaged).unwrap();
    flip_in_entry(&damaged, "pcb/outline.json");
    let bad = run(&["verify", damaged.to_str().unwrap()]);
    assert_ne!(bad.status.code(), Some(0));
}

#[test]
fn verify_asks_for_signatures_when_told_to() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let unsigned = run(&["verify", &file, "--require-signed", "pcb"]);
    assert_eq!(unsigned.status.code(), Some(2));
    let text = out(&unsigned);
    assert!(text.contains("EH3D-PKG-022"), "{text}");
    assert!(text.contains("VERIFY=FAIL"));
}

#[test]
fn extract_branch_writes_the_files_and_never_replaces_one() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let target = dir.path().join("cnc-out");
    let first = run(&["extract-branch", &file, "cnc", target.to_str().unwrap()]);
    assert_eq!(first.status.code(), Some(0));
    assert!(target.join("branch.json").is_file());
    assert!(target.join("part.json").is_file());
    let second = run(&["extract-branch", &file, "cnc", target.to_str().unwrap()]);
    assert_eq!(second.status.code(), Some(1));

    let package = dir.path().join("typical.eh3dproj");
    let made = run(&[
        "extract-branch",
        &file,
        "manufacturing",
        "--eh3dproj",
        package.to_str().unwrap(),
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", out(&made));
    assert_eq!(
        std::fs::read(&package).unwrap(),
        std::fs::read(Path::new(GOLDEN).join("typical.eh3dproj")).unwrap()
    );
}

#[test]
fn wrong_use_and_missing_files_have_their_own_exit_codes() {
    assert_eq!(run(&[]).status.code(), Some(64));
    assert_eq!(run(&["info"]).status.code(), Some(64));
    assert_eq!(run(&["frobnicate", "x"]).status.code(), Some(64));
    assert_eq!(run(&["--help"]).status.code(), Some(0));
    assert_eq!(run(&["info", "no-such-file.eh3d"]).status.code(), Some(1));
}

#[test]
fn keygen_sign_and_verify_from_the_command_line() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let key = dir.path().join("studio.key");
    let key = key.to_str().unwrap();
    let public = format!("{key}.pub");

    let made = run(&["keygen", key]);
    assert_eq!(made.status.code(), Some(0), "{made:?}");
    let secret = std::fs::read_to_string(key).unwrap();
    let digits = secret.lines().nth(1).unwrap();
    assert!(!out(&made).contains(digits));
    assert!(out(&made).contains("ed25519:"));
    assert_eq!(run(&["keygen", key]).status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(key).unwrap(), secret);

    let before = std::fs::read(&file).unwrap();
    for refused in [
        run(&["sign", &file, "--key", key, "--branch", "pcb,none"]),
        run(&["sign", &file, "--key", &public, "--branch", "pcb"]),
        run(&["sign", &file, "--key", "no-such.key", "--project"]),
    ] {
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    }
    assert_eq!(run(&["sign", &file, "--key", key]).status.code(), Some(64));
    assert_eq!(run(&["sign", &file, "--project"]).status.code(), Some(64));
    assert_eq!(std::fs::read(&file).unwrap(), before);

    let done = run(&["sign", &file, "--key", key, "--branch", "pcb", "--project"]);
    assert_eq!(done.status.code(), Some(0), "{done:?}");
    assert!(out(&done).contains("signed branch pcb"));
    assert!(!out(&done).contains(digits));

    let checked = run(&[
        "verify",
        &file,
        "--pubkey",
        &public,
        "--require-signed",
        "pcb",
        "--require-project-signature",
    ]);
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    let text = out(&checked);
    assert!(text.contains("signature pcb: signed, valid"), "{text}");
    assert!(text.contains("signature project: signed, valid"), "{text}");
    assert!(text.contains("signature cad: unsigned"), "{text}");

    let unchecked = run(&["verify", &file]);
    assert_eq!(unchecked.status.code(), Some(0));
    assert!(out(&unchecked).contains("signature pcb: signed, not checked"));
    assert_eq!(
        run(&["verify", &file, "--require-signed", "pcb"])
            .status
            .code(),
        Some(2)
    );
    // a secret key is not what checks a signature
    let wrong = run(&["verify", &file, "--pubkey", key]);
    assert_eq!(wrong.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&wrong.stderr).contains(digits));
}

#[test]
fn attachments_are_added_listed_and_taken_out_from_the_command_line() {
    let dir = tempdir().unwrap();
    let file = project(dir.path());
    let sheet = dir.path().join("NE555.pdf");
    std::fs::write(&sheet, b"%PDF-1.4 made up").unwrap();
    let script = dir.path().join("tool.py");
    std::fs::write(&script, b"print('made up')\n").unwrap();
    let at = |p: &Path| p.to_str().unwrap().to_owned();

    let done = run(&[
        "attach",
        &file,
        &at(&sheet),
        "--ref",
        "U3",
        "--maker-part",
        "Example",
        "NE555P",
        "--title",
        "Data sheet",
    ]);
    assert_eq!(done.status.code(), Some(0), "{done:?}");
    assert_eq!(run(&["attach", &file, &at(&script)]).status.code(), Some(0));
    let again = run(&["attach", &file, &at(&sheet), "--ref", "U3"]);
    assert!(out(&again).contains("nothing new"), "{again:?}");

    let listed = out(&run(&["attachments", &file]));
    assert!(listed.contains("attachments  2"), "{listed}");
    assert!(listed.contains("DANGEROUS (script)"), "{listed}");
    let of_u3 = run(&["attachments", &file, "--ref", "u3", "--json"]);
    let list: serde_json::Value = serde_json::from_str(&out(&of_u3)).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["name"], "NE555.pdf");
    assert_eq!(list[0]["kind"], "pdf");
    let sheet_id = list[0]["sha256"].as_str().unwrap().to_owned();
    assert_eq!(run(&["verify", &file]).status.code(), Some(0));

    let target = dir.path().join("out");
    let taken = run(&["extract-attachment", &file, &sheet_id[..12], &at(&target)]);
    assert_eq!(taken.status.code(), Some(0), "{taken:?}");
    assert_eq!(
        std::fs::read(target.join("NE555.pdf")).unwrap(),
        b"%PDF-1.4 made up"
    );
    // never over a file that is there
    let twice = run(&["extract-attachment", &file, &sheet_id, &at(&target)]);
    assert_eq!(twice.status.code(), Some(1));

    let all = run(&["attachments", &file, "--json"]);
    let all: serde_json::Value = serde_json::from_str(&out(&all)).unwrap();
    let script_id = all
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "tool.py")
        .and_then(|a| a["sha256"].as_str())
        .unwrap()
        .to_owned();
    let refused = run(&["extract-attachment", &file, &script_id, &at(&target)]);
    assert_eq!(refused.status.code(), Some(3), "{refused:?}");
    assert!(!target.join("tool.py").exists());
    let agreed = run(&[
        "extract-attachment",
        &file,
        &script_id,
        &at(&target),
        "--confirm-dangerous",
    ]);
    assert_eq!(agreed.status.code(), Some(0), "{agreed:?}");
    assert!(target.join("tool.py").exists());

    let linked = run(&[
        "attach-link",
        &file,
        "https://example.invalid/manual.pdf",
        "--sha256",
        &"a".repeat(64),
        "--value",
        "10k",
    ]);
    assert_eq!(linked.status.code(), Some(0), "{linked:?}");
    let link = run(&["extract-attachment", &file, &"a".repeat(64), &at(&target)]);
    assert_eq!(link.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&link.stderr).contains("nothing is downloaded"));

    let moved = run(&[
        "attachment-links",
        &file,
        &sheet_id,
        "remove",
        "--ref",
        "U3",
    ]);
    assert_eq!(moved.status.code(), Some(0), "{moved:?}");
    assert!(out(&run(&["attachments", &file, "--ref", "U3"])).contains("attachments  0"));
    assert_eq!(run(&["detach", &file, &sheet_id]).status.code(), Some(0));
    assert!(out(&run(&["attachments", &file])).contains("attachments  2"));
    assert_eq!(run(&["verify", &file]).status.code(), Some(0));

    for wrong in [
        vec!["attach", &file],
        vec!["attach", &file, "x", "--max-mb", "0"],
        vec!["attach-link", &file, "https://example.invalid/a"],
        vec!["attachments", &file, "--confirm-dangerous"],
        vec!["attachment-links", &file, &script_id, "add"],
    ] {
        let o = run(&wrong);
        assert!(matches!(o.status.code(), Some(1 | 64)), "{wrong:?} {o:?}");
    }
}
