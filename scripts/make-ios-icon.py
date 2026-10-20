#!/usr/bin/env python3
"""The iOS app icon: the logo, on a background that does not go anywhere.

    scripts/make-ios-icon.py [--source assets/icons/concat_logo_512.png]
                             [--out assets/icons/concat_logo_512_ios.png]

The logo is a rounded square, so its corners are transparent - which is
right for macOS, where the corners belong to the desktop showing through,
and wrong for iOS: an icon there is a plain square, and the system rounds
it itself. An icon with transparent corners is shown with black ones.

So the corners are filled with the brand blue the logo is drawn in, which
is what the square logo looks like - assets/new_concat_logo_512_square_light_theme.svg
- without needing a renderer to draw it.

No third-party library: the PNG is read and written here, because the
only thing needed is to put a colour under the pixels that have none.
"""
from __future__ import annotations

import argparse
import pathlib
import struct
import sys
import zlib

# The blue the logo's rounded square is filled with, and so the background
# the transparent corners take.
BLUE = (0x05, 0x68, 0xFD)

ROOT = pathlib.Path(__file__).resolve().parent.parent


def chunks(data: bytes):
    """The PNG's chunks, in order: (kind, body)."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos = 8
    while pos < len(data):
        length = struct.unpack(">I", data[pos : pos + 4])[0]
        kind = data[pos + 4 : pos + 8]
        yield kind, data[pos + 8 : pos + 8 + length]
        pos += 12 + length


def read(path: pathlib.Path) -> tuple[int, int, list[list[tuple[int, int, int, int]]]]:
    """The pixels of a PNG: its width, its height, and its rows of RGBA."""
    data = path.read_bytes()
    width = height = depth = colour = 0
    idat = b""
    for kind, body in chunks(data):
        if kind == b"IHDR":
            width, height, depth, colour = struct.unpack(">IIBB", body[:10])
        elif kind == b"IDAT":
            idat += body
    if depth != 8 or colour not in (2, 6):
        raise ValueError(f"only 8-bit RGB and RGBA PNGs are read, not {depth}-bit {colour}")
    channels = 4 if colour == 6 else 3
    raw = zlib.decompress(idat)
    stride = width * channels
    rows: list[list[tuple[int, int, int, int]]] = []
    previous = bytearray(stride)
    at = 0
    for _ in range(height):
        kind = raw[at]
        at += 1
        line = bytearray(raw[at : at + stride])
        at += stride
        for x in range(stride):
            a = line[x - channels] if x >= channels else 0
            b = previous[x]
            c = previous[x - channels] if x >= channels else 0
            if kind == 1:
                line[x] = (line[x] + a) & 255
            elif kind == 2:
                line[x] = (line[x] + b) & 255
            elif kind == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                nearest = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[x] = (line[x] + nearest) & 255
            elif kind != 0:
                raise ValueError(f"unknown filter {kind}")
        row = []
        for x in range(0, stride, channels):
            pixel = line[x : x + channels]
            row.append(
                (
                    pixel[0],
                    pixel[1],
                    pixel[2],
                    pixel[3] if channels == 4 else 255,
                )
            )
        rows.append(row)
        previous = line
    return width, height, rows


def write(path: pathlib.Path, width: int, height: int, rows) -> None:
    """The pixels of a PNG, as an RGBA file."""
    raw = bytearray()
    for row in rows:
        raw.append(0)  # no filter
        for r, g, b, a in row:
            raw += bytes((r, g, b, a))

    def chunk(kind: bytes, body: bytes) -> bytes:
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source", type=pathlib.Path, default=ROOT / "assets/icons/concat_logo_512.png"
    )
    parser.add_argument(
        "--out", type=pathlib.Path, default=ROOT / "assets/icons/concat_logo_512_ios.png"
    )
    args = parser.parse_args()

    width, height, rows = read(args.source)
    filled = 0
    for y in range(height):
        row = rows[y]
        for x in range(width):
            r, g, b, a = row[x]
            if a == 0:
                row[x] = (BLUE[0], BLUE[1], BLUE[2], 255)
                filled += 1
            elif a < 255:
                # Half-transparent pixels are the edge of the rounded
                # square: the blue shows through them, as it would
                # through the transparent ones.
                alpha = a / 255
                row[x] = (
                    round(BLUE[0] * (1 - alpha) + r * alpha),
                    round(BLUE[1] * (1 - alpha) + g * alpha),
                    round(BLUE[2] * (1 - alpha) + b * alpha),
                    255,
                )
    write(args.out, width, height, rows)
    print(f"{args.out}: {width}x{height}, {filled} pixels filled with the brand blue")
    return 0


if __name__ == "__main__":
    sys.exit(main())
