# 0004. Writes that take nothing away, and signature keys under a password

Status: accepted.

## Context

Several programs of the suite open the same `.eh3d` project. A write replaces the file in one step, so it was never half written, but a program that had read the file earlier could replace what another saved meanwhile. Signing put a new signature over an old one without a word. The secret key of a signature was a file with its seed in clear, protected only by the permissions of the file.

## Decision

1. **Compare content, not time.** The mark of a project is a hash of its integrity record. A write is refused (`EH3D-PKG-025`) when the file no longer has the mark that was read. Dates and sizes of files are not trusted: a touch is not a change and a change within the same second is one.
2. **Check at the write, remember at the window.** The container asks just before it renames the new file over the old one. A window keeps the mark of what it showed and the shared functions compare it before writing. The writes of the running program are remembered as links from one mark to the next, so its own window and panel do not refuse each other.
3. **No lock file.** A lock left behind by a program that stopped is worse than the instant that stays uncovered between the last check and the rename, and other tools would not honour it.
4. **Refuse and offer, never merge.** The program offers to read again or to save under another name.
5. **Keys under a password** in a text format of its own, versioned by its first line: Argon2id (64 MiB, 3 passes, 1 lane by default, the cost written in the file) and XChaCha20-Poly1305 with the other lines as associated data. Pure Rust crates of the RustCrypto project at fixed versions (`argon2` 0.6.0, `chacha20poly1305` 0.11.0); `rpassword` 7.5.5 asks at a terminal. The old files stay readable and a program offers to protect them. A new password has at least 12 characters (it was 8); a key already under a shorter one keeps opening, because the length is asked only where a password is set.
6. **Replacing a signature needs an agreement** (`EH3D-PKG-026`), given by calling another function, so a program cannot do it by leaving a flag at its default.
7. **Only the path of the key is remembered**, in the settings folder of the user.

## Consequences

* A program that held a `Project` while another handle of its own wrote must use the returned one: the stale one is refused.
* A program that shows a question before writing asks `suiteview::check_unchanged` again after the answer, because the person may take long.
* A forgotten password cannot be recovered: a new key is made and what is to be trusted is signed again.
* A program that signs again with another key must ask the person first and call the function that replaces.
