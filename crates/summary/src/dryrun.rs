//! Dry run of a planned job: the exact steps of the plan are followed on paper, without a machine, to know how far each axis goes and
//! how long the movements take. A plan that leaves the course of the machine is found here, before a single motor turns.
//!
//! Shared by DRILLPRO, ROUTEPRO, the interface of both and the EH3D server. The steps are decoded the way the control unit reads
//! them (the step bit and the direction bit of each axis), so what is measured is what the machine would be asked to do. The program that
//! plans the job hands the steps of each item of its plan to a [`Tracker`].

use eh3d_bungard_formats::config::MachineSettings;

/// The axes of the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn letter(self) -> char {
        match self {
            Axis::X => 'X',
            Axis::Y => 'Y',
            Axis::Z => 'Z',
        }
    }
}

/// How far each axis may go from its switch, in mm (Z downwards, from the top).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Course {
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_mm: f64,
}

impl Course {
    /// The course the configuration file of the machine gives: the clip of X and Y and the deepest Z.
    pub fn from_settings(m: &MachineSettings) -> Course {
        Course {
            x_mm: m.clip_mm.0,
            y_mm: m.clip_mm.1,
            z_mm: m.z_max_depth_mm,
        }
    }
}

/// An axis that goes past its course.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Violation {
    /// The item of the plan where it happens (the first one that does).
    pub item: usize,
    pub axis: Axis,
    /// Where the axis gets to, in mm from its switch.
    pub at_mm: f64,
    /// The end of the course it passes, in mm (0 for the switch side).
    pub limit_mm: f64,
}

impl Violation {
    /// How far past the limit, in mm.
    pub fn excess_mm(&self) -> f64 {
        (self.at_mm - self.limit_mm).abs()
    }
}

/// What the plan does, measured.
#[derive(Debug, Clone, PartialEq)]
pub struct DryRun {
    /// Lowest and highest position of each axis (X, Y, Z) reached by the movements, in mm from the switches (nothing goes behind a switch, so the
    /// lowest is 0).
    pub min_mm: (f64, f64, f64),
    pub max_mm: (f64, f64, f64),
    /// Where the plan ends, in mm.
    pub end_mm: (f64, f64, f64),
    /// Steps sent (one per motor event) and the seconds the movements and the waits take (the dialogs of the operator not counted).
    pub steps: u64,
    pub seconds: f64,
    /// Items of the plan that were followed.
    pub items: usize,
    /// Steps the switches swallowed: an axis cannot go behind its switch, the control unit takes the surplus (the approach of the calibration jump asks
    /// for a few steps more than it backed off on purpose).
    pub swallowed_steps: u64,
    /// The places where an axis goes past the far end of its course (at most one per axis).
    pub violations: Vec<Violation>,
}

impl DryRun {
    pub fn within_course(&self) -> bool {
        self.violations.is_empty()
    }
}

/// The movement a step byte asks for, in steps of X, Y and Z (positive: away from the switch, down for Z).
pub fn decode(byte: u8) -> (i64, i64, i64) {
    if byte & 0x80 != 0 {
        return (0, 0, 0);
    }
    let axis = |step: u8, dir: u8| -> i64 {
        if byte & step == 0 {
            0
        } else if byte & dir != 0 {
            1
        } else {
            -1
        }
    };
    (axis(0x01, 0x02), axis(0x04, 0x08), axis(0x10, 0x40))
}

/// Follows a plan from the switches (every axis at 0) item by item and measures it against the course of the machine.
pub struct Tracker {
    at: [i64; 3],
    max: [i64; 3],
    steps: u64,
    swallowed: u64,
    us: f64,
    per_mm: [f64; 3],
    limit: [f64; 3],
    margin_mm: f64,
    items: usize,
    violations: Vec<Violation>,
}

impl Tracker {
    /// `steps_per_mm` of X, Y and Z (the corrected ones), `course` of the machine and `margin_mm`: how far past the course an axis may go
    /// before it counts (the play of the machine); it comes from the configuration.
    pub fn new(steps_per_mm: [f64; 3], course: &Course, margin_mm: f64) -> Tracker {
        Tracker {
            at: [0; 3],
            max: [0; 3],
            steps: 0,
            swallowed: 0,
            us: 0.0,
            per_mm: steps_per_mm,
            limit: [course.x_mm, course.y_mm, course.z_mm],
            margin_mm,
            items: 0,
            violations: Vec::new(),
        }
    }

    /// Follows steps: each one is the byte sent to the control unit and the microseconds it takes.
    pub fn steps(&mut self, steps: impl IntoIterator<Item = (u8, f64)>) {
        for (byte, delay_us) in steps {
            let (dx, dy, dz) = decode(byte);
            for (i, d) in [dx, dy, dz].into_iter().enumerate() {
                if d != 0 {
                    self.at[i] += d;
                    if self.at[i] < 0 {
                        // the axis is on its switch: what it asked for beyond is swallowed by the control unit
                        self.at[i] = 0;
                        self.swallowed += 1;
                    }
                    self.max[i] = self.max[i].max(self.at[i]);
                }
            }
            self.steps += 1;
            self.us += delay_us;
        }
    }

    /// A wait with no movement (a latch that waits, the spindle getting to speed).
    pub fn wait_us(&mut self, us: f64) {
        self.us += us;
    }

    /// The item of the plan that was fed is complete: the course is checked and the next item begins.
    pub fn end_item(&mut self) {
        let axes = [Axis::X, Axis::Y, Axis::Z];
        for (i, &axis) in axes.iter().enumerate() {
            if self.per_mm[i] <= 0.0 {
                continue;
            }
            let high = self.max[i] as f64 / self.per_mm[i];
            if high > self.limit[i] + self.margin_mm
                && !self.violations.iter().any(|v| v.axis == axis)
            {
                self.violations.push(Violation {
                    item: self.items,
                    axis,
                    at_mm: high,
                    limit_mm: self.limit[i],
                });
            }
        }
        self.items += 1;
    }

    /// What the plan does, measured.
    pub fn finish(self) -> DryRun {
        let per_mm = self.per_mm;
        let mm = |v: [i64; 3]| -> (f64, f64, f64) {
            let f = |i: usize| {
                if per_mm[i] > 0.0 {
                    v[i] as f64 / per_mm[i]
                } else {
                    0.0
                }
            };
            (f(0), f(1), f(2))
        };
        DryRun {
            min_mm: (0.0, 0.0, 0.0),
            max_mm: mm(self.max),
            end_mm: mm(self.at),
            steps: self.steps,
            swallowed_steps: self.swallowed,
            seconds: self.us / 1.0e6,
            items: self.items,
            violations: self.violations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // the step bytes of the control unit
    const X_POS: u8 = 0x43;
    const X_NEG: u8 = 0x01;
    const Y_POS: u8 = 0x4C;
    const Y_NEG: u8 = 0x04;
    const Z_POS: u8 = 0x50;
    const Z_NEG: u8 = 0x30;
    const PER_MM_XY: f64 = 80.0;
    const PER_MM_Z: f64 = 400.0;

    fn run_of(byte: u8, n: usize) -> Vec<(u8, f64)> {
        vec![(byte, 500.0); n]
    }

    fn course(x: f64, y: f64, z: f64) -> Course {
        Course {
            x_mm: x,
            y_mm: y,
            z_mm: z,
        }
    }

    fn tracker(c: &Course, margin: f64) -> Tracker {
        Tracker::new([PER_MM_XY, PER_MM_XY, PER_MM_Z], c, margin)
    }

    #[test]
    fn step_bytes_are_read_with_their_direction() {
        assert_eq!(decode(X_POS), (1, 0, 0));
        assert_eq!(decode(X_NEG), (-1, 0, 0));
        assert_eq!(decode(Y_POS), (0, 1, 0));
        assert_eq!(decode(Y_NEG), (0, -1, 0));
        assert_eq!(decode(Z_POS), (0, 0, 1));
        assert_eq!(decode(Z_NEG), (0, 0, -1));
        assert_eq!(
            decode(0x4B),
            (1, 0, 0),
            "a plain +X move with the Y direction bit"
        );
        assert_eq!(decode(0x80), (0, 0, 0), "a latch byte moves nothing");
    }

    #[test]
    fn a_move_inside_the_course_is_measured() {
        let n = (10.0 * PER_MM_XY) as usize;
        let c = course(100.0, 100.0, 20.0);
        let mut t = tracker(&c, 0.5);
        t.steps(run_of(X_POS, n));
        t.end_item();
        let d = t.finish();
        assert!(d.within_course());
        assert!((d.max_mm.0 - 10.0).abs() < 0.1, "{:?}", d.max_mm);
        assert!((d.end_mm.0 - 10.0).abs() < 0.1);
        assert_eq!(d.steps, n as u64);
        assert_eq!(d.items, 1);
        assert!((d.seconds - n as f64 * 500e-6).abs() < 1e-9);
    }

    #[test]
    fn a_move_past_the_far_end_is_a_violation_with_its_item() {
        let n = (30.0 * PER_MM_XY) as usize;
        let c = course(20.0, 100.0, 20.0);
        let mut t = tracker(&c, 0.5);
        t.steps(run_of(Y_POS, 10));
        t.end_item();
        t.steps(run_of(X_POS, n));
        t.end_item();
        let d = t.finish();
        assert_eq!(d.violations.len(), 1);
        let v = d.violations[0];
        assert_eq!((v.axis, v.item), (Axis::X, 1));
        assert!(
            v.excess_mm() > 9.0 && v.excess_mm() < 10.5,
            "{}",
            v.excess_mm()
        );
    }

    #[test]
    fn going_behind_the_switch_is_swallowed_not_a_violation() {
        let c = course(100.0, 100.0, 20.0);
        let mut t = tracker(&c, 0.5);
        t.steps(run_of(X_NEG, 5000));
        t.end_item();
        let d = t.finish();
        assert!(d.within_course(), "a search stops on its switch");
        assert_eq!(d.end_mm.0, 0.0);
        assert_eq!(d.swallowed_steps, 5000);
        // the same for steps that are not a search: the axis cannot go behind its switch, and coming back starts from the switch
        let mut t = tracker(&c, 0.5);
        t.steps(run_of(X_NEG, 100));
        t.end_item();
        t.steps(run_of(X_POS, 50));
        t.end_item();
        let d = t.finish();
        assert!(d.within_course());
        assert_eq!(d.swallowed_steps, 100);
        assert!((d.end_mm.0 - 50.0 / PER_MM_XY).abs() < 1e-9);
    }

    #[test]
    fn z_down_is_measured_against_the_deepest_position() {
        let n = (15.0 * PER_MM_Z) as usize;
        let c = course(100.0, 100.0, 10.0);
        let mut t = tracker(&c, 0.5);
        t.steps(run_of(Z_POS, n));
        t.end_item();
        let d = t.finish();
        assert_eq!(d.violations.len(), 1);
        assert_eq!(d.violations[0].axis, Axis::Z);
    }

    #[test]
    fn the_margin_forgives_the_play() {
        let n = (10.4 * PER_MM_XY) as usize;
        let c = course(10.0, 10.0, 10.0);
        let of = |margin| {
            let mut t = tracker(&c, margin);
            t.steps(run_of(X_POS, n));
            t.end_item();
            t.finish()
        };
        assert!(of(0.5).within_course());
        assert!(!of(0.1).within_course());
    }

    #[test]
    fn waits_add_time_but_no_steps() {
        let c = course(1.0, 1.0, 1.0);
        let mut t = tracker(&c, 0.5);
        t.wait_us(250_000.0);
        t.end_item();
        t.wait_us(750_000.0);
        t.end_item();
        let d = t.finish();
        assert_eq!((d.steps, d.items), (0, 2));
        assert!((d.seconds - 1.0).abs() < 1e-9);
    }

    #[test]
    fn an_axis_with_no_steps_per_mm_is_not_measured() {
        let c = course(1.0, 1.0, 1.0);
        let mut t = Tracker::new([0.0, PER_MM_XY, PER_MM_Z], &c, 0.0);
        t.steps(run_of(X_POS, 1000));
        t.end_item();
        let d = t.finish();
        assert!(d.within_course());
        assert_eq!(d.max_mm.0, 0.0);
    }

    #[test]
    fn the_course_comes_from_the_machine_file() {
        let m = crate::summary::tests::settings();
        let c = Course::from_settings(&m);
        assert_eq!((c.x_mm, c.y_mm), m.clip_mm);
        assert_eq!(c.z_mm, m.z_max_depth_mm);
    }
}
