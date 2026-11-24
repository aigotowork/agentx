"""Check and run a cargo-dist CLI archive on its matching native runner."""

import argparse
import hashlib
from pathlib import Path, PurePosixPath
import shutil
import stat
import subprocess
import tarfile
import tempfile
import zipfile


def require(condition, message):
    if not condition:
        raise ValueError(message)


def check_checksum(archive):
    record = archive.with_name(archive.name + ".sha256").read_text().split()
    checksum = hashlib.sha256()
    with archive.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            checksum.update(chunk)
    require(
        len(record) == 2 and record[1].lstrip("*") == archive.name and record[0] == checksum.hexdigest(),
        f"SHA-256 mismatch for {archive.name}",
    )


def extract_cli(archive, executable):
    # Read only the executable bytes; no archive path controls the extraction destination.
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as bundle:
            matches = [item for item in bundle.infolist() if PurePosixPath(item.filename).name == executable.name]
            require(len(matches) == 1 and not matches[0].is_dir(), "expected one agentx.exe file")
            with bundle.open(matches[0]) as source, executable.open("wb") as dest:
                shutil.copyfileobj(source, dest)
    else:
        with tarfile.open(archive, "r:gz") as bundle:
            matches = [item for item in bundle.getmembers() if PurePosixPath(item.name).name == executable.name]
            require(len(matches) == 1 and matches[0].isfile(), "expected one agentx file")
            require(matches[0].mode & stat.S_IXUSR, "archived agentx must be executable")
            with bundle.extractfile(matches[0]) as source, executable.open("wb") as dest:
                shutil.copyfileobj(source, dest)
        executable.chmod(0o700)


def check_architecture(executable, target):
    if target.endswith("-apple-darwin"):
        expected = "arm64" if target.startswith("aarch64-") else "x86_64"
        actual = subprocess.check_output(["lipo", "-archs", str(executable)], text=True).strip()
        require(actual == expected, f"expected Mach-O {expected}, got {actual!r}")
    else:
        with executable.open("rb") as file:
            header = file.read(64)
            if target.endswith("-pc-windows-msvc"):
                require(len(header) == 64 and header.startswith(b"MZ"), "expected a Windows PE executable")
                file.seek(int.from_bytes(header[60:64], "little"))
                require(file.read(6) == b"PE\0\0\x64\x86", "expected a Windows AMD64 executable")
            else:
                require(
                    header.startswith(b"\x7fELF\x02\x01") and header[18:20] == b"\x3e\x00",
                    "expected a Linux x86-64 ELF executable",
                )


def verify(directory, target, version):
    windows = target.endswith("-pc-windows-msvc")
    extension = "zip" if windows else "tar.gz"
    archive = directory / f"agentx-{target}.{extension}"
    check_checksum(archive)
    with tempfile.TemporaryDirectory(prefix="agentx-archive-") as tmp:
        executable = Path(tmp) / ("agentx.exe" if windows else "agentx")
        extract_cli(archive, executable)
        check_architecture(executable, target)
        actual = subprocess.check_output([str(executable), "--version"], cwd=tmp, timeout=30, text=True).strip()
        require(actual == f"agentx {version}", f"expected agentx {version}, got {actual!r}")
    print(f"{archive.name}: checksum, architecture and {actual} verified")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact_directory", type=Path)
    parser.add_argument("target")
    parser.add_argument("version")
    args = parser.parse_args()
    try:
        verify(args.artifact_directory, args.target, args.version)
    except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile, subprocess.SubprocessError) as exc:
        parser.exit(1, f"{args.target}: {exc}\n")
