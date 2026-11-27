//! The layout of a window as plain data: a tree of divisions and groups of tabs, the panels that float, the ones that are hidden and the ones
//! that fold into an edge of the window. Every operation (move, float, dock, hide, show, reorder, resize, maximise, fold) is a method with a
//! rule that can be tested without a window, and every method that changes the arrangement keeps this promise: each panel is in exactly one
//! place, no group is empty and no division has a single part.

use crate::geom::{Axis, Rect, Side, Zone};

/// The most nested divisions a layout may have (a deeper one is flattened when it is repaired).
pub const MAX_DEPTH: usize = 24;
/// The most panels a layout may name.
pub const MAX_PANELS: usize = 256;
/// The smallest share a part of a division may have.
pub const MIN_RATIO: f32 = 0.05;
/// The largest share a part of a division may have.
pub const MAX_RATIO: f32 = 0.95;
/// The smallest side of a floating window, in points.
pub const MIN_FLOAT: f32 = 80.0;
/// The largest side of a floating window, in points.
pub const MAX_FLOAT: f32 = 20000.0;
/// The most floating windows a layout may have.
pub const MAX_FLOATING: usize = 64;

/// Whether `id` is a name a panel may have: 1 to 48 letters, digits, `_`, `-` or `.`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 48
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
}

/// A path from the root of the tree to a node: `false` is the first part of a division, `true` the second.
pub type Path = Vec<bool>;

/// Why an operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockError {
    /// The layout is locked.
    Locked,
    /// No panel with that name is in the layout.
    UnknownPanel,
    /// The place to drop the panel does not exist.
    NoSuchTarget,
    /// The panel would be dropped on itself.
    SamePanel,
    /// Only a docked panel can be maximised.
    NotDocked,
    /// There is no division at that path.
    NotASplit,
    /// The name of the workspace is not valid, is used, or the last one cannot be deleted.
    BadName,
    /// The text of a layout could not be read.
    Unreadable,
}

impl DockError {
    /// The word of the error, for logs and tests.
    pub fn name(self) -> &'static str {
        match self {
            DockError::Locked => "locked",
            DockError::UnknownPanel => "unknown_panel",
            DockError::NoSuchTarget => "no_such_target",
            DockError::SamePanel => "same_panel",
            DockError::NotDocked => "not_docked",
            DockError::NotASplit => "not_a_split",
            DockError::BadName => "bad_name",
            DockError::Unreadable => "unreadable",
        }
    }
}

impl std::fmt::Display for DockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::error::Error for DockError {}

/// A row of tabs: the panels that share one place and the one on show.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Group {
    /// The panels, in the order of the tabs.
    pub panels: Vec<String>,
    /// The index of the tab on show.
    pub active: usize,
}

impl Group {
    /// A group with these panels and the first one on show.
    pub fn new(ids: &[&str]) -> Group {
        Group {
            panels: ids.iter().map(|s| s.to_string()).collect(),
            active: 0,
        }
    }

    /// The panel on show.
    pub fn active_panel(&self) -> Option<&str> {
        self.panels.get(self.active).map(String::as_str)
    }
}

/// A node of the tree: a division in two parts, or a group of tabs.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Two parts side by side or one above the other; `ratio` is the share of the first.
    Split {
        /// The direction in which the parts are put.
        axis: Axis,
        /// The share of the first part, from [`MIN_RATIO`] to [`MAX_RATIO`].
        ratio: f32,
        /// The left or top part.
        first: Box<Node>,
        /// The right or bottom part.
        second: Box<Node>,
    },
    /// A group of tabs.
    Tabs(Group),
}

impl Node {
    /// A group of tabs with these panels.
    pub fn tabs(ids: &[&str]) -> Node {
        Node::Tabs(Group::new(ids))
    }

    /// Two parts side by side.
    pub fn row(ratio: f32, first: Node, second: Node) -> Node {
        Node::Split {
            axis: Axis::Row,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Two parts, one above the other.
    pub fn column(ratio: f32, first: Node, second: Node) -> Node {
        Node::Split {
            axis: Axis::Column,
            ratio,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// The node at `path`.
    pub fn get(&self, path: &[bool]) -> Option<&Node> {
        match (path.split_first(), self) {
            (None, n) => Some(n),
            (Some((side, rest)), Node::Split { first, second, .. }) => {
                if *side { second } else { first }.get(rest)
            }
            _ => None,
        }
    }

    /// The node at `path`, to change it.
    pub fn get_mut(&mut self, path: &[bool]) -> Option<&mut Node> {
        match (path.split_first(), self) {
            (None, n) => Some(n),
            (Some((side, rest)), Node::Split { first, second, .. }) => {
                if *side { second } else { first }.get_mut(rest)
            }
            _ => None,
        }
    }

    /// Every panel of the subtree, in reading order.
    pub fn panels(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<String>) {
        match self {
            Node::Tabs(g) => out.extend(g.panels.iter().cloned()),
            Node::Split { first, second, .. } => {
                first.collect(out);
                second.collect(out);
            }
        }
    }

    /// The first panel of the subtree.
    pub fn first_panel(&self) -> Option<&str> {
        match self {
            Node::Tabs(g) => g.panels.first().map(String::as_str),
            Node::Split { first, second, .. } => {
                first.first_panel().or_else(|| second.first_panel())
            }
        }
    }

    /// The paths of the groups of the subtree, in reading order.
    pub fn group_paths(&self) -> Vec<Path> {
        let mut out = Vec::new();
        self.walk(&mut Vec::new(), &mut out);
        out
    }

    fn walk(&self, here: &mut Path, out: &mut Vec<Path>) {
        match self {
            Node::Tabs(_) => out.push(here.clone()),
            Node::Split { first, second, .. } => {
                here.push(false);
                first.walk(here, out);
                here.pop();
                here.push(true);
                second.walk(here, out);
                here.pop();
            }
        }
    }

    /// How many divisions deep the subtree goes (a group is 0).
    pub fn depth(&self) -> usize {
        match self {
            Node::Tabs(_) => 0,
            Node::Split { first, second, .. } => 1 + first.depth().max(second.depth()),
        }
    }
}

/// A window that floats over the others.
#[derive(Debug, Clone, PartialEq)]
pub struct FloatWin {
    /// The tabs in it.
    pub group: Group,
    /// Where it is and how big.
    pub rect: Rect,
}

/// Where a panel was before it left the tree, so that it can come back there.
#[derive(Debug, Clone, PartialEq)]
pub struct Home {
    /// The panel.
    pub panel: String,
    /// A panel that was next to it.
    pub anchor: String,
    /// On which side of the anchor it was (the middle: in the same tabs).
    pub zone: Zone,
}

/// Where a panel is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelState {
    /// In the tree.
    Docked,
    /// In a floating window.
    Floating,
    /// Not shown.
    Hidden,
    /// Folded into an edge of the window.
    AutoHidden(Side),
}

/// Where a panel is dropped.
#[derive(Debug, Clone, PartialEq)]
pub enum DropTarget {
    /// On a group (named by one of its panels): in the middle as a tab (at `index`, or at the end), near a side as a new part.
    OnGroup {
        /// A panel of the group.
        anchor: String,
        /// Where in the group.
        zone: Zone,
        /// The place among the tabs, for the middle.
        index: Option<usize>,
    },
    /// Along an edge of the window, taking a quarter of it.
    WindowEdge(Side),
    /// In a floating window of its own.
    Float(Rect),
}

/// Something wrong that was found in a layout (and put right by [`Layout::repair`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// A name that is not a valid panel name.
    BadId(String),
    /// A panel the program does not have.
    UnknownPanel(String),
    /// A panel in two places (the first place wins).
    DuplicatePanel(String),
    /// A panel of the program that no place names (it is put among the hidden ones).
    MissingPanel(String),
    /// A share outside its range or not a number.
    BadRatio,
    /// A group with no panels, or a division with one part.
    EmptyGroup,
    /// A tab on show that does not exist.
    BadActive,
    /// A floating window with a size or place that cannot be used.
    BadFloatRect,
    /// A maximised panel that is not docked.
    BadMaximized,
    /// A tree deeper than [`MAX_DEPTH`].
    TooDeep,
    /// More floating windows than [`MAX_FLOATING`].
    TooManyFloating,
}

/// The arrangement of the panels of a window.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// The tree of divisions and tabs (`None`: nothing is docked).
    pub root: Option<Node>,
    /// The floating windows, the last one on top.
    pub floating: Vec<FloatWin>,
    /// The panels that are not shown.
    pub hidden: Vec<String>,
    /// The panels folded into an edge of the window.
    pub auto_hidden: Vec<(String, Side)>,
    /// The panel that fills the whole work area.
    pub maximized: Option<String>,
    /// Whether the arrangement is frozen.
    pub locked: bool,
    homes: Vec<Home>,
}

impl PartialEq for Layout {
    /// Two layouts are equal when their arrangement is; where a panel would go back to is not part of it.
    fn eq(&self, other: &Layout) -> bool {
        self.root == other.root
            && self.floating == other.floating
            && self.hidden == other.hidden
            && self.auto_hidden == other.auto_hidden
            && self.maximized == other.maximized
            && self.locked == other.locked
    }
}

fn sanitize_rect(r: Rect) -> Option<Rect> {
    if !r.is_sound() || r.x.abs() > 1.0e6 || r.y.abs() > 1.0e6 {
        return None;
    }
    Some(Rect::new(
        r.x,
        r.y,
        r.w.clamp(MIN_FLOAT, MAX_FLOAT),
        r.h.clamp(MIN_FLOAT, MAX_FLOAT),
    ))
}

impl Layout {
    /// An empty layout.
    pub fn new() -> Layout {
        Layout::default()
    }

    /// A layout whose tree is `root`.
    pub fn with_root(root: Node) -> Layout {
        Layout {
            root: Some(root),
            ..Layout::default()
        }
    }

    /// The same layout with these panels hidden.
    pub fn with_hidden(mut self, ids: &[&str]) -> Layout {
        self.hidden.extend(ids.iter().map(|s| s.to_string()));
        self
    }

    /// The same layout with a floating window.
    pub fn with_floating(mut self, ids: &[&str], rect: Rect) -> Layout {
        self.floating.push(FloatWin {
            group: Group::new(ids),
            rect,
        });
        self
    }

    /// The same layout with a panel folded into a side.
    pub fn with_auto_hidden(mut self, id: &str, side: Side) -> Layout {
        self.auto_hidden.push((id.to_string(), side));
        self
    }

    /// Every panel of the layout wherever it is: the tree, the floating windows, the folded ones, the hidden ones.
    pub fn panels(&self) -> Vec<String> {
        let mut out = self.root.as_ref().map(Node::panels).unwrap_or_default();
        for f in &self.floating {
            out.extend(f.group.panels.iter().cloned());
        }
        out.extend(self.auto_hidden.iter().map(|(p, _)| p.clone()));
        out.extend(self.hidden.iter().cloned());
        out
    }

    /// Whether the layout has this panel somewhere.
    pub fn contains(&self, id: &str) -> bool {
        self.state_of(id).is_some()
    }

    /// Where the panel is.
    pub fn state_of(&self, id: &str) -> Option<PanelState> {
        if self.path_of(id).is_some() {
            return Some(PanelState::Docked);
        }
        if self
            .floating
            .iter()
            .any(|f| f.group.panels.iter().any(|p| p == id))
        {
            return Some(PanelState::Floating);
        }
        if let Some((_, side)) = self.auto_hidden.iter().find(|(p, _)| p == id) {
            return Some(PanelState::AutoHidden(*side));
        }
        self.hidden
            .iter()
            .any(|p| p == id)
            .then_some(PanelState::Hidden)
    }

    /// Whether the panel can be seen without a click: docked or floating.
    pub fn is_shown(&self, id: &str) -> bool {
        matches!(
            self.state_of(id),
            Some(PanelState::Docked | PanelState::Floating)
        )
    }

    /// The path of the group that holds a docked panel.
    pub fn path_of(&self, id: &str) -> Option<Path> {
        let root = self.root.as_ref()?;
        root.group_paths().into_iter().find(
            |p| matches!(root.get(p), Some(Node::Tabs(g)) if g.panels.iter().any(|x| x == id)),
        )
    }

    /// The group that holds a docked panel.
    pub fn group_of(&self, id: &str) -> Option<&Group> {
        let p = self.path_of(id)?;
        match self.root.as_ref()?.get(&p)? {
            Node::Tabs(g) => Some(g),
            Node::Split { .. } => None,
        }
    }

    fn group_of_mut(&mut self, id: &str) -> Option<&mut Group> {
        if let Some(p) = self.path_of(id) {
            if let Some(Node::Tabs(g)) = self.root.as_mut().and_then(|r| r.get_mut(&p)) {
                return Some(g);
            }
        }
        self.floating
            .iter_mut()
            .map(|f| &mut f.group)
            .find(|g| g.panels.iter().any(|p| p == id))
    }

    /// The panel on show in the group of this panel (docked or floating).
    pub fn active_of(&self, id: &str) -> Option<&str> {
        if let Some(g) = self.group_of(id) {
            return g.active_panel();
        }
        self.floating
            .iter()
            .find(|f| f.group.panels.iter().any(|p| p == id))
            .and_then(|f| f.group.active_panel())
    }

    fn require_free(&self) -> Result<(), DockError> {
        if self.locked {
            Err(DockError::Locked)
        } else {
            Ok(())
        }
    }

    // ---- tabs --------------------------------------------------------------------------------------------------------------------------

    /// Puts the panel on show in its group (allowed while the layout is locked).
    pub fn set_active(&mut self, id: &str) -> Result<(), DockError> {
        let g = self.group_of_mut(id).ok_or(DockError::UnknownPanel)?;
        if let Some(i) = g.panels.iter().position(|p| p == id) {
            g.active = i;
        }
        Ok(())
    }

    /// Makes a docked or floating panel the one the person sees: its tab goes on show in its group, a floating window goes last (on top) and a
    /// maximised panel of another group gets its place back. A hidden or folded panel is left as it is (see [`Layout::show`]). Allowed while the
    /// layout is locked, for nothing is moved out of its group.
    pub fn bring_to_front(&mut self, id: &str) -> Result<(), DockError> {
        match self.state_of(id) {
            None => return Err(DockError::UnknownPanel),
            Some(PanelState::Docked) => {
                self.set_active(id)?;
                if let Some(m) = self.maximized.clone() {
                    let same_group = self.path_of(&m) == self.path_of(id);
                    self.maximized = same_group.then(|| id.to_string());
                }
            }
            Some(PanelState::Floating) => {
                self.set_active(id)?;
                let at = self
                    .floating
                    .iter()
                    .position(|f| f.group.panels.iter().any(|p| p == id));
                if let Some(i) = at {
                    let win = self.floating.remove(i);
                    self.floating.push(win);
                }
            }
            Some(PanelState::Hidden | PanelState::AutoHidden(_)) => {}
        }
        Ok(())
    }

    /// Moves the tab of a panel to place `index` of its group (the index is clamped).
    pub fn reorder(&mut self, id: &str, index: usize) -> Result<(), DockError> {
        self.require_free()?;
        let g = self.group_of_mut(id).ok_or(DockError::UnknownPanel)?;
        let from = g
            .panels
            .iter()
            .position(|p| p == id)
            .ok_or(DockError::UnknownPanel)?;
        let name = g.panels.remove(from);
        let to = index.min(g.panels.len());
        g.panels.insert(to, name);
        g.active = to;
        Ok(())
    }

    /// Shows the next (`delta` 1) or previous (`delta` -1) tab of the group of this panel, wrapping round; gives its name.
    pub fn cycle(&mut self, id: &str, delta: i32) -> Result<String, DockError> {
        let g = self.group_of_mut(id).ok_or(DockError::UnknownPanel)?;
        let n = g.panels.len() as i32;
        if n == 0 {
            return Err(DockError::UnknownPanel);
        }
        let from = g.panels.iter().position(|p| p == id).unwrap_or(0) as i32;
        let to = (from + delta).rem_euclid(n) as usize;
        g.active = to;
        Ok(g.panels[to].clone())
    }

    // ---- moving ------------------------------------------------------------------------------------------------------------------------

    fn record_home(&mut self, id: &str, path: &[bool]) {
        let Some(root) = self.root.as_ref() else {
            return;
        };
        let Some(Node::Tabs(g)) = root.get(path) else {
            return;
        };
        let home = if let Some(other) = g.panels.iter().find(|p| *p != id) {
            Some((other.clone(), Zone::Center))
        } else if let Some((last, parent_path)) = path.split_last() {
            match root.get(parent_path) {
                Some(Node::Split {
                    axis,
                    first,
                    second,
                    ..
                }) => {
                    let sibling = if *last { first } else { second };
                    sibling.first_panel().map(|a| {
                        let zone = match (axis, *last) {
                            (Axis::Row, false) => Zone::Left,
                            (Axis::Row, true) => Zone::Right,
                            (Axis::Column, false) => Zone::Top,
                            (Axis::Column, true) => Zone::Bottom,
                        };
                        (a.to_string(), zone)
                    })
                }
                _ => None,
            }
        } else {
            None
        };
        if let Some((anchor, zone)) = home {
            self.homes.retain(|h| h.panel != id);
            self.homes.push(Home {
                panel: id.to_string(),
                anchor,
                zone,
            });
        }
    }

    /// Takes the panel out of wherever it is (remembering where it was in the tree) and tidies up. Gives whether it was there.
    fn detach(&mut self, id: &str) -> bool {
        let mut found = false;
        if let Some(path) = self.path_of(id) {
            self.record_home(id, &path);
            if let Some(Node::Tabs(g)) = self.root.as_mut().and_then(|r| r.get_mut(&path)) {
                g.panels.retain(|p| p != id);
            }
            found = true;
        }
        for f in &mut self.floating {
            let before = f.group.panels.len();
            f.group.panels.retain(|p| p != id);
            found |= f.group.panels.len() != before;
        }
        let before = self.hidden.len() + self.auto_hidden.len();
        self.hidden.retain(|p| p != id);
        self.auto_hidden.retain(|(p, _)| p != id);
        found |= self.hidden.len() + self.auto_hidden.len() != before;
        if self.maximized.as_deref() == Some(id) {
            self.maximized = None;
        }
        self.normalize();
        found
    }

    fn insert_in_group(group: &mut Group, id: &str, index: Option<usize>) {
        let at = index.unwrap_or(group.panels.len()).min(group.panels.len());
        group.panels.insert(at, id.to_string());
        group.active = at;
    }

    /// Puts a panel that is in no place on the target. The target was checked before the panel was taken out.
    fn place(&mut self, id: &str, target: &DropTarget) -> bool {
        match target {
            DropTarget::OnGroup {
                anchor,
                zone,
                index,
            } => {
                if let Some(path) = self.path_of(anchor) {
                    let root = self.root.as_mut();
                    let Some(root) = root else { return false };
                    match zone.side() {
                        None => {
                            if let Some(Node::Tabs(g)) = root.get_mut(&path) {
                                Layout::insert_in_group(g, id, *index);
                                return true;
                            }
                            false
                        }
                        Some(side) => {
                            let Some(node) = root.get_mut(&path) else {
                                return false;
                            };
                            let old = std::mem::replace(node, Node::Tabs(Group::default()));
                            let new = Node::tabs(&[id]);
                            let (first, second) = if side.is_first() {
                                (new, old)
                            } else {
                                (old, new)
                            };
                            *node = Node::Split {
                                axis: side.axis(),
                                ratio: 0.5,
                                first: Box::new(first),
                                second: Box::new(second),
                            };
                            true
                        }
                    }
                } else if let Some(f) = self
                    .floating
                    .iter_mut()
                    .find(|f| f.group.panels.iter().any(|p| p == anchor))
                {
                    Layout::insert_in_group(&mut f.group, id, *index);
                    true
                } else {
                    false
                }
            }
            DropTarget::WindowEdge(side) => {
                let new = Node::tabs(&[id]);
                match self.root.take() {
                    None => self.root = Some(new),
                    Some(old) => {
                        let (first, second) = if side.is_first() {
                            (new, old)
                        } else {
                            (old, new)
                        };
                        let ratio = if side.is_first() { 0.25 } else { 0.75 };
                        self.root = Some(Node::Split {
                            axis: side.axis(),
                            ratio,
                            first: Box::new(first),
                            second: Box::new(second),
                        });
                    }
                }
                true
            }
            DropTarget::Float(rect) => {
                let rect = sanitize_rect(*rect).unwrap_or_else(|| self.default_float_rect());
                self.floating.push(FloatWin {
                    group: Group::new(&[id]),
                    rect,
                });
                true
            }
        }
    }

    fn default_float_rect(&self) -> Rect {
        let n = self.floating.len() as f32;
        Rect::new(
            80.0 + 28.0 * (n % 8.0),
            80.0 + 28.0 * (n % 8.0),
            360.0,
            260.0,
        )
    }

    /// Moves a panel to a target: a tab of a group, a new part beside a group, an edge of the window or a window of its own.
    pub fn move_panel(&mut self, id: &str, target: DropTarget) -> Result<(), DockError> {
        self.require_free()?;
        if !self.contains(id) {
            return Err(DockError::UnknownPanel);
        }
        let mut target = target;
        if let DropTarget::OnGroup { anchor, zone, .. } = &mut target {
            if !self.is_shown(anchor) {
                return Err(DockError::NoSuchTarget);
            }
            if anchor == id {
                // a drop on its own group: the group's other tabs are the anchor; alone, there is nothing to do
                let others: Vec<String> = self
                    .group_of(id)
                    .map(|g| g.panels.clone())
                    .or_else(|| {
                        self.floating
                            .iter()
                            .find(|f| f.group.panels.iter().any(|p| p == id))
                            .map(|f| f.group.panels.clone())
                    })
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|p| p != id)
                    .collect();
                match others.first() {
                    None => return Err(DockError::SamePanel),
                    Some(o) => *anchor = o.clone(),
                }
            }
            if self.state_of(anchor) == Some(PanelState::Floating) {
                *zone = Zone::Center;
            }
        }
        self.detach(id);
        if !self.place(id, &target) {
            // never lose a panel: if the target vanished while tidying, the panel goes to the hidden ones
            self.hidden.push(id.to_string());
        }
        self.normalize();
        Ok(())
    }

    /// Moves a docked panel one step towards a side: out of its tabs beside them, or, when it is alone, to that edge of the window.
    pub fn move_toward(&mut self, id: &str, side: Side) -> Result<(), DockError> {
        self.require_free()?;
        match self.state_of(id) {
            None => Err(DockError::UnknownPanel),
            Some(PanelState::Docked) => {
                let others: Vec<String> = self
                    .group_of(id)
                    .map(|g| g.panels.iter().filter(|p| *p != id).cloned().collect())
                    .unwrap_or_default();
                match others.first() {
                    Some(o) => self.move_panel(
                        id,
                        DropTarget::OnGroup {
                            anchor: o.clone(),
                            zone: Zone::of_side(side),
                            index: None,
                        },
                    ),
                    None => self.move_panel(id, DropTarget::WindowEdge(side)),
                }
            }
            Some(_) => Err(DockError::NotDocked),
        }
    }

    /// Makes the panel float in a window of its own (at `rect`, or at a default place).
    pub fn float(&mut self, id: &str, rect: Option<Rect>) -> Result<(), DockError> {
        let rect = rect.unwrap_or_else(|| self.default_float_rect());
        self.move_panel(id, DropTarget::Float(rect))
    }

    /// Brings back to the tree a panel that floats, is hidden or is folded, to the place it came from when that still exists.
    pub fn dock(&mut self, id: &str) -> Result<(), DockError> {
        self.require_free()?;
        match self.state_of(id) {
            None => return Err(DockError::UnknownPanel),
            Some(PanelState::Docked) => return Ok(()),
            Some(_) => {}
        }
        let home = self.homes.iter().find(|h| h.panel == id).cloned();
        let target = match home {
            Some(h) if self.path_of(&h.anchor).is_some() => DropTarget::OnGroup {
                anchor: h.anchor,
                zone: h.zone,
                index: None,
            },
            _ => match self.root.as_ref().and_then(Node::first_panel) {
                Some(first) => DropTarget::OnGroup {
                    anchor: first.to_string(),
                    zone: Zone::Center,
                    index: None,
                },
                None => DropTarget::WindowEdge(Side::Left),
            },
        };
        self.move_panel(id, target)
    }

    /// Shows a hidden panel (alias of [`Layout::dock`]; a panel already shown stays).
    pub fn show(&mut self, id: &str) -> Result<(), DockError> {
        if self.is_shown(id) {
            return Ok(());
        }
        self.dock(id)
    }

    /// Hides a panel. It stays in the layout, so that the View menu can show it again.
    pub fn hide(&mut self, id: &str) -> Result<(), DockError> {
        self.require_free()?;
        if !self.contains(id) {
            return Err(DockError::UnknownPanel);
        }
        if self.state_of(id) == Some(PanelState::Hidden) {
            return Ok(());
        }
        self.detach(id);
        self.hidden.push(id.to_string());
        Ok(())
    }

    /// Shows a hidden panel or hides a shown one; gives whether it is shown now.
    pub fn toggle(&mut self, id: &str) -> Result<bool, DockError> {
        if self.is_shown(id) {
            self.hide(id)?;
            Ok(false)
        } else {
            self.show(id)?;
            Ok(true)
        }
    }

    /// Folds a panel into an edge of the window, where only its tab shows until it is clicked.
    pub fn auto_hide(&mut self, id: &str, side: Side) -> Result<(), DockError> {
        self.require_free()?;
        if !self.contains(id) {
            return Err(DockError::UnknownPanel);
        }
        if self.state_of(id) == Some(PanelState::AutoHidden(side)) {
            return Ok(());
        }
        self.detach(id);
        self.auto_hidden.push((id.to_string(), side));
        Ok(())
    }

    // ---- sizes -------------------------------------------------------------------------------------------------------------------------

    /// Sets the share of the first part of the division at `path`, kept between [`MIN_RATIO`] and [`MAX_RATIO`].
    pub fn set_ratio(&mut self, path: &[bool], ratio: f32) -> Result<(), DockError> {
        self.require_free()?;
        match self.root.as_mut().and_then(|r| r.get_mut(path)) {
            Some(Node::Split { ratio: r, .. }) => {
                *r = clamp_ratio(ratio);
                Ok(())
            }
            _ => Err(DockError::NotASplit),
        }
    }

    /// Maximises a docked panel: it fills the work area until [`Layout::restore`].
    pub fn maximize(&mut self, id: &str) -> Result<(), DockError> {
        match self.state_of(id) {
            None => Err(DockError::UnknownPanel),
            Some(PanelState::Docked) => {
                self.set_active(id)?;
                self.maximized = Some(id.to_string());
                Ok(())
            }
            Some(_) => Err(DockError::NotDocked),
        }
    }

    /// Gives the maximised panel its place back.
    pub fn restore(&mut self) {
        self.maximized = None;
    }

    /// Maximises the panel, or restores it when it already is the maximised one; gives whether it is maximised now.
    pub fn toggle_maximize(&mut self, id: &str) -> Result<bool, DockError> {
        if self.maximized.as_deref() == Some(id) {
            self.restore();
            Ok(false)
        } else {
            self.maximize(id)?;
            Ok(true)
        }
    }

    /// Freezes or unfreezes the arrangement.
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    // ---- tidying, checking and repairing -----------------------------------------------------------------------------------------------

    /// Tidies the arrangement: empty groups go, a division with one part is replaced by it, shares and active tabs are put in range.
    pub fn normalize(&mut self) {
        let _ = self.normalize_report();
    }

    fn normalize_report(&mut self) -> Vec<Problem> {
        let mut problems = Vec::new();
        self.root = self.root.take().and_then(|n| norm(n, &mut problems));
        let before = self.floating.len();
        self.floating.retain(|f| !f.group.panels.is_empty());
        if self.floating.len() != before {
            problems.push(Problem::EmptyGroup);
        }
        for f in &mut self.floating {
            if f.group.active >= f.group.panels.len() {
                f.group.active = f.group.panels.len() - 1;
                problems.push(Problem::BadActive);
            }
            match sanitize_rect(f.rect) {
                Some(r) => {
                    if r != f.rect {
                        problems.push(Problem::BadFloatRect);
                    }
                    f.rect = r;
                }
                None => {
                    f.rect = Rect::new(80.0, 80.0, 360.0, 260.0);
                    problems.push(Problem::BadFloatRect);
                }
            }
        }
        if let Some(m) = self.maximized.clone() {
            if self.path_of(&m).is_none() {
                self.maximized = None;
                problems.push(Problem::BadMaximized);
            }
        }
        if self.homes.len() > MAX_PANELS {
            let extra = self.homes.len() - MAX_PANELS;
            self.homes.drain(..extra);
        }
        problems
    }

    /// What is wrong with the layout, given the panels the program has. Nothing is changed.
    pub fn validate(&self, known: &[&str]) -> Vec<Problem> {
        self.clone().repair(known)
    }

    /// Puts right what [`Layout::validate`] finds and gives it: names that are not valid, panels the program does not have and panels in two places
    /// go; panels of the program that nobody names are hidden; shares, active tabs and floating windows are put in range; a tree that is too deep
    /// becomes one group. `known` is every panel the program has (an empty list removes everything).
    pub fn repair(&mut self, known: &[&str]) -> Vec<Problem> {
        let mut problems = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        let mut keep = |id: &str, problems: &mut Vec<Problem>| -> bool {
            if !valid_id(id) {
                problems.push(Problem::BadId(id.to_string()));
                false
            } else if !known.contains(&id) {
                problems.push(Problem::UnknownPanel(id.to_string()));
                false
            } else if seen.iter().any(|s| s == id) {
                problems.push(Problem::DuplicatePanel(id.to_string()));
                false
            } else {
                seen.push(id.to_string());
                true
            }
        };
        if let Some(root) = self.root.as_mut() {
            for path in root.group_paths() {
                if let Some(Node::Tabs(g)) = root.get_mut(&path) {
                    let old = std::mem::take(&mut g.panels);
                    let active_name = old.get(g.active).cloned();
                    g.panels = old.into_iter().filter(|p| keep(p, &mut problems)).collect();
                    g.active = active_name
                        .and_then(|a| g.panels.iter().position(|p| *p == a))
                        .unwrap_or(0);
                }
            }
        }
        if self.floating.len() > MAX_FLOATING {
            self.floating.truncate(MAX_FLOATING);
            problems.push(Problem::TooManyFloating);
        }
        for f in &mut self.floating {
            f.group.panels.retain(|p| keep(p, &mut problems));
        }
        self.auto_hidden.retain(|(p, _)| keep(p, &mut problems));
        self.hidden.retain(|p| keep(p, &mut problems));
        problems.extend(self.normalize_report());
        self.homes
            .retain(|h| seen.contains(&h.panel) && seen.contains(&h.anchor));
        if self.root.as_ref().is_some_and(|r| r.depth() > MAX_DEPTH) {
            let flat = self.root.take().map(|r| r.panels()).unwrap_or_default();
            self.root = Some(Node::Tabs(Group {
                panels: flat,
                active: 0,
            }));
            problems.push(Problem::TooDeep);
        }
        for k in known {
            if !seen.iter().any(|s| s == k) && valid_id(k) {
                self.hidden.push((*k).to_string());
                problems.push(Problem::MissingPanel((*k).to_string()));
            }
        }
        problems
    }
}

/// A share kept between [`MIN_RATIO`] and [`MAX_RATIO`] (a number that is not one gives one half).
pub fn clamp_ratio(r: f32) -> f32 {
    if r.is_finite() {
        r.clamp(MIN_RATIO, MAX_RATIO)
    } else {
        0.5
    }
}

fn norm(node: Node, problems: &mut Vec<Problem>) -> Option<Node> {
    match node {
        Node::Tabs(mut g) => {
            if g.panels.is_empty() {
                problems.push(Problem::EmptyGroup);
                return None;
            }
            if g.active >= g.panels.len() {
                g.active = g.panels.len() - 1;
                problems.push(Problem::BadActive);
            }
            Some(Node::Tabs(g))
        }
        Node::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let a = norm(*first, problems);
            let b = norm(*second, problems);
            match (a, b) {
                (Some(a), Some(b)) => {
                    let fixed = clamp_ratio(ratio);
                    if fixed != ratio {
                        problems.push(Problem::BadRatio);
                    }
                    Some(Node::Split {
                        axis,
                        ratio: fixed,
                        first: Box::new(a),
                        second: Box::new(b),
                    })
                }
                (Some(only), None) | (None, Some(only)) => Some(only),
                (None, None) => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Layout {
        Layout::with_root(Node::row(
            0.2,
            Node::tabs(&["files", "layers"]),
            Node::column(0.7, Node::tabs(&["view"]), Node::tabs(&["console", "log"])),
        ))
        .with_hidden(&["help"])
    }

    const KNOWN: [&str; 6] = ["files", "layers", "view", "console", "log", "help"];

    fn assert_sound(l: &Layout) {
        let mut all = l.panels();
        all.sort();
        let mut want: Vec<String> = KNOWN.iter().map(|s| s.to_string()).collect();
        want.sort();
        assert_eq!(all, want, "each panel exactly once: {l:?}");
        assert!(l.validate(&KNOWN).is_empty(), "{:?}", l.validate(&KNOWN));
    }

    #[test]
    fn a_tab_is_dropped_into_a_group() {
        let mut l = sample();
        l.move_panel(
            "view",
            DropTarget::OnGroup {
                anchor: "files".into(),
                zone: Zone::Center,
                index: Some(1),
            },
        )
        .unwrap();
        assert_eq!(
            l.group_of("view").unwrap().panels,
            ["files", "view", "layers"]
        );
        assert_eq!(l.group_of("view").unwrap().active, 1);
        assert_sound(&l);
    }

    #[test]
    fn a_side_drop_splits_and_an_emptied_group_disappears() {
        let mut l = sample();
        l.move_panel(
            "view",
            DropTarget::OnGroup {
                anchor: "console".into(),
                zone: Zone::Right,
                index: None,
            },
        )
        .unwrap();
        // the group of "view" is gone, so the column collapsed into the new row
        match l.root.as_ref().unwrap() {
            Node::Split { second, .. } => assert!(matches!(
                **second,
                Node::Split {
                    axis: Axis::Row,
                    ..
                }
            )),
            _ => panic!(),
        }
        assert_sound(&l);
    }

    #[test]
    fn the_edge_of_the_window_wraps_the_root() {
        let mut l = sample();
        l.move_panel("log", DropTarget::WindowEdge(Side::Bottom))
            .unwrap();
        match l.root.as_ref().unwrap() {
            Node::Split { axis, ratio, .. } => {
                assert_eq!(*axis, Axis::Column);
                assert_eq!(*ratio, 0.75);
            }
            _ => panic!(),
        }
        assert_sound(&l);
    }

    #[test]
    fn floating_hiding_and_coming_back_to_the_same_place() {
        let mut l = sample();
        l.float("layers", Some(Rect::new(10.0, 10.0, 300.0, 200.0)))
            .unwrap();
        assert_eq!(l.state_of("layers"), Some(PanelState::Floating));
        assert_eq!(l.group_of("files").unwrap().panels, ["files"]);
        l.dock("layers").unwrap();
        assert_eq!(l.group_of("files").unwrap().panels, ["files", "layers"]);
        l.hide("view").unwrap();
        assert_eq!(l.state_of("view"), Some(PanelState::Hidden));
        l.show("view").unwrap();
        assert_eq!(l.state_of("view"), Some(PanelState::Docked));
        assert_sound(&l);
        // the lone panel of a division comes back on the side it left
        let mut m = sample();
        m.hide("view").unwrap();
        m.show("view").unwrap();
        match m.root.as_ref().unwrap() {
            Node::Split { second, .. } => match &**second {
                Node::Split { first, .. } => assert_eq!(first.first_panel(), Some("view")),
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn folding_into_an_edge_and_back() {
        let mut l = sample();
        l.auto_hide("console", Side::Bottom).unwrap();
        assert_eq!(
            l.state_of("console"),
            Some(PanelState::AutoHidden(Side::Bottom))
        );
        assert!(!l.is_shown("console"));
        l.show("console").unwrap();
        assert_eq!(l.state_of("console"), Some(PanelState::Docked));
        assert_sound(&l);
    }

    #[test]
    fn a_locked_layout_refuses_changes_but_not_tabs_or_maximise() {
        let mut l = sample();
        l.set_locked(true);
        assert_eq!(l.hide("view"), Err(DockError::Locked));
        assert_eq!(l.float("view", None), Err(DockError::Locked));
        assert_eq!(l.reorder("layers", 0), Err(DockError::Locked));
        assert_eq!(l.set_ratio(&[], 0.5), Err(DockError::Locked));
        assert_eq!(l.auto_hide("view", Side::Left), Err(DockError::Locked));
        assert_eq!(l.dock("help"), Err(DockError::Locked));
        assert!(l.set_active("layers").is_ok());
        assert_eq!(l.group_of("files").unwrap().active, 1);
        assert!(l.maximize("view").is_ok());
        assert_sound(&l);
    }

    #[test]
    fn ratios_are_kept_in_range() {
        let mut l = sample();
        l.set_ratio(&[], 5.0).unwrap();
        l.set_ratio(&[true], -1.0).unwrap();
        match l.root.as_ref().unwrap() {
            Node::Split { ratio, second, .. } => {
                assert_eq!(*ratio, MAX_RATIO);
                assert!(matches!(**second, Node::Split { ratio: r, .. } if r == MIN_RATIO));
            }
            _ => panic!(),
        }
        l.set_ratio(&[], f32::NAN).unwrap();
        assert!(matches!(l.root, Some(Node::Split { ratio, .. }) if ratio == 0.5));
        assert_eq!(l.set_ratio(&[false], 0.5), Err(DockError::NotASplit));
    }

    #[test]
    fn maximise_needs_a_docked_panel_and_restore_clears_it() {
        let mut l = sample();
        assert_eq!(l.maximize("help"), Err(DockError::NotDocked));
        assert_eq!(l.maximize("nope"), Err(DockError::UnknownPanel));
        assert_eq!(l.toggle_maximize("view"), Ok(true));
        assert_eq!(l.maximized.as_deref(), Some("view"));
        l.hide("view").unwrap();
        assert_eq!(l.maximized, None);
        l.show("view").unwrap();
        assert_eq!(l.toggle_maximize("view"), Ok(true));
        assert_eq!(l.toggle_maximize("view"), Ok(false));
    }

    #[test]
    fn tabs_cycle_and_reorder() {
        let mut l = sample();
        assert_eq!(l.cycle("files", 1).unwrap(), "layers");
        assert_eq!(l.cycle("layers", 1).unwrap(), "files");
        assert_eq!(l.cycle("files", -1).unwrap(), "layers");
        l.reorder("layers", 0).unwrap();
        assert_eq!(l.group_of("files").unwrap().panels, ["layers", "files"]);
        l.reorder("layers", 99).unwrap();
        assert_eq!(l.group_of("files").unwrap().panels, ["files", "layers"]);
        assert_eq!(l.cycle("nope", 1), Err(DockError::UnknownPanel));
    }

    #[test]
    fn move_toward_splits_off_or_goes_to_the_edge() {
        let mut l = sample();
        l.move_toward("layers", Side::Right).unwrap();
        assert_eq!(l.group_of("files").unwrap().panels, ["files"]);
        assert_ne!(l.path_of("layers"), l.path_of("files"));
        l.move_toward("view", Side::Left).unwrap();
        assert_eq!(l.state_of("view"), Some(PanelState::Docked));
        assert_sound(&l);
        assert_eq!(l.move_toward("help", Side::Left), Err(DockError::NotDocked));
    }

    #[test]
    fn bad_drops_are_refused_and_never_lose_a_panel() {
        let mut l = sample();
        assert_eq!(
            l.move_panel(
                "view",
                DropTarget::OnGroup {
                    anchor: "help".into(),
                    zone: Zone::Center,
                    index: None
                }
            ),
            Err(DockError::NoSuchTarget)
        );
        assert_eq!(
            l.move_panel(
                "view",
                DropTarget::OnGroup {
                    anchor: "view".into(),
                    zone: Zone::Left,
                    index: None
                }
            ),
            Err(DockError::SamePanel)
        );
        assert_eq!(
            l.move_panel("zzz", DropTarget::WindowEdge(Side::Left)),
            Err(DockError::UnknownPanel)
        );
        assert_sound(&l);
        // a drop of a tab on its own group changes nothing but is allowed
        l.move_panel(
            "layers",
            DropTarget::OnGroup {
                anchor: "layers".into(),
                zone: Zone::Bottom,
                index: None,
            },
        )
        .unwrap();
        assert_sound(&l);
    }

    #[test]
    fn floating_windows_take_tabs_and_keep_their_rect_sound() {
        let mut l = sample();
        l.float("view", Some(Rect::new(f32::NAN, 0.0, 10.0, 10.0)))
            .unwrap();
        let r = l.floating[0].rect;
        assert!(r.is_sound() && r.w >= MIN_FLOAT);
        l.move_panel(
            "log",
            DropTarget::OnGroup {
                anchor: "view".into(),
                zone: Zone::Left,
                index: None,
            },
        )
        .unwrap();
        assert_eq!(l.floating[0].group.panels, ["view", "log"]);
        assert_sound(&l);
    }

    #[test]
    fn repair_fixes_everything_wrong_at_once() {
        let mut l = Layout::with_root(Node::row(
            f32::NAN,
            Node::tabs(&["files", "files", "ghost", "bad id"]),
            Node::Tabs(Group {
                panels: vec![],
                active: 3,
            }),
        ))
        .with_hidden(&["files"])
        .with_floating(&["view"], Rect::new(0.0, 0.0, 1.0, 1.0));
        l.maximized = Some("view".into());
        let problems = l.repair(&KNOWN);
        assert!(problems.contains(&Problem::DuplicatePanel("files".into())));
        assert!(problems.contains(&Problem::UnknownPanel("ghost".into())));
        assert!(problems.contains(&Problem::BadId("bad id".into())));
        assert!(problems.contains(&Problem::EmptyGroup));
        assert!(problems.contains(&Problem::BadFloatRect));
        assert!(problems.contains(&Problem::BadMaximized));
        assert!(problems.contains(&Problem::MissingPanel("layers".into())));
        assert_sound(&l);
        assert!(
            l.repair(&KNOWN).is_empty(),
            "a repaired layout has nothing left to repair"
        );
    }

    #[test]
    fn a_tree_that_is_too_deep_is_flattened() {
        let mut node = Node::tabs(&["files"]);
        for _ in 0..(MAX_DEPTH + 3) {
            node = Node::row(0.5, node, Node::tabs(&["view"]));
        }
        let mut l = Layout::with_root(node);
        let p = l.repair(&["files", "view"]);
        assert!(
            p.contains(&Problem::TooDeep)
                || p.iter().any(|x| matches!(x, Problem::DuplicatePanel(_)))
        );
        assert!(l.root.as_ref().unwrap().depth() <= MAX_DEPTH);
    }

    #[test]
    fn valid_ids() {
        assert!(valid_id("pcb.layers-2_a"));
        assert!(!valid_id(""));
        assert!(!valid_id("a b"));
        assert!(!valid_id("a(b"));
        assert!(!valid_id(&"x".repeat(49)));
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    #[test]
    fn many_random_operations_keep_every_promise() {
        // a fixed seed: the same thousand operations on every run
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut l = sample();
        for _ in 0..1000 {
            let id = KNOWN[rng.below(KNOWN.len())];
            let other = KNOWN[rng.below(KNOWN.len())];
            let side = Side::ALL[rng.below(4)];
            let _ = match rng.below(12) {
                0 => l.hide(id),
                1 => l.show(id),
                2 => l.float(id, None),
                3 => l.dock(id),
                4 => l.auto_hide(id, side),
                5 => l.move_toward(id, side),
                6 => l.move_panel(id, DropTarget::WindowEdge(side)),
                7 => l.move_panel(
                    id,
                    DropTarget::OnGroup {
                        anchor: other.into(),
                        zone: Zone::of_side(side),
                        index: None,
                    },
                ),
                8 => l.move_panel(
                    id,
                    DropTarget::OnGroup {
                        anchor: other.into(),
                        zone: Zone::Center,
                        index: Some(rng.below(4)),
                    },
                ),
                9 => l.reorder(id, rng.below(5)),
                10 => l.toggle_maximize(id).map(|_| ()),
                _ => {
                    let paths = l.root.as_ref().map(Node::group_paths).unwrap_or_default();
                    let mut path = paths
                        .get(rng.below(paths.len().max(1)))
                        .cloned()
                        .unwrap_or_default();
                    path.pop();
                    l.set_ratio(&path, (rng.below(120) as f32) / 100.0 - 0.1)
                }
            };
            assert_sound(&l);
            let before = l.clone();
            l.normalize();
            assert_eq!(before, l, "normalising twice changes nothing");
        }
    }
}
