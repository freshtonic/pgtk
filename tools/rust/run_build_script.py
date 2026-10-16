#!/usr/bin/env python3
"""Runs a first-party Cargo build script for Buck and writes its OUT_DIR.

The prelude `buildscript_run` rule gives the build script a manifest directory of symlinks.
recursa-codegen does not accept symlinks in a grammar source path, and it reads the features
from CARGO_CFG_FEATURE, which `buildscript_run` does not set. This runner gives the build script
the environment that Cargo gives it, with CARGO_MANIFEST_DIR set to a real directory.

The runner accepts only build scripts that write OUT_DIR. It fails if the build script prints a
`cargo:` instruction that changes how the crate compiles, because Buck would not apply it.
"""

import argparse
import os
import re
import subprocess
import sys
from collections import defaultdict

# Instructions that do not change how the crate compiles.
IGNORED_INSTRUCTION = re.compile(r"^cargo::?(rerun-if-changed|rerun-if-env-changed|warning)=")


def rustc_cfg_env(rustc: str) -> dict[str, str]:
    """Returns the CARGO_CFG_* variables for the cfgs that rustc reports for the host."""
    values = defaultdict(list)
    output = subprocess.check_output([rustc, "--print", "cfg"], encoding="utf-8")
    for line in output.splitlines():
        name, _, value = line.partition("=")
        values[name].append(value.strip('"'))
    return {
        f"CARGO_CFG_{name.upper()}": ",".join(v for v in vals if v)
        for name, vals in values.items()
    }


def host_triple(rustc: str) -> str:
    output = subprocess.check_output([rustc, "-vV"], encoding="utf-8")
    for line in output.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ")
    sys.exit("rustc -vV did not report a host triple")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--build-script", required=True)
    parser.add_argument("--manifest-dir", required=True)
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--package-name", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--feature", action="append", default=[])
    args = parser.parse_args()

    rustc = os.environ.get("RUSTC", "rustc")
    manifest_dir = os.path.realpath(args.manifest_dir)
    out_dir = os.path.abspath(args.out_dir)
    os.makedirs(out_dir, exist_ok=True)
    host = host_triple(rustc)

    env = dict(os.environ)
    env.update(rustc_cfg_env(rustc))
    env.update(
        {
            "CARGO_CFG_FEATURE": ",".join(sorted(args.feature)),
            "CARGO_MANIFEST_DIR": manifest_dir,
            "CARGO_PKG_NAME": args.package_name,
            "CARGO_PKG_VERSION": args.version,
            "DEBUG": "false",
            "HOST": host,
            "OPT_LEVEL": "0",
            "OUT_DIR": out_dir,
            "PROFILE": "debug",
            "RUSTC": rustc,
            "TARGET": host,
        }
    )
    for feature in args.feature:
        env["CARGO_FEATURE_" + feature.upper().replace("-", "_")] = "1"

    result = subprocess.run(
        [os.path.abspath(args.build_script)],
        cwd=manifest_dir,
        env=env,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    )
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.exit(result.returncode)

    unsupported = [
        line
        for line in result.stdout.splitlines()
        if line.startswith("cargo:") and not IGNORED_INSTRUCTION.match(line)
    ]
    if unsupported:
        sys.exit(
            f"{args.package_name}: the build script printed instructions that Buck does not apply:\n"
            + "\n".join(unsupported)
        )


if __name__ == "__main__":
    main()
