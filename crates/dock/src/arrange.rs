//! From a tree to rectangles: where every group of tabs goes in an area, where the dividers are, how far a divider may be dragged, and the
//! smallest size a part needs. Pure numbers, so the geometry of the window is tested without a window.

use crate::geom::{drop_zone, edge_preview, edge_zone, zone_preview, Axis, Rect, Zone};
use crate::model::{DropTarget, Node, Path, MAX_RATIO, MIN_RATIO};

/// A group of tabs and the rectangle it fills.
#[derive(Debug, Clone, PartialEq)]
pub struct Leaf {
    /// The path of the group in the tree.
    pub path: Path,
    /// Its rectangle.
    pub rect: Rect,
}

/// A divider between the two parts of a division.
#[derive(Debug, Clone, PartialEq)]
pub struct Divider {
    /// The path of the division.
    pub path: Path,
    /// The direction in which the division puts its parts.
    pub axis: Axis,
    /// The strip the divider takes.
    pub rect: Rect,
    /// The rectangle of the whole division, which the share is measured on.
    pub parent: Rect,
}

/// The result of laying a tree out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Arranged {
    /// The groups.
    pub leaves: Vec<Leaf>,
    /// The dividers.
    pub dividers: Vec<Divider>,
}

/// The smallest size (width, height) a node needs, given the smallest size of each panel and the width of a divider.
pub fn min_size(node: &Node, panel_min: &dyn Fn(&str) -> (f32, f32), gap: f32) -> (f32, f32) {
    match node {
        Node::Tabs(g) => g.panels.iter().fold((0.0_f32, 0.0_f32), |acc, p| {
            let m = panel_min(p);
            (acc.0.max(m.0), acc.1.max(m.1))
        }),
        Node::Split {
            axis,
            first,
            second,
            ..
        } => {
            let a = min_size(first, panel_min, gap);
            let b = min_size(second, panel_min, gap);
            match axis {
                Axis::Row => (a.0 + b.0 + gap, a.1.max(b.1)),
                Axis::Column => (a.0.max(b.0), a.1 + b.1 + gap),
            }
        }
    }
}

/// The extent of the first part of a division of `total` points (the divider already taken off), kept so that neither part is smaller than its
/// minimum when that is possible, and otherwise as the share says.
pub fn first_extent(total: f32, ratio: f32, min_first: f32, min_second: f32) -> f32 {
    if !(total.is_finite() && total > 0.0) {
        return 0.0;
    }
    let want = total * ratio.clamp(MIN_RATIO, MAX_RATIO);
    let lo = min_first.max(0.0);
    let hi = total - min_second.max(0.0);
    if lo <= hi {
        want.clamp(lo, hi)
    } else {
        want
    }
}

/// Lays the tree out in `area`: a divider takes `gap` points and no part is smaller than the minimum of its panels when there is room.
pub fn arrange(
    root: &Node,
    area: Rect,
    gap: f32,
    panel_min: &dyn Fn(&str) -> (f32, f32),
) -> Arranged {
    let mut out = Arranged::default();
    go(root, area, gap, panel_min, &mut Vec::new(), &mut out);
    out
}

fn go(
    node: &Node,
    area: Rect,
    gap: f32,
    panel_min: &dyn Fn(&str) -> (f32, f32),
    path: &mut Path,
    out: &mut Arranged,
) {
    match node {
        Node::Tabs(_) => out.leaves.push(Leaf {
            path: path.clone(),
            rect: area,
        }),
        Node::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let ma = min_size(first, panel_min, gap);
            let mb = min_size(second, panel_min, gap);
            let (ra, rd, rb) = match axis {
                Axis::Row => {
                    let total = (area.w - gap).max(0.0);
                    let a = first_extent(total, *ratio, ma.0, mb.0);
                    (
                        Rect::new(area.x, area.y, a, area.h),
                        Rect::new(area.x + a, area.y, gap.min(area.w), area.h),
                        Rect::new(area.x + a + gap, area.y, (total - a).max(0.0), area.h),
                    )
                }
                Axis::Column => {
                    let total = (area.h - gap).max(0.0);
                    let a = first_extent(total, *ratio, ma.1, mb.1);
                    (
                        Rect::new(area.x, area.y, area.w, a),
                        Rect::new(area.x, area.y + a, area.w, gap.min(area.h)),
                        Rect::new(area.x, area.y + a + gap, area.w, (total - a).max(0.0)),
                    )
                }
            };
            out.dividers.push(Divider {
                path: path.clone(),
                axis: *axis,
                rect: rd,
                parent: area,
            });
            path.push(false);
            go(first, ra, gap, panel_min, path, out);
            path.pop();
            path.push(true);
            go(second, rb, gap, panel_min, path, out);
            path.pop();
        }
    }
}

/// The share a divider gets when the pointer is at `pointer` (the coordinate along the axis), kept inside what the minimum sizes allow.
pub fn ratio_from_pointer(
    parent: Rect,
    axis: Axis,
    pointer: f32,
    gap: f32,
    min_first: f32,
    min_second: f32,
) -> f32 {
    let (start, extent) = match axis {
        Axis::Row => (parent.x, parent.w),
        Axis::Column => (parent.y, parent.h),
    };
    let total = extent - gap;
    if !(total.is_finite() && total > 0.0) || !pointer.is_finite() {
        return 0.5;
    }
    let lo = min_first.max(0.0);
    let hi = total - min_second.max(0.0);
    let pos = pointer - start - gap / 2.0;
    let pos = if lo <= hi { pos.clamp(lo, hi) } else { pos };
    (pos / total).clamp(MIN_RATIO, MAX_RATIO)
}

/// A place a panel can be dropped on while it is dragged: a group of tabs, docked or floating.
#[derive(Debug, Clone, PartialEq)]
pub struct DropSlot {
    /// The panels of the group, in the order of the tabs.
    pub panels: Vec<String>,
    /// The rectangle of the group, tab bar included.
    pub rect: Rect,
    /// The rectangles of its tabs.
    pub tabs: Vec<Rect>,
    /// The height of the tab bar.
    pub bar_h: f32,
    /// Whether the group is in a floating window.
    pub floating: bool,
}

/// What dropping the dragged panel at (`x`, `y`) would do, and the rectangle to show as a preview. Floating windows are looked at first (the last
/// of `slots` is the one on top), then a pointer outside the window (`area`) floats the panel, then the strips along the edges of the window, then
/// the docked groups. `None`: nothing sensible to do there (a panel dropped on the group it is alone in).
pub fn pick_drop(
    slots: &[DropSlot],
    area: Rect,
    x: f32,
    y: f32,
    dragged: &str,
    edge_margin: f32,
) -> Option<(DropTarget, Rect)> {
    let anchor_of = |s: &DropSlot| s.panels.iter().find(|p| *p != dragged).cloned();
    let alone = |s: &DropSlot| s.panels.len() == 1 && s.panels[0] == dragged;
    let index_in = |s: &DropSlot| -> Option<usize> {
        if y < s.rect.y + s.bar_h {
            Some(s.tabs.iter().filter(|t| t.x + t.w / 2.0 < x).count())
        } else {
            None
        }
    };
    for s in slots.iter().rev().filter(|s| s.floating) {
        if s.rect.contains(x, y) {
            if alone(s) {
                return None;
            }
            let anchor = anchor_of(s)?;
            return Some((
                DropTarget::OnGroup {
                    anchor,
                    zone: Zone::Center,
                    index: index_in(s),
                },
                s.rect,
            ));
        }
    }
    if !area.contains(x, y) {
        let rect = Rect::new(x - 40.0, y - 10.0, 360.0, 260.0);
        return Some((DropTarget::Float(rect), rect));
    }
    if let Some(side) = edge_zone(area, x, y, edge_margin) {
        return Some((DropTarget::WindowEdge(side), edge_preview(area, side)));
    }
    for s in slots.iter().filter(|s| !s.floating) {
        if let Some(zone) = drop_zone(s.rect, x, y) {
            // over the tab bar the drop is always a tab, whatever the zone says
            let zone = if y < s.rect.y + s.bar_h {
                Zone::Center
            } else {
                zone
            };
            if alone(s) {
                return None;
            }
            let anchor = anchor_of(s)?;
            let index = if zone == Zone::Center {
                index_in(s)
            } else {
                None
            };
            return Some((
                DropTarget::OnGroup {
                    anchor,
                    zone,
                    index,
                },
                zone_preview(s.rect, zone),
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_min(_: &str) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn sample() -> Node {
        Node::row(
            0.25,
            Node::tabs(&["a"]),
            Node::column(0.5, Node::tabs(&["b"]), Node::tabs(&["c"])),
        )
    }

    #[test]
    fn the_parts_fill_the_area_without_overlap() {
        let area = Rect::new(0.0, 0.0, 1004.0, 504.0);
        let r = arrange(&sample(), area, 4.0, &no_min);
        assert_eq!(r.leaves.len(), 3);
        assert_eq!(r.dividers.len(), 2);
        let a = &r.leaves[0].rect;
        let b = &r.leaves[1].rect;
        let c = &r.leaves[2].rect;
        assert_eq!(a.w, 250.0);
        assert_eq!(b.x, 254.0);
        assert_eq!(b.w, 750.0);
        assert_eq!(b.h, 250.0);
        assert_eq!(c.y, 254.0);
        assert!((c.bottom() - area.bottom()).abs() < 1e-3);
        assert!((b.right() - area.right()).abs() < 1e-3);
    }

    #[test]
    fn minimum_sizes_win_over_the_share_when_there_is_room() {
        let min = |p: &str| {
            if p == "a" {
                (400.0, 10.0)
            } else {
                (50.0, 10.0)
            }
        };
        let r = arrange(&sample(), Rect::new(0.0, 0.0, 1004.0, 504.0), 4.0, &min);
        assert_eq!(r.leaves[0].rect.w, 400.0);
        // not enough room for both: the share is kept and nothing panics
        let tight = arrange(&sample(), Rect::new(0.0, 0.0, 100.0, 100.0), 4.0, &min);
        assert_eq!(tight.leaves.len(), 3);
        assert!(tight
            .leaves
            .iter()
            .all(|l| l.rect.w >= 0.0 && l.rect.h >= 0.0));
    }

    #[test]
    fn min_size_adds_up_along_the_axis() {
        let min = |_: &str| (100.0, 50.0);
        assert_eq!(min_size(&sample(), &min, 4.0), (204.0, 104.0));
    }

    #[test]
    fn the_pointer_gives_a_share_inside_the_limits() {
        let p = Rect::new(0.0, 0.0, 1004.0, 500.0);
        assert!((ratio_from_pointer(p, Axis::Row, 502.0, 4.0, 0.0, 0.0) - 0.5).abs() < 1e-3);
        assert_eq!(
            ratio_from_pointer(p, Axis::Row, -50.0, 4.0, 0.0, 0.0),
            MIN_RATIO
        );
        assert_eq!(
            ratio_from_pointer(p, Axis::Row, 5000.0, 4.0, 0.0, 0.0),
            MAX_RATIO
        );
        // the first part may not go below 300 points
        let r = ratio_from_pointer(p, Axis::Row, 10.0, 4.0, 300.0, 100.0);
        assert!((r - 0.3).abs() < 1e-3, "{r}");
        assert_eq!(
            ratio_from_pointer(p, Axis::Row, f32::NAN, 4.0, 0.0, 0.0),
            0.5
        );
        assert_eq!(
            ratio_from_pointer(Rect::new(0.0, 0.0, 2.0, 2.0), Axis::Row, 1.0, 4.0, 0.0, 0.0),
            0.5
        );
    }

    fn slot(panels: &[&str], rect: Rect, floating: bool) -> DropSlot {
        let n = panels.len();
        DropSlot {
            panels: panels.iter().map(|s| s.to_string()).collect(),
            rect,
            tabs: (0..n)
                .map(|i| Rect::new(rect.x + 80.0 * i as f32, rect.y, 80.0, 22.0))
                .collect(),
            bar_h: 22.0,
            floating,
        }
    }

    #[test]
    fn dropping_picks_a_tab_a_side_an_edge_or_a_window() {
        let area = Rect::new(0.0, 0.0, 1000.0, 600.0);
        let slots = vec![
            slot(&["a", "b"], Rect::new(10.0, 40.0, 300.0, 500.0), false),
            slot(&["c"], Rect::new(320.0, 40.0, 600.0, 500.0), false),
            slot(&["f", "g"], Rect::new(500.0, 200.0, 200.0, 150.0), true),
        ];
        // on the tab bar of a group, between its tabs: a tab at that place
        let (t, _) = pick_drop(&slots, area, 100.0, 50.0, "c", 24.0).unwrap();
        assert_eq!(
            t,
            DropTarget::OnGroup {
                anchor: "a".into(),
                zone: Zone::Center,
                index: Some(1)
            }
        );
        // in the middle of the content: at the end
        let (t, p) = pick_drop(&slots, area, 160.0, 290.0, "c", 24.0).unwrap();
        assert_eq!(
            t,
            DropTarget::OnGroup {
                anchor: "a".into(),
                zone: Zone::Center,
                index: None
            }
        );
        assert_eq!(p, slots[0].rect);
        // near the left side of the group: a new part
        let (t, p) = pick_drop(&slots, area, 40.0, 290.0, "c", 24.0).unwrap();
        assert_eq!(
            t,
            DropTarget::OnGroup {
                anchor: "a".into(),
                zone: Zone::Left,
                index: None
            }
        );
        assert_eq!(p.w, 150.0);
        // the strip along the window edge wins over the group
        let (t, _) = pick_drop(&slots, area, 5.0, 290.0, "c", 24.0).unwrap();
        assert_eq!(t, DropTarget::WindowEdge(crate::geom::Side::Left));
        // a floating window is on top of the group below it
        let (t, _) = pick_drop(&slots, area, 600.0, 300.0, "a", 24.0).unwrap();
        assert_eq!(
            t,
            DropTarget::OnGroup {
                anchor: "f".into(),
                zone: Zone::Center,
                index: None
            }
        );
        // outside the window: float
        let (t, _) = pick_drop(&slots, area, 1200.0, 300.0, "a", 24.0).unwrap();
        assert!(matches!(t, DropTarget::Float(_)));
    }

    #[test]
    fn a_panel_alone_in_its_group_cannot_be_dropped_on_it() {
        let area = Rect::new(0.0, 0.0, 1000.0, 600.0);
        let slots = vec![slot(&["c"], Rect::new(100.0, 100.0, 600.0, 400.0), false)];
        assert_eq!(pick_drop(&slots, area, 400.0, 300.0, "c", 24.0), None);
        assert_eq!(pick_drop(&slots, area, 120.0, 300.0, "c", 24.0), None);
        // with another tab it anchors on the other one
        let two = vec![slot(
            &["c", "d"],
            Rect::new(100.0, 100.0, 600.0, 400.0),
            false,
        )];
        let (t, _) = pick_drop(&two, area, 120.0, 300.0, "c", 24.0).unwrap();
        assert_eq!(
            t,
            DropTarget::OnGroup {
                anchor: "d".into(),
                zone: Zone::Left,
                index: None
            }
        );
    }

    #[test]
    fn every_drop_that_is_picked_is_accepted_by_the_layout() {
        use crate::model::Layout;
        let area = Rect::new(0.0, 0.0, 1000.0, 600.0);
        let mut seed = 0x1234_5678_9ABC_DEF0u64;
        let mut rnd = move |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        let ids = ["a", "b", "c", "d", "e"];
        let mut l = Layout::with_root(Node::row(
            0.3,
            Node::tabs(&["a", "b"]),
            Node::column(0.5, Node::tabs(&["c"]), Node::tabs(&["d"])),
        ))
        .with_hidden(&["e"]);
        for _ in 0..500 {
            let mut slots: Vec<DropSlot> = Vec::new();
            if let Some(root) = l.root.as_ref() {
                let r = arrange(root, area.shrink(2.0), 4.0, &no_min);
                for leaf in &r.leaves {
                    if let Some(Node::Tabs(g)) = root.get(&leaf.path) {
                        let names: Vec<&str> = g.panels.iter().map(String::as_str).collect();
                        slots.push(slot(&names, leaf.rect, false));
                    }
                }
            }
            for f in &l.floating {
                let names: Vec<&str> = f.group.panels.iter().map(String::as_str).collect();
                slots.push(slot(&names, f.rect, true));
            }
            let dragged = ids[rnd(5) as usize];
            if !l.is_shown(dragged) {
                let _ = l.show(dragged);
            }
            let (x, y) = (rnd(1100) as f32 - 50.0, rnd(700) as f32 - 50.0);
            if let Some((t, _)) = pick_drop(&slots, area, x, y, dragged, 24.0) {
                if let Err(e) = l.move_panel(dragged, t.clone()) {
                    panic!("a picked drop was refused ({e}): {t:?}");
                }
                assert!(l.validate(&ids).is_empty(), "{t:?}");
            }
        }
    }

    #[test]
    fn first_extent_survives_nonsense() {
        assert_eq!(first_extent(f32::NAN, 0.5, 0.0, 0.0), 0.0);
        assert_eq!(first_extent(-5.0, 0.5, 0.0, 0.0), 0.0);
        assert_eq!(first_extent(100.0, 0.5, 0.0, 0.0), 50.0);
    }
}
