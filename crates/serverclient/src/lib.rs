//! The client every EH3D program uses to talk to EH3D-SERVER.
//!
//! One crate, no dependency, no window: the studio, the machine programs (PHOENIX-CNC, DRILLPRO, ROUTEPRO...) and the screens of the nodes share it, so a
//! login, a renewal, a refusal or a lost connection means the same everywhere.
//!
//! * [`Settings`]: the keys `server_address`, `server_token`, `server_machine`, `server_timeout_s`, `server_retries`, `server_mdns` and
//!   `server_max_answer_mib`, with their ranges and conservative defaults.
//! * [`Client`]: `login` (`POST /auth/login` with the application), `refresh`, `logout`, `sessions`, `health` and the generic `request`, with retries
//!   (growing pauses, a `POST` is never sent twice), time limits and the [`Link`] state (connected, disconnected, logged out).
//! * [`Secret`] and [`TokenStore`]: the token cannot be printed by mistake and is kept only where the program says.
//! * [`ClientError`]: the refusal of the server by its code, with the text in eight languages ([`Lang`]) taken from a table generated from the error
//!   catalog of the server (`tools/gen_server_errors.py`).
//! * [`Transport`]: how a request travels; [`TcpTransport`] is plain HTTP/1.1, [`sim::SimServer`] is a server in memory for tests.
//! * Discovery by mDNS is optional and OFF (`server_mdns = off`); the crate has no resolver, a program gives one through [`Discover`].

pub mod client;
pub mod errors;
mod errors_table;
pub mod json;
pub mod secret;
pub mod settings;
pub mod sim;
pub mod transport;

pub use client::{
    backoff, Client, Clock, Discover, Health, Link, Session, SessionInfo, Sleeper, SystemClock,
    ThreadSleeper,
};
pub use errors::{all_codes, lookup, text_for, ClientError, ErrorEntry, Lang, TransportError};
pub use json::Json;
pub use secret::{FileStore, MemoryStore, Secret, TokenStore};
pub use settings::{KeySpec, SettingError, Settings, KEYS};
pub use transport::{path_segment, plain_token_warning, Reply, Request, TcpTransport, Transport};
