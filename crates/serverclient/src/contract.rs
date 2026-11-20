//! The contract of the server (its OpenAPI document), kept so that a route the server has withdrawn is noticed.
//!
//! A [`Contract`] is the list of `METHOD /path` pairs of an OpenAPI document. A [`ContractCache`] holds the last contract that was read and every route
//! that was in an earlier one and is not in the newest: those routes are *retired*. A program asks [`ContractCache::status`] before it relies on a route
//! and shows a warning for a retired one instead of waiting for a 404. Nothing here sends a request: [`crate::Client::refresh_contract`] does that.

use crate::json::Json;
use std::collections::{BTreeMap, BTreeSet};

/// Where the server is expected to serve its contract.
pub const DEFAULT_CONTRACT_PATH: &str = "/api/v1/openapi.json";

const METHODS: [&str; 7] = ["get", "put", "post", "delete", "patch", "head", "options"];
/// The most routes a contract may hold (a document larger than this is not a contract of this server).
pub const MAX_ROUTES: usize = 4096;

/// What the cache knows about a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteStatus {
    /// It is in the newest contract.
    Known,
    /// It was in an earlier contract and the server no longer has it.
    Retired,
    /// It was never in a contract this cache read (or no contract was read yet).
    Unknown,
}

/// The routes of a contract: path template to the methods it answers (upper case).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Contract {
    routes: BTreeMap<String, BTreeSet<String>>,
}

/// What changed between two contracts, each route as `METHOD /path`, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractDiff {
    pub retired: Vec<String>,
    pub added: Vec<String>,
}

impl ContractDiff {
    pub fn is_empty(&self) -> bool {
        self.retired.is_empty() && self.added.is_empty()
    }
}

fn segments(path: &str) -> Vec<&str> {
    path.split('?')
        .next()
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

fn template_matches(template: &str, path: &str) -> bool {
    let (t, p) = (segments(template), segments(path));
    t.len() == p.len()
        && t.iter()
            .zip(p.iter())
            .all(|(a, b)| (a.starts_with('{') && a.ends_with('}')) || a == b)
}

impl Contract {
    pub fn new() -> Contract {
        Contract::default()
    }

    /// The routes of an OpenAPI document (JSON). The path of every entry of `paths` is joined to the first of `servers` when it has a relative URL
    /// (`/api/v1`), so the routes are written as the client sends them.
    pub fn from_openapi(text: &str) -> Result<Contract, String> {
        let doc = Json::parse(text)?;
        let Some(Json::Obj(paths)) = doc.get("paths") else {
            return Err("the document has no paths".to_string());
        };
        let base = doc
            .get("servers")
            .and_then(Json::as_array)
            .and_then(|s| s.first())
            .and_then(|s| s.get("url"))
            .and_then(Json::as_str)
            .filter(|u| u.starts_with('/'))
            .unwrap_or("")
            .trim_end_matches('/');
        let mut contract = Contract::new();
        for (path, item) in paths {
            let Json::Obj(members) = item else {
                continue;
            };
            for (name, _) in members {
                if METHODS.contains(&name.as_str()) {
                    contract.add(&name.to_ascii_uppercase(), &format!("{base}{path}"))?;
                }
            }
        }
        if contract.is_empty() {
            return Err("the document has no routes".to_string());
        }
        Ok(contract)
    }

    fn add(&mut self, method: &str, path: &str) -> Result<(), String> {
        if self.len() >= MAX_ROUTES {
            return Err(format!("more than {MAX_ROUTES} routes"));
        }
        self.routes
            .entry(path.to_string())
            .or_default()
            .insert(method.to_string());
        Ok(())
    }

    /// The number of `METHOD /path` pairs.
    pub fn len(&self) -> usize {
        self.routes.values().map(BTreeSet::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Does the contract have a route for this method and this path? A segment written `{name}` in the contract matches any one segment.
    pub fn has(&self, method: &str, path: &str) -> bool {
        let method = method.to_ascii_uppercase();
        self.routes
            .iter()
            .any(|(t, ms)| ms.contains(&method) && template_matches(t, path))
    }

    /// Every route as `METHOD /path`, sorted.
    pub fn routes(&self) -> Vec<String> {
        self.routes
            .iter()
            .flat_map(|(p, ms)| ms.iter().map(move |m| format!("{m} {p}")))
            .collect()
    }

    /// What `newer` lacks and has, compared with this contract.
    pub fn diff(&self, newer: &Contract) -> ContractDiff {
        let (old, new) = (self.routes(), newer.routes());
        let old_set: BTreeSet<&String> = old.iter().collect();
        let new_set: BTreeSet<&String> = new.iter().collect();
        ContractDiff {
            retired: old
                .iter()
                .filter(|r| !new_set.contains(r))
                .cloned()
                .collect(),
            added: new
                .iter()
                .filter(|r| !old_set.contains(r))
                .cloned()
                .collect(),
        }
    }

    fn remove(&mut self, route: &str) {
        if let Some((method, path)) = route.split_once(' ') {
            if let Some(ms) = self.routes.get_mut(path) {
                ms.remove(method);
                if ms.is_empty() {
                    self.routes.remove(path);
                }
            }
        }
    }
}

/// The last contract read and the routes it lost, in a form that can be kept in a file between runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContractCache {
    current: Contract,
    retired: Contract,
}

const HEADER: &str = "# eh3d-serverclient contract cache 1";

impl ContractCache {
    pub fn new() -> ContractCache {
        ContractCache::default()
    }

    /// The newest contract read.
    pub fn current(&self) -> &Contract {
        &self.current
    }

    /// The routes that an earlier contract had and the newest does not.
    pub fn retired(&self) -> Vec<String> {
        self.retired.routes()
    }

    /// Takes a newer contract: the routes it lost become retired (and stay so), the routes it has stop being retired. Gives what changed. The first
    /// contract read has nothing to compare with and retires nothing.
    pub fn update(&mut self, newer: Contract) -> ContractDiff {
        let first = self.current.is_empty();
        let diff = if first {
            ContractDiff::default()
        } else {
            self.current.diff(&newer)
        };
        for route in &diff.retired {
            if let Some((method, path)) = route.split_once(' ') {
                // a route is retired only while the newest contract does not have it
                let _ = self.retired.add(method, path);
            }
        }
        for route in newer.routes() {
            self.retired.remove(&route);
        }
        self.current = newer;
        diff
    }

    /// What is known about a route the program is about to use.
    pub fn status(&self, method: &str, path: &str) -> RouteStatus {
        if self.current.has(method, path) {
            RouteStatus::Known
        } else if self.retired.has(method, path) {
            RouteStatus::Retired
        } else {
            RouteStatus::Unknown
        }
    }

    /// The cache as text, to keep in a file: a header, then `= current` and `= retired` sections of `METHOD /path` lines.
    pub fn to_text(&self) -> String {
        let mut out = format!("{HEADER}\n= current\n");
        for r in self.current.routes() {
            out.push_str(&r);
            out.push('\n');
        }
        out.push_str("= retired\n");
        for r in self.retired.routes() {
            out.push_str(&r);
            out.push('\n');
        }
        out
    }

    /// Reads what `to_text` wrote. Anything else (another header, a line that is not a route, too many routes) is an error and nothing is taken.
    pub fn from_text(text: &str) -> Result<ContractCache, String> {
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some(HEADER) {
            return Err("not a contract cache of this version".to_string());
        }
        let mut cache = ContractCache::new();
        let mut section = "";
        for (n, raw) in lines.enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            match line {
                "= current" => section = "current",
                "= retired" => section = "retired",
                _ => {
                    let (method, path) = line
                        .split_once(' ')
                        .filter(|(m, p)| {
                            !p.contains(' ')
                                && p.starts_with('/')
                                && METHODS.contains(&m.to_ascii_lowercase().as_str())
                                && m.chars().all(|c| c.is_ascii_uppercase())
                        })
                        .ok_or_else(|| format!("line {}: not a route", n + 2))?;
                    let target = match section {
                        "current" => &mut cache.current,
                        "retired" => &mut cache.retired,
                        _ => return Err(format!("line {}: a route before its section", n + 2)),
                    };
                    target.add(method, path)?;
                }
            }
        }
        Ok(cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"{
      "openapi": "3.1.0",
      "servers": [{"url": "/api/v1"}],
      "paths": {
        "/health": {"get": {}},
        "/jobs": {"get": {}, "post": {}, "parameters": []},
        "/jobs/{id}": {"get": {}, "delete": {}},
        "/machines/{name}/jobs": {"put": {}}
      }
    }"#;

    fn contract(routes: &[(&str, &str)]) -> Contract {
        let mut c = Contract::new();
        for (m, p) in routes {
            c.add(m, p).unwrap();
        }
        c
    }

    #[test]
    fn the_routes_of_an_openapi_document_are_written_as_the_client_sends_them() {
        let c = Contract::from_openapi(DOC).unwrap();
        assert_eq!(c.len(), 6);
        assert!(c.has("GET", "/api/v1/health"));
        assert!(c.has("get", "/api/v1/jobs"));
        assert!(c.has("POST", "/api/v1/jobs"));
        assert!(!c.has("DELETE", "/api/v1/jobs"));
        assert!(!c.has("GET", "/health"));
    }

    #[test]
    fn a_parameter_segment_matches_one_segment_and_a_query_is_ignored() {
        let c = Contract::from_openapi(DOC).unwrap();
        assert!(c.has("DELETE", "/api/v1/jobs/42"));
        assert!(c.has("GET", "/api/v1/jobs/42?full=1"));
        assert!(c.has("PUT", "/api/v1/machines/bungard/jobs"));
        assert!(!c.has("GET", "/api/v1/jobs/42/extra"));
        assert!(!c.has("PUT", "/api/v1/machines/jobs"));
    }

    #[test]
    fn a_document_that_is_not_a_contract_is_refused() {
        assert!(Contract::from_openapi("not json").is_err());
        assert!(Contract::from_openapi("{}").is_err());
        assert!(Contract::from_openapi(r#"{"paths":{}}"#).is_err());
        assert!(Contract::from_openapi(r#"{"paths":{"/a":{"x-note":1}}}"#).is_err());
        // without servers the paths are taken as they are
        let c = Contract::from_openapi(r#"{"paths":{"/a":{"get":{}}}}"#).unwrap();
        assert!(c.has("GET", "/a"));
    }

    #[test]
    fn a_document_with_too_many_routes_is_refused() {
        let mut paths = String::from("{\"paths\":{");
        for i in 0..=MAX_ROUTES {
            if i > 0 {
                paths.push(',');
            }
            paths.push_str(&format!("\"/r{i}\":{{\"get\":{{}}}}"));
        }
        paths.push_str("}}");
        assert!(Contract::from_openapi(&paths).is_err());
    }

    #[test]
    fn the_diff_names_what_was_withdrawn_and_what_is_new() {
        let old = contract(&[("GET", "/a"), ("GET", "/b"), ("PUT", "/b")]);
        let new = contract(&[("GET", "/a"), ("PUT", "/b"), ("POST", "/c")]);
        let d = old.diff(&new);
        assert_eq!(d.retired, vec!["GET /b".to_string()]);
        assert_eq!(d.added, vec!["POST /c".to_string()]);
        assert!(old.diff(&old).is_empty());
    }

    #[test]
    fn a_route_that_leaves_is_retired_and_one_that_comes_back_is_not() {
        let mut cache = ContractCache::new();
        let first = cache.update(contract(&[("GET", "/a"), ("GET", "/b/{id}")]));
        assert!(first.is_empty());
        assert_eq!(cache.status("GET", "/a"), RouteStatus::Known);
        assert_eq!(cache.status("GET", "/zzz"), RouteStatus::Unknown);

        let d = cache.update(contract(&[("GET", "/a")]));
        assert_eq!(d.retired, vec!["GET /b/{id}".to_string()]);
        assert_eq!(cache.status("GET", "/b/7"), RouteStatus::Retired);
        assert_eq!(cache.status("POST", "/b/7"), RouteStatus::Unknown);

        // it stays retired through a contract that does not mention it
        cache.update(contract(&[("GET", "/a"), ("GET", "/c")]));
        assert_eq!(cache.retired(), vec!["GET /b/{id}".to_string()]);

        cache.update(contract(&[("GET", "/a"), ("GET", "/b/{id}")]));
        assert_eq!(cache.status("GET", "/b/7"), RouteStatus::Known);
        // only the route that left this time is retired now
        assert_eq!(cache.retired(), vec!["GET /c".to_string()]);
    }

    #[test]
    fn the_cache_survives_a_file() {
        let mut cache = ContractCache::new();
        cache.update(contract(&[("GET", "/a"), ("DELETE", "/b/{id}")]));
        cache.update(contract(&[("GET", "/a")]));
        let text = cache.to_text();
        assert!(text.starts_with(HEADER));
        let back = ContractCache::from_text(&text).unwrap();
        assert_eq!(back, cache);
        assert_eq!(back.status("DELETE", "/b/1"), RouteStatus::Retired);
    }

    #[test]
    fn a_damaged_cache_is_refused_whole() {
        let with_header = |rest: &str| format!("{HEADER}\n{rest}");
        let bad_ones = [
            String::new(),
            "# another header\n= current\nGET /a\n".to_string(),
            with_header("GET /a\n"),
            with_header("= current\nGET /a b\n"),
            with_header("= current\nFETCH /a\n"),
            with_header("= current\nget /a\n"),
            with_header("= current\nGET a\n"),
        ];
        for bad in &bad_ones {
            assert!(ContractCache::from_text(bad).is_err(), "{bad:?}");
        }
    }
}
