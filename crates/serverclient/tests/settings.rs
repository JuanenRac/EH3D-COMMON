//! The keys of the link: ranges, defaults, the file text and the token that is never shown.

use eh3d_serverclient::json::{quote, Json};
use eh3d_serverclient::transport::{check_request_part, parse_response};
use eh3d_serverclient::{
    plain_token_warning, FileStore, Lang, Secret, Settings, TokenStore, TransportError, KEYS,
};

#[test]
fn the_defaults_are_conservative() {
    let s = Settings::default();
    assert_eq!(s.server_address, "");
    assert!(s.server_token.is_empty());
    assert_eq!(s.server_machine, "");
    assert_eq!(s.server_timeout_s, 15);
    assert_eq!(s.server_retries, 2);
    assert!(
        !s.server_mdns,
        "discovery is off unless the owner turns it on"
    );
    assert_eq!(s.server_max_answer_mib, 8);
    assert!(
        !s.server_token_fixed,
        "a 401 forgets the token unless the owner says it is fixed"
    );
    assert_eq!(s.server_error_body_bytes, 0, "no body of a refusal is kept");
    assert!(!s.is_configured());
}

#[test]
fn every_key_has_a_default_that_the_settings_start_with() {
    assert_eq!(KEYS.len(), 9);
    let text = Settings::default().to_text(true);
    for k in KEYS {
        let line = text
            .lines()
            .find(|l| l.starts_with(&format!("{} =", k.key)))
            .unwrap_or_else(|| panic!("{} is not written", k.key));
        let value = line.split_once('=').unwrap().1.trim();
        assert_eq!(value, k.default, "{}", k.key);
    }
}

#[test]
fn ranges_are_enforced_at_both_ends() {
    let mut s = Settings::default();
    for (key, ok, bad) in [
        ("server_timeout_s", ["1", "120"], ["0", "121"]),
        ("server_retries", ["0", "5"], ["-1", "6"]),
        ("server_max_answer_mib", ["1", "256"], ["0", "257"]),
        ("server_error_body_bytes", ["0", "4096"], ["-1", "4097"]),
    ] {
        for v in ok {
            assert_eq!(s.set(key, v), Ok(true), "{key}={v}");
        }
        for v in bad {
            let before = s.clone();
            assert!(s.set(key, v).is_err(), "{key}={v}");
            assert_eq!(s, before, "a refused value changes nothing");
        }
    }
    assert!(s.set("server_timeout_s", "fast").is_err());
    assert!(s.set("server_mdns", "maybe").is_err());
    for on in ["on", "ON", "true", "yes", "1"] {
        s.set("server_mdns", on).unwrap();
        assert!(s.server_mdns);
    }
    for off in ["off", "false", "no", "0"] {
        s.set("server_mdns", off).unwrap();
        assert!(!s.server_mdns);
    }
}

#[test]
fn an_unknown_key_is_not_ours() {
    let mut s = Settings::default();
    assert_eq!(s.set("jog_max_mm", "10"), Ok(false));
    assert_eq!(s.set("SERVER_MACHINE", " bungard "), Ok(true));
    assert_eq!(s.server_machine, "bungard");
}

#[test]
fn an_address_or_token_with_a_space_or_a_line_break_is_refused() {
    let mut s = Settings::default();
    assert!(s.set("server_address", "a b:1").is_err());
    assert!(s.set("server_token", "abc\u{7}def").is_err());
    assert!(s.set("server_token", "abc def").is_err());
}

#[test]
fn the_file_text_is_read_with_comments_and_the_other_keys_of_the_program_skipped() {
    let text = "# EH3D machine connection\nconnection = server\nserver_address = 192.168.0.180:18200  # the CM5\n; another comment\nserver_token = tok-123\nserver_machine = bungard\nserver_timeout_s = 30\nserver_retries = 4\nserver_mdns = on\njog_max_mm = 20\n";
    let s = Settings::from_text(text).unwrap();
    assert_eq!(s.server_address, "192.168.0.180:18200");
    assert_eq!(s.server_token.expose(), "tok-123");
    assert_eq!(s.server_machine, "bungard");
    assert_eq!(s.server_timeout_s, 30);
    assert_eq!(s.server_retries, 4);
    assert!(s.server_mdns);
    assert!(s.is_configured());
}

#[test]
fn a_bad_value_names_its_line() {
    let e = Settings::from_text("server_machine = a\n\nserver_retries = 9\n").unwrap_err();
    assert!(e.contains("line 3"), "{e}");
    assert!(e.contains("server_retries"), "{e}");
}

#[test]
fn the_token_is_never_printed() {
    let mut s = Settings::default();
    s.set("server_token", "very-secret-token").unwrap();
    assert!(!format!("{s:?}").contains("very-secret-token"));
    assert!(!format!("{:?}", s.server_token).contains("very-secret-token"));
    assert_eq!(s.server_token.to_string(), "***");
    assert!(!s.to_text(false).contains("very-secret-token"));
    assert!(s.to_text(false).contains("server_token = ***"));
    // the program that writes its own file may ask for it
    assert!(s.to_text(true).contains("very-secret-token"));
    // and a file text written with the secret reads back the same
    assert_eq!(Settings::from_text(&s.to_text(true)).unwrap(), s);
}

#[test]
fn the_file_store_keeps_the_token_in_a_file_and_clears_it() {
    let dir = std::env::temp_dir().join(format!("eh3d-serverclient-test-{}", std::process::id()));
    let path = dir.join("sub").join("token");
    let store = FileStore::new(&path);
    assert!(store.load().is_none());
    store.save(&Secret::new("tok-abc")).unwrap();
    assert_eq!(store.load().unwrap().expose(), "tok-abc");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "only the owner may read it");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(store.load().is_none(), "a file open to others is refused");
    }
    store.clear();
    assert!(store.load().is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn languages_are_read_from_tags() {
    assert_eq!(Lang::from_tag("pt-PT"), Some(Lang::Pt));
    assert_eq!(Lang::from_tag("zh_CN"), Some(Lang::Zh));
    assert_eq!(Lang::from_tag(" ES "), Some(Lang::Es));
    assert_eq!(Lang::from_tag("ja"), Some(Lang::Ja));
    assert_eq!(Lang::from_tag("xx"), None);
    assert_eq!(Lang::ALL.len(), 8);
}

#[test]
fn the_plain_token_warning_is_for_networks_that_are_not_the_owners() {
    for quiet in [
        "",
        "127.0.0.1:18200",
        "localhost:18200",
        "192.168.0.180:18200",
        "10.1.2.3:1",
        "172.20.0.1:1",
        "[::1]:18200",
        "cm5.local:18200",
        "https://eh3d.example.org",
    ] {
        assert!(!plain_token_warning(quiet), "{quiet}");
    }
    for warn in ["8.8.8.8:18200", "eh3d.example.org:18200", "172.32.0.1:1"] {
        assert!(plain_token_warning(warn), "{warn}");
    }
}

#[test]
fn request_parts_refuse_what_could_split_a_request() {
    assert!(check_request_part("p", "/api/v1/x").is_ok());
    assert!(check_request_part("p", "/a b").is_err());
    assert!(check_request_part("p", "/a\r\nX: 1").is_err());
    assert!(check_request_part("p", "/\u{f1}").is_err());
}

#[test]
fn http_answers_are_read_plain_and_chunked() {
    let plain = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\":1}";
    let r = parse_response(plain).unwrap();
    assert_eq!((r.status, r.body.as_str()), (200, "{\"a\":1}"));
    let chunked = b"HTTP/1.1 401 Unauthorized\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"er\r\n9\r\nror\":\"x\"}\r\n0\r\n\r\n";
    let r = parse_response(chunked).unwrap();
    assert_eq!(r.status, 401);
    assert_eq!(r.body, "{\"error\":\"x\"}");
    assert!(matches!(
        parse_response(b"HTTP/1.1 200 OK\r\n"),
        Err(TransportError::Io(_))
    ));
    assert!(
        parse_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nffff\r\nab").is_err()
    );
}

#[test]
fn json_reads_what_the_server_writes() {
    let j = Json::parse(
        " {\"token\":\"a\\u00e9\\n\",\"n\":[1,2.5,-3],\"b\":true,\"z\":null,\"o\":{}} ",
    )
    .unwrap();
    assert_eq!(j.get("token").and_then(Json::as_str), Some("a\u{e9}\n"));
    assert_eq!(j.get("b").and_then(Json::as_bool), Some(true));
    assert_eq!(
        j.get("n").and_then(Json::as_array).map(<[Json]>::len),
        Some(3)
    );
    assert_eq!(j.get("z"), Some(&Json::Null));
    assert_eq!(Json::Num(3.0).as_u64(), Some(3));
    assert_eq!(Json::Num(-1.0).as_u64(), None);
    assert_eq!(Json::Num(1.5).as_u64(), None);
    assert!(Json::parse("{\"a\":}").is_err());
    assert!(Json::parse("{\"a\":1} x").is_err());
    assert!(Json::parse("\"open").is_err());
    assert!(Json::parse(&"[".repeat(200)).is_err());
    // an astral character comes through a surrogate pair
    assert_eq!(
        Json::parse("\"\\ud83d\\ude00\"").unwrap().as_str(),
        Some("\u{1f600}")
    );
}

#[test]
fn quote_round_trips_through_the_reader() {
    for text in [
        "plain",
        "a\"b\\c",
        "line\nbreak\ttab",
        "\u{1}ctl",
        "\u{f1} \u{65e5}\u{672c} \u{1f600}",
        "",
    ] {
        let q = quote(text);
        assert_eq!(Json::parse(&q).unwrap().as_str(), Some(text), "{text:?}");
    }
}

#[test]
fn the_fixed_token_switch_reads_and_writes_like_the_others() {
    let mut s = Settings::default();
    for on in ["on", "ON", "true", "yes", "1"] {
        s.set("server_token_fixed", on).unwrap();
        assert!(s.server_token_fixed, "{on}");
    }
    assert!(s.set("server_token_fixed", "sometimes").is_err());
    assert!(s.server_token_fixed, "a refused value changes nothing");
    let text = s.to_text(false);
    assert!(text.contains(
        "server_token_fixed = on
"
    ));
    let back = Settings::from_text(&s.to_text(true)).unwrap();
    assert_eq!(back, s);
    s.set("server_token_fixed", "off").unwrap();
    assert!(!s.server_token_fixed);
}

#[test]
fn a_file_without_the_new_keys_still_reads_with_their_defaults() {
    let s = Settings::from_text(
        "server_address = 10.0.0.5:18200
server_retries = 1
",
    )
    .unwrap();
    assert!(!s.server_token_fixed);
    assert_eq!(s.server_error_body_bytes, 0);
    assert_eq!(s.server_retries, 1);
}
