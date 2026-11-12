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

The reader of the plot (`hpgl::parse`, `hpgl::parse_with`) does not refuse a file: it drops what it cannot read, as the original program does. `hpgl::diagnose` lists what it drops and `hpgl::parse_strict` turns the first problem into an error; a file with nothing to tell gives exactly the same plot with or without the check. The messages that `ncd::parse` and `dxf::outline` give as text start with the code.
