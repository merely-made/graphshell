#!/usr/bin/env python3

# Copyright 2026 Mark Alan Boykin
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
# SPDX-License-Identifier: MPL-2.0

"""Generate an isolated Cargo consumer using the current root patch tables.

The output is deliberately outside the Mere workspace. Pass a new or empty
output directory; existing receipts are never overwritten. The root lock is a
starting point, not evidence that the generated dependency graph is resolved:
the lock on disk when there is one, else the one `house resolved show` has for
HEAD, else none (cargo then resolves afresh).
"""
import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

fixture = Path(__file__).resolve().parent
root = fixture.parents[3]
parser = argparse.ArgumentParser()
parser.add_argument("output", type=Path)
args = parser.parse_args()
output = args.output.resolve()
if output == root or root in output.parents:
    parser.error("output must be outside the Mere workspace")
if output.exists() and any(output.iterdir()):
    parser.error("output must be a new or empty directory")
output.mkdir(parents=True, exist_ok=True)
source = (root / "Cargo.toml").read_text()
workspace = tomllib.loads(source)["workspace"]["dependencies"]
def toml_value(value):
    return str(value).lower() if isinstance(value, bool) else json.dumps(value)
def from_root(name, features):
    dependency = workspace[name].copy()
    if "path" in dependency:
        dependency["path"] = str((root / dependency["path"]).resolve())
    dependency["default-features"] = False
    dependency["features"] = features
    return "{ " + ", ".join(key + " = " + toml_value(value) for key, value in dependency.items()) + " }"
# Root patch sections are top-level tables; preserve their exact entries and
# rewrite relative path values against the root that owns those entries.
sections = re.split(r"(?m)(?=^\[)", source)
patches = []
for section in sections:
    if not section.startswith("[patch."):
        continue
    section = re.sub(r'\bpath\s*=\s*"([^"\n]+)"',
                     lambda m: "path = " + json.dumps(str((root / m[1]).resolve())), section)
    patches.append(section.rstrip())
manifest = '''[workspace]
[package]
name = "combined-resident-consumer-fixture"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
mere-resident = { path = %s, default-features = false }
djinn = %s
ostiary = %s
castellan-client = %s
graphshell-door = { path = %s, default-features = false, features = ["projection-door", "projection-provider"] }
''' % (json.dumps(str(root / "ports/mere-resident")), from_root("djinn", ["resident"]),
       from_root("ostiary", ["native"]), from_root("castellan-client", ["client"]),
       json.dumps(str(root / "crates/graphshell/graphshell-door")))
manifest += "\n\n".join(["", *patches]) + "\n"
# Fail before writing if extraction accidentally produced invalid TOML.
tomllib.loads(manifest)
(output / "Cargo.toml").write_text(manifest)
(output / "src").mkdir()
shutil.copyfile(fixture / "src/lib.rs", output / "src/lib.rs")


def lock_from_note(rel):
    """The `# ---- <rel>` section of HEAD's resolved note, or None."""
    try:
        shown = subprocess.run(["house", "resolved", "show", "HEAD"], cwd=root,
                               capture_output=True, text=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return None
    section = None
    for line in shown.splitlines(keepends=True):
        if line.startswith("# ---- "):
            if section is not None:
                break
            section = [] if line.strip() == "# ---- " + rel else None
        elif section is not None:
            section.append(line)
    return "".join(section) if section is not None else None


root_lock = root / "Cargo.lock"
if root_lock.exists():
    shutil.copyfile(root_lock, output / "Cargo.lock")
else:
    top = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=root,
                         capture_output=True, text=True).stdout.strip()
    rel = (root_lock.relative_to(Path(top)) if top else Path("Cargo.lock")).as_posix()
    note = lock_from_note(rel)
    if note is not None:
        (output / "Cargo.lock").write_text(note)
print(output)
