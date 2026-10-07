"""Read and write ZIP archive files.

Stored (uncompressed) archives can be written in mode "w". Reading,
append and exclusive-creation modes, compression, and ZIP64 are not
implemented.
"""

import os
import struct
import zlib

ZIP_STORED = 0
ZIP_DEFLATED = 8

error = Exception
BadZipFile = error


def _encode_filename(filename):
    if all(ord(character) < 128 for character in filename):
        return filename.encode("ascii"), 0
    return filename.encode("utf-8"), 0x800


class ZipFile:
    """A ZIP archive, written in the stored form."""

    def __init__(self, file, mode="r", compression=ZIP_STORED, allowZip64=True,
                 compresslevel=None, *, strict_timestamps=True,
                 metadata_encoding=None):
        if mode not in ("r", "w", "a", "x"):
            raise ValueError("mode must be 'r', 'w', 'a', or 'x'")
        self.filename = file
        self.mode = mode
        self.compression = compression
        self._entries = []
        self._file = None
        self._closed = False
        if mode == "w":
            self._file = open(file, "wb")

    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        self.close()

    def _check_write(self):
        if self._closed:
            raise ValueError("Attempt to write to ZIP archive that was already closed")
        if self.mode == "r":
            raise ValueError("write() requires mode 'w', 'x', or 'a'")
        if self.mode != "w":
            raise NotImplementedError("only mode 'w' is supported for writing")

    def write(self, filename, arcname=None, compress_type=None, *,
              filterfunc=None):
        self._check_write()
        if compress_type is None:
            compress_type = self.compression
        if compress_type != ZIP_STORED:
            raise NotImplementedError("only the stored (uncompressed) form is supported")
        if arcname is None:
            arcname = filename
        arcname = arcname.replace(os.sep, "/")
        with open(filename, "rb") as source:
            data = source.read()
        self._entries.append((arcname, data))

    def close(self):
        if self._file is None:
            self._closed = True
            return
        out = self._file
        records = []
        for arcname, data in self._entries:
            records.append((arcname, data, out.tell()))
            name, flags = _encode_filename(arcname)
            crc = zlib.crc32(data) & 0xffffffff
            header = struct.pack(
                "<4sHHHHHIIIHH",
                b"PK\x03\x04", 20, flags, 0, 0, 0,
                crc, len(data), len(data), len(name), 0,
            )
            out.write(header)
            out.write(name)
            out.write(data)
        directory_offset = out.tell()
        for arcname, data, header_offset in records:
            name, flags = _encode_filename(arcname)
            crc = zlib.crc32(data) & 0xffffffff
            header = struct.pack(
                "<4sHHHHHHIIIHHHHHII",
                b"PK\x01\x02", 20, 20, flags, 0, 0, 0,
                crc, len(data), len(data), len(name), 0, 0, 0, 0, 0, header_offset,
            )
            out.write(header)
            out.write(name)
        directory_size = out.tell() - directory_offset
        end = struct.pack(
            "<4s4H2LH",
            b"PK\x05\x06", 0, 0, len(self._entries), len(self._entries),
            directory_size, directory_offset, 0,
        )
        out.write(end)
        out.close()
        self._file = None
        self._closed = True
        self._entries = []

    def writestr(self, zinfo_or_arcname, data, compress_type=None,
                 compresslevel=None):
        if isinstance(data, str):
            data = data.encode("utf-8")
        self._check_write()
        if compress_type is None:
            compress_type = self.compression
        if compress_type != ZIP_STORED:
            raise NotImplementedError("only the stored (uncompressed) form is supported")
        if not isinstance(zinfo_or_arcname, str):
            raise TypeError("only a plain string arcname is supported")
        self._entries.append((zinfo_or_arcname.replace(os.sep, "/"), data))
