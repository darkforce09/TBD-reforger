#!/usr/bin/env python3
"""Extract Arma PBO archives. No dependencies.

    ./unpbo.py <file.pbo|dir-of-pbos> <outdir> [--list]

PBO layout: a header of null-terminated-filename + 5 uint32 entries
(packing, original_size, reserved, timestamp, data_size), terminated by an entry
with an empty filename, then every entry's payload concatenated in header order.

Packing methods seen in the wild:
  0x00000000  uncompressed
  0x43707273  'Cprs' — LZSS/LZH compressed, decompressed below
  0x56657273  'Vers' — the leading version/properties entry, carries no payload
"""
import os
import struct
import sys


def read_cstr(buf, pos):
    end = buf.index(b"\x00", pos)
    return buf[pos:end].decode("utf-8", "replace"), end + 1


def lzss_decompress(src, expected):
    """Arma's LZSS. 4096-byte ring buffer preloaded with 0x20, 18-byte max match.

    The trailing 4-byte checksum is not verified — a PBO that lies about its
    checksum still decompresses, and refusing it would lose readable content.
    """
    out = bytearray()
    text = bytearray(b"\x20" * 4096)
    r = 4096 - 18
    i = 0
    flags = 0
    while len(out) < expected and i < len(src):
        flags >>= 1
        if not (flags & 0x100):
            if i >= len(src):
                break
            flags = src[i] | 0xFF00
            i += 1
        if flags & 1:                                   # literal
            if i >= len(src):
                break
            b = src[i]; i += 1
            out.append(b); text[r] = b; r = (r + 1) & 4095
        else:                                           # back-reference
            if i + 1 >= len(src):
                break
            a, b = src[i], src[i + 1]; i += 2
            off = a | ((b & 0xF0) << 4)
            length = (b & 0x0F) + 3
            for k in range(length):
                if len(out) >= expected:
                    break
                c = text[(off + k) & 4095]
                out.append(c); text[r] = c; r = (r + 1) & 4095
    return bytes(out)


def extract(pbo_path, outdir, list_only=False):
    with open(pbo_path, "rb") as fh:
        buf = fh.read()
    pos = 0
    entries = []
    while True:
        name, pos = read_cstr(buf, pos)
        if pos + 20 > len(buf):
            break
        packing, orig, _res, _ts, size = struct.unpack_from("<5I", buf, pos)
        pos += 20
        if name == "" and packing == 0:                 # header terminator
            break
        if packing == 0x56657273:                       # 'Vers' properties block
            while True:                                 # key\0value\0 … \0
                k, pos = read_cstr(buf, pos)
                if k == "":
                    break
                _v, pos = read_cstr(buf, pos)
            continue
        entries.append((name, packing, orig, size))

    data = pos
    written = 0
    for name, packing, orig, size in entries:
        payload = buf[data:data + size]
        data += size
        if list_only:
            print(f"  {size:>10}  {name}")
            continue
        if packing == 0x43707273:                       # 'Cprs'
            try:
                payload = lzss_decompress(payload, orig)
            except Exception as exc:                    # noqa: BLE001
                print(f"    ! lzss failed {name}: {exc}", file=sys.stderr)
        rel = name.replace("\\", os.sep)
        dest = os.path.join(outdir, rel)
        os.makedirs(os.path.dirname(dest) or ".", exist_ok=True)
        with open(dest, "wb") as out:
            out.write(payload)
        written += 1
    return len(entries), written


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(2)
    src, outroot = sys.argv[1], sys.argv[2]
    list_only = "--list" in sys.argv
    pbos = ([os.path.join(src, f) for f in sorted(os.listdir(src)) if f.lower().endswith(".pbo")]
            if os.path.isdir(src) else [src])
    total_e = total_w = failed = 0
    for p in pbos:
        stem = os.path.splitext(os.path.basename(p))[0]
        dest = os.path.join(outroot, stem)
        try:
            e, w = extract(p, dest, list_only)
            total_e += e; total_w += w
            print(f"{os.path.basename(p):<50} {e:>5} entries")
        except Exception as exc:                        # noqa: BLE001
            failed += 1
            print(f"{os.path.basename(p):<50} FAILED: {exc}", file=sys.stderr)
    print(f"\n{len(pbos)} pbo(s), {total_e} entries, {total_w} written, {failed} failed")


if __name__ == "__main__":
    main()
