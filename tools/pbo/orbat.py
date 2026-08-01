#!/usr/bin/env python3
"""Parse Eden mission.sqm (plaintext or de-rapified) into a class tree and
report ORBAT / slot statistics."""
import os
import re
import sys
import json
from collections import Counter, defaultdict


TOK = re.compile(r'''
    (?P<class>class\s+([A-Za-z_][\w]*)\s*(?::\s*([A-Za-z_][\w]*)\s*)?\{)
  | (?P<close>\};?)
  | (?P<arr>([A-Za-z_][\w]*)\s*\[\]\s*\+?=\s*\{)
  | (?P<kv>([A-Za-z_][\w]*)\s*=\s*("(?:[^"]|"")*"|[^;\n]*)\s*;)
''', re.X)


class Node:
    __slots__ = ("name", "parent", "props", "kids")

    def __init__(self, name, parent=None):
        self.name = name
        self.parent = parent
        self.props = {}
        self.kids = []


def skip_array(s, i):
    """i points just past the opening '{' of an array literal."""
    depth = 1
    instr = False
    while i < len(s) and depth:
        c = s[i]
        if instr:
            if c == '"':
                instr = False
        elif c == '"':
            instr = True
        elif c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
        i += 1
    while i < len(s) and s[i] in ' \t\r\n':
        i += 1
    if i < len(s) and s[i] == ';':
        i += 1
    return i


def parse(text):
    root = Node("")
    stack = [root]
    i = 0
    n = len(text)
    while i < n:
        m = TOK.search(text, i)
        if not m:
            break
        if m.group("class"):
            nd = Node(m.group(2), m.group(3))
            stack[-1].kids.append(nd)
            stack.append(nd)
            i = m.end()
        elif m.group("close"):
            if len(stack) > 1:
                stack.pop()
            i = m.end()
        elif m.group("arr"):
            key = m.group(6)
            j = skip_array(text, m.end())
            stack[-1].props[key + "[]"] = text[m.end():j].rstrip(" ;}\n\t\r")
            i = j
        else:
            v = m.group(9).strip()
            if v.startswith('"') and v.endswith('"'):
                v = v[1:-1].replace('""', '"')
            stack[-1].props[m.group(8)] = v
            i = m.end()
    return root


def walk(nd, depth=0):
    yield nd, depth
    for k in nd.kids:
        yield from walk(k, depth + 1)


def find(nd, name):
    for k in nd.kids:
        if k.name == name:
            return k
    return None


def analyse(path):
    text = open(path, encoding="utf-8", errors="replace").read()
    root = parse(text)
    out = {
        "file": os.path.basename(path)[:-4],
        "slots": 0, "groups": 0, "sides": Counter(), "side_slots": Counter(),
        "descs": [], "markers": 0, "marker_types": Counter(),
        "modules": Counter(), "triggers": 0, "logics": 0,
        "veh": 0, "objects": 0, "layers": 0, "waypoints": 0,
        "addons": 0, "respawn": None, "sqm_binarized": None,
    }
    # addons list
    for nd, _ in walk(root):
        if "addons[]" in nd.props:
            out["addons"] = max(out["addons"],
                                nd.props["addons[]"].count('"') // 2)

    def group_side(nd):
        s = nd.props.get("side", "")
        return s

    for nd, _ in walk(root):
        dt = nd.props.get("dataType", "")
        if dt == "Group":
            out["groups"] += 1
            out["sides"][group_side(nd) or "?"] += 1
        elif dt == "Object":
            out["objects"] += 1
            at = find(nd, "Attributes")
            ap = at.props if at is not None else {}
            if ap.get("isPlayable", "") == "1" or ap.get("isPlayer", "") == "1":
                out["slots"] += 1
                out["side_slots"][nd.props.get("side", "?")] += 1
                if "description" in ap:
                    out["descs"].append(ap["description"])
                if ap.get("isPlayer", "") == "1":
                    out["player_slot"] = out.get("player_slot", 0) + 1
        elif dt == "Marker":
            out["markers"] += 1
            out["marker_types"][nd.props.get("type", "?")] += 1
        elif dt == "Trigger":
            out["triggers"] += 1
        elif dt == "Logic":
            out["logics"] += 1
            out["modules"][nd.props.get("type", "?")] += 1
        elif dt == "Layer":
            out["layers"] += 1
        elif dt == "Waypoint":
            out["waypoints"] += 1

    # respawn / scenario attributes
    for nd, _ in walk(root):
        for key in ("respawn", "respawnDelay", "respawnTemplates[]",
                    "respawnOnStart"):
            if key in nd.props and out["respawn"] is None:
                pass
    sa = None
    for nd, _ in walk(root):
        if nd.name == "ScenarioData":
            sa = nd
    if sa is not None:
        out["scenarioData"] = dict(sa.props)
    return out


if __name__ == "__main__":
    d = sys.argv[1]
    rows = []
    for f in sorted(os.listdir(d)):
        if f.endswith(".sqm"):
            try:
                rows.append(analyse(os.path.join(d, f)))
            except Exception as e:
                sys.stderr.write("ERR %s: %s\n" % (f, e))
    json.dump(rows, open(sys.argv[2], "w"), default=str)
    print("parsed", len(rows))
