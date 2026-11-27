//! Signature keys under a password: made, opened, protected later, and what never happens to the
//! password and the key. The cost of Argon2id is low here, except where the file made by default
//! is looked at.

mod support;

use std::fs;
use std::path::Path;

use eh3d_container::PkgCode;
use eh3d_project::ed25519::{
    generate_key_files, generate_locked_key_files, generate_locked_key_files_with, key_protection,
    protect_key_file, protect_key_file_with,
};
use eh3d_project::keylock::{LOCKED_KEY_HEADER, MIN_PASSWORD_CHARS};
use eh3d_project::{
    BranchUpdate, Ed25519Keyring, Ed25519Signer, KdfParams, KeyError, KeyProtection, Password,
    SaveOptions, Signer, Writer,
};
use support::*;
use tempfile::tempdir;

const CHEAP: KdfParams = KdfParams {
    memory_kib: 256,
    passes: 1,
    lanes: 1,
};

/// Put together here so that no password is written in the source.
fn words(tail: &str) -> Password {
    Password::new(["made", "up", "phrase", tail].join(" "))
}

fn names(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

#[test]
fn a_new_key_under_a_password_signs_only_with_it() {
    let dir = tempdir().unwrap();
    let secret = dir.path().join("studio.key");
    let pair = generate_locked_key_files_with(&secret, &words("one"), CHEAP).unwrap();
    let text = fs::read_to_string(&secret).unwrap();
    assert!(text.starts_with(LOCKED_KEY_HEADER), "{text}");
    assert!(!text.contains("phrase"));
    assert_eq!(
        key_protection(&secret).unwrap(),
        KeyProtection::Password {
            key_id: pair.key_id.clone()
        }
    );
    let key = Ed25519Signer::read_with(&secret, Some(&words("one"))).unwrap();
    assert_eq!(key.key_id(), pair.key_id);
    let mut ring = Ed25519Keyring::new();
    assert_eq!(ring.read_file(&pair.public_path).unwrap(), pair.key_id);
    assert!(matches!(
        ring.read_file(&secret),
        Err(KeyError::SecretKeyGiven(_))
    ));

    assert!(matches!(
        Ed25519Signer::read(&secret),
        Err(KeyError::NeedsPassword(_))
    ));
    let wrong = Ed25519Signer::read_with(&secret, Some(&words("two"))).unwrap_err();
    assert!(matches!(wrong, KeyError::WrongPassword(_)), "{wrong:?}");
    let shown = format!("{wrong} {wrong:?} {key:?} {:?}", words("one"));
    assert!(!shown.contains("phrase"), "{shown}");

    assert!(matches!(
        generate_locked_key_files_with(&secret, &words("one"), CHEAP),
        Err(KeyError::AlreadyExists(_))
    ));
    let short = Password::new("x".repeat(MIN_PASSWORD_CHARS - 1));
    assert!(matches!(
        generate_locked_key_files(&dir.path().join("other.key"), &short),
        Err(KeyError::UnusablePassword)
    ));
    assert_eq!(names(dir.path()), ["studio.key", "studio.key.pub"]);
}

#[test]
fn the_key_made_by_default_costs_sixty_four_megabytes_and_opens() {
    let dir = tempdir().unwrap();
    let secret = dir.path().join("studio.key");
    generate_locked_key_files(&secret, &words("one")).unwrap();
    let text = fs::read_to_string(&secret).unwrap();
    assert_eq!(text.lines().nth(1), Some("argon2id-v19 m=65536 t=3 p=1"));
    Ed25519Signer::read_with(&secret, Some(&words("one"))).unwrap();
}

#[test]
fn an_old_key_is_read_as_before_and_can_be_protected() {
    let dir = tempdir().unwrap();
    let secret = dir.path().join("old.key");
    let pair = generate_key_files(&secret).unwrap();
    let public = fs::read(&pair.public_path).unwrap();
    let clear = fs::read_to_string(&secret).unwrap();
    let digits = clear.lines().nth(1).unwrap();
    assert_eq!(
        key_protection(&secret).unwrap(),
        KeyProtection::Unprotected {
            key_id: pair.key_id.clone()
        }
    );
    // a password given for a file without one does no harm
    Ed25519Signer::read_with(&secret, Some(&words("one"))).unwrap();
    let message = Ed25519Signer::read(&secret).unwrap().sign(b"m");

    let short = Password::new("short".into());
    assert!(matches!(
        protect_key_file(&secret, &short),
        Err(KeyError::UnusablePassword)
    ));
    assert!(matches!(
        protect_key_file_with(&pair.public_path, &words("one"), CHEAP),
        Err(KeyError::PublicKeyGiven(_))
    ));
    assert_eq!(fs::read_to_string(&secret).unwrap(), clear);

    protect_key_file_with(&secret, &words("one"), CHEAP).unwrap();
    let locked = fs::read_to_string(&secret).unwrap();
    assert!(locked.starts_with(LOCKED_KEY_HEADER) && !locked.contains(digits));
    assert_eq!(fs::read(&pair.public_path).unwrap(), public);
    assert_eq!(names(dir.path()), ["old.key", "old.key.pub"]);
    let key = Ed25519Signer::read_with(&secret, Some(&words("one"))).unwrap();
    assert_eq!(key.sign(b"m"), message);
    assert!(matches!(
        protect_key_file_with(&secret, &words("two"), CHEAP),
        Err(KeyError::AlreadyProtected(_))
    ));
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

#[cfg(unix)]
#[test]
fn protecting_a_link_protects_the_key_it_points_at() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let apart = dir.path().join("apart");
    fs::create_dir(&apart).unwrap();
    let real = apart.join("real.key");
    generate_key_files(&real).unwrap();
    // a relative link to a link
    symlink("apart/real.key", dir.path().join("first.key")).unwrap();
    let link = dir.path().join("second.key");
    symlink("first.key", &link).unwrap();

    protect_key_file_with(&link, &words("one"), CHEAP).unwrap();
    assert!(fs::read_to_string(&real)
        .unwrap()
        .starts_with(LOCKED_KEY_HEADER));
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    Ed25519Signer::read_with(&link, Some(&words("one"))).unwrap();
    assert_eq!(names(&apart), ["real.key", "real.key.pub"]);

    // links that go round and a link that points nowhere are errors, and nothing is made
    symlink("round.key", dir.path().join("round.key")).unwrap();
    symlink("nowhere", dir.path().join("lost.key")).unwrap();
    for bad in ["round.key", "lost.key"] {
        assert!(matches!(
            protect_key_file_with(&dir.path().join(bad), &words("one"), CHEAP),
            Err(KeyError::Io { .. })
        ));
    }
    assert!(!dir.path().join("nowhere").exists());
}

#[test]
fn a_changed_locked_file_is_refused_and_it_is_never_put_into_a_project() {
    let dir = tempdir().unwrap();
    let secret = dir.path().join("studio.key");
    generate_locked_key_files_with(&secret, &words("one"), CHEAP).unwrap();
    let clean = fs::read(&secret).unwrap();

    let p = new_project(dir.path());
    let update = BranchUpdate::new("pcb", "1.0", "0.1.0").with_entry("notes.txt", &clean);
    match p.save_branch(&Writer::for_app("pcb"), update, &SaveOptions::default()) {
        Err(eh3d_container::ContainerError::Rejected(v)) => {
            assert_eq!(v.code, PkgCode::BranchRecord)
        }
        other => panic!("{:?}", other.map(|_| ())),
    }

    let damaged = dir.path().join("damaged.key");
    let mut seed = Seed(0x5EED_0007);
    for _ in 0..200 {
        let mut bytes = clean.clone();
        let at = seed.below(bytes.len());
        match seed.below(3) {
            0 => bytes[at] ^= 1 << seed.below(8),
            1 => bytes.insert(at, (seed.next() & 0xff) as u8),
            _ => bytes.truncate(at),
        }
        fs::write(&damaged, &bytes).unwrap();
        // a change of a line end may leave the same key; anything else is refused without a word of it
        if let Err(e) = Ed25519Signer::read_with(&damaged, Some(&words("one"))) {
            assert!(!format!("{e} {e:?}").contains("phrase"));
        } else {
            let same = String::from_utf8(bytes).unwrap();
            assert_eq!(
                same.lines().collect::<Vec<_>>(),
                std::str::from_utf8(&clean)
                    .unwrap()
                    .lines()
                    .collect::<Vec<_>>()
            );
        }
    }
}
