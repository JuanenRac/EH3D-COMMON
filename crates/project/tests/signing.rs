//! Ed25519 signatures of branches and projects: the vectors of RFC 8032, what a changed byte does
//! to a signature, damaged signature records and key files, and the rules of the key files.

mod support;

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use eh3d_container::{ContainerError, PkgCode};
use eh3d_project::ed25519::{
    generate_key_files, key_id_of, public_key_path, PUBLIC_KEY_HEADER, SECRET_KEY_HEADER,
};
use eh3d_project::records::{from_hex, to_hex};
use eh3d_project::{
    BranchUpdate, Checks, Ed25519Keyring, Ed25519Signer, KeyError, OpenOptions, Project,
    SaveOptions, Seal, SignatureStatus, Signer, Verifier, VerifyOutcome, Writer,
};
use serde_json::{json, Value};
use support::*;
use tempfile::tempdir;

/// RFC 8032, section 7.1: secret key, public key, message, signature.
const VECTORS: [(&str, &str, &str, &str); 4] = [
    (
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        "",
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    ),
    (
        "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        "72",
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
    ),
    (
        "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
        "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        "af82",
        "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
    ),
    (
        "833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42",
        "ec172b93ad5e563bf4932c70e1245034c35467ef2efd4d64ebf819683467e2bf",
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        "dc2a4459e7369633a52b1bf277839a00201009a3efbf3ecb69bea2186c26b58909351fc9ac90b3ecfdfbc7c66431e0303dca179c138ac17ad9bef1177331a704",
    ),
];

fn bytes32(hex: &str) -> [u8; 32] {
    from_hex(hex).unwrap().try_into().unwrap()
}

fn signer() -> Ed25519Signer {
    Ed25519Signer::from_seed(&bytes32(VECTORS[0].0))
}

fn keyring() -> Ed25519Keyring {
    let mut ring = Ed25519Keyring::new();
    ring.add(&signer().public_key()).unwrap();
    ring
}

fn valid() -> SignatureStatus {
    SignatureStatus::Valid(signer().key_id().to_owned())
}

fn code_of<T>(result: Result<T, ContainerError>) -> PkgCode {
    match result {
        Err(ContainerError::Rejected(v)) => v.code,
        Err(other) => panic!("not a refusal: {other}"),
        Ok(_) => panic!("it was not refused"),
    }
}

/// The synthetic project with `pcb`, `laser` and the project itself signed.
fn signed(dir: &Path) -> PathBuf {
    let key = signer();
    let options = SaveOptions::default();
    let p = synthetic(dir);
    let p = p.sign_branch("pcb", &key, &options).unwrap();
    let p = p.sign_project(&key, &options).unwrap();
    // signing a branch afterwards does not touch the project record
    let p = p.sign_branch("laser", &key, &options).unwrap();
    p.path().to_owned()
}

fn report_of(path: &Path) -> eh3d_project::SignatureReport {
    let ring = keyring();
    Project::open_verified(path, &OpenOptions::default(), Some(&ring))
        .unwrap()
        .1
}

/// The uncompressed entries of a file, the integrity record left out.
fn entries_of(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut out = BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().expect("entry name").into_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if name != "integrity.json" {
            out.insert(name, bytes);
        }
    }
    out
}

/// Writes the entries back with a fresh, correct integrity record: what someone who edits a file
/// with care would leave.
fn rebuild(path: &Path, entries: &BTreeMap<String, Vec<u8>>) {
    let project: Value = serde_json::from_slice(&entries["project.json"]).unwrap();
    let rest: Vec<(&str, Vec<u8>)> = entries
        .iter()
        .filter(|(n, _)| *n != "project.json")
        .map(|(n, b)| (n.as_str(), b.clone()))
        .collect();
    raw_project(path, &project, &rest);
}

fn edit_json(entries: &mut BTreeMap<String, Vec<u8>>, name: &str, edit: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&entries[name]).unwrap();
    edit(&mut value);
    entries.insert(name.to_owned(), canon(&value));
}

#[test]
fn the_vectors_of_rfc_8032_are_reproduced_and_checked() {
    for (secret, public, message, signature) in VECTORS {
        let key = Ed25519Signer::from_seed(&bytes32(secret));
        assert_eq!(to_hex(&key.public_key()), public);
        let message = from_hex(message).unwrap();
        let made = key.sign(&message);
        assert_eq!(to_hex(&made), signature);

        let mut ring = Ed25519Keyring::new();
        let id = ring.add(&bytes32(public)).unwrap();
        assert_eq!(id, key.key_id());
        assert_eq!(id, key_id_of(&bytes32(public)));
        assert_eq!(
            ring.verify("ed25519", &id, &message, &made),
            VerifyOutcome::Valid
        );
        for at in 0..made.len() {
            let mut wrong = made.clone();
            wrong[at] ^= 0x04;
            assert_eq!(
                ring.verify("ed25519", &id, &message, &wrong),
                VerifyOutcome::Invalid,
                "byte {at}"
            );
        }
        let mut longer = message.clone();
        longer.push(0);
        assert_eq!(
            ring.verify("ed25519", &id, &longer, &made),
            VerifyOutcome::Invalid
        );
        assert_eq!(
            ring.verify("ed25519", &id, &message, &made[..63]),
            VerifyOutcome::Invalid
        );
        assert_eq!(
            ring.verify("rsa", &id, &message, &made),
            VerifyOutcome::UnsupportedAlgorithm
        );
        assert_eq!(
            ring.verify("ed25519", "ed25519:other", &message, &made),
            VerifyOutcome::UnknownKey
        );
    }
}

#[test]
fn a_weak_public_key_is_not_trusted() {
    let mut identity = [0u8; 32];
    identity[0] = 1;
    let mut ring = Ed25519Keyring::new();
    assert!(matches!(ring.add(&identity), Err(KeyError::BadPublicKey)));
    assert!(ring.is_empty());
    assert_eq!(ring.len(), 0);
}

#[test]
fn branches_and_the_project_are_signed_and_checked_on_opening() {
    let dir = tempdir().unwrap();
    let path = signed(dir.path());
    let ring = keyring();
    let (mut p, report) =
        Project::open_verified(&path, &OpenOptions::default(), Some(&ring)).unwrap();
    assert_eq!(report.project, valid());
    assert_eq!(report.branches["pcb"], valid());
    assert_eq!(report.branches["laser"], valid());
    assert_eq!(report.branches["cad"], SignatureStatus::Unsigned);
    assert!(!report.any_invalid());
    assert_eq!(p.branch_signature("pcb", Some(&ring)), Some(valid()));
    assert_eq!(p.branch_signature("none", Some(&ring)), None);

    let mut checks = Checks {
        verifier: Some(&ring),
        require_project_signature: true,
        ..Checks::default()
    };
    checks.require_signed.extend(["pcb".into(), "laser".into()]);
    assert_eq!(p.diagnose(&checks), vec![]);

    // without a key a signature is seen but not judged
    let unchecked = p.signatures(None);
    assert_eq!(unchecked.project, SignatureStatus::Unchecked);
    assert_eq!(unchecked.branches["pcb"], SignatureStatus::Unchecked);
    // another key does not know this one
    let mut stranger = Ed25519Keyring::new();
    stranger
        .add(&Ed25519Signer::from_seed(&bytes32(VECTORS[1].0)).public_key())
        .unwrap();
    assert_eq!(
        p.signatures(Some(&stranger)).branches["pcb"],
        SignatureStatus::UnknownKey
    );
    // the same file opened without the integrity check says nothing about the signatures
    let loose = Project::open(
        &path,
        &OpenOptions {
            verify_integrity: false,
            ..OpenOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        loose.signatures(Some(&ring)).project,
        SignatureStatus::Unchecked
    );
    assert_eq!(
        code_of(loose.sign_branch("cad", &signer(), &SaveOptions::default())),
        PkgCode::Integrity
    );
}

#[test]
fn signing_a_branch_changes_only_its_record_and_logs_when_told() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let before = raw_entries(p.path());
    let options = SaveOptions {
        audit_at: Some("2000-02-01T00:00:00Z".into()),
        ..SaveOptions::default()
    };
    let p = p.sign_branch("cad", &signer(), &options).unwrap();
    let after = raw_entries(p.path());
    for (name, kept) in &before {
        if name != "cad/branch.json" && name != "integrity.json" {
            assert_eq!(after.get(name), Some(kept), "{name}");
        }
    }
    let log = zip_text(p.path(), "audit/log.jsonl");
    assert!(log.contains("\"action\":\"sign\""), "{log}");
    assert!(log.contains(signer().key_id()));
    // the key is not anywhere in the file
    for bytes in entries_of(p.path()).values() {
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains(VECTORS[0].0));
        assert!(!text.contains(VECTORS[0].1));
    }
}

#[test]
fn what_cannot_be_signed_is_refused() {
    let dir = tempdir().unwrap();
    let key = signer();
    let options = SaveOptions::default();
    let p = synthetic(dir.path());
    assert_eq!(code_of(p.check_signable("nothing")), PkgCode::BranchRef);
    let path = p.path().to_owned();
    // a branch whose files are not the ones its record hashes
    drop(p);
    let mut entries = entries_of(&path);
    entries.insert("cad/case.json".into(), b"{}\n".to_vec());
    let broken = dir.path().join("broken.eh3d");
    rebuild(&broken, &entries);
    let p = Project::open(&broken, &OpenOptions::default()).unwrap();
    assert_eq!(code_of(p.check_signable("cad")), PkgCode::BranchHash);
    assert_eq!(code_of(p.sign_project(&key, &options)), PkgCode::BranchHash);
    // sealed
    let p = Project::open(&path, &OpenOptions::default()).unwrap();
    let seal = Seal {
        by: "tests".into(),
        at: "2000-01-01T00:00:00Z".into(),
        reason: None,
    };
    let p = p.seal_project(seal, &options).unwrap();
    assert_eq!(
        code_of(p.sign_branch("cad", &key, &options)),
        PkgCode::Sealed
    );
}

#[test]
fn a_record_with_a_number_without_a_canonical_form_is_never_signed() {
    let dir = tempdir().unwrap();
    let p = new_project(dir.path());
    let mut update = BranchUpdate::new("pcb", "1.0", "0.1.0").with_entry("a.json", b"{}\n");
    update.extra.insert("huge".into(), json!(u64::MAX));
    let key = signer();
    let options = SaveOptions {
        signer: Some(&key),
        ..SaveOptions::default()
    };
    let refused = p.save_branch(&Writer::for_app("pcb"), update, &options);
    assert_eq!(code_of(refused), PkgCode::BranchRecord);
}

#[test]
fn a_secret_key_is_never_put_into_a_project() {
    let dir = tempdir().unwrap();
    let key = dir.path().join("mine.key");
    generate_key_files(&key).unwrap();
    let p = new_project(dir.path());
    let update =
        BranchUpdate::new("pcb", "1.0", "0.1.0").with_entry("notes.txt", &fs::read(&key).unwrap());
    let refused = p.save_branch(&Writer::for_app("pcb"), update, &SaveOptions::default());
    assert_eq!(code_of(refused), PkgCode::BranchRecord);
    let kept = Project::open(&dir.path().join("synthetic.eh3d"), &OpenOptions::default()).unwrap();
    assert_eq!(kept.branches().count(), 0);
}

#[test]
fn a_changed_byte_is_noticed_even_with_a_rewritten_integrity_record() {
    let dir = tempdir().unwrap();
    let path = signed(dir.path());
    let original = entries_of(&path);
    let out = dir.path().join("changed.eh3d");

    // an unchanged rebuild keeps every signature
    rebuild(&out, &original);
    let report = report_of(&out);
    assert_eq!(report.project, valid());
    assert_eq!(report.branches["pcb"], valid());

    // a file of a signed branch
    let mut entries = original.clone();
    let mut bytes = entries["pcb/outline.json"].clone();
    bytes[3] ^= 0x01;
    entries.insert("pcb/outline.json".into(), bytes);
    rebuild(&out, &entries);
    let report = report_of(&out);
    assert_eq!(report.branches["pcb"], SignatureStatus::Invalid);
    assert_eq!(report.project, SignatureStatus::Invalid);
    assert_eq!(report.branches["laser"], valid());
    assert!(report.any_invalid());
    let ring = keyring();
    let checks = Checks {
        verifier: Some(&ring),
        ..Checks::default()
    };
    let (_, findings) = Project::verify(&out, &OpenOptions::default(), &checks).unwrap();
    let codes: Vec<PkgCode> = findings.iter().map(|f| f.code).collect();
    assert!(codes.contains(&PkgCode::SignatureInvalid), "{codes:?}");

    // the same change with the hashes of both records rewritten to match
    let hash = Project::open(&out, &OpenOptions::default())
        .unwrap()
        .branch("pcb")
        .unwrap()
        .hash
        .clone();
    edit_json(&mut entries, "pcb/branch.json", |v| {
        v["content_hash"] = json!(hash)
    });
    edit_json(&mut entries, "project.json", |v| {
        for line in v["branches"].as_array_mut().unwrap() {
            if line["id"] == "pcb" {
                line["content_hash"] = json!(hash);
            }
        }
    });
    rebuild(&out, &entries);
    let report = report_of(&out);
    assert_eq!(report.branches["pcb"], SignatureStatus::Invalid);
    assert_eq!(report.project, SignatureStatus::Invalid);

    // the record of the project
    let mut entries = original.clone();
    edit_json(&mut entries, "project.json", |v| v["name"] = json!("Other"));
    rebuild(&out, &entries);
    let report = report_of(&out);
    assert_eq!(report.project, SignatureStatus::Invalid);
    assert_eq!(report.branches["pcb"], valid());

    // the record of a branch
    let mut entries = original.clone();
    edit_json(&mut entries, "pcb/branch.json", |v| {
        v["app_version"] = json!("9.9.9")
    });
    rebuild(&out, &entries);
    assert_eq!(report_of(&out).branches["pcb"], SignatureStatus::Invalid);

    // a signed branch copied into another folder
    let mut entries = original.clone();
    for (name, bytes) in &original {
        if let Some(rel) = name.strip_prefix("pcb/") {
            entries.insert(format!("pcb2/{rel}"), bytes.clone());
        }
    }
    rebuild(&out, &entries);
    let report = report_of(&out);
    assert_eq!(report.branches["pcb2"], SignatureStatus::Invalid);
    // and the project does not index it
    assert_eq!(report.project, SignatureStatus::Invalid);

    // a signature that was taken away shows as unsigned and fails when it is required
    let mut entries = original.clone();
    edit_json(&mut entries, "pcb/branch.json", |v| {
        v.as_object_mut().unwrap().remove("signature");
    });
    rebuild(&out, &entries);
    assert_eq!(report_of(&out).branches["pcb"], SignatureStatus::Unsigned);
    let mut checks = Checks {
        verifier: Some(&ring),
        ..Checks::default()
    };
    checks.require_signed.insert("pcb".into());
    let (_, findings) = Project::verify(&out, &OpenOptions::default(), &checks).unwrap();
    assert!(findings.iter().any(|f| f.code == PkgCode::SignatureMissing));
}

#[test]
fn damaged_signature_records_are_never_taken_for_valid() {
    let dir = tempdir().unwrap();
    let path = signed(dir.path());
    let original = entries_of(&path);
    let out = dir.path().join("damaged.eh3d");
    let record: Value = serde_json::from_slice(&original["pcb/branch.json"]).unwrap();
    let value = record["signature"]["value"].as_str().unwrap().to_owned();

    let cases = [
        json!({"algorithm": "ed25519", "key_id": signer().key_id(), "value": ""}),
        json!({"algorithm": "ed25519", "key_id": signer().key_id(), "value": &value[..126]}),
        json!({"algorithm": "ed25519", "key_id": signer().key_id(), "value": &value[..127]}),
        json!({"algorithm": "ed25519", "key_id": signer().key_id(), "value": value.to_uppercase()}),
        json!({"algorithm": "ed25519", "key_id": signer().key_id(), "value": "zz".repeat(64)}),
        json!({"algorithm": "ed25519", "key_id": signer().key_id(), "value": "00".repeat(64)}),
        json!({"algorithm": "ed25519", "key_id": signer().key_id(), "value": "ff".repeat(64)}),
        json!({"algorithm": "ed25519", "key_id": signer().key_id(), "value": "0".repeat(4_000)}),
        json!({"algorithm": "ed25519", "key_id": "", "value": value}),
        json!({"algorithm": "none", "key_id": signer().key_id(), "value": value}),
        json!({"algorithm": "ed25519", "value": value}),
        json!({"algorithm": 1, "key_id": 2, "value": 3}),
        json!([1, 2, 3]),
        json!("signed"),
    ];
    for case in cases {
        let mut entries = original.clone();
        edit_json(&mut entries, "pcb/branch.json", |v| {
            v["signature"] = case.clone()
        });
        rebuild(&out, &entries);
        let status = report_of(&out).branches["pcb"].clone();
        assert!(
            matches!(
                status,
                SignatureStatus::Invalid | SignatureStatus::UnknownKey
            ),
            "{status:?}"
        );
    }

    // random bytes changed, added and cut in the record: never a valid signature over another record
    let clean = original["pcb/branch.json"].clone();
    let clean_value: Value = serde_json::from_slice(&clean).unwrap();
    let mut seed = Seed(0x5EED_0001);
    for round in 0..120 {
        let mut bytes = clean.clone();
        let at = seed.below(bytes.len());
        match seed.below(3) {
            0 => bytes[at] ^= 1 << seed.below(8),
            1 => bytes.insert(at, b"0123456789abcdef\"{}[],: "[seed.below(24)]),
            _ => bytes.truncate(at),
        }
        let mut entries = original.clone();
        entries.insert("pcb/branch.json".into(), bytes.clone());
        rebuild(&out, &entries);
        if report_of(&out).branches["pcb"] == valid() {
            let now: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(now, clean_value, "round {round}");
        }
    }
}

#[test]
fn random_damage_to_the_whole_file_never_leaves_a_changed_branch_valid() {
    let dir = tempdir().unwrap();
    let path = signed(dir.path());
    let clean = fs::read(&path).unwrap();
    let hash = Project::open(&path, &OpenOptions::default())
        .unwrap()
        .branch("pcb")
        .unwrap()
        .hash
        .clone();
    let out = dir.path().join("flipped.eh3d");
    let ring = keyring();
    let mut seed = Seed(0x5EED_0002);
    for round in 0..120 {
        let mut bytes = clean.clone();
        let at = seed.below(bytes.len());
        bytes[at] ^= 1 << seed.below(8);
        fs::write(&out, &bytes).unwrap();
        let Ok((p, report)) = Project::open_verified(&out, &OpenOptions::default(), Some(&ring))
        else {
            continue;
        };
        if report.branches.get("pcb") == Some(&valid()) {
            assert_eq!(p.branch("pcb").unwrap().hash, hash, "round {round}");
        }
    }
}

#[test]
fn key_files_are_made_once_and_for_the_owner_only() {
    let dir = tempdir().unwrap();
    let secret = dir.path().join("studio.key");
    let pair = generate_key_files(&secret).unwrap();
    assert_eq!(pair.secret_path, secret);
    assert_eq!(pair.public_path, public_key_path(&secret));
    let secret_text = fs::read_to_string(&secret).unwrap();
    let public_text = fs::read_to_string(&pair.public_path).unwrap();
    assert!(secret_text.starts_with(SECRET_KEY_HEADER));
    assert!(public_text.starts_with(PUBLIC_KEY_HEADER));

    let key = Ed25519Signer::read(&secret).unwrap();
    let mut ring = Ed25519Keyring::new();
    assert_eq!(ring.read_file(&pair.public_path).unwrap(), pair.key_id);
    assert_eq!(key.key_id(), pair.key_id);
    assert_eq!(
        ring.verify("ed25519", key.key_id(), b"m", &key.sign(b"m")),
        VerifyOutcome::Valid
    );
    // nothing printed of a signer carries the key
    let seed_hex = secret_text.lines().nth(1).unwrap();
    let shown = format!("{key:?} {pair:?}");
    assert!(!shown.contains(seed_hex), "{shown}");

    // never over a file, whichever of the two is there
    assert!(matches!(
        generate_key_files(&secret),
        Err(KeyError::AlreadyExists(_))
    ));
    assert_eq!(fs::read_to_string(&secret).unwrap(), secret_text);
    let other = dir.path().join("other.key");
    fs::write(public_key_path(&other), b"kept").unwrap();
    assert!(matches!(
        generate_key_files(&other),
        Err(KeyError::AlreadyExists(_))
    ));
    assert!(!other.exists());
    assert_eq!(fs::read(public_key_path(&other)).unwrap(), b"kept");
    // two keys are never the same
    let second = generate_key_files(&dir.path().join("second.key")).unwrap();
    assert_ne!(second.key_id, pair.key_id);
    // a project name is not a name for a key
    for name in ["p.eh3d", "P.EH3DPROJ"] {
        assert!(matches!(
            generate_key_files(&dir.path().join(name)),
            Err(KeyError::InsideProject(_))
        ));
        assert!(!dir.path().join(name).exists());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&secret).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    #[cfg(windows)]
    {
        let listed = std::process::Command::new("icacls")
            .arg(&secret)
            .output()
            .unwrap();
        let listed = String::from_utf8_lossy(&listed.stdout).into_owned();
        let granted = listed.lines().filter(|l| l.contains(":(")).count();
        assert_eq!(granted, 1, "{listed}");
    }
}

#[test]
fn damaged_key_files_are_refused_without_saying_what_they_hold() {
    let dir = tempdir().unwrap();
    let secret = dir.path().join("k.key");
    let pair = generate_key_files(&secret).unwrap();
    let clean = fs::read(&secret).unwrap();
    let seed_hex = String::from_utf8(clean.clone())
        .unwrap()
        .lines()
        .nth(1)
        .unwrap()
        .to_owned();
    let quiet = |error: KeyError| {
        let text = format!("{error} {error:?}");
        assert!(!text.contains(&seed_hex[..16]), "{text}");
    };

    assert!(matches!(
        Ed25519Signer::read(&pair.public_path),
        Err(KeyError::PublicKeyGiven(_))
    ));
    let mut ring = Ed25519Keyring::new();
    match ring.read_file(&secret) {
        Err(error @ KeyError::SecretKeyGiven(_)) => quiet(error),
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        Ed25519Signer::read(&dir.path().join("none")),
        Err(KeyError::Io { .. })
    ));

    let damaged = dir.path().join("damaged.key");
    let mut fixed: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"\n".to_vec(),
        SECRET_KEY_HEADER.as_bytes().to_vec(),
        vec![0u8; 100],
        vec![0xff; 4096],
        clean[..clean.len() - 3].to_vec(),
    ];
    fixed.push([clean.clone(), clean.clone()].concat());
    for bytes in fixed {
        fs::write(&damaged, &bytes).unwrap();
        match Ed25519Signer::read(&damaged) {
            Err(error) => quiet(error),
            Ok(_) => panic!("{} bytes were taken for a key", bytes.len()),
        }
    }

    let mut seed = Seed(0x5EED_0003);
    for _ in 0..300 {
        let mut bytes = clean.clone();
        let at = seed.below(bytes.len());
        match seed.below(3) {
            0 => bytes[at] ^= 1 << seed.below(8),
            1 => bytes.insert(at, (seed.next() & 0xff) as u8),
            _ => bytes.truncate(at),
        }
        fs::write(&damaged, &bytes).unwrap();
        // a change inside the digits may give another key, never the same one
        match Ed25519Signer::read(&damaged) {
            Ok(other) => {
                let same_digits = String::from_utf8_lossy(&bytes).contains(&seed_hex);
                assert_eq!(other.key_id() == pair.key_id, same_digits);
            }
            Err(error) => quiet(error),
        }
        if let Err(error) = ring.read_file(&damaged) {
            quiet(error);
        }
    }
}
