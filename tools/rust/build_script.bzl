"""Runs the build script of a first-party crate that uses recursa-codegen.

Use `first_party_build_script` in place of the prelude `buildscript_run` when the build script
only writes OUT_DIR. See tools/rust/run_build_script.py for why.
"""

def first_party_build_script(name, build_script, manifest_dir, package_name, version, features):
    """Defines `name`, a directory that is the OUT_DIR of the build script.

    Args:
        name: the target name.
        build_script: the `rust_binary` target of the build script.
        manifest_dir: a `filegroup` target with the crate sources. A filegroup copies its
            sources, so the directory has no symlinks.
        package_name: the Cargo package name.
        version: the Cargo package version.
        features: the enabled Cargo features.
    """
    native.genrule(
        name = name,
        out = "OUT_DIR",
        cmd = " ".join([
            "python3 $(location //tools/rust:run_build_script.py)",
            "--build-script $(exe {})".format(build_script),
            "--manifest-dir $(location {})".format(manifest_dir),
            "--out-dir $OUT",
            "--package-name {}".format(package_name),
            "--version {}".format(version),
        ] + ["--feature {}".format(feature) for feature in features]),
    )
