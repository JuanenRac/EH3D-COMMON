//! The job of a project as the manufacturing programs use it, and what a window shows of a project before it writes there. Every file is made here from invented data.

use std::path::{Path, PathBuf};

use eh3d_project::manufacturing::{
    choose, describe, keep_result, kit, layers_of, out_of_date_message, read_job, read_result,
    result_name, scratch_dir, state, SOURCE_BRANCH,
};
use eh3d_project::suitetext::{Said, S};
use eh3d_project::suiteview::{banner, copy_project, is_project, sets_key, view, warnings};
use eh3d_project::{BranchUpdate, NewProject, OpenOptions, Project, SaveOptions, Writer};

const DRILL: &str = "M48\nMETRIC\nT1C0.8\n%\nT1\nX005.000Y005.000\nM30\n";

fn write_branch(path: &Path, branch: &str, files: &[(&str, &[u8])]) {
    let mut update = BranchUpdate::new(branch, "1.0", "0.1.0");
    for (name, bytes) in files {
        update.entries.insert((*name).to_string(), bytes.to_vec());
    }
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .save_branch(&Writer::for_app(branch), update, &SaveOptions::default())
        .unwrap();
}

fn write_job(path: &Path, plated: &str) {
    let record = br#"{"format_version":"1.0.0","layers":[
        {"path":"layers/plated.drl","kind":"drill","name":"Plated holes"},
        {"path":"layers/loose.drl","kind":"drill"},
        {"path":"layers/plated.drl","kind":"drill"}]}"#;
    write_branch(
        path,
        SOURCE_BRANCH,
        &[
            ("project.json", record),
            ("layers/plated.drl", plated.as_bytes()),
            ("layers/loose.drl", DRILL.as_bytes()),
        ],
    );
}

fn project(dir: &Path) -> PathBuf {
    let path = dir.join("board.eh3d");
    let new = NewProject {
        id: "test-id".into(),
        name: "Invented board".into(),
        units: "millimetres".into(),
        created: None,
    };
    Project::create(
        &path,
        new,
        &OpenOptions::default(),
        &SaveOptions::default().atomic,
    )
    .unwrap();
    write_branch(&path, "pcb", &[("board.json", b"{}")]);
    write_job(&path, DRILL);
    path
}

fn hash(path: &Path, branch: &str) -> String {
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .branch(branch)
        .unwrap()
        .hash
        .clone()
}

#[test]
fn the_layers_of_the_job_are_read_once_each_chosen_and_named() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    assert!(is_project(&path));
    assert!(!is_project(Path::new("board.ncd")));
    let layers = layers_of(&path, "drill").unwrap();
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].name, "Plated holes");
    assert_eq!(layers[1].name, "loose.drl");
    let first = choose(&layers, "drill", "plated.drl").unwrap();
    assert_eq!(
        first,
        choose(&layers, "drill", "layers/plated.drl").unwrap()
    );
    let why = choose(&layers, "drill", "other.drl").unwrap_err();
    assert!(why.contains("no drill layer other.drl"), "{why}");
    assert_eq!(result_name("drill", first, "txt"), "drill-plated.txt");
    let why = layers_of(&path, "copper").unwrap_err();
    assert!(why.contains("no copper layer"), "{why}");
    assert_eq!(read_job(&path, &[]).unwrap().len(), 2);
}

#[test]
fn results_stay_in_the_branch_of_the_program_and_a_changed_job_is_out_of_date() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let program = kit("drillpro", "drillpro.json", "0.1.0");
    let (pcb, job) = (hash(&path, "pcb"), hash(&path, SOURCE_BRANCH));
    let layers = layers_of(&path, "drill").unwrap();
    keep_result(&program, &path, &layers[0], "a.txt", b"one".to_vec()).unwrap();
    keep_result(&program, &path, &layers[1], "b.txt", b"two".to_vec()).unwrap();
    let now = state(&program, &path).unwrap();
    assert_eq!(now.results, ["a.txt", "b.txt"]);
    assert_eq!(now.inputs, ["layers/loose.drl", "layers/plated.drl"]);
    assert!(!now.out_of_date);
    assert_eq!(read_result(&program, &path, "a.txt").unwrap(), b"one");
    assert!(read_result(&program, &path, "c.txt").is_err());
    assert_eq!(hash(&path, "pcb"), pcb);
    assert_eq!(hash(&path, SOURCE_BRANCH), job);

    write_job(&path, "M48\nMETRIC\nT1C1.0\n%\nT1\nX001.000Y001.000\nM30\n");
    let (lines, now) = describe(&program, &path).unwrap();
    assert!(now.out_of_date, "{lines:?}");
    assert!(out_of_date_message(&program).contains("OUT OF DATE"));
    // a layer that was read before the job changed is not kept
    let why = keep_result(&program, &path, &layers[0], "a.txt", b"old".to_vec()).unwrap_err();
    assert!(why.contains("changed"), "{why}");
    let layers = layers_of(&path, "drill").unwrap();
    keep_result(&program, &path, &layers[0], "a.txt", b"new".to_vec()).unwrap();
    let now = state(&program, &path).unwrap();
    assert_eq!(now.results, ["a.txt"]);
    assert!(!now.out_of_date);
}

#[test]
fn the_view_tells_the_branches_and_what_a_write_needs_to_be_agreed_to() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path());
    let found = view(&path, "pcb").unwrap();
    assert!(found.own_hash.is_some());
    assert!(found
        .takeable
        .iter()
        .any(|(b, n)| b == SOURCE_BRANCH && n == "layers/loose.drl"));
    assert_eq!(banner(&path, Some(&found), "pcb", "es").len(), 1);
    let own = found.own_hash.clone();
    assert!(warnings(&path, Some(&path), own.as_deref(), "pcb")
        .unwrap()
        .is_empty());
    assert_eq!(
        warnings(&path, None, None, "pcb").unwrap(),
        [Said(
            S::WarnForeign,
            vec!["board.eh3d".into(), "pcb".into()]
        )]
    );
    let copy = dir.path().join("copy.eh3d");
    copy_project(&path, &copy).unwrap();
    assert_eq!(std::fs::read(&copy).unwrap(), std::fs::read(&path).unwrap());
    assert!(copy_project(&path, &copy).is_err());
}

#[test]
fn a_settings_file_that_gives_a_credential_is_found_however_it_is_written() {
    // the value is put together here so that no credential is written in the source
    let value = ["abc", "123"].concat();
    for line in [
        format!("server_token = {value}\n"),
        format!("[server]\r\n  Server_Token={value}\r\n"),
        format!("\u{feff}server_token = {value}"),
        format!("this line is broken\nserver_token = {value} # mine"),
    ] {
        assert!(sets_key(line.as_bytes(), "server_token"), "{line:?}");
    }
    for line in [
        "",
        "server_token =\n",
        "server_token\n",
        "# server_token = x\n",
        "server_token_fixed = on\n",
    ] {
        assert!(!sets_key(line.as_bytes(), "server_token"), "{line:?}");
    }
}

#[test]
fn every_scratch_folder_is_new() {
    let a = scratch_dir("test").unwrap();
    let b = scratch_dir("test").unwrap();
    assert_ne!(a, b);
    std::fs::remove_dir(a).unwrap();
    std::fs::remove_dir(b).unwrap();
}
