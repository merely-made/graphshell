#!/usr/bin/env python3

# Copyright 2026 Mark Alan Boykin
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
# SPDX-License-Identifier: MPL-2.0

"""Enforce the host layer's one-way dependency boundaries: ports into crates, and
the toolkit into the document stack. (The Graphshell app's own checks are
Turnstone's scripts/check_port_boundaries.py.)"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
CRATES = (ROOT / "crates").resolve()
PORTS = (ROOT / "ports").resolve()
TOOLKIT = (CRATES / "cambium").resolve()
# The document stack: content sessions, lanes, canvases, loaders and capture.
# The toolkit renders what they produce; it does not link them.
DOCUMENT_STACK_PACKAGES = {
    "inker",
    "document-canvas",
    "mere-lanes",
    "nematic",
    "fleece",
    "mere-pageload",
    "mere-fetch",
    "knot-editor-host",
}
def fail(message: str) -> None:
    print(f"port boundary failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def is_beneath(path: pathlib.Path, parent: pathlib.Path) -> bool:
    try:
        path.resolve().relative_to(parent)
    except ValueError:
        return False
    return True


def check_toolkit_document_links(packages: list[dict]) -> None:
    for package in packages:
        manifest = pathlib.Path(package["manifest_path"]).resolve()
        if not is_beneath(manifest, TOOLKIT):
            continue
        for dependency in package["dependencies"]:
            if dependency.get("kind") is not None or dependency.get("optional"):
                continue
            if dependency["name"] not in DOCUMENT_STACK_PACKAGES:
                continue
            fail(
                f"toolkit crate {manifest.relative_to(ROOT)} links the document "
                f"stack: {package['name']} -> {dependency['name']}"
            )


def main() -> None:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        fail(f"cargo metadata failed:\n{result.stderr}")

    packages = json.loads(result.stdout)["packages"]
    for package in packages:
        manifest = pathlib.Path(package["manifest_path"]).resolve()
        if not is_beneath(manifest, CRATES):
            continue
        for dependency in package["dependencies"]:
            path = dependency.get("path")
            if path is None or not is_beneath(pathlib.Path(path), PORTS):
                continue
            # Every kind, dev-dependencies included: an example or test that
            # mounts a product surface lives with that product (Mark, 2026-10-26).
            fail(
                f"{manifest.relative_to(ROOT)} depends on port package "
                f"{dependency['name']} at {pathlib.Path(path).relative_to(ROOT)}"
            )

    check_toolkit_document_links(packages)
    print("Host port and toolkit dependency boundaries passed")


if __name__ == "__main__":
    main()
