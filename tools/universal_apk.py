#!/usr/bin/env python3
"""Merges the per-architecture APKs into one universal APK.

Android will only install the native libraries that match the device, so a single APK
carrying both `lib/arm64-v8a` and `lib/armeabi-v7a` runs on either phone — which is what
someone who does not know their device's ABI needs.

The two inputs are built from the same commit and the same Gradle project, so everything
except `lib/` is identical; the arm64 APK donates every non-library entry, and the armv7
APK donates its libraries. Existing signatures are dropped (they only ever cover the
original file) and the result is aligned and signed again with the release identity.

    python3 tools/universal_apk.py \
        --arm64 /path/arm64.apk --armv7 /path/armv7.apk --out /path/universal.apk \
        --build-tools "$ANDROID_HOME/build-tools/35.0.0"
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

SIGNATURE_SUFFIXES = (".SF", ".RSA", ".DSA", ".EC", ".MF")


def is_signature(name: str) -> bool:
    upper = name.upper()
    return upper.startswith("META-INF/") and upper.endswith(SIGNATURE_SUFFIXES)


def is_library(name: str) -> bool:
    return name.startswith("lib/")


def abi_of(name: str) -> str:
    parts = name.split("/")
    return parts[1] if len(parts) > 2 else ""


def merge(primary: Path, secondary: Path, out: Path) -> tuple[list[str], list[str]]:
    """Copies `primary` and takes every library `secondary` has that `primary` lacks."""
    kept: dict[str, tuple[Path, int]] = {}
    order: list[str] = []
    with zipfile.ZipFile(primary) as first:
        for info in first.infolist():
            if is_signature(info.filename) or info.filename.endswith("/"):
                continue
            kept[info.filename] = (primary, info.compress_type)
            order.append(info.filename)
    added: list[str] = []
    with zipfile.ZipFile(secondary) as second:
        for info in second.infolist():
            if is_signature(info.filename) or info.filename.endswith("/"):
                continue
            if not is_library(info.filename) or info.filename in kept:
                continue
            kept[info.filename] = (secondary, info.compress_type)
            order.append(info.filename)
            added.append(info.filename)
    if not added:
        raise SystemExit(f"{secondary} contributed no native libraries; is it the same ABI?")
    with zipfile.ZipFile(out, "w") as target:
        for name in order:
            source, compress = kept[name]
            with zipfile.ZipFile(source) as archive:
                data = archive.read(name)
            # resources.arsc must stay uncompressed to stay mmap-able.
            stored = compress == zipfile.ZIP_STORED or name == "resources.arsc"
            target.writestr(zipfile.ZipInfo(name), data, zipfile.ZIP_STORED if stored else zipfile.ZIP_DEFLATED)
    return order, added


SCHEME_LABELS = {
    "v1": "v1 scheme (JAR signing)",
    "v2": "v2 scheme (APK Signature Scheme v2)",
    "v3": "v3 scheme (APK Signature Scheme v3)",
}


def verify_signature_schemes(output: str, required: tuple[str, ...] = ("v1", "v2", "v3")) -> None:
    for key in required:
        label = SCHEME_LABELS[key]
        if f"Verified using {label}: true" not in output:
            raise SystemExit(f"universal APK is not {key} signed")


def align_and_sign(
    apk: Path,
    build_tools: Path,
    keystore: str,
    password: str,
    alias: str,
    key_password: str,
    required_schemes: tuple[str, ...] = ("v1", "v2", "v3"),
) -> None:
    """zipalign then apksigner, verifying the required APK signature schemes."""
    align = build_tools / "zipalign"
    wrapper = build_tools / "apksigner"
    signer = build_tools / "lib" / "apksigner.jar"
    if not align.is_file() or not (wrapper.is_file() or signer.is_file()):
        raise SystemExit(f"build-tools at {build_tools} lacks zipalign or apksigner")
    # The wrapper script sets up the JVM the way the SDK expects; the jar is the fallback.
    launcher = [str(wrapper)] if wrapper.is_file() else ["java", "-jar", str(signer)]
    with tempfile.TemporaryDirectory() as directory:
        aligned = Path(directory) / apk.name
        subprocess.run([str(align), "-p", "-f", "4", str(apk), str(aligned)], check=True)
        shutil.copyfile(aligned, apk)
    subprocess.run(
        [
            *launcher, "sign",
            "--ks", keystore,
            "--ks-pass", f"env:HORIZON_ANDROID_KEYSTORE_PASSWORD",
            "--ks-key-alias", alias,
            "--key-pass", f"env:HORIZON_ANDROID_KEY_PASSWORD",
            "--v1-signing-enabled", "true" if "v1" in required_schemes else "false",
            "--v2-signing-enabled", "true",
            "--v3-signing-enabled", "true",
            str(apk),
        ],
        check=True,
    )
    verified = subprocess.run(
        [*launcher, "verify", "--verbose", "--print-certs", str(apk)],
        capture_output=True, text=True,
    )
    print(verified.stdout, verified.stderr)
    # apksigner exits non-zero when any scheme it was asked for is missing.
    if verified.returncode != 0:
        raise SystemExit(f"universal APK did not verify: exit {verified.returncode}")
    verify_signature_schemes(verified.stdout, required_schemes)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arm64", required=True, type=Path)
    parser.add_argument("--armv7", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--build-tools", type=Path)
    parser.add_argument("--sign", action="store_true", help="align and sign with the CI identity")
    parser.add_argument(
        "--schemes",
        default="v1,v2,v3",
        help="Comma-separated APK signature schemes to require (subset of v1,v2,v3; default v1,v2,v3)",
    )
    options = parser.parse_args()
    required_schemes = tuple(part.strip() for part in options.schemes.split(",") if part.strip())
    if not required_schemes or any(s not in SCHEME_LABELS for s in required_schemes):
        parser.error(f"--schemes must be a comma-separated subset of {','.join(SCHEME_LABELS)}")
    for apk in (options.arm64, options.armv7):
        if not apk.is_file():
            parser.error(f"{apk} does not exist")
    options.out.parent.mkdir(parents=True, exist_ok=True)
    for apk in (options.arm64, options.armv7):
        with zipfile.ZipFile(apk) as archive:
            dirs = sorted({abi_of(name) for name in archive.namelist() if is_library(name)})
        print(f"{apk.name}: {apk.stat().st_size:,} bytes, library ABIs {dirs}")
    entries, added = merge(options.arm64, options.armv7, options.out)
    abis = sorted({abi_of(name) for name in entries if is_library(name)})
    print(f"{options.out}: {len(entries)} entries, {len(added)} libraries added, ABIs {abis}")
    if not {"arm64-v8a", "armeabi-v7a"} <= set(abis):
        raise SystemExit(f"universal APK does not carry both architectures: {abis}")
    if options.sign:
        import os

        if not options.build_tools:
            parser.error("--sign needs --build-tools")
        align_and_sign(
            options.out,
            options.build_tools,
            os.environ["HORIZON_ANDROID_KEYSTORE"],
            os.environ["HORIZON_ANDROID_KEYSTORE_PASSWORD"],
            os.environ["HORIZON_ANDROID_KEY_ALIAS"],
            os.environ.get("HORIZON_ANDROID_KEY_PASSWORD", ""),
            required_schemes=required_schemes,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
