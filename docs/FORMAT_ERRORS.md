# Errors of the file readers (`eh3d-bungard-formats`)

The readers of the drill file (`ncd`), the plot (`hpgl`) and the drawing (`dxf`) and the Gerber outline (`gerber`) tell what they cannot read with a stable code, the line and, when the problem is at a place of the line, the column. The code never changes from one version to the next; a new problem gets the next number. The words of each code exist in the eight languages of the project (`Code::message`), and `FormatError::localized` writes the whole error in one of them.

An error reads like this: `EH3D-FORMATS-001 line 2 column 3: a line of the drill file cannot be read "T1Cx"`.

| Code | Reader | What it is | Line | Column |
|---|---|---|---|---|
| `EH3D-FORMATS-001` | `ncd::parse` | A line of the drill file that is neither a tool, a coordinate nor a command. | the line | where the statement starts |
| `EH3D-FORMATS-002` | `ncd::parse` | A coordinate before any tool was chosen. | the line | none |
| `EH3D-FORMATS-003` | `dxf::outline`, `dxf::outline_checked` | A group code of the drawing that is not a number. | the line of the group code | where it starts |
| `EH3D-FORMATS-004` | `dxf::outline`, `dxf::outline_checked` | A drawing file with nothing in it. | none | none |
| `EH3D-FORMATS-005` | `dxf::outline`, `dxf::outline_checked` | A drawing without a line, an arc or a circle. | none | none |
| `EH3D-FORMATS-006` | `hpgl::diagnose`, `hpgl::parse_strict` | A number of a plot command that cannot be read. | the line | the field |
| `EH3D-FORMATS-007` | `hpgl::diagnose`, `hpgl::parse_strict` | A position command (`PU`, `PD`, `PA`, `PR`) whose last coordinate has no pair. | the line | the command |
| `EH3D-FORMATS-008` | `gerber::outline`, `gerber::outline_checked` | A Gerber outline without the format statement (`FS`). | none | none |
| `EH3D-FORMATS-009` | `gerber::outline`, `gerber::outline_checked` | A Gerber file with no outline (no line or arc drawn with the pen down). | none | none |
| `EH3D-FORMATS-010` | `gerber::outline`, `gerber::outline_checked` | A coordinate of the Gerber outline that cannot be read. | none | none |
| `EH3D-FORMATS-011` | `gerber::outline_checked`, `dxf::outline_checked`, `hpgl::parse_strict`, `ncd::parse_limited`, `hpgl::parse_limited` | A file bigger than the hard limit of the readers (`reader_limits.cfg`). | none | none |
| `EH3D-FORMATS-012` | `hpgl::diagnose`, `hpgl::parse_strict` | A command that changes what is cut and is not followed: `RO`, `IW`, `IR`, `CT`, `LB`, `CP`, `EA`, `ER`, `RA`, `RR`, `EW`, `WG`, `PM`, `EP`, `FP`, `AT`, `RT`, `BZ`, `BR`, `PE`, `UC`, `SM`, `RP`. The forms that only put a thing back (`RO`, `RO0`, `IW`, `IR`, `CT0`, `SM`) say nothing. | the line | the command |
| `EH3D-FORMATS-013` | `hpgl::diagnose`, `hpgl::parse_strict` | A scaling that is not applied as written: `SC` or `IP` with a count of numbers other than none or four (a fifth `0` of `SC` is accepted), an empty window of user units, or user units drawn with no `IP`. | the line | the command |
| `EH3D-FORMATS-014` | `ncd::diagnose`, `ncd::parse_declared`, `ncd::parse_as` | A drill file read by supposing something: no unit (read as mils), an `INCH` or `M72` header with nothing else (read as whole mils; other programs read 2.4 inches, ten times less), a unit with no digits for numbers without a point, or digits with no `LZ`/`TZ` for a number that does not fill them. Told once, for the first coordinate. | the line | none |

The reader of the plot (`hpgl::parse`, `hpgl::parse_with`) does not refuse a file: it drops what it cannot read, as the original program does. `hpgl::diagnose` lists what it drops and `hpgl::parse_strict` turns the first problem into an error; a file with nothing to tell gives exactly the same plot with or without the check. The messages that `ncd::parse` and `dxf::outline` give as text start with the code.

In a plot, `PR` and `PA` set how the coordinates of the following `PU` and `PD` are read; `IN` and `DF` put them back to absolute.

The drill reader (`ncd::parse`) reads a file with a supposed format as it always did, for showing it. A program that makes a job from a file or moves a machine uses `ncd::parse_as`: a file that does not declare its format is refused, unless the person has chosen one (`mils`, `inch-2.4-tz`, `inch-2.4-lz`, `metric-3.3-tz`, `metric-3.3-lz` or other digits; `ncd::NumberFormat`). What the file declares wins over the choice, and a choice of the other unit is refused. `ncd::how_to_choose` gives the explanation in the eight languages, and a program that writes whole mils declares it with `ncd::MILS_DECLARATION` (`;FILE_FORMAT=3:3` and `INCH,TZ`) inside the header. The lines that are left out of the holes (slots, routed moves) are apart, in `DrillFile::ignored`: a program that makes a job from the holes looks at them.
