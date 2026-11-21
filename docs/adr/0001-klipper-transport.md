# 0001 - How the Klipper controller reaches a real Moonraker

Status: option A is implemented (`eh3d_controller::httprpc`); option B (a WebSocket, with the notifications Moonraker pushes) is not written and stays pending.

## Context

Klipper is driven through Moonraker, which answers the same JSON-RPC 2.0 methods over a WebSocket (`/websocket`) and, one request at a time, over HTTP (`POST /printer/gcode/script`, `GET /printer/objects/query`, `POST /printer/emergency_stop`, and the rest, named after the methods). `eh3d_controller::klipper::KlipperController` speaks the methods and is written against the trait `JsonRpcTransport`: `send(&str)` puts one JSON text out, `recv(Duration)` gives the next text that came in or `None`. Before option A the crate had no network code and no dependency for the network: the controller was tried only against `klippersim::KlipperDevice` behind `klippersim::RpcLink`, a cable that spoils the messages by a fixed seed.

Until option A existed, `Family::is_driven()` did not list Klipper because no transport opened a socket. It does now.

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

Option A is implemented as `httprpc::HttpJsonRpc`, with no new dependency. The route is the one JSON-RPC route of Moonraker, `POST /server/jsonrpc`, whose body is the JSON-RPC request and whose answer is the JSON-RPC response: the controller's messages go out unchanged and nothing has to be mapped to the per-method routes (`/printer/gcode/script`, `/printer/objects/query`, `/printer/emergency_stop`, ...) that the first draft of this text named, which would need an envelope rebuilt on each side. That the route exists and answers this way is from the documentation of Moonraker and has not been tried against a real one (see Evidence). Every message is one request on a connection of its own (`Connection: close`) made by a short-lived thread, and the answers come back through a queue read by `recv`; so requirements 3 and 4 hold (the probe and the emergency stop never wait behind the line) and requirement 5 costs nothing (there is no connection to reopen after `printer.firmware_restart`). The credential is the `X-Api-Key` header, set only when configured and never shown. `Family::Klipper.is_driven()` is `true`.

What option A still does not give, and stays pending for option B behind a feature: the notifications Moonraker pushes (`notify_klippy_shutdown` and the rest: a shutdown is noticed from `webhooks.state` in the status, and a notification that happens to arrive as the body of an answer is passed on), TLS, and one connection for everything.

The settings (times, limits, the address and the switch for addresses outside the local network) are keys of the configuration file with a range each, listed in `docs/CONTROLLERS.md`. The failures have the numbers 209 to 219 and texts in the eight languages.

## Tests of the real transport

All against a fake Moonraker on `127.0.0.1` in `crates/controller/tests/httprpc_link.rs` (a thread with a `TcpListener`), with fixed seeds; none needs a real printer. The seven the first draft listed, each with its test:

* a request is answered, and a request with no answer in the time gives `None`, not an error: `a_request_is_answered_and_a_request_with_no_answer_in_the_time_gives_none_not_an_error`;
* the answer to a probe arrives while the answer to the line is missing (two connections for A): `the_answer_to_a_probe_arrives_while_the_answer_to_the_line_is_missing_on_a_second_connection`;
* a closed connection is `Error::Link` (and a move that lost its link is stopped); a connection that Klipper closes during `printer.firmware_restart` is reopened and the next question is answered: `a_closed_connection_is_a_link_error_with_its_number_and_a_move_that_lost_its_link_is_stopped` and `a_connection_that_closes_during_the_firmware_restart_is_reopened_and_the_next_question_is_answered`;
* the credential is sent in the header and never appears in an error text or in a debug text: `the_credential_is_sent_in_the_header_and_never_appears_in_an_error_a_debug_text_or_a_setting_error`;
* a message cut into pieces by the socket is read whole (a fragmented frame is B): `a_message_cut_into_pieces_is_read_whole_with_a_length_with_chunks_and_one_byte_at_a_time`;
* the notification `notify_klippy_shutdown` halts the controller (B, where it is pushed): over A the same notification, passed on as an answer, and a status that says `shutdown` halt it: `a_notification_that_is_passed_on_halts_the_controller_and_so_does_a_status_that_says_shutdown`; the pushed notification itself is pending with B;
* `printer.emergency_stop` goes out while another thread is blocked in a read: `the_emergency_stop_goes_out_while_another_thread_is_blocked_in_a_read`.

And the ones option A added: redirections never followed, error statuses, a body of any size cut by the limit, an answer that is not JSON, hostile headers and framing, slow and silent answers, nothing listening, the address checks, the settings and their ranges, the places that run out, a dropped transport, the request that is not JSON, a whole controller over the socket, and the numbers with their texts.

## Evidence

Everything above the transport, and the transport itself, is `simulator` evidence (a fake Moonraker). A real Klipper has to be tried for: that `POST /server/jsonrpc` answers the messages of the controller as the fake one does (the exact route and the status it gives to a JSON-RPC error), the exact words of its errors (the classification in `klipper::classify` is by the words documented for Klipper and Moonraker and may need more phrases), the behaviour of `printer.print.cancel` and `PAUSE` on a machine without a `[pause_resume]` section, the time a line that moves blocks on the planner (which decides the length of the liveness probe), and `printer.firmware_restart` after an emergency stop.
