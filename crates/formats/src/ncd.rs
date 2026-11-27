//! Excellon drill files (`.NCD`): imperial, integer coordinates in mils.
//!
//! Real-world files come in two shapes: a bare body (`%`, `T06`, `X..Y..`) and
//! an Excellon header (`M48 ... %`) with optional tool definitions (`T1C0.8`).

use std::fmt;

use crate::error::{Code, FormatError};

/// One mil is 0.001 inch.
pub const MIL_MM: f64 = 0.0254;

#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    pub tool: u8,
    pub x_mil: i32,
    pub y_mil: i32,
}

impl Hole {
    pub fn x_mm(&self) -> f64 {
        self.x_mil as f64 * MIL_MM
    }
    pub fn y_mm(&self) -> f64 {
        self.y_mil as f64 * MIL_MM
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct DrillFile {
    pub holes: Vec<Hole>,
    /// Tools in order of first use.
    pub tools: Vec<u8>,
    /// Diameters declared in the header (`T1C0.8`), as `(tool, value)` in the file's unit.
    pub diameters: Vec<(u8, f64)>,
    /// True when the header declares metric units.
    pub metric: bool,
    /// True when the file says its unit (`METRIC`, `INCH`, `M71` or `M72`). When it does not, the numbers are read as mils and a program should say that the unit was
    /// supposed, not declared: a file in inches of 2.4 digits without a header comes out ten times larger.
    pub units_declared: bool,
    /// The lines that move the head in ways a drill file of holes does not hold (a slot written as `G00`/`G01`, `M15`/`M16` up and down) and are not part of the holes:
    /// they are named, not dropped without a word.
    pub ignored: Vec<Ignored>,
    /// How many more such lines there were after the first [`MAX_IGNORED`]: they are counted, not kept, so that a file of millions of them cannot fill the memory.
    pub ignored_more: usize,
}

/// The most lines that [`DrillFile::ignored`] keeps.
pub const MAX_IGNORED: usize = 10_000;

/// A line of the file that is not a hole and was left out: its number (from 1) and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct Ignored {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    BadLine {
        line: usize,
        text: String,
    },
    HoleBeforeTool {
        line: usize,
    },
    /// The file is past a hard limit of the readers (`reader_limits.cfg`): it is refused before it is read.
    TooLarge {
        detail: String,
    },
}

impl ParseError {
    /// The stable code of the problem.
    pub fn code(&self) -> Code {
        match self {
            ParseError::BadLine { .. } => Code::NcdBadLine,
            ParseError::HoleBeforeTool { .. } => Code::NcdHoleBeforeTool,
            ParseError::TooLarge { .. } => Code::FileTooLarge,
        }
    }

    /// The line of the problem, counted from 1.
    pub fn line(&self) -> usize {
        match self {
            ParseError::BadLine { line, .. } | ParseError::HoleBeforeTool { line } => *line,
            ParseError::TooLarge { .. } => 0,
        }
    }

    /// The error with its code, its line and, for a line that cannot be read, the column where the statement starts (after the blanks).
    pub fn to_error(&self) -> FormatError {
        match self {
            ParseError::BadLine { line, text } => FormatError {
                code: Code::NcdBadLine,
                line: Some(*line),
                column: Some(1 + text.chars().take_while(|c| c.is_whitespace()).count()),
                detail: text.trim().to_string(),
            },
            ParseError::HoleBeforeTool { line } => FormatError {
                code: Code::NcdHoleBeforeTool,
                line: Some(*line),
                column: None,
                detail: String::new(),
            },
            ParseError::TooLarge { detail } => FormatError {
                code: Code::FileTooLarge,
                line: None,
                column: None,
                detail: detail.clone(),
            },
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_error())
    }
}

impl std::error::Error for ParseError {}

/// Keeps a line that is left out, up to [`MAX_IGNORED`]; the rest are only counted.
fn note_ignored(out: &mut DrillFile, line: usize, raw: &str) {
    if out.ignored.len() < MAX_IGNORED {
        out.ignored.push(Ignored {
            line,
            text: raw.trim().into(),
        });
    } else {
        out.ignored_more += 1;
    }
}

/// How the coordinates of a file with a header are written: the unit, the digits and which zeros are left out.
/// The most digits on either side of the point that a `;FILE_FORMAT=` line or a `000.000` option may ask for.
const MAX_FORMAT_DIGITS: u32 = 6;

/// The header lines by which the suite declares that the numbers of a file it writes are whole mils: inches with three decimals and the zeros in front left out. Every reader of
/// drill files takes them for what they say; they go inside the header (after `M48`).
pub const MILS_DECLARATION: &str = ";FILE_FORMAT=3:3\nINCH,TZ\n";

/// How the numbers of a drill file are written, as the person chooses it for a file that does not say it. What a file declares always wins over the choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberFormat {
    /// Whole mils (0.001 inch), as the files of the Bungard machines and their programs have them.
    Mils,
    /// The digits before and after the point of a number written without one, and the side whose zeros are left out.
    Digits {
        metric: bool,
        integer: u8,
        decimal: u8,
        /// True for `LZ` (the zeros at the end are left out); false for `TZ` (the zeros in front are left out).
        trailing_left_out: bool,
    },
}

impl NumberFormat {
    /// The choices a program offers in a list. Other digits can be written by name (`inch-2.5-tz`).
    pub const PRESETS: [NumberFormat; 5] = [
        NumberFormat::Mils,
        NumberFormat::Digits {
            metric: false,
            integer: 2,
            decimal: 4,
            trailing_left_out: false,
        },
        NumberFormat::Digits {
            metric: false,
            integer: 2,
            decimal: 4,
            trailing_left_out: true,
        },
        NumberFormat::Digits {
            metric: true,
            integer: 3,
            decimal: 3,
            trailing_left_out: false,
        },
        NumberFormat::Digits {
            metric: true,
            integer: 3,
            decimal: 3,
            trailing_left_out: true,
        },
    ];

    /// The name that a command line and a configuration use: `mils`, or `inch-2.4-tz`, `inch-2.4-lz`, `metric-3.3-tz`... (`tz` and `lz` as the header words of the file).
    pub fn name(self) -> String {
        match self {
            NumberFormat::Mils => "mils".into(),
            NumberFormat::Digits {
                metric,
                integer,
                decimal,
                trailing_left_out,
            } => format!(
                "{}-{integer}.{decimal}-{}",
                if metric { "metric" } else { "inch" },
                if trailing_left_out { "lz" } else { "tz" }
            ),
        }
    }

    /// The format with the name `text` ([`NumberFormat::name`]), with one to six digits on each side; `None` for any other text.
    pub fn from_name(text: &str) -> Option<NumberFormat> {
        let text = text.trim().to_ascii_lowercase();
        if text == "mils" {
            return Some(NumberFormat::Mils);
        }
        let mut parts = text.split('-');
        let metric = match parts.next()? {
            "inch" => false,
            "metric" => true,
            _ => return None,
        };
        let (integer, decimal) = parts.next()?.split_once('.')?;
        let digits = |t: &str| {
            t.parse::<u8>()
                .ok()
                .filter(|n| (1..=MAX_FORMAT_DIGITS).contains(&u32::from(*n)))
        };
        let (integer, decimal) = (digits(integer)?, digits(decimal)?);
        let trailing_left_out = match parts.next()? {
            "tz" => false,
            "lz" => true,
            _ => return None,
        };
        parts.next().is_none().then_some(NumberFormat::Digits {
            metric,
            integer,
            decimal,
            trailing_left_out,
        })
    }

    /// The value of a setting: empty or `ask` is no choice (a file that does not say its format is refused), else [`NumberFormat::from_name`]. The error is the text
    /// that is none of them.
    pub fn from_setting(text: &str) -> Result<Option<NumberFormat>, String> {
        let t = text.trim();
        if t.is_empty() || t.eq_ignore_ascii_case("ask") {
            return Ok(None);
        }
        NumberFormat::from_name(t)
            .map(Some)
            .ok_or_else(|| t.to_string())
    }

    /// The value of the setting for `chosen` ([`NumberFormat::from_setting`]).
    pub fn setting(chosen: Option<NumberFormat>) -> String {
        chosen.map_or_else(|| "ask".into(), NumberFormat::name)
    }
}

/// What a program says after it refused a file whose format is not declared (`EH3D-FORMATS-014`): the names to choose from, in the language of the code `lang`
/// (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`; English for any other). The program adds how it takes the choice (its option or its menu).
pub fn how_to_choose(lang: &str) -> &'static str {
    match lang {
        "es" => "El archivo no declara cómo están escritos sus números y no se adivina la escala. Elija el formato: mils (milésimas de pulgada enteras), inch-2.4-tz, inch-2.4-lz, metric-3.3-tz o metric-3.3-lz (tz: se omiten los ceros de delante; lz: se omiten los ceros del final)",
        "de" => "Die Datei gibt nicht an, wie ihre Zahlen geschrieben sind, und der Maßstab wird nicht geraten. Wählen Sie das Format: mils (ganze Tausendstel Zoll), inch-2.4-tz, inch-2.4-lz, metric-3.3-tz oder metric-3.3-lz (tz: führende Nullen fehlen; lz: Nullen am Ende fehlen)",
        "fr" => "Le fichier ne déclare pas comment ses nombres sont écrits et l'échelle n'est pas devinée. Choisissez le format : mils (millièmes de pouce entiers), inch-2.4-tz, inch-2.4-lz, metric-3.3-tz ou metric-3.3-lz (tz : les zéros de tête sont omis ; lz : les zéros de fin sont omis)",
        "it" => "Il file non dichiara come sono scritti i suoi numeri e la scala non viene indovinata. Scegliere il formato: mils (millesimi di pollice interi), inch-2.4-tz, inch-2.4-lz, metric-3.3-tz o metric-3.3-lz (tz: gli zeri iniziali sono omessi; lz: gli zeri finali sono omessi)",
        "pt" => "O ficheiro não declara como estão escritos os seus números e a escala não é adivinhada. Escolha o formato: mils (milésimas de polegada inteiras), inch-2.4-tz, inch-2.4-lz, metric-3.3-tz ou metric-3.3-lz (tz: os zeros à esquerda são omitidos; lz: os zeros no fim são omitidos)",
        "zh" => "文件未声明数值的书写方式，不会猜测比例。请选择格式：mils（整数千分之一英寸）、inch-2.4-tz、inch-2.4-lz、metric-3.3-tz 或 metric-3.3-lz（tz：省略前导零；lz：省略末尾零）",
        "ja" => "ファイルに数値の書式が宣言されていないため、縮尺は推定しません。書式を選んでください：mils（整数の1/1000インチ）、inch-2.4-tz、inch-2.4-lz、metric-3.3-tz、metric-3.3-lz（tz：先頭のゼロを省略、lz：末尾のゼロを省略）",
        _ => "The file does not declare how its numbers are written and the scale is not guessed. Choose the format: mils (whole thousandths of an inch), inch-2.4-tz, inch-2.4-lz, metric-3.3-tz or metric-3.3-lz (tz: the zeros in front are left out; lz: the zeros at the end are left out)",
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Format {
    /// `Some(true)` for metric, `Some(false)` for inch; `None` when the header does not say: the numbers are then mils, as in the files of the machine.
    metric: Option<bool>,
    /// Digits before and after the point; `None` for the usual one of the unit (2.4 inch, 3.3 metric).
    digits: Option<(u32, u32)>,
    /// `LZ`: the leading zeros are written and the trailing ones left out; `TZ` (and the default): the other way.
    trailing_suppressed: bool,
    /// An `INCH` (or `M72`) header with nothing else: the files of the machine, and the ones this suite writes, say it and then give whole mils.
    plain_inch: bool,
    /// True when the header says `LZ` or `TZ`.
    zeros_declared: bool,
    /// The unit the header says (`Some(true)` for metric), kept apart from [`Format::metric`], which a plain `INCH` leaves empty.
    unit_said: Option<bool>,
}

impl Format {
    /// A coordinate in the unit of the file as a whole number of mils.
    fn mils(&self, text: &str) -> Option<i32> {
        let Some(metric) = self.metric else {
            return text.parse().ok();
        };
        let (negative, body) = match text.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        if body.is_empty() || !body.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return None;
        }
        let value: f64 = if body.contains('.') {
            body.parse().ok()?
        } else {
            let (i, d) = self.digits.unwrap_or(if metric { (3, 3) } else { (2, 4) });
            let mut digits = body.to_string();
            if self.trailing_suppressed {
                while (digits.len() as u32) < i.saturating_add(d).min(2 * MAX_FORMAT_DIGITS) {
                    digits.push('0');
                }
            }
            digits.parse::<f64>().ok()? / 10f64.powi(d as i32)
        };
        let mils = if metric {
            value / MIL_MM
        } else {
            value * 1000.0
        };
        let mils = if negative { -mils } else { mils }.round();
        (mils.abs() < i32::MAX as f64).then_some(mils as i32)
    }

    /// The format with what the file does not say taken from `chosen`; `None` when the choice cannot be what the header says.
    fn completed(&self, chosen: NumberFormat) -> Option<Format> {
        let (metric, integer, decimal, trailing_left_out) = match chosen {
            NumberFormat::Mils => (false, 3, 3, false),
            NumberFormat::Digits {
                metric,
                integer,
                decimal,
                trailing_left_out,
            } => (
                metric,
                u32::from(integer),
                u32::from(decimal),
                trailing_left_out,
            ),
        };
        if self.unit_said.is_some_and(|said| said != metric) {
            return None;
        }
        if chosen == NumberFormat::Mils
            && (self.digits.is_some_and(|(_, d)| d != 3)
                || (self.zeros_declared && self.trailing_suppressed))
        {
            return None;
        }
        Some(Format {
            metric: Some(metric),
            digits: Some(self.digits.unwrap_or((integer, decimal))),
            trailing_suppressed: if self.zeros_declared {
                self.trailing_suppressed
            } else {
                trailing_left_out
            },
            ..*self
        })
    }

    /// Whether reading the coordinates of `line` (already read as `X..Y..`) takes something the file does not say: the unit or, after a unit alone,
    /// what it stands for (`INCH` alone is read as mils, other programs read it as 2.4 inches), the digits, or the side of the zeros of a number
    /// that does not fill the digits.
    fn supposes(&self, line: &str) -> bool {
        if self.metric.is_none() {
            return true;
        }
        line.split(['X', 'Y'])
            // a number with a point, and a zero, mean the same whatever the format
            .filter(|n| !n.contains('.') && n.bytes().any(|b| b.is_ascii_digit() && b != b'0'))
            .any(|n| match self.digits {
                None => true,
                Some((i, d)) => {
                    !self.zeros_declared
                        && (n.trim_start_matches(['+', '-']).len() as u32) < i.saturating_add(d)
                }
            })
    }

    /// Reads the options of a header line such as `INCH,LZ`, `METRIC,TZ,000.000` or `;FILE_FORMAT=2:4`.
    fn header(&mut self, line: &str) {
        if let Some(f) = line.strip_prefix(";FILE_FORMAT=") {
            if let Some((i, d)) = f.trim().split_once(':') {
                // a count of digits past 1..=6 is no real format and would ask for a string of that many zeros
                if let (Ok(i), Ok(d)) = (i.trim().parse::<u32>(), d.trim().parse::<u32>()) {
                    if !(1..=MAX_FORMAT_DIGITS).contains(&i)
                        || !(1..=MAX_FORMAT_DIGITS).contains(&d)
                    {
                        return;
                    }
                    self.digits = Some((i, d));
                    if self.plain_inch {
                        // a format given in numbers: the digits of the file, not mils
                        self.metric = Some(false);
                    }
                }
            }
            return;
        }
        let mut parts = line.split(',');
        match parts.next().map(str::trim) {
            Some("METRIC") => self.metric = Some(true),
            Some("INCH") => {
                self.metric = Some(false);
                self.plain_inch = true;
            }
            Some("M71") => self.metric = Some(true),
            Some("M72") => {
                self.metric = Some(false);
                self.plain_inch = true;
            }
            _ => return,
        }
        self.unit_said = self.metric;
        let mut options = false;
        for part in parts {
            options = true;
            match part.trim() {
                "LZ" => (self.trailing_suppressed, self.zeros_declared) = (true, true),
                "TZ" => (self.trailing_suppressed, self.zeros_declared) = (false, true),
                other => {
                    if let Some((i, d)) = other.split_once('.') {
                        if other.chars().all(|c| c.is_ascii_digit() || c == '.') {
                            self.digits = Some((i.len() as u32, d.len() as u32));
                        }
                    }
                }
            }
        }
        if options {
            self.plain_inch = false;
        }
        if self.plain_inch {
            // no options: whole mils
            self.metric = None;
        }
    }
}

/// Parses `X..Y..` where either axis may be omitted (modal, keeps the previous value).
fn parse_coord(s: &str, fmt: &Format, x: &mut i32, y: &mut i32) -> Option<()> {
    let mut rest = s;
    let mut seen = false;
    while let Some(c) = rest.chars().next() {
        if c != 'X' && c != 'Y' {
            return None;
        }
        let body = &rest[1..];
        let end = body
            .find(|d: char| !(d.is_ascii_digit() || d == '-' || d == '+' || d == '.'))
            .unwrap_or(body.len());
        let v: i32 = fmt.mils(&body[..end])?;
        if c == 'X' {
            *x = v;
        } else {
            *y = v;
        }
        seen = true;
        rest = &body[end..];
    }
    seen.then_some(())
}

/// Whether a first line is the start of an Excellon file and not the free text of an old teach-in file.
fn looks_like_excellon(l: &str) -> bool {
    let mut c = l.chars();
    match c.next() {
        Some('T') => c.next().is_some_and(|d| d.is_ascii_digit()),
        Some('X') | Some('Y') => c
            .next()
            .is_some_and(|d| d.is_ascii_digit() || d == '-' || d == '+' || d == '.'),
        Some(';') => true,
        Some('M') => l == "M71" || l == "M72",
        _ => l.starts_with("METRIC") || l.starts_with("INCH"),
    }
}

/// [`parse`] after the check of the size of the file: a file past a hard limit of `limits` is refused with `EH3D-FORMATS-011`, one past a warning limit
/// is read and the verdict says so, so that a program can tell the user. Any other problem comes as its [`FormatError`] too.
pub fn parse_limited(
    text: &str,
    limits: &crate::limits::ReaderLimits,
) -> Result<(DrillFile, crate::limits::LimitVerdict), FormatError> {
    let verdict = limits.guard(text)?;
    let file = parse_unchecked(text, None).map_err(|e| e.to_error())?.0;
    Ok((file, verdict))
}

/// Reads a drill file. A file past a hard limit of the shipped `reader_limits.cfg` (generous: 64 MiB and 4 million lines) is refused with [`ParseError::TooLarge`]; a program that
/// wants other limits, or the warning of a big file, uses [`parse_limited`] with its own [`crate::limits::ReaderLimits`].
pub fn parse(text: &str) -> Result<DrillFile, ParseError> {
    crate::limits::ReaderLimits::default()
        .check(text)
        .map_err(|e| ParseError::TooLarge {
            detail: e.to_string(),
        })?;
    Ok(parse_unchecked(text, None)?.0)
}

/// What the reader of a drill file ([`parse`]) supposes without a word. When the file does not say how its numbers are written, the first coordinate that
/// needs a supposition gives `EH3D-FORMATS-014`: the position may then be at another scale than the program that wrote the file meant (an `INCH` header
/// with nothing else is read as mils, while other programs read it as 2.4 inches: ten times less). A file that cannot be read gives that error. The reading
/// of a file is the same whether or not this is asked; a file that says its format gives an empty list.
pub fn diagnose(text: &str) -> Vec<FormatError> {
    match read_noting(text) {
        Ok((_, supposed)) => supposed.into_iter().collect(),
        Err(error) => vec![error],
    }
}

/// [`parse`], refusing a file whose format would have to be supposed ([`diagnose`]), for a program that must not guess a position. The lines left out of
/// the holes are not refused here: they are in [`DrillFile::ignored`].
pub fn parse_declared(text: &str) -> Result<DrillFile, FormatError> {
    match read_noting(text)? {
        (_, Some(supposed)) => Err(supposed),
        (file, None) => Ok(file),
    }
}

/// The file and what was supposed to read it, with the size limits of [`parse`].
fn read_noting(text: &str) -> Result<(DrillFile, Option<FormatError>), FormatError> {
    crate::limits::ReaderLimits::default().guard(text)?;
    parse_unchecked(text, None).map_err(|e| e.to_error())
}

/// Reads a drill file for a program that makes something from it or moves a machine. A file that declares its format is read as [`parse`] reads it and `chosen` is
/// not looked at. A file that does not is refused with `EH3D-FORMATS-014` when there is no choice, and read with the choice when there is one; a choice that cannot
/// be what the header says (another unit) is refused with the same code. [`how_to_choose`] is what to tell the person.
pub fn parse_as(text: &str, chosen: Option<NumberFormat>) -> Result<DrillFile, FormatError> {
    parse_as_limited(text, chosen, &crate::limits::ReaderLimits::default()).map(|(file, _)| file)
}

/// [`parse_as`] with the limits of [`parse_limited`] and its verdict.
pub fn parse_as_limited(
    text: &str,
    chosen: Option<NumberFormat>,
    limits: &crate::limits::ReaderLimits,
) -> Result<(DrillFile, crate::limits::LimitVerdict), FormatError> {
    let verdict = limits.guard(text)?;
    let plain = parse_unchecked(text, None);
    if let Ok((file, None)) = plain {
        return Ok((file, verdict));
    }
    let read = match chosen {
        None => plain,
        Some(chosen) => parse_unchecked(text, Some(chosen)),
    };
    match read.map_err(|e| e.to_error())? {
        (_, Some(refused)) => Err(refused),
        (file, None) => Ok((file, verdict)),
    }
}

/// The reading itself. With `chosen`, what the header does not say is taken from it and the second value is only a choice that does not fit the header.
fn parse_unchecked(
    text: &str,
    chosen: Option<NumberFormat>,
) -> Result<(DrillFile, Option<FormatError>), ParseError> {
    let mut out = DrillFile::default();
    let mut supposed = None::<FormatError>;
    let (mut tool, mut x, mut y) = (None::<u8>, 0, 0);
    let mut in_header = false;
    let mut fmt = Format::default();
    // Old teach-in files start with free text (title, unit note) that ends at the first `%`.
    // (a file whose first line is already Excellon - a tool, a coordinate, a comment, a unit - has no such text: a `%` at its end is not the end of a title)
    let free_text = text.lines().next().is_some_and(|l| {
        let l = l.trim();
        l != "M48" && !l.starts_with('%') && !looks_like_excellon(l)
    }) && text.lines().any(|l| l.trim().starts_with('%'))
        && !text.lines().any(|l| l.trim() == "M48");
    let mut skipping = free_text;
    for (n, raw) in text.lines().enumerate() {
        if skipping {
            skipping = !raw.trim().starts_with('%');
            continue;
        }
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let bad = || ParseError::BadLine {
            line: n + 1,
            text: raw.into(),
        };
        if line == "M48" {
            in_header = true;
            continue;
        }
        if line.starts_with('%') {
            in_header = false;
            continue;
        }
        // the comment may be written with blanks after its `;`
        let comment_format = line
            .strip_prefix(';')
            .map(str::trim_start)
            .filter(|c| c.starts_with("FILE_FORMAT="));
        if line.starts_with("METRIC")
            || line.starts_with("INCH")
            || line == "M71"
            || line == "M72"
            || comment_format.is_some()
        {
            // the unit, the digits and the zeros of the coordinates that follow
            match comment_format {
                Some(c) => fmt.header(&format!(";{c}")),
                None => fmt.header(line),
            }
            if comment_format.is_none() {
                out.units_declared = true;
            }
            if line.starts_with("METRIC") || line == "M71" {
                out.metric = true;
            }
            continue;
        }
        if line.starts_with('M') {
            // M30 end of program; M15/M16 put the tool down and up for a routed slot, which this format of holes does not hold: said, not lost
            if line == "M15" || line == "M16" {
                note_ignored(&mut out, n + 1, raw);
            }
            continue;
        }
        if in_header {
            if let Some(def) = line.strip_prefix('T') {
                if let Some((num, dia)) = def.split_once('C') {
                    let t: u8 = num.parse().map_err(|_| bad())?;
                    // Trailing feed/speed words (F.., S..) may follow the diameter.
                    let end = dia
                        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                        .unwrap_or(dia.len());
                    out.diameters
                        .push((t, dia[..end].parse().map_err(|_| bad())?));
                }
            }
            continue;
        }
        if line.starts_with('G') || line.starts_with("INCH") || line.starts_with("METRIC") {
            // `G05` (drill mode) and `G90` (absolute) say nothing to do; a `G00`/`G01`/`G85` with a position is a slot, which is not read
            if line.len() > 3 && line[1..].chars().any(|c| c == 'X' || c == 'Y') {
                note_ignored(&mut out, n + 1, raw);
            }
            continue;
        }
        if let Some(num) = line.strip_prefix('T') {
            let t: u8 = num.parse().map_err(|_| bad())?;
            if t == 0 {
                // T0: the tool is put away (KiCad and Altium write it before the end); it is no tool of the job
                tool = None;
                continue;
            }
            if !out.tools.contains(&t) {
                out.tools.push(t);
            }
            tool = Some(t);
            continue;
        }
        let t = tool.ok_or(ParseError::HoleBeforeTool { line: n + 1 })?;
        let used = match chosen {
            None => fmt,
            Some(c) => match fmt.completed(c) {
                Some(used) => used,
                None => {
                    supposed = Some(FormatError {
                        code: Code::NcdFormatAssumed,
                        line: Some(n + 1),
                        column: None,
                        detail: format!("the chosen format {} does not fit the header", c.name()),
                    });
                    break;
                }
            },
        };
        if chosen.is_some() {
            out.metric = used.metric == Some(true);
        }
        parse_coord(line, &used, &mut x, &mut y).ok_or_else(bad)?;
        if chosen.is_none() && supposed.is_none() && fmt.supposes(line) {
            supposed = Some(FormatError {
                code: Code::NcdFormatAssumed,
                line: Some(n + 1),
                column: None,
                detail: line.to_string(),
            });
        }
        out.holes.push(Hole {
            tool: t,
            x_mil: x,
            y_mil: y,
        });
    }
    Ok((out, supposed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_with_inches_and_four_decimals_gives_the_right_position() {
        // TZ: the leading zeros are left out, 5850 is 0.5850 inch = 585 mils
        let f = parse(
            "M48
INCH,TZ
;FILE_FORMAT=2:4
T1C0.0315
%
T1
X5850Y10000
M30
",
        )
        .unwrap();
        assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (585, 1000));
        assert!((f.holes[0].x_mm() - 14.859).abs() < 1e-6);
    }

    #[test]
    fn leading_zeros_present_means_the_trailing_ones_are_left_out() {
        // LZ with 2.4: 05 is 05.0000 inch
        let f = parse(
            "M48
INCH,LZ
T1C0.0315
%
T1
X05Y1
M30
",
        )
        .unwrap();
        assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (5000, 10000));
    }

    #[test]
    fn metric_with_trailing_zeros_present() {
        let f = parse(
            "M48
METRIC,TZ,000.000
T1C0.8
%
T1
X12700Y0
M30
",
        )
        .unwrap();
        assert!(
            (f.holes[0].x_mm() - 12.7).abs() < 0.02,
            "{}",
            f.holes[0].x_mm()
        );
    }

    #[test]
    fn decimal_coordinates_and_t0() {
        let f = parse(
            "M48
METRIC
T1C0.8
%
T1
X12.7Y-25.4
T0
M30
",
        )
        .unwrap();
        assert!((f.holes[0].x_mm() - 12.7).abs() < 0.02);
        assert!((f.holes[0].y_mm() + 25.4).abs() < 0.02);
        assert_eq!(f.tools, vec![1]);
    }

    #[test]
    fn a_metric_file_is_read_in_millimetres_so_the_holes_are_where_they_were_drawn() {
        // 3.3 metric: X12345 is 12.345 mm, not 12345 mils (313.6 mm)
        let f = parse(
            "M48
METRIC
T1C0.8
%
T1
X12345Y20000
M30
",
        )
        .unwrap();
        assert!(
            (f.holes[0].x_mm() - 12.345).abs() < 0.02,
            "{}",
            f.holes[0].x_mm()
        );
        assert!((f.holes[0].y_mm() - 20.0).abs() < 0.02);
        assert!(f.metric);
    }

    #[test]
    fn a_file_past_the_default_limits_is_refused_and_the_ignored_lines_are_bounded() {
        // more lines than the shipped limit
        let big = "\n".repeat(4_000_001);
        match parse(&big) {
            Err(ParseError::TooLarge { detail }) => assert!(detail.contains("lines"), "{detail}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            parse(&big).unwrap_err().to_error().code,
            crate::error::Code::FileTooLarge
        );
        // millions of slot lines keep only the first ones and count the rest
        let mut text = String::from("M48\nMETRIC\n%\nT1\n");
        for _ in 0..(MAX_IGNORED + 50) {
            text.push_str("G01X1Y1\n");
        }
        let f = parse(&text).unwrap();
        assert_eq!((f.ignored.len(), f.ignored_more), (MAX_IGNORED, 50));
        // a program with its own, wider limits is not held by the default ones
        let wide = crate::limits::ReaderLimits::from_config(
            "max_bytes = 1000000000\nmax_lines = 9000000\nwarn_bytes = 1000000000\nwarn_lines = 9000000\n",
        )
        .unwrap();
        assert!(parse_limited(&big, &wide).is_ok());
    }

    #[test]
    fn a_file_says_whether_its_unit_was_declared_or_supposed() {
        for text in [
            "M48\nMETRIC\n%\nT1\nX1000Y1000\n",
            "M48\nINCH\n%\nT1\nX1000Y1000\n",
            "M48\nM71\n%\nT1\nX1000Y1000\n",
            "M72\nT1\nX1000Y1000\n",
        ] {
            assert!(parse(text).unwrap().units_declared, "{text:?}");
        }
        // only the digits are given, or nothing: the numbers are read as mils
        for text in [
            "M48\n;FILE_FORMAT=2:4\n%\nT1\nX1000Y1000\n",
            "T1\nX1000Y1000\n",
        ] {
            assert!(!parse(text).unwrap().units_declared, "{text:?}");
        }
    }

    #[test]
    fn slots_are_named_and_a_body_with_a_percent_at_its_end_keeps_its_holes() {
        let f = parse(
            "M48
INCH
T1C0.0315
%
T1
X1000Y1000
G00X2000Y2000
M15
G01X3000Y2000
M16
G05
M30
",
        )
        .unwrap();
        assert_eq!(f.holes.len(), 1);
        assert_eq!(
            f.ignored.iter().map(|i| i.line).collect::<Vec<_>>(),
            vec![7, 8, 9, 10]
        );
        assert_eq!(f.ignored[0].text, "G00X2000Y2000");
        // a file with no header whose first line is a tool is a file of holes, though a `%` closes it
        let g = parse(
            "T01
X1000Y1000
X2000Y2000
M30
%
",
        )
        .unwrap();
        assert_eq!(g.holes.len(), 2);
        // an old teach-in file with a title still has it skipped
        let h = parse(
            "TEACH IN FILE
%
T01
X1000Y1000
M30
",
        )
        .unwrap();
        assert_eq!(h.holes.len(), 1);
    }

    #[test]
    fn a_plain_inch_header_keeps_whole_mils_as_the_machine_files_have_them() {
        let f = parse("M48\nINCH\nT1C0.0315\n%\nT1\nX1000Y2000\nM30\n").unwrap();
        assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (1000, 2000));
    }

    #[test]
    fn a_line_that_cannot_be_read_is_told_with_its_code_line_and_column() {
        let e = parse("M48\n  T1Cx\n%\n").unwrap_err();
        assert_eq!(e.code(), Code::NcdBadLine);
        assert_eq!(e.line(), 2);
        let error = e.to_error();
        assert_eq!((error.line, error.column), (Some(2), Some(3)));
        assert_eq!(
            e.to_string(),
            "EH3D-FORMATS-001 line 2 column 3: a line of the drill file cannot be read \"T1Cx\""
        );
    }

    #[test]
    fn a_coordinate_before_any_tool_is_told_with_its_code_and_line() {
        let e = parse("%\nX1000Y1000\n").unwrap_err();
        assert_eq!(e, ParseError::HoleBeforeTool { line: 2 });
        assert_eq!(e.code().id(), "EH3D-FORMATS-002");
        assert_eq!(
            e.to_string(),
            "EH3D-FORMATS-002 line 2: a coordinate comes before any tool"
        );
        assert_eq!(e.to_error().column, None);
    }

    #[test]
    fn a_file_without_a_unit_keeps_its_mils() {
        let f = parse(
            "%
T06
X1000Y2000
",
        )
        .unwrap();
        assert_eq!((f.holes[0].x_mil, f.holes[0].y_mil), (1000, 2000));
    }

    #[test]
    fn a_format_line_with_a_huge_count_of_digits_is_ignored_and_does_not_eat_the_memory() {
        for line in [
            ";FILE_FORMAT=2000000000:0",
            ";FILE_FORMAT=4294967295:4294967295",
            ";FILE_FORMAT=0:3",
            ";FILE_FORMAT=3:7",
        ] {
            let text = format!(
                "METRIC,LZ
{line}
T1
X1Y1
"
            );
            let f = parse(&text).unwrap();
            assert_eq!(f.holes.len(), 1, "{line}");
        }
        // a real format is still taken
        let f = parse(
            "METRIC,LZ
;FILE_FORMAT=3:3
T1
X1Y1
",
        )
        .unwrap();
        assert_eq!(f.holes.len(), 1);
    }
}
