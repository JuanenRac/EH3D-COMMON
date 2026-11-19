# The client of the EH3D server

`crates/serverclient` (`eh3d-serverclient`) is the one client that the programs of the suite use to talk to EH3D-SERVER: the machine programs, the studio's tools and the screens that are written in Rust. It has no window and no dependency (not even a JSON library), so any program can take it from a sibling folder (`docs/CONSUMING.md`):

```toml
eh3d-serverclient = { version = ">=0.0.1", path = "../EH3D-COMMON/crates/serverclient" }
```

## What it does

| Part | What it gives |
|---|---|
| `Settings` | the seven keys below, read from the text of the machine file or set one by one, with ranges and conservative defaults |
| `Client::login(name, password, app)` | `POST /api/v1/auth/login` with the application the program is (`phoenix-cnc`, `bungard-gui`, `studio`, `hmi`...); gives a `Session` (name, role, app, session id, seconds left) and keeps the token inside |
| `Client::refresh()` | `POST /api/v1/auth/refresh`; a request renews a login that is in the last quarter of its life by itself (`with_auto_refresh(false)` turns it off) |
| `Client::logout()`, `sessions()`, `close_session(id)` | end this login, list the open logins of the caller, close one (the phone that was lost) |
| `Client::health()` | `GET /api/v1/health`, no login, no token sent |
| `Client::request/get/post/put/delete` | any route; the bearer token goes with it; a 4xx or 5xx comes back as a `ClientError` |
| `Client::machine_path("/jobs")` | `/api/v1/machines/<server_machine>/jobs`, with the name made safe for a path |
| `Client::link()` | `Unknown`, `Connected`, `Disconnected` (the last request did not reach the server) or `LoggedOut` (the server answered, but there is no login): the corner of a window |
| `ClientError` | `Config`, `Unreachable { error, attempts }`, `Unauthorized`, `Forbidden`, `Server { status, code, detail }`, `BadReply`; `code()`, `status()`, `needs_login()` and `text(Lang)` |
| `Lang`, `lookup`, `text_for` | the text of every code of the server's catalog in en, es, it, de, fr, pt (Portugal), zh and ja |
| `Secret`, `TokenStore`, `MemoryStore`, `FileStore` | the token that cannot be printed, and where it is kept |
| `Transport`, `TcpTransport`, `sim::SimServer` | how a request travels: plain HTTP/1.1, or a server in memory for tests |
| `Discover` | the hook for mDNS (off, see below) |

### The keys

| Key | Range | Default | Meaning |
|---|---|---|---|
| `server_address` | `host:port`, or empty | empty | the server; empty means there is none and nothing is sent |
| `server_token` | text without spaces | empty | the token of a program (write-only: never shown, never logged) |
| `server_machine` | text | empty | the machine the jobs are for |
| `server_timeout_s` | 1 to 120 | 15 | how long one try may take |
| `server_retries` | 0 to 5 | 2 | how many more tries after a failure on the network |
| `server_mdns` | `on` or `off` | `off` | look for the server on the local network |
| `server_max_answer_mib` | 1 to 256 | 8 | the most MiB read from one answer |

A value out of its range is an error that names the key (and the line, when it comes from a file) and changes nothing. A key that is not one of these is left to the program that reads the file. `Settings::to_text(false)` writes the token as `***`, for a screen of settings; only the program that writes its own file asks for it with `to_text(true)`.

### Retries, time limits and what is never sent twice

Each try has `server_timeout_s` seconds. After a failure the client waits 200 ms, 400 ms, 800 ms... up to 2 s and tries again, up to `server_retries` more times. A connection that could not be made is tried again for any method, because nothing was sent. A time-out or a broken connection is tried again only for `GET`, `HEAD`, `PUT`, `DELETE` and `OPTIONS`, so a `POST` (a job, a login) is never sent twice by the client. A 502, 503 or 504 is tried again for those same methods. When the tries run out the error is `Unreachable { attempts }` and the link is `Disconnected`; the login is kept, because the server did not refuse it.

### The token

The token is a `Secret`: its `Debug` and `Display` write `***`, the bytes are overwritten when it is dropped, and the only way to read it is `expose()`, which only the transport calls. A `Request` and a `Settings` have no `Debug` output of the token, and a `ClientError` never holds the token or the password. A 401 makes the client forget the token (in memory and in the store) and answer `Unauthorized` without sending anything until there is a new login. Where the token is kept between runs is the program's decision: `MemoryStore` (the default, nothing written), `FileStore` (a file only its owner can read, mode 0600 on Unix; on Windows the user's own settings folder), or its own `TokenStore` over the keychain or the Keystore. A store that fails does not fail the login: `store_warning()` says so.

### Errors by code in eight languages

The server answers `{"error": "<code>", "detail": "..."}`. `ClientError::text(lang)` gives the text of the code in the language of the person; a code a newer server has and this table does not is shown as the code itself, so nothing is hidden. The table is `src/errors_table.rs`, generated by `tools/gen_server_errors.py` from `EH3D-SERVER/docs/error-codes.json` (the codes and statuses) and the rows `err_<code>` of `EH3D-STUDIO/src/texts.ts` (the eight texts, so Studio and the programs say the same words). The generator fails when a code has no eight texts. After the server adds a code: add its row to Studio, run `py -3 tools/gen_server_errors.py`, commit the table. `py -3 tools/gen_server_errors.py --check` says whether the table on disk is current, and the test `the_table_is_the_catalog_of_the_server` compares the codes and the statuses with the sibling server (it is skipped when `EH3D-SERVER` is not next to this repository).

### Discovery by mDNS: optional and off

`server_mdns` is `off` by default and the crate has no resolver of its own: with the key off `Client::discover()` asks nothing of the network and answers a `Config` error; with it on and no resolver given it says so; with a `Discover` resolver (given with `with_discovery`) it returns the `host:port` list the resolver found. The server does not announce itself today, so a program that wants discovery needs the announcement first (an open point, below).

### Testing a program that uses it

`sim::SimServer` is the server in memory: it logs in (`admin` / `secret-password`, `viewer` / `viewer-password`, a program token `program-token`), renews, closes sessions, refuses with 401/403/404 and the right code, answers `/api/v1/health`, `/api/v1/echo` and `/api/v1/admin/only`, and fails on demand (`script(vec![Fault::NoConnection, Fault::Timeout, Fault::Status(503, "unavailable")])`). `sim.clock()` and `sim.sleeper()` make time pass without waiting. A program gives it to the client with `Client::new(settings, Box::new(sim.clone()))`. It is a double of the routes the client uses, not the server.

## Migration guide, app by app

The apps are not changed by this crate; each takes these steps in its own round. In every one the keys are read through `Settings::set` (or `from_text`) so the ranges are the same, the old keys keep their names, and the token goes to `Secret`.

**EH3D-PHOENIX-CNC** (`src/server.rs`, `src/settings.rs`, `src/gui/remote.rs`)
1. Add the dependency. Replace `server::Client` (its own HTTP, `call`, `parse_response`, `error_text`) by `eh3d_serverclient::Client`; keep `JobInfo`, `job_request` and `Follow` (they are the program's own).
2. `settings.rs`: keep `server_address`, `server_token`, `server_machine` and `server_restore_outputs` (the last is not of the link); add `server_timeout_s`, `server_retries`, `server_mdns` and `server_max_answer_mib` (a field `Settings` of the crate inside the program's settings, written back with `to_text(false)` for display and the real value for the file).
3. Log in with `login(name, password, "phoenix-cnc")` when there is no token; use `server_token` as a program token otherwise. Show `link()` in the status line and `ClientError::text(lang)` in the dialogs instead of the hand-written table of `error_text`.
4. Delete the program's own `plain_token_warning` and use the crate's.

**EH3D-DRILLPRO and EH3D-ROUTEPRO** (`protocol/src/connection.rs`, `src/server_link.rs`, `src/cli.rs`, `src/runner.rs`)
1. `connection.rs` already has `server_address`, `server_token`, `server_machine` and `server_max_answer_mib`: delegate those keys to `Settings::set` and add the other three.
2. `server_link.rs`: replace `Client::call`, `parse_response`, `check_request_part`, `path_segment` and `plain_token_warning` by the crate's; keep `fleet()` (it reads the answer into `eh3d_fleet::Fleet`) on top of `Client::get("/api/v1/fleet")` and `Reply::body`.
3. The `--server`, `--token` and `--machine` options of the command line become `Settings::set` calls. Application names: `drillpro` and `routepro` (to be added to `auth_extra_apps` of the server until the server knows them).
4. Replace the text of the failures with `text(lang)`; a `Disconnected` link is the "server unreachable" state of the window.

**EH3D-BUNGARD-GUI** (the mode "through the server")
1. The same steps as DRILLPRO; the application is `bungard-gui`, which the server already knows. Keep the node token out of this program: it never sees it.

**EH3D-STUDIO** (TypeScript) and the other web pages
1. The crate is Rust and does not apply. Studio already logs in with `app = "studio"`, renews, keeps the token in `sessionStorage` and reads the same table (`src/texts.ts` is the source of the generated one). What the migration owes is the contract: the test that compares the codes of the server with the texts already exists; keep it and the generator in step.

**EH3D-ANDROID-CONTROL and EH3D-IOS-CONTROL** (Kotlin, Swift)
1. They cannot use the crate. They follow its behaviour: login with `app = "android"` / `"ios"`, renewal at the last quarter of the login, a 401 forgets the token, retries with the same pauses and the same rule for `POST`, the same seven keys with the same ranges where a setting makes sense, and the eight texts from the same table (generate the string files from `error-codes.json` and `texts.ts` as the crate does). Android first, iOS as the port.

**HMI screens** (EH3D-NODE-HMI-GUI, EH3D-NODE-BUNGARD-HMI, C++ on the ESP32-P4)
1. Not Rust: they keep `server_link.hpp` and follow the same contract. Map their keys to these (`server_host` + `server_port` are `server_address`, `token` is `server_token`, `poll_s` stays theirs) and add `server_timeout_s` and `server_retries` with the same ranges. The HMI has no login screen: its token is the token of a program with the scopes it needs. The error texts come from the same table, as `.tsv` locales.

**Nodes** (EH3D-NODE-BUNGARD, EH3D-NODE-BUNGARD-AUX): the server connects to them, not the other way round; they have nothing to migrate.

## Open points

* mDNS: the server does not announce a service yet and the crate has no resolver; both are needed before `server_mdns` can find anything.
* The crate speaks plain HTTP; HTTPS goes through a tunnel or a proxy in front of the server (`plain_token_warning` says when an address deserves a warning).
* A real file lock of the token file on Windows (access control) is not done: `FileStore` relies on the user's settings folder there.
