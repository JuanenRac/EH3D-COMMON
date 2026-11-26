# 0003 - Attachments of the `.eh3d` project

Status: accepted. Implemented in the crate `eh3d-project` (`attachments`) with its command line; the classification of files is the crate `eh3d-filekind` and the windows of the programs show attachments through `eh3d-attachpanel`.

## Context

A project of a design is more than the design: data sheets, manuals, drawings of other programs, spreadsheets, photos and videos of an assembly travel with it. The owner decided that any file may be attached; that macros, programs, scripts and compressed archives are accepted, marked dangerous and taken out or opened only after a confirmation; that nothing is ever run or downloaded by itself; that a data sheet is linked to a part (value, reference, or manufacturer and part number) and stored once however many pieces use it; that a file is inside by default and may be only a link (address and hash); and that one attachment may be up to 1 GB, configurable, with the whole project under 4 GB.

## Decisions

1. **One shared branch, `attachments/`.** Attachments are used by several programs (the board and the assembly machine look up the same data sheet), so they belong to the project as a whole, like `common` and `docs/project`: any program writes the branch, through this module.
2. **Content addressed.** The identifier of an attachment is the SHA-256 of its content, and its bytes are stored under it. The same content is therefore stored once, and no name chosen by a person or found in a file is ever a path inside the project.
3. **One index** (`index.json`, canonical JSON) with the safe name, size, kind, danger, storage, address, titles and the components. The components are part of the attachment and not of a program's branch, so attaching a data sheet changes no branch of a program and the link survives whichever program wrote the design. A component is named by its designator, its value or its part number (with or without manufacturer), not by an internal identifier of one program.
4. **Large files in parts of 32 MiB.** A reader of format 1.0.0 refuses an entry over 256 MiB outside a few branches, so a gigabyte in one entry would make older programs refuse the whole project. In parts, every program that exists keeps opening it, keeps the branch byte for byte and verifies it, with no change of the format version. It also bounds the memory of a reader to one part. A file up to 32 MiB is one entry; a larger one is its parts in order.
5. **A streaming write** inside the crate: the file is hashed in blocks, its parts are added to the container writer from the disk, and what the writer hashed is compared with what was announced before the project is replaced. Everything else follows the rules of saving a branch (seal, signatures, atomic replacement, audit log).
6. **Danger is decided by name and content, twice.** It is recorded when the file is attached and judged again when it is taken out, because a record can be written by anyone. There is one set of lists, in `eh3d-filekind`, used by the container, the viewer and the panel; a kind that none of them knows asks like a dangerous one.
7. **Taking out is careful:** safe name inside the chosen folder, never over a file, not executable, marked as foreign on Windows, checked against its hash, removed when refused or incomplete. The crate opens and runs nothing.
8. **A link is data.** Only `https` and `http` addresses without credentials, with the hash the file must have. Downloading is something a person does outside.
9. **What does not change is not compressed again.** A save copies the compressed bytes and the CRC of every entry that stays, so an attachment is compressed only the first time. The new file is checked entry by entry (SHA-256) before it replaces the old one, so a save still reads every entry once.

## Options left out

* **Attachments inside the documentation branches.** `docs/<program>` is owned by one program, the index of documents takes only five kinds of file and needs a title for each, and a save of a branch holds all its files in memory.
* **A new format version of the container with ZIP64 and bigger entries.** It would shut out every program built so far for no gain: the total limit stays 4 GB.
* **One entry for each file, with its name.** It would break old readers for large files and would put names from outside in the paths.
* **Storing the link of a component in the branch of each design.** The same data sheet would be linked and stored several times.

## Pending, with the reason

* The windows of the programs (attach from a menu, open from a part, the confirmation) and the viewer inside the programs: separate work.
* Compressed archives are not looked into, and the macro check reads the list of entries of an office file, not its content.
