//! The list of files opened last, kept in the settings file with a limit.

/// The limit when the settings say nothing.
pub const DEFAULT_LIMIT: usize = 10;
/// The largest limit that is accepted.
pub const MAX_LIMIT: usize = 30;
/// The longest path that is kept.
pub const MAX_PATH: usize = 1024;

/// The files opened last, the newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFiles {
    limit: usize,
    items: Vec<String>,
}

impl Default for RecentFiles {
    fn default() -> Self {
        RecentFiles::new(DEFAULT_LIMIT)
    }
}

fn usable(path: &str) -> bool {
    !path.trim().is_empty() && path.len() <= MAX_PATH && !path.chars().any(|c| c.is_control())
}

impl RecentFiles {
    /// An empty list that keeps at most `limit` files (from 1 to [`MAX_LIMIT`]).
    pub fn new(limit: usize) -> RecentFiles {
        RecentFiles {
            limit: limit.clamp(1, MAX_LIMIT),
            items: Vec::new(),
        }
    }

    /// The files, the newest first.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// How many files are kept at most.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Changes the limit (kept from 1 to [`MAX_LIMIT`]) and drops the oldest files that no longer fit.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.clamp(1, MAX_LIMIT);
        self.items.truncate(self.limit);
    }

    /// Puts a file first (a file already in the list moves up, it is not repeated). A path that cannot be kept (empty, too long, with a control
    /// character) is refused.
    pub fn push(&mut self, path: &str) -> bool {
        if !usable(path) {
            return false;
        }
        self.items.retain(|p| p != path);
        self.items.insert(0, path.to_string());
        self.items.truncate(self.limit);
        true
    }

    /// Takes a file out of the list (a file that no longer exists).
    pub fn remove(&mut self, path: &str) {
        self.items.retain(|p| p != path);
    }

    /// Empties the list.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// The lines for the settings file: `dock.recent_limit` and `dock.recent.<n>`.
    pub fn to_pairs(&self) -> Vec<(String, String)> {
        let mut out = vec![("dock.recent_limit".to_string(), self.limit.to_string())];
        for (i, p) in self.items.iter().enumerate() {
            out.push((format!("dock.recent.{i}"), p.clone()));
        }
        out
    }

    /// Reads the lines of the settings file; what cannot be used is left out.
    pub fn from_pairs(pairs: &[(String, String)]) -> RecentFiles {
        let limit = pairs
            .iter()
            .find(|(k, _)| k == "dock.recent_limit")
            .and_then(|(_, v)| v.trim().parse::<usize>().ok())
            .unwrap_or(DEFAULT_LIMIT);
        let mut r = RecentFiles::new(limit);
        let mut numbered: Vec<(usize, &String)> = pairs
            .iter()
            .filter_map(|(k, v)| {
                k.strip_prefix("dock.recent.")
                    .and_then(|n| n.parse::<usize>().ok())
                    .map(|n| (n, v))
            })
            .collect();
        numbered.sort_by_key(|(n, _)| *n);
        for (_, p) in numbered {
            if usable(p) && !r.items.iter().any(|x| x == p) && r.items.len() < r.limit {
                r.items.push(p.clone());
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_first_without_repeats_and_within_the_limit() {
        let mut r = RecentFiles::new(3);
        for p in ["a", "b", "c", "a", "d"] {
            assert!(r.push(p));
        }
        assert_eq!(r.items(), ["d", "a", "c"]);
        r.remove("a");
        assert_eq!(r.items(), ["d", "c"]);
        r.clear();
        assert!(r.items().is_empty());
    }

    #[test]
    fn unusable_paths_are_refused() {
        let mut r = RecentFiles::default();
        assert!(!r.push(""));
        assert!(!r.push("   "));
        assert!(!r.push("a\nb"));
        assert!(!r.push(&"x".repeat(MAX_PATH + 1)));
        assert!(r.items().is_empty());
    }

    #[test]
    fn the_limit_is_kept_in_range_and_trims_the_list() {
        assert_eq!(RecentFiles::new(0).limit(), 1);
        assert_eq!(RecentFiles::new(500).limit(), MAX_LIMIT);
        let mut r = RecentFiles::new(5);
        for p in ["a", "b", "c", "d"] {
            r.push(p);
        }
        r.set_limit(2);
        assert_eq!(r.items(), ["d", "c"]);
    }

    #[test]
    fn the_settings_file_round_trips() {
        let mut r = RecentFiles::new(4);
        for p in ["/home/me/one.eh3dproj", "C:\\jobs\\two.eh3dproj", "three"] {
            r.push(p);
        }
        let back = RecentFiles::from_pairs(&r.to_pairs());
        assert_eq!(back, r);
    }

    #[test]
    fn a_damaged_list_gives_what_can_be_used() {
        let pairs: Vec<(String, String)> = [
            ("dock.recent_limit", "2"),
            ("dock.recent.1", "second"),
            ("dock.recent.0", "first"),
            ("dock.recent.2", "third"),
            ("dock.recent.x", "nope"),
            ("dock.recent.3", "first"),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
        let r = RecentFiles::from_pairs(&pairs);
        assert_eq!(r.items(), ["first", "second"]);
        let bad = RecentFiles::from_pairs(&[("dock.recent_limit".into(), "abc".into())]);
        assert_eq!(bad.limit(), DEFAULT_LIMIT);
    }
}
