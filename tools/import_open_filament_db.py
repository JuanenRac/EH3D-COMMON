#!/usr/bin/env python3
"""Writes the seed filaments of `crates/printmaterials` from a clone of the Open Filament Database.

Usage:  py -3 -I tools/import_open_filament_db.py <clone>/data [--per-brand 30]

The clone is only read (clone it into a temporary folder, shallow, and delete it afterwards). The
database is published under the MIT licence: its notice is kept in `THIRD_PARTY_NOTICES` of the
crate. The facts of a filament are copied (temperatures, density, flow, drying, spools, colours
with their hex); no image, no long text and no purchase link is. Every record names where it comes
from (`source` and `source_ref`) and is marked `verified = false`.

The script is a tool: the crate does not depend on it, and the generated files are what is
versioned. It also writes `src/filament_files.rs`, the list of the embedded filament files.
"""

import json
import os
import re
import sys

CRATE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "crates", "printmaterials")

# brands of the seed, by their folder in the database: the ones most spools are bought from
BRANDS = [
    "bambu_lab", "polymaker", "esun_3d", "sunlu", "elegoo", "creality", "overture", "eryone",
    "formfutura", "fiberlogy", "3dxtech", "colorfabb", "spectrum", "prusament", "qidi_tech",
    "sainsmart", "anycubic", "protopasta", "add_north", "amolen", "hatchbox", "geeetech", "kingroon", "jayo",
]
# the polymers the schema names; the rest go to `other` with the grade as the database writes it
MATERIALS = {
    "PLA": ("pla", None), "PETG": ("petg", None), "ABS": ("abs", None), "ASA": ("asa", None),
    "TPU": ("tpu", None), "TPE": ("tpe", None), "PA6": ("pa", "PA6"), "PA12": ("pa", "PA12"),
    "PA11": ("pa", "PA11"), "PC": ("pc", None), "PP": ("pp", None), "PVA": ("pva", None),
    "BVOH": ("bvoh", None), "HIPS": ("hips", None), "PET": ("pet", None), "PPA": ("ppa", None),
    "PPS": ("pps", None), "PEEK": ("peek", None), "PEI": ("pei", None),
}
POPULAR = ["PLA", "PETG", "ABS", "ASA", "TPU", "PA6", "PA12", "PC", "PVA", "HIPS", "PP", "PET"]
TRAIT_FILL = {
    "contains_carbon_fiber": "carbon_fiber", "contains_glass_fiber": "glass_fiber",
    "contains_wood": "wood", "contains_metal": "metal", "glitter": "glitter", "glow": "glow",
    "contains_stone": "mineral", "contains_ceramic": "mineral",
}
MAX_COLORS = 12


def slug(text):
    return "-".join(re.findall(r"[^\W_]+", text.lower()))


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def num(v):
    """A plain number or None (the database sometimes writes null or text)."""
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        return None
    return int(v) if float(v) == int(v) else round(float(v), 4)


def within(v, lo, hi, dropped, what):
    v = num(v)
    if v is None:
        return None
    if not lo <= v <= hi:
        dropped[what] = dropped.get(what, 0) + 1
        return None
    return v


def a_range(lo, hi, bounds, dropped, what):
    lo = within(lo, *bounds, dropped, what)
    hi = within(hi, *bounds, dropped, what)
    if lo is not None and hi is not None and lo > hi:
        dropped[what] = dropped.get(what, 0) + 1
        return None
    if lo is None and hi is None:
        return None
    return {"min": lo, "max": hi, "recommended": None}


def https(url):
    return isinstance(url, str) and url.startswith("https://") and len(url) <= 400 and not re.search(r"\s", url)


def mechanical(entries, stats):
    out = {}
    for e in entries or []:
        prop = str(e.get("property", ""))
        value = num(e.get("value"))
        unit = str(e.get("unit", ""))
        stats[prop] = stats.get(prop, 0) + 1
        if value is None:
            continue
        if "deflection" in prop and "C" in unit:
            out.setdefault("hdt_c", value)
        elif "glass_transition" in prop and "C" in unit:
            out.setdefault("tg_c", value)
        elif prop in ("tensile_strength", "ultimate_tensile_strength") and unit == "MPa":
            out.setdefault("tensile_mpa", value)
        elif prop in ("tensile_modulus", "youngs_modulus") and unit in ("MPa", "GPa"):
            out.setdefault("modulus_mpa", value * (1000 if unit == "GPa" else 1))
    return out


def build(brand_dir, brand, per_brand, dropped, stats):
    lines = []
    for material_dir in sorted(os.listdir(brand_dir)):
        mpath = os.path.join(brand_dir, material_dir)
        if not os.path.isdir(mpath) or not os.path.exists(os.path.join(mpath, "material.json")):
            continue
        for line_dir in sorted(os.listdir(mpath)):
            lpath = os.path.join(mpath, line_dir)
            fpath = os.path.join(lpath, "filament.json")
            if os.path.isdir(lpath) and os.path.exists(fpath):
                lines.append((material_dir, line_dir, lpath, load(fpath)))
    # the most colours first (a line with many colours is a line many people buy), popular polymers first
    def rank(item):
        material, _, lpath, f = item
        colours = sum(1 for d in os.listdir(lpath) if os.path.isdir(os.path.join(lpath, d)))
        popular = POPULAR.index(material) if material in POPULAR else len(POPULAR)
        return (popular >= 6, -colours, popular)

    lines = [x for x in lines if not x[3].get("discontinued")]
    lines.sort(key=rank)
    lines = lines[:per_brand]
    records = []
    seen = set()
    for material_dir, line_dir, lpath, f in lines:
        mat, grade = MATERIALS.get(material_dir, ("other", material_dir))
        name = str(f["name"]).strip()
        if slug(name) in seen:
            name = f"{name} {material_dir}"
        if slug(name) in seen:
            continue
        seen.add(slug(name))
        rec = {
            "id": f"{slug(brand['name'])}/{slug(name)}",
            "manufacturer": brand["name"],
            "name": name,
            "material": mat,
        }
        if grade:
            rec["grade"] = grade
        colours, diameters, spools, fill, abrasive = [], set(), [], [], None
        for vdir in sorted(os.listdir(lpath)):
            vpath = os.path.join(lpath, vdir)
            if not os.path.isdir(vpath) or not os.path.exists(os.path.join(vpath, "variant.json")):
                continue
            v = load(os.path.join(vpath, "variant.json"))
            if v.get("discontinued"):
                continue
            hexes = v.get("color_hex")
            if len(colours) < MAX_COLORS and v.get("name"):
                c = {"name": str(v["name"]).strip(), "hex": None, "td": None}
                if isinstance(hexes, str) and re.fullmatch(r"#[0-9A-Fa-f]{6}", hexes):
                    c["hex"] = hexes.upper()
                c["td"] = within(v.get("transmission_distance"), 0, 100, dropped, "td")
                colours.append(c)
            for key, f_name in TRAIT_FILL.items():
                if (v.get("traits") or {}).get(key) and f_name not in fill:
                    fill.append(f_name)
            if (v.get("traits") or {}).get("abrasive"):
                abrasive = True
            spath = os.path.join(vpath, "sizes.json")
            if os.path.exists(spath):
                for s in load(spath):
                    d = num(s.get("diameter"))
                    if d is not None and 1.0 <= d <= 4.0:
                        diameters.add(d)
                    w = within(s.get("filament_weight"), 10, 20000, dropped, "spool.weight")
                    if w is None or s.get("spool_refill"):
                        continue
                    sp = {
                        "weight_g": w,
                        "diameter_mm": within(s.get("container_outer_diameter"), 50, 500, dropped, "spool.diameter"),
                        "width_mm": within(s.get("container_width"), 10, 250, dropped, "spool.width"),
                        "hub_mm": within(s.get("container_hole_diameter"), 10, 120, dropped, "spool.hub"),
                    }
                    if sp not in spools and len(spools) < 6:
                        spools.append(sp)
        if fill:
            rec["fill"] = fill
        shore_a, shore_d = num(f.get("shore_hardness_a")), num(f.get("shore_hardness_d"))
        if shore_a is not None and 1 <= shore_a <= 100:
            rec["shore"] = {"value": shore_a, "scale": "a"}
        elif shore_d is not None and 1 <= shore_d <= 100:
            rec["shore"] = {"value": shore_d, "scale": "d"}
        if diameters:
            rec["diameters_mm"] = sorted(diameters)
        rec["diameter_tolerance_mm"] = within(f.get("diameter_tolerance"), 0, 0.5, dropped, "tolerance")
        rec["density_g_cm3"] = within(f.get("density"), 0.5, 6.0, dropped, "density")
        if colours:
            rec["colors"] = colours
        rec["nozzle_c"] = a_range(f.get("min_print_temperature"), f.get("max_print_temperature"), (100, 500), dropped, "nozzle")
        rec["bed_c"] = a_range(f.get("min_bed_temperature"), f.get("max_bed_temperature"), (0, 200), dropped, "bed")
        rec["chamber_c"] = within(f.get("chamber_temperature"), 0, 150, dropped, "chamber")
        rec["max_speed_mm_s"] = within(f.get("max_print_speed"), 1, 2000, dropped, "speed")
        rec["max_flow_mm3_s"] = within(f.get("max_volumetric_speed"), 0.1, 150, dropped, "flow")
        rec["fan_pct"] = a_range(f.get("min_fan_speed"), f.get("max_fan_speed"), (0, 100), dropped, "fan")
        t = within(f.get("dry_temperature"), 20, 150, dropped, "dry.temp")
        h = within(f.get("dry_time"), 0.1, 48, dropped, "dry.hours")
        if t is not None and h is not None:
            rec["drying"] = {"temp_c": t, "hours": h}
        rec["abrasive"] = abrasive
        rec["min_nozzle_mm"] = within(f.get("min_nozzle_diameter"), 0.1, 2.0, dropped, "min_nozzle")
        enc = f.get("enclosure")
        if isinstance(enc, str):
            rec["enclosure_advised"] = enc.lower() in ("recommended", "required")
        mech = mechanical(f.get("mechanical"), stats)
        rec["tg_c"] = within(mech.get("tg_c"), -100, 400, dropped, "tg")
        rec["hdt_c"] = within(mech.get("hdt_c"), 20, 400, dropped, "hdt")
        rec["tensile_mpa"] = within(mech.get("tensile_mpa"), 1, 500, dropped, "tensile")
        rec["modulus_mpa"] = within(mech.get("modulus_mpa"), 1, 100000, dropped, "modulus")
        if spools:
            rec["spools"] = spools
        links = [u for u in (f.get("data_sheet_url"), f.get("safety_sheet_url")) if https(u)]
        if links:
            rec["links"] = links
        rec["source"] = "open-filament-database"
        rec["source_ref"] = f"{os.path.basename(brand_dir)}/{material_dir}/{line_dir}"
        rec["verified"] = False
        records.append({k: v for k, v in rec.items() if v is not None})
    return records


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    per_brand = 22
    if "--per-brand" in sys.argv:
        per_brand = int(sys.argv[sys.argv.index("--per-brand") + 1])
        args = [a for a in args if a != str(per_brand)]
    if len(args) != 1:
        sys.exit(__doc__)
    data = args[0]
    out_dir = os.path.join(CRATE, "data", "filaments")
    os.makedirs(out_dir, exist_ok=True)
    dropped, stats, total = {}, {}, 0
    names = ["generic.json"]
    for brand_id in BRANDS:
        bdir = os.path.join(data, brand_id)
        if not os.path.isdir(bdir):
            print("missing brand", brand_id)
            continue
        brand = load(os.path.join(bdir, "brand.json"))
        records = build(bdir, brand, per_brand, dropped, stats)
        name = f"{slug(brand['name'])}.json"
        doc = {"schema_version": 1, "manufacturer": brand["name"], "filaments": records}
        with open(os.path.join(out_dir, name), "w", encoding="utf-8", newline="\n") as f:
            json.dump(doc, f, indent=1, ensure_ascii=False)
            f.write("\n")
        names.append(name)
        total += len(records)
        print(f"{brand['name']}: {len(records)}")
    names.sort()
    lines = [
        "//! The list of the embedded filament files (written by `tools/import_open_filament_db.py`).",
        "",
        "/// The embedded files as `(file name, JSON text)`.",
        "pub const FILES: &[(&str, &str)] = &[",
    ]
    for n in names:
        lines.append(f'    ("{n}", include_str!("../data/filaments/{n}")),')
    lines.append("];")
    with open(os.path.join(CRATE, "src", "filament_files.rs"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")
    print("total", total, "dropped", dropped)
    print("mechanical properties seen", sorted(stats.items(), key=lambda kv: -kv[1])[:12])


if __name__ == "__main__":
    main()
