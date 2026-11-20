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
| `LinkMetrics`, `Client::metrics()` | requests, tries, retries, answers, 5xx answers, losses and disconnects, and the latency of the answers (last, smallest, largest, mean, median and 95th percentile of the latest 128) |
| `OfflineQueue`, `Client::send_or_queue`, `Client::flush` | the `PUT` and `DELETE` requests that could not be sent wait (bounded) and go out in order when the server is back |
| `ContractCache`, `Client::refresh_contract` | the OpenAPI contract of the server, to know which routes it has withdrawn |
| `mdns::MdnsResolver` (feature `mdns`) | a resolver of its own for `_eh3d._tcp`, off by default |

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

`server_mdns` is `off` by default. With the key off `Client::discover()` asks nothing of the network and answers a `Config` error; with it on and no resolver given it says so; with a `Discover` resolver (given with `with_discovery`) it returns the `host:port` list the resolver found. Two things stay true whatever resolver is used: what it finds is a *candidate* that the program shows to the person (nothing connects by itself), and any device of the network can answer, so a name that answered is not a proof of anything.

**The resolver of this crate (feature `mdns`, off by default).** `eh3d-serverclient = { ..., features = ["mdns"] }` adds `mdns::MdnsResolver`, a small multicast DNS client of its own over UDP with no dependency; `Client::with_mdns()` gives it to a client (the key `server_mdns` still decides whether it is used). It asks who offers `_eh3d._tcp.local` (a question of type PTR from a port that is not 5353, so the servers answer to that port directly and the program does not join the multicast group) and reads the answers. It only reports what the network said: a server needs a pointer record to the service **and** a service record with a port; the address is the one of an address record when one came (IPv4 first; an IPv6 link-local address is skipped because it cannot be used without its interface), else the name of the host that the service record gives (`cm5.local:18200`), never an invented address. The limits are fixed: the search lasts at most 10 s (the client asks for `server_timeout_s`, at most 10), the question is repeated every second, at most 64 packets are read, a packet is at most 9000 bytes and 128 records are read from it, a name is at most 255 bytes, a compression pointer is followed at most 16 times, at most 32 services and 64 addresses are kept, and a record with a time to live of 0 (a goodbye) is ignored. `mdns::parse_response` and `mdns::build_query` are public to test it; `mdns::build_response` builds an answer for the tests of a program or for a simulator of the server.

**How the server announces itself (the card for EH3D-SERVER; the announcement is done there, in its own round).**

| Part | Value |
|---|---|
| Service type | `_eh3d._tcp` (domain `local`) |
| Instance name | `EH3D Server on <host name>` |
| Port | the one of the API (18200 by default) |
| Host name | `<host name>.local`, with its address records (A, and AAAA when it has a global address) in the same packet |
| TXT | `path=/api/v1`, `ver=<version of the server>`, `api=<api revision>`; never a token, a user, a machine list or anything that identifies the owner |
| Time to live | 120 s for the service, 120 s for the address records; a goodbye (TTL 0) when the server stops |
| Answers | multicast to the group, and unicast to the sender when the question came from a port that is not 5353 (what this client sends) |

On a CM5 with Avahi the same announcement is a file `/etc/avahi/services/eh3d.service`:

```xml
<?xml version="1.0" standalone='no'?>
<!DOCTYPE service-group SYSTEM "avahi-service.dtd">
<service-group>
  <name replace-wildcards="yes">EH3D Server on %h</name>
  <service>
    <type>_eh3d._tcp</type>
    <port>18200</port>
    <txt-record>path=/api/v1</txt-record>
  </service>
</service-group>
```

The server should announce only while its setting for announcing is on (off by default, as `server_mdns` is for the programs): a server on a shared network is not meant to be found by everyone.

### Metrics of the link

`Client::metrics()` is a `LinkMetrics` that every call of the client feeds: `requests()` (calls that went to the transport), `attempts()` (tries, the first and the repeated ones), `retries()`, `replies()` (tries that got an answer, of any status), `server_errors()` (answers of 500 or more), `failures()` (requests that ended without an answer, after all their tries) and `disconnects()` (times the link went from reachable or unknown to unreachable; a second failure while it is already down is not another one). The latency is the time the transport took to give an answer: `last_latency()`, `min_latency()`, `max_latency()`, `mean_latency()` (of all the answers) and `percentile(50)` / `percentile(95)` (nearest rank, of the latest 128). `to_text()` writes `key = value` lines (times in microseconds, `-` where there is nothing yet) for a log or a page of diagnostics; it never holds a token, a body or an address. `reset_metrics()` starts again.

### Requests that wait while the server is away

`Client::send_or_queue(&mut queue, method, path, body)` sends the request and, if the server cannot be reached and the request can wait, keeps it in an `OfflineQueue` instead of failing: the answer is `Outcome::Done(reply)`, `Outcome::Queued` or `Outcome::NotQueued(reason)`; any other error (a refusal, no login, no address) comes back as it is. Only `PUT` and `DELETE` wait: they can be sent again without harm. A `POST` (a job, a login) is never kept, because sending it later could do the work twice, and a `GET` has nothing to replay. The queue is bounded by `QueueLimits` (64 requests, 64 KiB of body each, 1 MiB of bodies together and 3600 s of age by default) and refuses with `Rejected::{NotRepeatable, BadPath, Full, BodyTooLarge, TotalTooLarge}`; two requests for the same path are one (the latest wins and goes to the end). Nothing is written to disk and no token is kept in the queue.

`Client::flush(&mut queue)` sends what waits, oldest first, once the server is back. It drops what is older than `max_age_s`, stops at the first request for which the server cannot be reached or that needs a login (that one and the rest stay in the queue, in order) and drops a request the server refuses, reporting it in `FlushReport { sent, refused, expired, stopped, remaining }`.

### The contract of the server and the routes it withdraws

`Client::fetch_contract(path)` reads an OpenAPI document (JSON, `contract::DEFAULT_CONTRACT_PATH` = `/api/v1/openapi.json` unless the server serves it elsewhere; the document the server ships is `docs/openapi.json`) and turns it into a `Contract`: the `METHOD /path` pairs, with the paths joined to the first relative `servers` URL (`/api/v1`) as the client sends them. A segment written `{name}` matches any one segment, and a query string is ignored. A `ContractCache` holds the newest contract and the routes that an earlier one had and the newest does not (retired); `Client::refresh_contract(&mut cache, path)` reads the contract and updates the cache, and `cache.status(method, path)` says `Known`, `Retired` or `Unknown` before a program relies on a route. The first contract read is the baseline and retires nothing. `to_text()` and `from_text()` keep the cache in a file between runs; a damaged file is refused whole. A document that is not a contract (no paths, no routes, more than 4096 routes) is a `BadReply` and leaves the cache as it was. The server does not serve its contract on a route today (an open point, below), so a program that has the file reads it with `Contract::from_openapi`.

### The settings panel every machine program shows (`eh3d-view`)

`eh3d_view::server_panel` is the same logic for every program, with no window in it: `ServerForm` (the six boxes `server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries` and `server_mdns`, checked against the ranges of `Settings` and the shape `host:port`; the saved token is never copied into the form, a new one is typed in a box that is write-only, and the saved one can be removed), `LinkTracker` and `LinkState` (connected, disconnected, session expired, no session, not checked yet, no server set), `LoginDialog` (the password is wiped from memory as soon as the call is made) and `ServerPanel`, which keeps them together and does what a window asks through `handle(Action, &mut Client)`; applying gives the new `Settings` for the program to save and to make a new client with. `error_text(lang, &error)` gives any failure of the client in the language of the person: the text of the code of the server, or the sentence of the kind of failure (with the number of tries when the server could not be reached). The words are `eh3d_view::server_texts::ServerText`, 38 texts in en, es, it, de, fr, pt (Portugal), zh and ja (the other languages of `eh3d-view` use English, and the variants their main language, until they are translated). With the feature `gui` of `eh3d-view`, `server_gui::show(ui, &mut panel, lang)` draws the panel with egui 0.37 and gives back the `Action` chosen. The model is tested against `sim::SimServer`; the egui code is checked apart in the CI and has not been run in a window.

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

* mDNS: the resolver of the crate exists (feature `mdns`) but the server does not announce a service yet (the card above says how); until it does, `server_mdns` finds nothing.
* The server does not serve its OpenAPI contract on a route yet; `fetch_contract` expects `/api/v1/openapi.json` and answers the 404 as an error until it does.
* The crate speaks plain HTTP; HTTPS goes through a tunnel or a proxy in front of the server (`plain_token_warning` says when an address deserves a warning).
* A real file lock of the token file on Windows (access control) is not done: `FileStore` relies on the user's settings folder there.
