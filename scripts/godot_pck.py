#!/usr/bin/env python3
"""
godot_pck.py - Godot 4 PCK Reader, Extractor and Patcher.

Supports reading, extracting, and fast in-place / copy patching for Godot 4 .pck archives.
"""

import hashlib
import os
import shutil
import struct
from typing import Dict, List, Optional, Tuple


class PCKEntry:
    def __init__(self, path: str, offset: int, size: int, md5: bytes, flags: int):
        self.path = path
        self.offset = offset  # Relative to file_base
        self.size = size
        self.md5 = md5
        self.flags = flags


class GodotPCK:
    def __init__(self, pck_path: str):
        self.pck_path = pck_path
        self.magic: bytes = b""
        self.pack_format: int = 0
        self.ver_major: int = 0
        self.ver_minor: int = 0
        self.ver_patch: int = 0
        self.flags: int = 0
        self.file_base: int = 0
        self.directory_offset: int = 0
        self.entries: Dict[str, PCKEntry] = {}
        self._load()

    def _load(self) -> None:
        with open(self.pck_path, "rb") as f:
            self.magic = f.read(4)
            if self.magic != b"GDPC":
                raise ValueError(f"Invalid PCK magic: expected GDPC, got {self.magic}")

            self.pack_format = struct.unpack("<I", f.read(4))[0]
            self.ver_major = struct.unpack("<I", f.read(4))[0]
            self.ver_minor = struct.unpack("<I", f.read(4))[0]
            self.ver_patch = struct.unpack("<I", f.read(4))[0]
            self.flags = struct.unpack("<I", f.read(4))[0]
            self.file_base = struct.unpack("<Q", f.read(8))[0]
            self.directory_offset = struct.unpack("<Q", f.read(8))[0]

            # Seek to directory
            f.seek(self.directory_offset)
            file_count = struct.unpack("<I", f.read(4))[0]

            self.entries.clear()
            for _ in range(file_count):
                plen = struct.unpack("<I", f.read(4))[0]
                path_bytes = f.read(plen)
                path = path_bytes.decode("utf-8", errors="replace").strip("\x00")
                offset, size = struct.unpack("<QQ", f.read(16))
                md5 = f.read(16)
                flags = struct.unpack("<I", f.read(4))[0]
                self.entries[path] = PCKEntry(path, offset, size, md5, flags)

    def list_files(self) -> List[str]:
        return sorted(self.entries.keys())

    def has_file(self, path: str) -> bool:
        return path in self.entries

    def extract_file(self, path: str) -> bytes:
        if path not in self.entries:
            raise KeyError(f"File not found in PCK: {path}")
        entry = self.entries[path]
        with open(self.pck_path, "rb") as f:
            f.seek(self.file_base + entry.offset)
            return f.read(entry.size)

    def extract_to_disk(self, pck_file_path: str, destination_path: str) -> None:
        data = self.extract_file(pck_file_path)
        os.makedirs(os.path.dirname(destination_path), exist_ok=True)
        with open(destination_path, "wb") as f:
            f.write(data)

    def patch(self, output_pck_path: str, files_to_add_or_replace: Dict[str, bytes]) -> None:
        """
        Creates a patched PCK by copying original file payloads, appending new/modified
        files, and updating the directory table. Extremely fast because original data is preserved.
        """
        is_inplace = os.path.abspath(output_pck_path) == os.path.abspath(self.pck_path)
        temp_out = output_pck_path + ".tmp" if is_inplace else output_pck_path

        with open(self.pck_path, "rb") as src, open(temp_out, "wb") as dst:
            # 1. Copy everything from beginning up to the original directory offset
            src.seek(0)
            remaining = self.directory_offset
            chunk_size = 1024 * 1024 * 16  # 16 MB chunks
            while remaining > 0:
                to_read = min(remaining, chunk_size)
                buf = src.read(to_read)
                if not buf:
                    break
                dst.write(buf)
                remaining -= len(buf)

            # 2. Append new / modified files at the end of the data section
            updated_entries = dict(self.entries)
            dst.seek(0, os.SEEK_END)
            current_payload_pos = dst.tell()

            for path, data in files_to_add_or_replace.items():
                offset = current_payload_pos - self.file_base
                size = len(data)
                md5 = hashlib.md5(data).digest()
                flags = 0

                dst.write(data)
                current_payload_pos += size
                updated_entries[path] = PCKEntry(path, offset, size, md5, flags)

            # 3. Write new directory table
            new_dir_offset = dst.tell()
            dst.write(struct.pack("<I", len(updated_entries)))

            for path in sorted(updated_entries.keys()):
                entry = updated_entries[path]
                encoded_path = path.encode("utf-8") + b"\x00"
                # Padding to 4 bytes if necessary? Godot stores plen and string directly
                dst.write(struct.pack("<I", len(encoded_path)))
                dst.write(encoded_path)
                dst.write(struct.pack("<QQ", entry.offset, entry.size))
                dst.write(entry.md5)
                dst.write(struct.pack("<I", entry.flags))

            # 4. Update directory_offset in header (offset 32: uint64)
            dst.seek(32)
            dst.write(struct.pack("<Q", new_dir_offset))

        if is_inplace:
            shutil.move(temp_out, output_pck_path)

        # Reload state
        self._load()
