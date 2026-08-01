#!/usr/bin/env python3
"""De-rapify Arma binarized configs (\\0raP) back to readable config text.

Format (well documented, deterministic):
  header: "\\0raP", u32 always0, u32 always8, u32 enum_offset
  class body: parent asciiz, compressed-int nEntries, entries
  entry: u8 type
    0 class      : asciiz name, u32 body_offset
    1 value      : u8 subtype (0 str,1 float,2 long,3 ??), asciiz name, value
    2 array      : asciiz name, array
    3 extern     : asciiz name
    4 delete     : asciiz name
    5 array+=    : u32 flags, asciiz name, array
  array: compressed-int n, elements: u8 type (0 str,1 float,2 long,3 nested array)
"""
import struct
import sys


class R:
    def __init__(self, b):
        self.b = b
        self.p = 0

    def u8(self):
        v = self.b[self.p]
        self.p += 1
        return v

    def u32(self):
        v = struct.unpack_from("<I", self.b, self.p)[0]
        self.p += 4
        return v

    def i32(self):
        v = struct.unpack_from("<i", self.b, self.p)[0]
        self.p += 4
        return v

    def f32(self):
        v = struct.unpack_from("<f", self.b, self.p)[0]
        self.p += 4
        return v

    def cstr(self):
        e = self.b.index(b"\x00", self.p)
        s = self.b[self.p:e].decode("utf-8", "replace")
        self.p = e + 1
        return s

    def cint(self):
        v = 0
        shift = 0
        while True:
            byte = self.u8()
            v |= (byte & 0x7F) << shift
            if not (byte & 0x80):
                return v
            shift += 7


def esc(s):
    return '"' + s.replace('"', '""') + '"'


def num(v):
    if isinstance(v, float):
        if v == int(v) and abs(v) < 1e15:
            return str(int(v))
        return repr(v)
    return str(v)


def read_array(r):
    n = r.cint()
    out = []
    for _ in range(n):
        t = r.u8()
        if t == 0:
            out.append(esc(r.cstr()))
        elif t == 1:
            out.append(num(r.f32()))
        elif t == 2:
            out.append(num(r.i32()))
        elif t == 3:
            out.append("{" + ",".join(read_array(r)) + "}")
        else:
            out.append("/*?type%d*/" % t)
    return out


def read_class(r, off, name, depth, out, seen):
    if off in seen:
        out.append("\t" * depth + "// (cycle) class %s;" % name)
        return
    seen = seen | {off}
    r.p = off
    parent = r.cstr()
    n = r.cint()
    head = "class %s" % name + (": %s" % parent if parent else "")
    out.append("\t" * depth + head)
    out.append("\t" * depth + "{")
    subclasses = []
    for _ in range(n):
        t = r.u8()
        if t == 0:
            nm = r.cstr()
            bo = r.u32()
            subclasses.append((nm, bo))
        elif t == 1:
            st = r.u8()
            nm = r.cstr()
            if st == 0:
                v = esc(r.cstr())
            elif st == 1:
                v = num(r.f32())
            elif st == 2:
                v = num(r.i32())
            else:
                v = "/*?sub%d*/" % st
            out.append("\t" * (depth + 1) + "%s = %s;" % (nm, v))
        elif t == 2:
            nm = r.cstr()
            a = read_array(r)
            out.append("\t" * (depth + 1) + "%s[] = {%s};" % (nm, ",".join(a)))
        elif t == 3:
            out.append("\t" * (depth + 1) + "class %s;" % r.cstr())
        elif t == 4:
            out.append("\t" * (depth + 1) + "delete %s;" % r.cstr())
        elif t == 5:
            r.u32()
            nm = r.cstr()
            a = read_array(r)
            out.append("\t" * (depth + 1) + "%s[] += {%s};" % (nm, ",".join(a)))
        else:
            out.append("\t" * (depth + 1) + "// UNKNOWN entry type %d at %d" % (t, r.p))
            break
    save = r.p
    for nm, bo in subclasses:
        read_class(r, bo, nm, depth + 1, out, seen)
    r.p = save
    out.append("\t" * depth + "};")


def derap(path):
    b = open(path, "rb").read()
    if b[:4] != b"\x00raP":
        return None
    r = R(b)
    r.p = 4
    r.u32()
    r.u32()
    enum_off = r.u32()
    out = []
    read_class(r, r.p, "", 0, out, set())
    # strip the synthetic outer wrapper braces
    if out and out[0].startswith("class "):
        out = out[1:]
        if out and out[0] == "{":
            out = out[1:]
        if out and out[-1] == "};":
            out = out[:-1]
        out = [ln[1:] if ln.startswith("\t") else ln for ln in out]
    return "\n".join(out)


if __name__ == "__main__":
    for p in sys.argv[1:]:
        t = derap(p)
        if t is None:
            sys.stderr.write("NOT RAPIFIED: %s\n" % p)
        else:
            print("// ===== %s =====" % p)
            print(t)
