"""The universal APK must carry both architectures and none of the old signatures."""

import sys
import unittest
import zipfile
from pathlib import Path
from tempfile import TemporaryDirectory

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "tools"))

import universal_apk  # noqa: E402


def build_apk(path: Path, abi: str, extra_library: str | None = None) -> None:
    with zipfile.ZipFile(path, "w") as apk:
        apk.writestr("AndroidManifest.xml", b"manifest-" + abi.encode())
        apk.writestr("resources.arsc", b"arsc", zipfile.ZIP_STORED)
        apk.writestr("classes.dex", b"dex")
        apk.writestr(f"lib/{abi}/libhorizon.so", b"so-" + abi.encode())
        if extra_library:
            apk.writestr(extra_library, b"extra")
        apk.writestr("META-INF/CERT.SF", b"signature")
        apk.writestr("META-INF/CERT.RSA", b"signature")


class UniversalApkTest(unittest.TestCase):
    def setUp(self):
        self.directory = TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.arm64 = self.root / "arm64.apk"
        self.armv7 = self.root / "armv7.apk"
        self.out = self.root / "universal.apk"
        build_apk(self.arm64, "arm64-v8a")
        build_apk(self.armv7, "armeabi-v7a")

    def tearDown(self):
        self.directory.cleanup()

    def test_both_architectures_survive(self):
        universal_apk.merge(self.arm64, self.armv7, self.out)
        with zipfile.ZipFile(self.out) as apk:
            names = apk.namelist()
            self.assertIn("lib/arm64-v8a/libhorizon.so", names)
            self.assertIn("lib/armeabi-v7a/libhorizon.so", names)
            self.assertEqual(apk.read("lib/arm64-v8a/libhorizon.so"), b"so-arm64-v8a")
            self.assertEqual(apk.read("lib/armeabi-v7a/libhorizon.so"), b"so-armeabi-v7a")

    def test_old_signatures_are_dropped(self):
        universal_apk.merge(self.arm64, self.armv7, self.out)
        with zipfile.ZipFile(self.out) as apk:
            self.assertFalse([n for n in apk.namelist() if n.startswith("META-INF/")])

    def test_the_primary_apk_supplies_everything_but_libraries(self):
        universal_apk.merge(self.arm64, self.armv7, self.out)
        with zipfile.ZipFile(self.out) as apk:
            self.assertEqual(apk.read("AndroidManifest.xml"), b"manifest-arm64-v8a")
            self.assertEqual(apk.read("classes.dex"), b"dex")

    def test_resources_stay_uncompressed(self):
        universal_apk.merge(self.arm64, self.armv7, self.out)
        with zipfile.ZipFile(self.out) as apk:
            self.assertEqual(apk.getinfo("resources.arsc").compress_type, zipfile.ZIP_STORED)

    def test_a_second_apk_without_libraries_is_refused(self):
        same_abi = self.root / "same.apk"
        build_apk(same_abi, "arm64-v8a")
        with self.assertRaises(SystemExit):
            universal_apk.merge(self.arm64, same_abi, self.out)
        self.assertFalse(self.out.exists())

    def test_signature_scheme_verification(self):
        v2_v3_only = (
            "Verifies\n"
            "Verified using v1 scheme (JAR signing): false\n"
            "Verified using v2 scheme (APK Signature Scheme v2): true\n"
            "Verified using v3 scheme (APK Signature Scheme v3): true\n"
        )
        with self.assertRaises(SystemExit):
            universal_apk.verify_signature_schemes(v2_v3_only, ("v1", "v2", "v3"))
        universal_apk.verify_signature_schemes(v2_v3_only, ("v2", "v3"))


if __name__ == "__main__":
    unittest.main()
