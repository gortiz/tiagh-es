#!/usr/bin/env python3
"""
godot_translation.py - Godot 4 Binary Translation (.translation / RSRC) Parser & Serializer.

Handles reading and writing Godot 4 `Translation` resources with full fidelity.
"""

import io
import struct
from typing import Dict, List, Optional, Tuple, Any


def save_unicode_string(bio: io.BytesIO, s: str) -> None:
    """Writes a null-terminated UTF-8 string prefixed by its total byte length."""
    encoded = s.encode("utf-8") + b"\x00"
    bio.write(struct.pack("<I", len(encoded)))
    bio.write(encoded)


def read_unicode_string(bio: io.BytesIO) -> str:
    """Reads a null-terminated UTF-8 string prefixed by its total byte length."""
    length = struct.unpack("<I", bio.read(4))[0]
    if length == 0:
        return ""
    b = bio.read(length)
    if b.endswith(b"\x00"):
        b = b[:-1]
    return b.decode("utf-8", errors="replace")


def write_variant(bio: io.BytesIO, val: Any, string_map: List[str]) -> None:
    """Encodes a Variant according to Godot 4 binary resource format."""
    if val is None:
        bio.write(struct.pack("<I", 1))  # VARIANT_NIL
    elif isinstance(val, bool):
        bio.write(struct.pack("<II", 2, 1 if val else 0))  # VARIANT_BOOL
    elif isinstance(val, int):
        bio.write(struct.pack("<II", 3, val))  # VARIANT_INT
    elif isinstance(val, str):
        bio.write(struct.pack("<I", 5))  # VARIANT_STRING
        save_unicode_string(bio, val)
    elif isinstance(val, list):
        bio.write(struct.pack("<II", 30, len(val)))  # VARIANT_ARRAY
        for item in val:
            write_variant(bio, item, string_map)
    elif isinstance(val, tuple):
        # Tuple representing [msgctxt, msgid]
        bio.write(struct.pack("<II", 30, len(val)))  # VARIANT_ARRAY
        for item in val:
            write_variant(bio, item, string_map)
    elif isinstance(val, dict):
        bio.write(struct.pack("<II", 26, len(val)))  # VARIANT_DICTIONARY
        for k, v in val.items():
            write_variant(bio, k, string_map)
            write_variant(bio, v, string_map)
    else:
        raise ValueError(f"Unsupported variant type for Translation: {type(val)}")


def parse_variant(bio: io.BytesIO, string_map: List[str]) -> Any:
    """Parses a Godot 4 Variant value."""
    vtype = struct.unpack("<I", bio.read(4))[0]
    if vtype == 1:  # NIL
        return None
    elif vtype == 2:  # BOOL
        return bool(struct.unpack("<I", bio.read(4))[0])
    elif vtype == 3:  # INT
        return struct.unpack("<I", bio.read(4))[0]
    elif vtype == 40:  # INT64
        return struct.unpack("<Q", bio.read(8))[0]
    elif vtype in (5, 44):  # STRING or STRING_NAME
        return read_unicode_string(bio)
    elif vtype == 26:  # DICTIONARY
        dlen = struct.unpack("<I", bio.read(4))[0] & 0x7FFFFFFF
        d = {}
        for _ in range(dlen):
            k = parse_variant(bio, string_map)
            v = parse_variant(bio, string_map)
            if isinstance(k, list):
                k = tuple(k)
            d[k] = v
        return d
    elif vtype == 30:  # ARRAY
        alen = struct.unpack("<I", bio.read(4))[0] & 0x7FFFFFFF
        return [parse_variant(bio, string_map) for _ in range(alen)]
    elif vtype == 31:  # PACKED_BYTE_ARRAY
        blen = struct.unpack("<I", bio.read(4))[0]
        data = bio.read(blen)
        extra = 4 - (blen % 4)
        if extra < 4:
            bio.read(extra)
        return data
    elif vtype == 32:  # PACKED_INT32_ARRAY
        ilen = struct.unpack("<I", bio.read(4))[0]
        return [struct.unpack("<i", bio.read(4))[0] for _ in range(ilen)]
    elif vtype == 34:  # PACKED_STRING_ARRAY
        slen = struct.unpack("<I", bio.read(4))[0]
        return [read_unicode_string(bio) for _ in range(slen)]
    else:
        raise ValueError(f"Unhandled variant type: {vtype} at offset {bio.tell()}")


def parse_translation_bytes(raw: bytes) -> Dict[str, Any]:
    """
    Parses a raw Godot 4 .translation binary file into a dictionary:
    {
        'locale': str,
        'messages': Dict[Tuple[str, str], List[str]],
        'uid': int
    }
    """
    bio = io.BytesIO(raw)
    magic = bio.read(4)
    if magic != b"RSRC":
        raise ValueError(f"Invalid magic: expected RSRC, got {magic}")

    big_endian = struct.unpack("<I", bio.read(4))[0]
    use_real64 = struct.unpack("<I", bio.read(4))[0]
    ver_major = struct.unpack("<I", bio.read(4))[0]
    ver_minor = struct.unpack("<I", bio.read(4))[0]
    ver_format = struct.unpack("<I", bio.read(4))[0]
    res_type = read_unicode_string(bio)
    importmd_ofs = struct.unpack("<Q", bio.read(8))[0]
    flags = struct.unpack("<I", bio.read(4))[0]
    uid = struct.unpack("<Q", bio.read(8))[0]

    if flags & 8:  # Has script class
        read_unicode_string(bio)

    # 11 reserved uint32 fields
    for _ in range(11):
        struct.unpack("<I", bio.read(4))[0]

    # String table
    str_table_size = struct.unpack("<I", bio.read(4))[0]
    string_map = [read_unicode_string(bio) for _ in range(str_table_size)]

    # Ext resources
    ext_res_size = struct.unpack("<I", bio.read(4))[0]
    for _ in range(ext_res_size):
        read_unicode_string(bio)
        read_unicode_string(bio)
        if flags & 2:
            struct.unpack("<Q", bio.read(8))[0]

    # Int resources
    int_res_size = struct.unpack("<I", bio.read(4))[0]
    int_resources = []
    for _ in range(int_res_size):
        ir_path = read_unicode_string(bio)
        ir_offset = struct.unpack("<Q", bio.read(8))[0]
        int_resources.append((ir_path, ir_offset))

    def get_string_name():
        sid = struct.unpack("<I", bio.read(4))[0]
        if sid & 0x80000000:
            length = sid & 0x7FFFFFFF
            if length == 0:
                return ""
            b = bio.read(length)
            if b.endswith(b"\x00"):
                b = b[:-1]
            return b.decode("utf-8", errors="replace")
        return string_map[sid]

    # Parse primary internal resource
    bio.seek(int_resources[0][1])
    ir_type = read_unicode_string(bio)
    prop_count = struct.unpack("<I", bio.read(4))[0]
    props = {}
    for _ in range(prop_count):
        pname = get_string_name()
        pval = parse_variant(bio, string_map)
        props[pname] = pval

    return {
        "locale": props.get("locale", ""),
        "messages": props.get("messages", {}),
        "uid": uid,
    }


def save_translation_bytes(messages: Dict[Tuple[str, str], List[str]], locale: str = "es", uid: int = 0) -> bytes:
    """
    Serializes a message dictionary into Godot 4 .translation binary format (RSRC).
    """
    bio = io.BytesIO()

    # Magic & Engine version
    bio.write(b"RSRC")
    bio.write(struct.pack("<IIIII", 0, 0, 4, 6, 6))  # little endian, 32-bit float, major 4, minor 6, format 6
    save_unicode_string(bio, "Translation")
    bio.write(struct.pack("<Q", 0))  # importmd_ofs
    bio.write(struct.pack("<I", 3))  # flags: NAMED_SCENE_IDS | UIDS
    bio.write(struct.pack("<Q", uid))

    # 11 reserved fields
    for _ in range(11):
        bio.write(struct.pack("<I", 0))

    # String table
    strings = ["resource_local_to_scene", "resource_name", "messages", "locale", "plural_rules_override", "script"]
    str_indices = {s: i for i, s in enumerate(strings)}
    bio.write(struct.pack("<I", len(strings)))
    for s in strings:
        save_unicode_string(bio, s)

    # Ext resources: 0
    bio.write(struct.pack("<I", 0))

    # Int resources: 1
    bio.write(struct.pack("<I", 1))
    save_unicode_string(bio, f"local://Translation_{locale}")
    ofs_pos = bio.tell()
    bio.write(struct.pack("<Q", 0))  # Placeholder offset

    # Internal resource body
    body_offset = bio.tell()
    save_unicode_string(bio, "Translation")

    props = [
        (str_indices["messages"], messages),
        (str_indices["locale"], locale),
        (str_indices["script"], None),
    ]
    bio.write(struct.pack("<I", len(props)))
    for name_idx, val in props:
        bio.write(struct.pack("<I", name_idx))
        write_variant(bio, val, strings)

    # Patch offset of internal resource
    end_pos = bio.tell()
    bio.seek(ofs_pos)
    bio.write(struct.pack("<Q", body_offset))
    bio.seek(end_pos)

    # End marker
    bio.write(b"RSRC")

    return bio.getvalue()
