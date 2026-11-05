# The board materials database

The crate `crates/materials` (`eh3d-materials`) holds the materials of printed circuit boards and the catalogue of the reference documents. Its README explains the fields, the checks, the queries and the categories of the catalogue; this page says how the data is kept.

## Adding a manufacturer or a material

1. Write the material in `crates/materials/data/<manufacturer>.json`, with the identifier `<manufacturer>/<name>` in lower case with hyphens, every value as the sheet publishes it (units in the name of the field, frequencies in GHz) and the file name of the PDF in `source` (or in `sources`, with the pages and the revision when the sheet gives them).
2. A value the sheet does not give is left out. A bound ("more than 60 min") or a range ("9-12 ppm/C") is written in `notes`, not as a number. A figure in a unit the schema does not use is converted, and the original goes in the note when it could be misread.
3. A new file is added to the list in `crates/materials/src/data.rs`; the test of that module fails when a file of the folder is missing from it.
4. `cargo test -p eh3d-materials` checks every file: schema version, unique and coherent identifiers, believable Dk, Df, Tg and Td, a plain file name as source, no field the schema does not know.

## Who reads it

The design tool reads the whole record. The machine program that drills and mills boards reads `Material::bungard_summary()` (name, manufacturer, kind, thicknesses, copper, panel sizes, Tg, Dk and Df at the frequency nearest to 1 GHz, thermal conductivity for metal cores, notes) so that the two do not keep two copies of the same data.

## The catalogue of documents

`data/documents.json` classifies every document of the reference folders by category and by what it serves (the machine program or the design tool). A record classified by keywords has the status `auto`; a person who has read the document changes it to `checked`. See the README of the crate for the meaning of each category and of each relevance.
