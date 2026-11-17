#!/usr/bin/env python3
"""Repair a macOS linker bug that leaves a dylib's symbol string table unaligned.

Some Apple ld releases (seen with ld-27106) do not pad the LINKEDIT string pool
to 8 bytes after an odd number of 4-byte indirect symbols, and dyld then refuses
to load the library ("mis-aligned LINKEDIT string pool"). When there is padding
room before the next LINKEDIT item, this moves the string table to the next
aligned offset and re-signs the library ad hoc. Aligned files are left untouched.
"""
import os
import struct
import subprocess
import sys

MH_MAGIC_64 = 0xFEEDFACF
LC_SYMTAB = 0x2
LC_CODE_SIGNATURE = 0x1D
ALIGN = 8


def repair(path):
    data = bytearray(open(path, "rb").read())
    magic, _, _, _, ncmds, _, _, _ = struct.unpack_from("<IiiIIIII", data, 0)
    if magic != MH_MAGIC_64:
        raise SystemExit(f"{path}: not a 64-bit little-endian Mach-O file")
    offset, symtab, signature = 32, None, None
    for _ in range(ncmds):
        cmd, size = struct.unpack_from("<II", data, offset)
        if cmd == LC_SYMTAB:
            symtab = offset
        elif cmd == LC_CODE_SIGNATURE:
            signature = struct.unpack_from("<II", data, offset + 8)[0]
        offset += size
    if symtab is None:
        return False
    _, _, symoff, nsyms, stroff, strsize = struct.unpack_from("<IIIIII", data, symtab)
    padding = (-stroff) % ALIGN
    if padding == 0:
        return False
    limit = signature if signature is not None else len(data)
    if stroff + strsize + padding > limit:
        raise SystemExit(f"{path}: no room to align the string table; relink required")
    data[stroff + padding : stroff + padding + strsize] = data[stroff : stroff + strsize]
    data[stroff : stroff + padding] = bytes(padding)
    struct.pack_into("<I", data, symtab + 16, stroff + padding)
    # Replace the file rather than rewrite it: macOS kills a process that loads
    # a library modified in place, because the kernel keeps its old signature.
    temporary = f"{path}.align.tmp"
    open(temporary, "wb").write(data)
    subprocess.run(["codesign", "--force", "--sign", "-", temporary], check=True, capture_output=True)
    os.replace(temporary, path)
    return True


if __name__ == "__main__":
    for library in sys.argv[1:]:
        if repair(library):
            print(f"aligned LINKEDIT string table: {library}")
