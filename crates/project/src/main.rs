//! `eh3d-project`: look into a `.eh3d` project from a terminal.
//!
//! ```text
//! eh3d-project info FILE [--json]
//! eh3d-project verify FILE [--strict] [--require-signed BRANCH[,BRANCH...]]
//!                          [--require-project-signature] [--pubkey KEY.pub]...
//! eh3d-project keygen KEYFILE [--password-stdin | --no-password]
//! eh3d-project protect-key KEYFILE [--password-stdin]
//! eh3d-project sign FILE --key KEYFILE [--branch BRANCH[,BRANCH...]] [--project] [--password-stdin] [--replace-signature]
//! eh3d-project extract-branch FILE BRANCH OUT_DIR
//! eh3d-project extract-branch FILE manufacturing --eh3dproj OUT.eh3dproj
//! eh3d-project attach FILE PATH [--name NAME] [--title TEXT] [--url URL] [LINKS] [--max-mb N]
//! eh3d-project attach-link FILE URL (--sha256 HEX [--size BYTES] | --of PATH) [--name NAME] [--title TEXT] [LINKS]
//! eh3d-project attachments FILE [--json] [LINKS]
//! eh3d-project extract-attachment FILE ID OUT_DIR [--confirm-dangerous] [--max-mb N]
//! eh3d-project attachment-links FILE ID (add|remove) LINKS
//! eh3d-project detach FILE ID
//! ```
//!
//! A new secret key is under a password, asked twice at the terminal without showing it, or read
//! as one line of the standard input with `--password-stdin`; `--no-password` makes one without.
//! A password is never taken from the command line. `sign` replaces a signature that is there only
//! with `--replace-signature`.
//!
//! `LINKS` are the pieces of the design: `--ref R5`, `--value 10k`, `--part NUMBER`, `--maker-part MAKER NUMBER`, each one as
//! often as needed.
//!
//! Exit codes: 0 sound, 2 verification not passed (an error finding, or a warning with
//! `--strict`), 3 something needed an agreement that was not given (a dangerous or unknown
//! attachment, a signature to replace), 1 anything else that failed, 64 wrong use. Nothing is
//! written except by `extract-branch`, `extract-attachment` and `keygen`, which never replace a
//! file, by `protect-key`, which rewrites the key file given to it, and by `sign`, `attach`,
//! `attach-link`, `attachment-links` and `detach`, which rewrite the project given to them.
//! Nothing is opened, run or downloaded. The secret key is read from the path given; it and the
//! password are never printed.

use std::collections::BTreeSet;
use std::io::{BufRead, IsTerminal, Read};
use std::path::Path;
use std::process::ExitCode;

use eh3d_container::AtomicOptions;
use eh3d_project::attachments::{self, Attached};
use eh3d_project::ed25519::{
    generate_key_files, generate_locked_key_files, key_protection, protect_key_file,
};
use eh3d_project::{
    AttachError, AttachRequest, Attacher, Attachment, Checks, ComponentLink, ComponentQuery,
    Consent, Ed25519Keyring, Ed25519Signer, KeyError, KeyProtection, OpenOptions, Password,
    Project, SaveOptions, Signer, Verifier,
};
use zeroize::Zeroizing;

const USAGE: &str = "usage:\n  eh3d-project info FILE [--json]\n  eh3d-project verify FILE [--strict] [--require-signed BRANCH[,BRANCH...]] [--require-project-signature] [--pubkey KEY.pub]...\n  eh3d-project extract-branch FILE BRANCH OUT_DIR\n  eh3d-project extract-branch FILE manufacturing --eh3dproj OUT.eh3dproj\n  eh3d-project keygen KEYFILE [--password-stdin | --no-password]\n  eh3d-project protect-key KEYFILE [--password-stdin]\n  eh3d-project sign FILE --key KEYFILE [--branch BRANCH[,BRANCH...]] [--project] [--password-stdin] [--replace-signature]\n  eh3d-project attach FILE PATH [--name NAME] [--title TEXT] [--url URL] [LINKS] [--max-mb N]\n  eh3d-project attach-link FILE URL (--sha256 HEX [--size BYTES] | --of PATH) [--name NAME] [--title TEXT] [LINKS]\n  eh3d-project attachments FILE [--json] [LINKS]\n  eh3d-project extract-attachment FILE ID OUT_DIR [--confirm-dangerous] [--max-mb N]\n  eh3d-project attachment-links FILE ID (add|remove) LINKS\n  eh3d-project detach FILE ID\nLINKS: --ref R5, --value 10k, --part NUMBER, --maker-part MAKER NUMBER (each as often as needed)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let outcome = match args.first().map(String::as_str) {
        Some("info") => info(&args[1..]),
        Some("verify") => verify(&args[1..]),
        Some("extract-branch") => extract(&args[1..]),
        Some("keygen") => keygen(&args[1..]),
        Some("protect-key") => protect_key(&args[1..]),
        Some("sign") => sign(&args[1..]),
        Some("attach") => attach(&args[1..]),
        Some("attach-link") => attach_link(&args[1..]),
        Some("attachments") => list_attachments(&args[1..]),
        Some("extract-attachment") => extract_attachment(&args[1..]),
        Some("attachment-links") => attachment_links(&args[1..]),
        Some("detach") => detach(&args[1..]),
        _ => Err(Failure::Usage),
    };
    match outcome {
        Ok(code) => ExitCode::from(code),
        Err(Failure::Usage) => {
            eprintln!("{USAGE}");
            ExitCode::from(64)
        }
        Err(Failure::Error(text)) => {
            eprintln!("error: {text}");
            ExitCode::from(1)
        }
        Err(Failure::NeedsConfirmation(text)) => {
            eprintln!("error: {text}; add --confirm-dangerous if you trust it");
            ExitCode::from(3)
        }
        Err(Failure::NeedsReplacing(text)) => {
            eprintln!("error: {text}; add --replace-signature to replace it");
            ExitCode::from(3)
        }
    }
}

enum Failure {
    Usage,
    Error(String),
    NeedsConfirmation(String),
    NeedsReplacing(String),
}

impl From<AttachError> for Failure {
    fn from(e: AttachError) -> Self {
        match e {
            AttachError::NeedsConfirmation { .. } => Failure::NeedsConfirmation(e.to_string()),
            _ => Failure::Error(e.to_string()),
        }
    }
}

impl From<eh3d_container::ContainerError> for Failure {
    fn from(e: eh3d_container::ContainerError) -> Self {
        Failure::Error(e.to_string())
    }
}

impl From<KeyError> for Failure {
    fn from(e: KeyError) -> Self {
        Failure::Error(e.to_string())
    }
}

fn open(path: &str) -> Result<Project, Failure> {
    Ok(Project::open(Path::new(path), &OpenOptions::default())?)
}

fn info(args: &[String]) -> Result<u8, Failure> {
    let [file, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let json = match rest {
        [] => false,
        [flag] if flag == "--json" => true,
        _ => return Err(Failure::Usage),
    };
    let project = open(file)?;
    let record = project.record();
    let stale = project.stale_refs();
    if json {
        let branches: Vec<serde_json::Value> = project
            .branches()
            .map(|b| {
                serde_json::json!({
                    "id": b.id,
                    "known": b.is_known(),
                    "files": b.files.len(),
                    "hash": b.hash,
                    "app": b.record.as_ref().map(|r| r.app.clone()),
                    "format": b.record.as_ref().map(|r| r.format.clone()),
                    "app_version": b.record.as_ref().map(|r| r.app_version.clone()),
                    "sealed": b.is_sealed(),
                    "encrypted": b.is_encrypted(),
                    "signed": b.record.as_ref().is_some_and(|r| r.signature.is_some()),
                    "out_of_date": stale.iter().any(|s| s.branch == b.id),
                })
            })
            .collect();
        let document = serde_json::json!({
            "id": record.id,
            "name": record.name,
            "format_version": record.format_version,
            "container_version": record.container_version,
            "units": record.units,
            "sealed": record.seal.is_some(),
            "content_hash": project.content_hash()?,
            "branches": branches,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&document).unwrap_or_default()
        );
        return Ok(0);
    }
    println!("project   {} ({})", record.name, record.id);
    println!(
        "format    {} (container rules {})",
        record.format_version, record.container_version
    );
    println!("units     {}", record.units);
    println!(
        "sealed    {}",
        if record.seal.is_some() { "yes" } else { "no" }
    );
    println!("content   {}", project.content_hash()?);
    println!("branches  {}", project.branches().count());
    for b in project.branches() {
        let (app, format, version) = b.record.as_ref().map_or(("-", "-", "-"), |r| {
            (r.app.as_str(), r.format.as_str(), r.app_version.as_str())
        });
        let mut notes = Vec::new();
        if !b.is_known() {
            notes.push("not known by name, kept as it is");
        }
        if b.is_sealed() {
            notes.push("sealed");
        }
        if b.is_encrypted() {
            notes.push("encrypted");
        }
        if stale.iter().any(|s| s.branch == b.id) {
            notes.push("OUT OF DATE");
        }
        println!(
            "  {:<16} {:>4} files  format {:<6} {} {}  {}",
            b.id,
            b.files.len(),
            format,
            app,
            version,
            notes.join(", ")
        );
    }
    for s in &stale {
        println!(
            "  '{}' was derived from '{}' ({}) before it changed",
            s.branch, s.dependency.branch, s.dependency.artifact
        );
    }
    Ok(0)
}

fn verify(args: &[String]) -> Result<u8, Failure> {
    let [file, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let mut strict = false;
    let mut required: BTreeSet<String> = BTreeSet::new();
    let mut project_required = false;
    let mut keyring = Ed25519Keyring::new();
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--strict" => strict = true,
            "--require-project-signature" => project_required = true,
            "--pubkey" => {
                i += 1;
                let path = rest.get(i).ok_or(Failure::Usage)?;
                keyring.read_file(Path::new(path))?;
            }
            "--require-signed" => {
                i += 1;
                let list = rest.get(i).ok_or(Failure::Usage)?;
                required.extend(list.split(',').filter(|s| !s.is_empty()).map(str::to_owned));
            }
            _ => return Err(Failure::Usage),
        }
        i += 1;
    }
    let verifier: Option<&dyn Verifier> = if keyring.is_empty() {
        None
    } else {
        Some(&keyring)
    };
    let checks = Checks {
        require_signed: required,
        require_project_signature: project_required,
        verifier,
        ..Checks::default()
    };
    let mut project = open(file)?;
    let report = project.signatures(verifier);
    println!("signature project: {}", report.project);
    for (branch, status) in &report.branches {
        println!("signature {branch}: {status}");
    }
    let findings = project.diagnose(&checks);
    let mut failed = false;
    for f in &findings {
        let level = if f.is_error() { "error" } else { "warning" };
        println!(
            "{level} {}: {} [{}]{}",
            f.code,
            f.detail,
            f.branch.as_deref().unwrap_or("project"),
            f.entry
                .as_deref()
                .map_or(String::new(), |e| format!(" {e}"))
        );
        failed |= f.is_error() || strict;
    }
    if failed {
        println!("VERIFY=FAIL");
        Ok(2)
    } else {
        println!("VERIFY=PASS");
        Ok(0)
    }
}

/// One line of the standard input, without its end: the password of a script.
fn password_from_stdin() -> Result<Password, Failure> {
    let mut line = Zeroizing::new(String::with_capacity(2048));
    std::io::stdin()
        .lock()
        .take(2048)
        .read_line(&mut line)
        .map_err(|e| Failure::Error(format!("the password could not be read: {e}")))?;
    // a mark of the byte order that a pipe or a saved file puts first is not typed at a terminal
    let text = line.strip_prefix('\u{feff}').unwrap_or(&line);
    let text = text.trim_end_matches(['\r', '\n']);
    Ok(Password::new(text.to_owned()))
}

/// A password asked at the terminal, which does not show it.
fn password_from_terminal(prompt: &str) -> Result<Password, Failure> {
    if !std::io::stdin().is_terminal() {
        return Err(Failure::Error(
            "there is no terminal to ask for the password; give it with --password-stdin".into(),
        ));
    }
    rpassword::prompt_password(prompt)
        .map(Password::new)
        .map_err(|e| Failure::Error(format!("the password could not be read: {e}")))
}

/// A new password: one line of the standard input, or asked twice at the terminal.
fn new_password(from_stdin: bool) -> Result<Password, Failure> {
    if from_stdin {
        return password_from_stdin();
    }
    let first = password_from_terminal("New password: ")?;
    if !first.same_as(&password_from_terminal("The same password again: ")?) {
        return Err(Failure::Error("the two passwords are not the same".into()));
    }
    Ok(first)
}

fn keygen(args: &[String]) -> Result<u8, Failure> {
    let [path, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    if path.starts_with("--") {
        return Err(Failure::Usage);
    }
    let path = Path::new(path);
    let pair = match rest {
        [flag] if flag == "--no-password" => {
            eprintln!(
                "warning: the key file has no password: whoever copies it can sign as you ('eh3d-project protect-key' adds one)"
            );
            generate_key_files(path)?
        }
        [] => generate_locked_key_files(path, &new_password(false)?)?,
        [flag] if flag == "--password-stdin" => {
            generate_locked_key_files(path, &new_password(true)?)?
        }
        _ => return Err(Failure::Usage),
    };
    println!("key        {}", pair.key_id);
    println!(
        "secret key {} (keep it to yourself, outside any project)",
        pair.secret_path.display()
    );
    println!(
        "public key {} (give it to those who check)",
        pair.public_path.display()
    );
    Ok(0)
}

fn protect_key(args: &[String]) -> Result<u8, Failure> {
    let [path, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let from_stdin = match rest {
        [] => false,
        [flag] if flag == "--password-stdin" => true,
        _ => return Err(Failure::Usage),
    };
    let path = Path::new(path);
    // said before the person types anything
    if matches!(key_protection(path)?, KeyProtection::Password { .. }) {
        return Err(KeyError::AlreadyProtected(path.to_owned()).into());
    }
    protect_key_file(path, &new_password(from_stdin)?)?;
    println!("the key file {} has a password now", path.display());
    Ok(0)
}

/// The signer of the key file, with its password when it has one; a file without one is told.
fn signer_of(path: &Path, from_stdin: bool) -> Result<Ed25519Signer, Failure> {
    match key_protection(path)? {
        KeyProtection::Password { .. } => {
            let password = if from_stdin {
                password_from_stdin()?
            } else {
                password_from_terminal("Password of the key file: ")?
            };
            Ok(Ed25519Signer::read_with(path, Some(&password))?)
        }
        _ => {
            eprintln!(
                "warning: the key file has no password ('eh3d-project protect-key' adds one)"
            );
            Ok(Ed25519Signer::read(path)?)
        }
    }
}

fn sign(args: &[String]) -> Result<u8, Failure> {
    let [file, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let mut key = None;
    let mut branches: BTreeSet<String> = BTreeSet::new();
    let (mut whole, mut from_stdin, mut replace) = (false, false, false);
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--project" => whole = true,
            "--password-stdin" => from_stdin = true,
            "--replace-signature" => replace = true,
            "--key" if key.is_none() => {
                i += 1;
                key = Some(rest.get(i).ok_or(Failure::Usage)?);
            }
            "--branch" => {
                i += 1;
                let list = rest.get(i).ok_or(Failure::Usage)?;
                branches.extend(list.split(',').filter(|s| !s.is_empty()).map(str::to_owned));
            }
            _ => return Err(Failure::Usage),
        }
        i += 1;
    }
    let key = key.ok_or(Failure::Usage)?;
    if branches.is_empty() && !whole {
        return Err(Failure::Usage);
    }
    let mut project = open(file)?;
    // everything is checked before the password is asked and before the first signature, so a
    // refusal leaves the file as it was
    for branch in &branches {
        project.check_signable(branch)?;
    }
    if whole && project.record().seal.is_some() {
        return Err(Failure::Error("the project is sealed read-only".into()));
    }
    if !replace {
        let asked = branches
            .iter()
            .map(|b| Some(b.as_str()))
            .chain(whole.then_some(None));
        for target in asked {
            if let Some(old) = project.signature_to_replace(target) {
                return Err(Failure::NeedsReplacing(format!(
                    "{} is signed already with the key {}",
                    target.unwrap_or("the project"),
                    old.key_id
                )));
            }
        }
    }
    let signer = signer_of(Path::new(key), from_stdin)?;
    let options = SaveOptions::default();
    for branch in &branches {
        project = if replace {
            project.sign_branch_replacing(branch, &signer, &options)?
        } else {
            project.sign_branch(branch, &signer, &options)?
        };
        println!("signed branch {branch} with {}", signer.key_id());
    }
    if whole {
        if replace {
            project.sign_project_replacing(&signer, &options)?;
        } else {
            project.sign_project(&signer, &options)?;
        }
        println!("signed the project with {}", signer.key_id());
    }
    Ok(0)
}

fn extract(args: &[String]) -> Result<u8, Failure> {
    match args {
        [file, branch, out] => {
            let mut project = open(file)?;
            let count = project.extract_branch(branch, Path::new(out))?;
            println!("{count} files written to {out}");
            Ok(0)
        }
        [file, branch, flag, out] if flag == "--eh3dproj" && branch == "manufacturing" => {
            let mut project = open(file)?;
            project.export_manufacturing(Path::new(out), &AtomicOptions::default())?;
            println!("package written to {out}");
            Ok(0)
        }
        _ => Err(Failure::Usage),
    }
}

/// The options of the commands of the attachments, in any order.
#[derive(Default)]
struct Options {
    request: AttachRequest,
    sha256: Option<String>,
    size: Option<u64>,
    of: Option<String>,
    max_mb: Option<String>,
    json: bool,
    confirmed: bool,
}

fn options(args: &[String], allowed: &[&str]) -> Result<Options, Failure> {
    const LINKS: [&str; 4] = ["--ref", "--value", "--part", "--maker-part"];
    let mut out = Options::default();
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let name = flag.as_str();
        let known =
            allowed.contains(&name) || (allowed.contains(&"LINKS") && LINKS.contains(&name));
        if !known {
            return Err(Failure::Usage);
        }
        let mut word = || rest.next().cloned().ok_or(Failure::Usage);
        match name {
            "--json" => out.json = true,
            "--confirm-dangerous" => out.confirmed = true,
            "--ref" => {
                let link = ComponentLink::designator(&word()?);
                out.request.components.push(link);
            }
            "--value" => {
                let link = ComponentLink::of_value(&word()?);
                out.request.components.push(link);
            }
            "--part" => {
                let link = ComponentLink::part(None, &word()?);
                out.request.components.push(link);
            }
            "--maker-part" => {
                let maker = word()?;
                let link = ComponentLink::part(Some(&maker), &word()?);
                out.request.components.push(link);
            }
            "--name" => out.request.name = Some(word()?),
            "--title" => {
                let text = word()?;
                out.request.title.insert("en".to_owned(), text);
            }
            "--url" => out.request.url = Some(word()?),
            "--sha256" => out.sha256 = Some(word()?),
            "--size" => out.size = Some(word()?.parse().map_err(|_| Failure::Usage)?),
            "--of" => out.of = Some(word()?),
            "--max-mb" => out.max_mb = Some(word()?),
            _ => return Err(Failure::Usage),
        }
    }
    Ok(out)
}

fn attacher(max_mb: Option<&str>) -> Result<Attacher, Failure> {
    let mut writer = Attacher::new("eh3d-project", env!("CARGO_PKG_VERSION"));
    if let Some(mb) = max_mb {
        if !writer.limits.apply("attachments.max_mb", mb) {
            return Err(Failure::Usage);
        }
    }
    Ok(writer)
}

fn say(done: &Attached, what: &str) {
    if !done.written {
        println!("nothing new: {} is as it was", done.id);
        return;
    }
    println!("{what} {}", done.id);
    for notice in &done.notices {
        println!("note: {notice}");
    }
}

fn attach(args: &[String]) -> Result<u8, Failure> {
    let [file, source, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let given = options(rest, &["--name", "--title", "--url", "--max-mb", "LINKS"])?;
    let done = attacher(given.max_mb.as_deref())?.attach_file(
        open(file)?,
        Path::new(source),
        &given.request,
        &SaveOptions::default(),
    )?;
    let what = if done.already_there {
        "already stored, added to"
    } else {
        "attached"
    };
    say(&done, what);
    Ok(0)
}

fn attach_link(args: &[String]) -> Result<u8, Failure> {
    let [file, url, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let given = options(
        rest,
        &["--name", "--title", "--sha256", "--size", "--of", "LINKS"],
    )?;
    let (sha256, size) = match (given.sha256, given.of) {
        (Some(sha256), None) => (sha256, given.size),
        (None, Some(path)) if given.size.is_none() => {
            let (sha256, size) = attachments::hash_file(Path::new(&path))?;
            (sha256, Some(size))
        }
        _ => return Err(Failure::Usage),
    };
    let done = attacher(None)?.attach_link(
        open(file)?,
        url,
        &sha256,
        size,
        &given.request,
        &SaveOptions::default(),
    )?;
    say(&done, "linked");
    Ok(0)
}

fn components_of(a: &Attachment) -> String {
    a.components
        .iter()
        .map(|c| {
            if let Some(reference) = &c.reference {
                reference.clone()
            } else if let Some(value) = &c.value {
                format!("value {value}")
            } else if let Some(number) = &c.part_number {
                match &c.manufacturer {
                    Some(maker) => format!("{maker} {number}"),
                    None => format!("part {number}"),
                }
            } else {
                "another kind of link".to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn list_attachments(args: &[String]) -> Result<u8, Failure> {
    let [file, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let given = options(rest, &["--json", "LINKS"])?;
    let mut project = open(file)?;
    let index = attachments::read_index(&mut project)?;
    let asked = &given.request.components;
    let found: Vec<&Attachment> = if asked.is_empty() {
        index.attachments.iter().collect()
    } else {
        // every piece named in the command line is one question; an attachment is listed once
        let mut found: Vec<&Attachment> = Vec::new();
        for link in asked {
            let mut query = ComponentQuery::new();
            if let Some(reference) = &link.reference {
                query = query.reference(reference);
            }
            if let Some(value) = &link.value {
                query = query.value(value);
            }
            if let Some(number) = &link.part_number {
                query = query.part_number(number);
            }
            if let Some(maker) = &link.manufacturer {
                query = query.manufacturer(maker);
            }
            for a in index.for_component(&query) {
                if !found.iter().any(|f| f.sha256 == a.sha256) {
                    found.push(a);
                }
            }
        }
        found
    };
    if given.json {
        let list: Vec<serde_json::Value> = found
            .iter()
            .map(|a| serde_json::to_value(a).unwrap_or_default())
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&list).unwrap_or_default()
        );
        return Ok(0);
    }
    println!("attachments  {}", found.len());
    for a in found {
        let size = a.size.map_or_else(|| "?".to_owned(), |s| s.to_string());
        let place = if a.is_link() { "link" } else { "inside" };
        let danger = match a.danger() {
            Some(d) => format!("  DANGEROUS ({})", d.name()),
            None if a.needs_confirmation() => "  UNKNOWN KIND".to_owned(),
            None => String::new(),
        };
        println!(
            "  {}  {:<8} {size:>11}  {place:<6}  {}{danger}",
            &a.sha256[..16.min(a.sha256.len())],
            a.kind().name(),
            a.file_name()
        );
        if !a.components.is_empty() {
            println!("      for {}", components_of(a));
        }
        if let Some(url) = &a.url {
            println!("      from {url}");
        }
    }
    Ok(0)
}

fn extract_attachment(args: &[String]) -> Result<u8, Failure> {
    let [file, id, out, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let given = options(rest, &["--confirm-dangerous", "--max-mb"])?;
    let limits = attacher(given.max_mb.as_deref())?.limits;
    let consent = if given.confirmed {
        Consent::Given
    } else {
        Consent::NotAsked
    };
    let mut project = open(file)?;
    let written = attachments::extract(&mut project, id, Path::new(out), consent, &limits)?;
    println!("written to {}", written.display());
    Ok(0)
}

fn attachment_links(args: &[String]) -> Result<u8, Failure> {
    let [file, id, action, rest @ ..] = args else {
        return Err(Failure::Usage);
    };
    let links = options(rest, &["LINKS"])?.request.components;
    let (add, remove): (&[ComponentLink], &[ComponentLink]) = match action.as_str() {
        "add" => (&links, &[]),
        "remove" => (&[], &links),
        _ => return Err(Failure::Usage),
    };
    if links.is_empty() {
        return Err(Failure::Usage);
    }
    let done =
        attacher(None)?.set_components(open(file)?, id, add, remove, &SaveOptions::default())?;
    say(&done, "changed");
    Ok(0)
}

fn detach(args: &[String]) -> Result<u8, Failure> {
    let [file, id] = args else {
        return Err(Failure::Usage);
    };
    let done = attacher(None)?.detach(open(file)?, id, &SaveOptions::default())?;
    say(&done, "removed");
    Ok(0)
}
