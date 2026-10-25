#!/usr/bin/env python3
"""Build the EXIF fixtures byte by byte.

piexif writes an IFD1 pointer that kamadak-exif rejects, so the fixture is built
here instead: a TIFF header, three IFDs and the values they point at, spliced into
a plain JPEG as an APP1 segment. Every offset is computed, not guessed, and PIL
reads the result back as a check that the file is genuinely valid EXIF.

    python3 scripts/make_exif_fixtures.py
"""
import struct
from pathlib import Path

from PIL import Image

FIX = Path("/root/work/imgtool/tests/fixtures")

ASCII, SHORT, LONG, RATIONAL = 2, 3, 4, 5


class Ifd:
    """One image file directory, with entries added in tag order."""

    def __init__(self):
        self.entries = []          # (tag, type, values)

    def add(self, tag, kind, values):
        self.entries.append((tag, kind, values))
        return self

    def count_of(self, kind, values):
        if kind == ASCII:
            return len(values.encode("ascii")) + 1
        return len(values)

    def size_of(self, kind, values):
        unit = {ASCII: 1, SHORT: 2, LONG: 4, RATIONAL: 8}[kind]
        return unit * self.count_of(kind, values)

    def encode(self, kind, values):
        if kind == ASCII:
            return values.encode("ascii") + b"\x00"
        if kind == SHORT:
            return b"".join(struct.pack("<H", v) for v in values)
        if kind == LONG:
            return b"".join(struct.pack("<I", v) for v in values)
        if kind == RATIONAL:
            return b"".join(struct.pack("<II", n, d) for n, d in values)
        raise ValueError(kind)


def build_tiff() -> bytes:
    ifd0 = (
        Ifd()
        .add(0x010F, ASCII, "Canon")          # Make
        .add(0x0110, ASCII, "Canon EOS R5")   # Model
        .add(0x0112, SHORT, [1])              # Orientation
        .add(0x0131, ASCII, "imgsmith fixtures")
    )
    exif_ifd = (
        Ifd()
        .add(0x829A, RATIONAL, [(1, 250)])    # ExposureTime
        .add(0x829D, RATIONAL, [(4, 1)])      # FNumber
        .add(0x8827, SHORT, [400])            # ISO
        .add(0x9003, ASCII, "2026:03:01 14:22:31")  # DateTimeOriginal
        .add(0x920A, RATIONAL, [(105, 1)])    # FocalLength
        .add(0xA002, LONG, [64])              # PixelXDimension
        .add(0xA003, LONG, [48])              # PixelYDimension
        .add(0xA434, ASCII, "RF24-105mm F4 L IS USM")  # LensModel
    )
    gps_ifd = (
        Ifd()
        .add(0x0001, ASCII, "N")
        .add(0x0002, RATIONAL, [(51, 1), (30, 1), (2660, 100)])
        .add(0x0003, ASCII, "W")
        .add(0x0004, RATIONAL, [(0, 1), (7, 1), (4008, 100)])
    )

    # Tag for the pointers to the two sub-IFDs.
    EXIF_POINTER, GPS_POINTER = 0x8769, 0x8825
    ifd0.add(EXIF_POINTER, LONG, [0]).add(GPS_POINTER, LONG, [0])
    ifd0.entries.sort(key=lambda e: e[0])

    def encoded_ifd(ifd, base_offset):
        """Entries, then the out-of-line values, then the next-IFD pointer."""
        count = len(ifd.entries)
        header_len = 2 + count * 12 + 4
        data = bytearray()
        entries = bytearray(struct.pack("<H", count))
        cursor = base_offset + header_len
        for tag, kind, values in ifd.entries:
            payload = ifd.encode(kind, values)
            if len(payload) <= 4:
                entries += struct.pack("<HHI", tag, kind, ifd.count_of(kind, values))
                entries += payload.ljust(4, b"\x00")
            else:
                entries += struct.pack("<HHI", tag, kind, ifd.count_of(kind, values))
                entries += struct.pack("<I", cursor + len(data))
                data += payload
                if len(data) % 2:
                    data += b"\x00"
        entries += struct.pack("<I", 0)   # no next IFD
        return bytes(entries) + bytes(data), cursor + len(data)

    tiff_header = struct.pack("<2sHI", b"II", 42, 8)
    # Lay out IFD0, then Exif, then GPS. Sizes are known once each is encoded, so
    # encode IFD0 twice: once to learn its length, once with the real pointers.
    probe, _ = encoded_ifd(ifd0, 8)
    exif_offset = 8 + len(probe)
    exif_bytes, exif_end = encoded_ifd(exif_ifd, exif_offset)
    gps_offset = exif_end
    gps_bytes, _ = encoded_ifd(gps_ifd, gps_offset)

    exif_ifd_placeholder = EXIF_POINTER.to_bytes(2, "little")
    index = 0
    raw = bytearray(probe)
    while True:
        found = raw.find(exif_ifd_placeholder, index + 2)
        if found == -1:
            break
        # entry: tag(2) type(2) count(4) value(4)
        raw[found + 8:found + 12] = struct.pack("<I", exif_offset)
        index = found + 12
        break
    gps_placeholder = GPS_POINTER.to_bytes(2, "little")
    index = 0
    while True:
        found = raw.find(gps_placeholder, index + 2)
        if found == -1:
            break
        raw[found + 8:found + 12] = struct.pack("<I", gps_offset)
        index = found + 12
        break

    return tiff_header + bytes(raw) + exif_bytes + gps_bytes


def splice_exif(jpeg: bytes, tiff: bytes) -> bytes:
    payload = b"Exif\x00\x00" + tiff
    if len(payload) + 2 > 0xFFFF:
        raise ValueError("EXIF too large for one APP1 segment")
    segment = b"\xFF\xE1" + struct.pack(">H", len(payload) + 2) + payload
    return jpeg[:2] + segment + jpeg[2:]


def pattern(width: int, height: int):
    """A gradient with a different colour in each corner.

    A solid colour makes rotate, flip and resize produce identical bytes, which
    hides a broken transform in a golden test. This one moves.
    """
    image = Image.new("RGB", (width, height))
    pixels = image.load()
    assert pixels is not None
    for y in range(height):
        for x in range(width):
            pixels[x, y] = (
                (x * 255) // max(1, width - 1),
                (y * 255) // max(1, height - 1),
                128,
            )
    pixels[0, 0] = (255, 0, 0)
    pixels[width - 1, 0] = (0, 255, 0)
    pixels[0, height - 1] = (0, 0, 255)
    pixels[width - 1, height - 1] = (255, 255, 0)
    return image


def build_tagged() -> Path:
    path = FIX / "tagged.jpg"
    plain = FIX / "plain.jpg"
    pattern(64, 48).save(path, "JPEG", quality=95)
    pattern(32, 32).save(plain, "JPEG", quality=95)
    path.write_bytes(splice_exif(path.read_bytes(), build_tiff()))
    return path


def check(path: Path) -> None:
    """A second reader has to agree, or the fixture is only valid to its author."""
    with Image.open(path) as image:
        exif = image.getexif()
        assert exif.get(0x0110) == "Canon EOS R5", exif.get(0x0110)
        assert exif.get(0x0112) == 1
        sub = exif.get_ifd(0x8769)
        assert sub.get(0x9003) == "2026:03:01 14:22:31", sub.get(0x9003)
        assert sub.get(0x8827) == 400, sub.get(0x8827)
        gps = exif.get_ifd(0x8825)
        assert gps.get(1) == "N", gps
    print(f"{path.name}: PIL reads camera, date, ISO and GPS back")


if __name__ == "__main__":
    tagged = build_tagged()
    check(tagged)
    print(f"built {tagged} and {FIX / 'plain.jpg'}")
