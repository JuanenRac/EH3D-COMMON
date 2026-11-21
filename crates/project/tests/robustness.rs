//! Properties with a fixed seed, damaged files, limits and the `manufacturing` branch against the
//! reference packages of the `.eh3dproj` 1.0.0 format.

mod support;

use std::path::PathBuf;

use eh3d_container::{AtomicOptions, PkgCode};
use eh3d_project::spec::{eh3d, BRANCH_FILE};
use eh3d_project::{manufacturing_update, BranchUpdate, Checks, OpenOptions, Project, Writer};
use support::*;
use tempfile::tempdir;

/// A small generator with a fixed seed (xorshift): the same numbers on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap()).unwrap()
    }
}

fn random_update(rng: &mut Rng, id: &str) -> BranchUpdate {
    let mut update = BranchUpdate::new(id, "1.0", "0.1.0");
    for i in 0..=rng.below(3) {
        let len = rng.below(200);
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next() & 0xff) as u8).collect();
        update.entries.insert(format!("part{i}.bin"), bytes);
    }
    update
}

fn at(step: usize) -> String {
    format!("2000-01-02T00:{:02}:{:02}Z", step / 60, step % 60)
}

#[test]
fn the_golden_packages_come_out_of_the_manufacturing_branch_byte_for_byte() {
    for name in ["minimal", "typical", "unicode_names"] {
        let package = PathBuf::from(GOLDEN).join(format!("{name}.eh3dproj"));
        let dir = tempdir().unwrap();
        let p = new_project(dir.path());
        let update = manufacturing_update(&package, "0.1.0").unwrap();
        let mut p = save(p, "manufacturing", update, "2000-01-01T00:00:01Z");
        let out = dir.path().join("out.eh3dproj");
        p.export_manufacturing(&out, &AtomicOptions::default())
            .unwrap();
        assert_eq!(
            std::fs::read(&out).unwrap(),
            std::fs::read(&package).unwrap(),
            "{name}"
        );
    }
}

#[test]
fn saving_a_branch_never_changes_the_bytes_of_the_others_with_a_fixed_seed() {
    let dir = tempdir().unwrap();
    let mut p = synthetic(dir.path());
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let apps = ["pcb", "cad", "3dp", "laser", "cnc"];
    for step in 0..24 {
        let app = apps[rng.below(apps.len())];
        let before = raw_entries(p.path());
        let update = random_update(&mut rng, app);
        let own = update.entries.len();
        p = save(p, app, update, &at(step));
        let after = raw_entries(p.path());
        for (name, kept) in &before {
            let mine = name.starts_with(&format!("{app}/"));
            let record = name == "project.json" || name == "integrity.json";
            if !mine && !record {
                assert_eq!(after.get(name), Some(kept), "step {step}: {name}");
            }
        }
        // the branch has its own files and its record, nothing from the old content
        let ours = after
            .keys()
            .filter(|n| n.starts_with(&format!("{app}/")))
            .count();
        assert_eq!(ours, own + 1, "step {step}");
        assert!(p.integrity_verified());
        assert_eq!(p.diagnose(&Checks::default()).len(), p.stale_refs().len());
    }
}

#[test]
fn opening_saving_the_same_and_opening_again_gives_the_same_bytes_with_a_fixed_seed() {
    let dir = tempdir().unwrap();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut p = synthetic(dir.path());
    for step in 0..8 {
        let update = random_update(&mut rng, "laser");
        p = save(p, "laser", update.clone(), &at(step));
        let once = std::fs::read(p.path()).unwrap();
        let hash = p.content_hash().unwrap();
        let reopened = Project::open(p.path(), &OpenOptions::default()).unwrap();
        let again = reopened
            .save_branch(&Writer::for_app("laser"), update, &options_at(&at(step)))
            .unwrap();
        assert_eq!(std::fs::read(again.path()).unwrap(), once, "step {step}");
        assert_eq!(again.content_hash().unwrap(), hash);
        p = again;
    }
}

#[test]
fn a_damaged_file_is_refused_or_reported_and_never_panics() {
    let dir = tempdir().unwrap();
    let original = {
        let p = synthetic(dir.path());
        std::fs::read(p.path()).unwrap()
    };
    let baseline = Project::open(
        &write_to(dir.path(), "base.eh3d", &original),
        &OpenOptions::default(),
    )
    .unwrap()
    .content_hash()
    .unwrap();
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    let mut refused = 0;
    for round in 0..120 {
        let mut bytes = original.clone();
        match round % 4 {
            0 => {
                let at = rng.below(bytes.len());
                bytes[at] ^= 1 << rng.below(8);
            }
            1 => bytes.truncate(rng.below(bytes.len())),
            2 => {
                let from = rng.below(bytes.len());
                let to = (from + 1 + rng.below(32)).min(bytes.len());
                for b in &mut bytes[from..to] {
                    *b = (rng.next() & 0xff) as u8;
                }
            }
            _ => bytes.extend((0..=rng.below(64)).map(|_| (rng.next() & 0xff) as u8)),
        }
        let path = write_to(dir.path(), "damaged.eh3d", &bytes);
        match Project::open(&path, &OpenOptions::default()) {
            Err(_) => refused += 1,
            Ok(mut project) => {
                let findings = project.diagnose(&Checks::default());
                if findings.is_empty() {
                    // a change that no entry shows (a time stamp, a comment) leaves the content as it was
                    assert_eq!(project.content_hash().unwrap(), baseline, "round {round}");
                }
            }
        }
    }
    assert!(refused > 60, "only {refused} of 120 damaged files refused");
}

fn write_to(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn the_limits_of_the_preset_apply_to_a_project() {
    let dir = tempdir().unwrap();
    let p = synthetic(dir.path());
    let mut spec = eh3d();
    spec.limits.max_entries = 5;
    let few = OpenOptions {
        spec,
        ..OpenOptions::default()
    };
    let refusal = Project::open(p.path(), &few)
        .err()
        .expect("too many entries");
    assert_eq!(refusal.code(), Some(PkgCode::Limit));

    let mut spec = eh3d();
    spec.limits.default_max_entry_bytes = 8;
    spec.limits.per_root_max_entry_bytes.clear();
    let small = OpenOptions {
        spec,
        ..OpenOptions::default()
    };
    assert!(Project::open(p.path(), &small).is_err());
}

#[test]
fn a_file_that_is_not_a_project_is_refused_with_a_code() {
    let dir = tempdir().unwrap();
    for (name, bytes) in [
        ("empty.eh3d", Vec::new()),
        ("text.eh3d", b"not a zip at all".to_vec()),
        ("zeros.eh3d", vec![0u8; 4096]),
    ] {
        let path = write_to(dir.path(), name, &bytes);
        let refusal = Project::open(&path, &OpenOptions::default())
            .err()
            .expect("refused");
        assert!(refusal.code().is_some(), "{name}");
    }
    assert!(Project::open(&dir.path().join("missing.eh3d"), &OpenOptions::default()).is_err());
}

#[test]
fn the_branch_record_name_is_reserved_inside_a_branch() {
    let dir = tempdir().unwrap();
    let p = new_project(dir.path());
    let update = BranchUpdate::new("pcb", "1.0", "0.1.0").with_entry(BRANCH_FILE, b"{}");
    let refusal = p
        .save_branch(
            &Writer::for_app("pcb"),
            update,
            &options_at("2000-01-01T00:00:01Z"),
        )
        .err()
        .expect("refused");
    assert_eq!(refusal.code(), Some(PkgCode::BranchRecord));
}
