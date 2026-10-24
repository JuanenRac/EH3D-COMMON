# The firmware base of the nodes

The EH3D nodes are ESP32-S3 devices (today EH3D-NODE-BUNGARD; later ones for other machines) and they all need the same things around what they control: a settings document in flash that a web panel edits, the network (Ethernet or Wi-Fi, a set-up access point), users and sessions, an update from the panel with rollback, a log, and protection against running out of memory. That part is the **firmware base**. It lives here once, in `firmware_base/`, and each node project keeps its own copy in its own tree, so each node builds on its own and nothing is shared at build time.

## What is shared

About fifty files, listed in `firmware_base/manifest.json`:

- **`core/`** (no hardware, tested on a computer): the settings every node has (`node_settings.hpp`, `config_common.hpp`), users, roles, sessions and the throttle on wrong passwords (`auth.hpp`), the network plan (`netplan.hpp`, `net_text.hpp`, `node_id.hpp`), the pins of the ESP32-S3 boards (`board_s3.hpp`), the JSON reader and writer (`json.hpp`), Improv Wi-Fi over serial (`improv.hpp`), the schedule of retries, the guard of tasks and memory (`node_guard.hpp`, `task_health.hpp`, `memory_budget.hpp`), the panel's access policy (`web_policy.hpp`, `admin_sources.hpp`), the clock (`civil_time.hpp`), version numbers (`semver.hpp`) and the loading of what is stored (`stored_load.hpp`).
- **`main/`** (ESP-IDF): the Ethernet board (`board_ethernet.*`), the network (`network.*`), the settings store, users and the setup code in flash (`node_store.*`), the shared part of the panel's API (`api_shared.*`), the certificate of the HTTPS panel (`tls_cert.*`), the clock (`clock_sync.*`), entropy, the log buffer, the external-RAM allocator, the task report, Improv over USB and the guard that restarts a node in trouble.
- **`tools/`**: `pack_panel.py` (packs the web panel into the firmware) and `check_sdkconfig.py` (lists every line of `sdkconfig.defaults` that the build did not take).
- **`tests/`**: the host tests of all of the above.

## What stays in the node

Everything that says what the node controls: the `machine` section of its settings, its link and hardware code, the routes of its panel that are about the machine, its panel pages, its `sdkconfig` profile, its docs. A file that says something else in one project than in the others is not shared.

## The tool

`tools/sync_firmware_base.py` keeps the copies identical. The name of the project appears in a shared file in up to four forms, each written in the base as a placeholder: `EH3D-NODE-BUNGARD` (`@PROJECT@`), `eh3d-node-bungard` (`@project-@`), `eh3d_node_bungard` (`@project_@`) and, only where a file says it by hand, `bungard` (`@kind@`).

```bash
python -I tools/sync_firmware_base.py check     # exit 1 when a copy differs from the base; CI runs it
python -I tools/sync_firmware_base.py sync      # write the base into every node that uses each file
python -I tools/sync_firmware_base.py import EH3D-NODE-BUNGARD   # (re)make the base from the files of a node
python -I tools/sync_firmware_base.py status    # which file goes where
```

`check` compares each copy with the base after putting the project's own names back; a copy that was edited in one node only is reported as a drift. A node whose folder is not next to this repository (as in CI, which has this repository alone) is skipped.

## The normal way to change the base

1. Fix or improve the file in the node where you found the need, and test it there.
2. `python tools/sync_firmware_base.py import <that node>` makes it the base.
3. `python tools/sync_firmware_base.py sync` writes it into every other node.
4. Build and run the host tests of each node.

## Adding a node

1. Create the node project with the same layout (`core/`, `main/`, `tools/`, `tests/`, `eh3d.project.json`).
2. Add its four names to `PROJECTS` in the tool.
3. Run `sync` to bring in the shared files, then write what is its own.
4. Add it to `nodos_eh3d.json` of the visual compiler, if you use it.

## State

Today there is one node, so the base is the copy of EH3D-NODE-BUNGARD with placeholders. It starts to pay for itself when the second node exists. Nothing in the base has run on a board yet.
