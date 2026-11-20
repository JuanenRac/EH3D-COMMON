//! The plain shapes of the frame: a rectangle, the four sides, the two axes of a division and the five zones a panel can be dropped on.
//! Nothing here knows about a window, so every rule of the frame can be tested with numbers.

/// A rectangle in points: the corner at the top left and the size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// A rectangle from its corner and its size.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The right edge.
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// The bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Whether the point lies inside (the left and top edges count, the right and bottom ones do not).
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    /// Whether every number is finite and the size is positive.
    pub fn is_sound(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.w.is_finite()
            && self.h.is_finite()
            && self.w > 0.0
            && self.h > 0.0
    }

    /// The rectangle shrunk by `m` on every side (never below zero in size).
    pub fn shrink(&self, m: f32) -> Rect {
        let w = (self.w - 2.0 * m).max(0.0);
        let h = (self.h - 2.0 * m).max(0.0);
        Rect::new(
            self.x + (self.w - w) / 2.0,
            self.y + (self.h - h) / 2.0,
            w,
            h,
        )
    }
}

/// One of the four sides of a rectangle or of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The left side.
    Left,
    /// The right side.
    Right,
    /// The top side.
    Top,
    /// The bottom side.
    Bottom,
}

impl Side {
    /// The four sides in a fixed order.
    pub const ALL: [Side; 4] = [Side::Left, Side::Right, Side::Top, Side::Bottom];

    /// The word the files of the layouts use.
    pub fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
            Side::Top => "top",
            Side::Bottom => "bottom",
        }
    }

    /// The side with that word.
    pub fn from_name(name: &str) -> Option<Side> {
        Side::ALL.into_iter().find(|s| s.name() == name)
    }

    /// The axis along which a division at this side is cut (left and right put things in a row).
    pub fn axis(self) -> Axis {
        match self {
            Side::Left | Side::Right => Axis::Row,
            Side::Top | Side::Bottom => Axis::Column,
        }
    }

    /// Whether the new part goes first (left or top) in the division.
    pub fn is_first(self) -> bool {
        matches!(self, Side::Left | Side::Top)
    }
}

/// The direction in which a division puts its two parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Side by side, the first on the left (the divider is a vertical line).
    Row,
    /// One above the other, the first on top (the divider is a horizontal line).
    Column,
}

/// Where inside a group of tabs a panel is dropped: in the middle it becomes a tab, near a side it splits the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    /// As one more tab of the group.
    Center,
    /// A new part on the left of the group.
    Left,
    /// A new part on the right of the group.
    Right,
    /// A new part above the group.
    Top,
    /// A new part below the group.
    Bottom,
}

impl Zone {
    /// The word the files of the layouts use.
    pub fn name(self) -> &'static str {
        match self {
            Zone::Center => "center",
            Zone::Left => "left",
            Zone::Right => "right",
            Zone::Top => "top",
            Zone::Bottom => "bottom",
        }
    }

    /// The zone with that word.
    pub fn from_name(name: &str) -> Option<Zone> {
        [
            Zone::Center,
            Zone::Left,
            Zone::Right,
            Zone::Top,
            Zone::Bottom,
        ]
        .into_iter()
        .find(|z| z.name() == name)
    }

    /// The side of a zone (the middle has none).
    pub fn side(self) -> Option<Side> {
        match self {
            Zone::Center => None,
            Zone::Left => Some(Side::Left),
            Zone::Right => Some(Side::Right),
            Zone::Top => Some(Side::Top),
            Zone::Bottom => Some(Side::Bottom),
        }
    }

    /// The zone of a side.
    pub fn of_side(side: Side) -> Zone {
        match side {
            Side::Left => Zone::Left,
            Side::Right => Zone::Right,
            Side::Top => Zone::Top,
            Side::Bottom => Zone::Bottom,
        }
    }
}

/// The zone of `rect` under the point, or `None` when the point is outside. The middle half is the centre; the rest belongs to the nearest side.
pub fn drop_zone(rect: Rect, px: f32, py: f32) -> Option<Zone> {
    if !rect.is_sound() || !rect.contains(px, py) {
        return None;
    }
    let u = (px - rect.x) / rect.w;
    let v = (py - rect.y) / rect.h;
    if (0.25..0.75).contains(&u) && (0.25..0.75).contains(&v) {
        return Some(Zone::Center);
    }
    // the nearest edge wins; on a tie the horizontal ones do
    let candidates = [
        (u, Zone::Left),
        (1.0 - u, Zone::Right),
        (v, Zone::Top),
        (1.0 - v, Zone::Bottom),
    ];
    let mut best = candidates[0];
    for c in &candidates[1..] {
        if c.0 < best.0 {
            best = *c;
        }
    }
    Some(best.1)
}

/// The part of `rect` a drop on `zone` would take: all of it for the centre, one half for a side.
pub fn zone_preview(rect: Rect, zone: Zone) -> Rect {
    match zone {
        Zone::Center => rect,
        Zone::Left => Rect::new(rect.x, rect.y, rect.w / 2.0, rect.h),
        Zone::Right => Rect::new(rect.x + rect.w / 2.0, rect.y, rect.w / 2.0, rect.h),
        Zone::Top => Rect::new(rect.x, rect.y, rect.w, rect.h / 2.0),
        Zone::Bottom => Rect::new(rect.x, rect.y + rect.h / 2.0, rect.w, rect.h / 2.0),
    }
}

/// The side of the window whose edge strip (of `margin` points) holds the point, if any; the nearest edge wins in a corner.
pub fn edge_zone(area: Rect, px: f32, py: f32, margin: f32) -> Option<Side> {
    if !area.is_sound() || !area.contains(px, py) {
        return None;
    }
    let d = [
        (px - area.x, Side::Left),
        (area.right() - px, Side::Right),
        (py - area.y, Side::Top),
        (area.bottom() - py, Side::Bottom),
    ];
    let mut best = d[0];
    for c in &d[1..] {
        if c.0 < best.0 {
            best = *c;
        }
    }
    (best.0 < margin).then_some(best.1)
}

/// The strip a drop on the edge of the window would take: a quarter of the window along that side.
pub fn edge_preview(area: Rect, side: Side) -> Rect {
    match side {
        Side::Left => Rect::new(area.x, area.y, area.w / 4.0, area.h),
        Side::Right => Rect::new(area.x + area.w * 0.75, area.y, area.w / 4.0, area.h),
        Side::Top => Rect::new(area.x, area.y, area.w, area.h / 4.0),
        Side::Bottom => Rect::new(area.x, area.y + area.h * 0.75, area.w, area.h / 4.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_is_a_tab_and_the_edges_are_sides() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(drop_zone(r, 50.0, 50.0), Some(Zone::Center));
        assert_eq!(drop_zone(r, 5.0, 50.0), Some(Zone::Left));
        assert_eq!(drop_zone(r, 95.0, 50.0), Some(Zone::Right));
        assert_eq!(drop_zone(r, 50.0, 5.0), Some(Zone::Top));
        assert_eq!(drop_zone(r, 50.0, 95.0), Some(Zone::Bottom));
        assert_eq!(drop_zone(r, 150.0, 50.0), None);
        assert_eq!(drop_zone(Rect::new(0.0, 0.0, 0.0, 10.0), 0.0, 0.0), None);
    }

    #[test]
    fn a_corner_goes_to_the_nearest_edge() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(drop_zone(r, 10.0, 20.0), Some(Zone::Left));
        assert_eq!(drop_zone(r, 20.0, 10.0), Some(Zone::Top));
    }

    #[test]
    fn previews_stay_inside_the_rectangle() {
        let r = Rect::new(10.0, 20.0, 200.0, 100.0);
        for z in [
            Zone::Center,
            Zone::Left,
            Zone::Right,
            Zone::Top,
            Zone::Bottom,
        ] {
            let p = zone_preview(r, z);
            assert!(
                p.x >= r.x
                    && p.y >= r.y
                    && p.right() <= r.right() + 1e-3
                    && p.bottom() <= r.bottom() + 1e-3
            );
        }
        for s in Side::ALL {
            let p = edge_preview(r, s);
            assert!(p.right() <= r.right() + 1e-3 && p.bottom() <= r.bottom() + 1e-3);
        }
    }

    #[test]
    fn the_edge_strip_of_the_window() {
        let a = Rect::new(0.0, 0.0, 400.0, 300.0);
        assert_eq!(edge_zone(a, 3.0, 150.0, 24.0), Some(Side::Left));
        assert_eq!(edge_zone(a, 397.0, 150.0, 24.0), Some(Side::Right));
        assert_eq!(edge_zone(a, 200.0, 2.0, 24.0), Some(Side::Top));
        assert_eq!(edge_zone(a, 200.0, 299.0, 24.0), Some(Side::Bottom));
        assert_eq!(edge_zone(a, 200.0, 150.0, 24.0), None);
        assert_eq!(edge_zone(a, 500.0, 150.0, 24.0), None);
    }

    #[test]
    fn names_round_trip() {
        for s in Side::ALL {
            assert_eq!(Side::from_name(s.name()), Some(s));
            assert_eq!(Zone::of_side(s).side(), Some(s));
        }
        assert_eq!(Zone::from_name("center"), Some(Zone::Center));
        assert_eq!(Zone::from_name("nope"), None);
        assert_eq!(Side::Left.axis(), Axis::Row);
        assert_eq!(Side::Bottom.axis(), Axis::Column);
        assert!(Side::Top.is_first() && !Side::Right.is_first());
    }

    #[test]
    fn shrink_never_goes_negative() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0).shrink(20.0);
        assert!(r.w == 0.0 && r.h == 0.0);
        assert!(!r.is_sound());
    }
}
