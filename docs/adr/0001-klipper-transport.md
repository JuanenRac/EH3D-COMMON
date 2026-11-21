# 0001 - How the Klipper controller reaches a real Moonraker

Status: accepted for the controller and the simulator; the real transport is not written.

## Context

Klipper is driven through Moonraker, which answers the same JSON-RPC 2.0 methods over a WebSocket (`/websocket`) and, one request at a time, over HTTP (`POST /printer/gcode/script`, `GET /printer/objects/query`, `POST /printer/emergency_stop`, and the rest, named after the methods). `eh3d_controller::klipper::KlipperController` speaks the methods and is written against the trait `JsonRpcTransport`: `send(&str)` puts one JSON text out, `recv(Duration)` gives the next text that came in or `None`. The crate has no network code and no new dependency for the network: a controller that nobody can connect is tried against `klippersim::KlipperDevice` behind `klippersim::RpcLink`, a cable that spoils the messages by a fixed seed.

`Family::is_driven()` does not list Klipper because no transport opens a socket yet.

## What the controller needs from a transport

1. One message out is one JSON text; one message in is one JSON text. Messages that are not the answer to the request in flight (the notifications of Moonraker such as `notify_status_update`, `notify_klippy_shutdown`, `notify_klippy_disconnected`) are passed on, in order; the controller reads the ones that mean a shutdown and ignores the rest.
2. A read that finds nothing in the time given says `None`; it does not fail. A closed or reset link is an `io::Error`, which the controller reports as `Error::Link`.
3. While a line that moves waits for its answer, the controller sends a liveness probe (`printer.info`) on the same transport. Moonraker answers it out of the queue of G-code, so the transport must deliver the answer to the probe while the answer to the line is still missing. Over a WebSocket this is natural. Over HTTP it needs a second connection.
4. `printer.emergency_stop` must be delivered without waiting for anything else.
5. The transport owns the connection, the reconnection after `printer.firmware_restart` (Klipper closes the link while it restarts) and the credential. It never prints the credential.

## Options

A. **HTTP/1.1 over `std::net::TcpStream`, no new crate.** Plain HTTP only (Moonraker listens on port 7125 without TLS by default, on a trusted network). Each request is a POST or GET to the endpoint named after the method; the JSON-RPC envelope is rebuilt around the body. A `TcpStream` pair (one for the line that waits, one for the probes and the stop) gives requirement 3 and 4. The credential is the `X-Api-Key` header. No TLS, no WebSocket notifications (the shutdown is noticed from `webhooks.state` in the status, which the controller already reads). The `serverclient` crate of this workspace is written the same way, with no dependency.
B. **WebSocket with the `tungstenite` crate (or `tokio-tungstenite`).** Gives the notifications and one connection for everything, and TLS with `rustls`. It adds about ten crates to the lock of every program that depends on this crate unless it is behind a feature; it is not a dependency of this workspace today, so it was not added.
C. **A WebSocket written by hand over `std::net`** (handshake with SHA-1 and Base64, framing, masking). Possible, about 300 lines, and it needs SHA-1 and Base64 (the workspace has `sha2` only). Not recommended: more to get wrong than option B.

## Decision

The controller, the simulator and the tests are done. The real transport is left for the owner to choose between A (no dependency, no TLS, no push notifications) and B behind a feature `moonraker-ws` (full protocol, new dependencies). Until one exists `Family::Klipper.is_driven()` stays `false`, and a program lists Klipper as "not driven yet".

## Tests that are missing for the real transport

All against a fake Moonraker on `127.0.0.1` in the test itself (a thread with a `TcpListener`), with fixed seeds; none needs a real printer:

* a request is answered, and a request with no answer in the time gives `None`, not an error;
* the answer to a probe arrives while the answer to the line is missing (two connections for A, one for B);
* a closed connection is `Error::Link`; a connection that Klipper closes during `printer.firmware_restart` is reopened and the next question is answered;
* the credential is sent in the header and never appears in an error text or in a log line;
* a message cut into pieces by the socket (A) or by a fragmented frame (B) is read whole;
* the notification `notify_klippy_shutdown` halts the controller (B);
* `printer.emergency_stop` goes out while another thread is blocked in a read.

## Evidence

Everything above the transport is `simulator` evidence. A real Klipper has to be tried for: the exact words of its errors (the classification in `klipper::classify` is by the words documented for Klipper and Moonraker and may need more phrases), the behaviour of `printer.print.cancel` and `PAUSE` on a machine without a `[pause_resume]` section, the time a line that moves blocks on the planner (which decides the length of the liveness probe), and `printer.firmware_restart` after an emergency stop.
