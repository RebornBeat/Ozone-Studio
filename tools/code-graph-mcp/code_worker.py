"""Static call graph for Python sources, stdlib ast only.

Argv: one JSON object {"root": repo_root, "path": path relative to root}.
Prints one JSON object. Resolution is conservative: an edge is emitted only when the
callee is unique inside the scanned set; everything else is listed as unresolved.
"""
import ast
import builtins
import json
import os
import sys

SKIP_DIRS = {"__pycache__", ".venv", "venv", "node_modules", "target", ".git"}
BUILTINS = set(dir(builtins))
DEF_NODES = (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)
MAX_LISTED = 200


def fail(msg):
    print(json.dumps({"ok": False, "error": msg}))
    sys.exit(0)


def collect_files(root, target):
    if os.path.isfile(target):
        if not target.endswith(".py"):
            fail("path is not a .py file or a directory")
        return [target]
    files = []
    for dp, dns, fns in os.walk(target, followlinks=False):
        dns[:] = sorted(d for d in dns if d not in SKIP_DIRS and not d.startswith("."))
        for fn in sorted(fns):
            if not fn.endswith(".py"):
                continue
            full = os.path.join(dp, fn)
            if os.path.realpath(full).startswith(root + os.sep):
                files.append(full)
    return files


def dotted_name(root, file_path):
    rel = os.path.relpath(os.path.realpath(file_path), root)[:-3]
    parts = rel.split(os.sep)
    if parts[-1] == "__init__":
        parts = parts[:-1]
    return ".".join(parts)


def resolve_relative(module_dotted, is_init, level, module):
    package = module_dotted if is_init else module_dotted.rsplit(".", 1)[0] if "." in module_dotted else ""
    base_parts = package.split(".") if package else []
    if level - 1 > len(base_parts):
        return None
    base_parts = base_parts[: len(base_parts) - (level - 1)]
    base = ".".join(base_parts)
    if module:
        return f"{base}.{module}" if base else module
    return base or None


def main():
    args = json.loads(sys.argv[1])
    root = os.path.realpath(args["root"])
    target = os.path.realpath(os.path.join(root, args["path"]))
    if target != root and not target.startswith(root + os.sep):
        fail("path is outside the configured root")
    if not os.path.exists(target):
        fail("path does not exist")

    parse_errors = []
    modules = {}
    for f in collect_files(root, target):
        dotted = dotted_name(root, f)
        try:
            with open(f, encoding="utf-8") as fh:
                tree = ast.parse(fh.read(), filename=f)
        except (SyntaxError, UnicodeDecodeError, ValueError) as e:
            parse_errors.append({"file": os.path.relpath(f, root), "error": str(e)})
            continue
        modules[dotted] = {
            "file": os.path.relpath(f, root),
            "is_init": os.path.basename(f) == "__init__.py",
            "tree": tree,
        }

    nodes = [{
        "key": "root",
        "kind": "CodeRoot",
        "label": args["path"] or ".",
        "attributes": {"scanned_path": args["path"], "modules": len(modules)},
    }]
    edges = []
    defs = []  # dicts: key, module, qual, node, class_key, scope
    module_keys = {}
    for i, (dotted, info) in enumerate(sorted(modules.items())):
        mkey = f"m{i}"
        module_keys[dotted] = mkey
        nodes.append({"key": mkey, "kind": "Module", "label": dotted, "parent": "root",
                      "attributes": {"file": info["file"]}})

    counter = {"n": 0}

    def visit(node, module, parent_key, qual_prefix, class_key, in_function):
        for child in ast.iter_child_nodes(node):
            if isinstance(child, DEF_NODES):
                counter["n"] += 1
                key = f"d{counter['n']}"
                qual = f"{qual_prefix}.{child.name}" if qual_prefix else child.name
                kind = "Class" if isinstance(child, ast.ClassDef) else "Function"
                attrs = {"lineno": child.lineno, "end_lineno": child.end_lineno}
                if kind == "Function":
                    a = child.args
                    attrs["args"] = len(a.posonlyargs) + len(a.args) + len(a.kwonlyargs) \
                        + (1 if a.vararg else 0) + (1 if a.kwarg else 0)
                nodes.append({"key": key, "kind": kind, "label": f"{module}.{qual}",
                              "parent": parent_key, "attributes": attrs})
                defs.append({"key": key, "module": module, "qual": qual, "node": child,
                             "kind": kind, "class_key": class_key,
                             "scope": "function" if in_function else ("class" if class_key else "module")})
                if kind == "Class":
                    visit(child, module, key, qual, key, False)
                else:
                    visit(child, module, key, qual, None, True)
            else:
                visit(child, module, parent_key, qual_prefix, class_key, in_function)

    for dotted, info in sorted(modules.items()):
        visit(info["tree"], dotted, module_keys[dotted], "", None, False)

    by_key = {d["key"]: d for d in defs}
    module_funcs = {}  # module -> name -> [keys] for module-scope functions
    class_methods = {}  # class key -> name -> [keys]
    for d in defs:
        if d["kind"] != "Function":
            continue
        if d["scope"] == "module":
            module_funcs.setdefault(d["module"], {}).setdefault(d["node"].name, []).append(d["key"])
        elif d["scope"] == "class":
            class_methods.setdefault(d["class_key"], {}).setdefault(d["node"].name, []).append(d["key"])

    def resolve_name(name, importer, is_init):
        """Absolute dotted name, else the importer's own package (script-directory import)."""
        if name in modules:
            return name
        pkg = importer if is_init else (importer.rsplit(".", 1)[0] if "." in importer else "")
        cand = f"{pkg}.{name}" if pkg else name
        return cand if cand in modules else None

    # import bindings per module: alias -> ("module", dotted) | ("name", module, name) | ("external", text)
    unresolved_imports = []
    bindings = {}
    for dotted, info in sorted(modules.items()):
        b = {}
        for stmt in ast.walk(info["tree"]):
            if isinstance(stmt, ast.Import):
                for alias in stmt.names:
                    top = alias.name.split(".")[0]
                    local = alias.asname or top
                    full = resolve_name(alias.name, dotted, info["is_init"])
                    top_r = resolve_name(top, dotted, info["is_init"])
                    if alias.asname:
                        b[local] = ("module", full) if full else ("external", alias.name)
                    else:
                        b[local] = ("module", top_r) if top_r else ("external", alias.name)
                    if full:
                        edges.append({"from": module_keys[dotted], "to": module_keys[full],
                                      "relation": "ImportsFrom", "confidence": 1.0, "kind": "ImportsFrom", "line": stmt.lineno})
                    else:
                        unresolved_imports.append({"module": info["file"], "imports": alias.name, "line": stmt.lineno})
            elif isinstance(stmt, ast.ImportFrom):
                if stmt.level:
                    base = resolve_relative(dotted, info["is_init"], stmt.level, stmt.module)
                    base_r = base if base in modules else None
                else:
                    base = stmt.module or ""
                    base_r = resolve_name(base, dotted, info["is_init"]) if base else None
                if base_r is None and base:
                    for alias in stmt.names:
                        unresolved_imports.append({"module": info["file"], "imports": base, "line": stmt.lineno})
                    continue
                if base_r is None:
                    unresolved_imports.append({"module": info["file"], "imports": "relative", "line": stmt.lineno})
                    continue
                for alias in stmt.names:
                    if alias.name == "*":
                        unresolved_imports.append({"module": info["file"], "imports": f"{base_r}.*", "line": stmt.lineno})
                        continue
                    local = alias.asname or alias.name
                    sub = f"{base_r}.{alias.name}"
                    if sub in modules:
                        b[local] = ("module", sub)
                        target_mod = sub
                    else:
                        b[local] = ("name", base_r, alias.name)
                        target_mod = base_r
                    edges.append({"from": module_keys[dotted], "to": module_keys[target_mod],
                                  "relation": "ImportsFrom", "confidence": 1.0, "kind": "ImportsFrom", "line": stmt.lineno})
        bindings[dotted] = b

    unresolved_calls = []
    external_calls = 0
    seen_edges = set()

    def add_call_edge(caller_key, callee_key, line):
        k = (caller_key, callee_key)
        if k in seen_edges:
            return
        seen_edges.add(k)
        edges.append({"from": caller_key, "to": callee_key, "relation": "CallsTo",
                      "confidence": 1.0, "kind": "CallsTo", "line": line})

    def body_calls(fn_node):
        stack = list(fn_node.body) if hasattr(fn_node, "body") else []
        out = []
        while stack:
            n = stack.pop()
            if isinstance(n, DEF_NODES):
                continue
            if isinstance(n, ast.Call):
                out.append(n)
            stack.extend(ast.iter_child_nodes(n))
        return out

    def dotted_chain(expr):
        parts = []
        while isinstance(expr, ast.Attribute):
            parts.append(expr.attr)
            expr = expr.value
        if isinstance(expr, ast.Name):
            parts.append(expr.id)
            return list(reversed(parts))
        return None

    for d in defs:
        if d["kind"] != "Function":
            continue
        caller_key = d["key"]
        module = d["module"]
        here_b = bindings.get(module, {})
        for call in body_calls(d["node"]):
            fn = call.func
            line = call.lineno
            if isinstance(fn, ast.Name):
                name = fn.id
                if name in module_funcs.get(module, {}):
                    keys = module_funcs[module][name]
                    if len(keys) == 1:
                        add_call_edge(caller_key, keys[0], line)
                    else:
                        unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": name,
                                                 "reason": "ambiguous: defined more than once in this module"})
                elif name in here_b:
                    kind = here_b[name][0]
                    if kind == "name":
                        _, src_mod, src_name = here_b[name]
                        keys = module_funcs.get(src_mod, {}).get(src_name, [])
                        if len(keys) == 1:
                            add_call_edge(caller_key, keys[0], line)
                        else:
                            unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": name,
                                                     "reason": "imported name is not a unique module-level function in the scanned set"})
                    elif kind == "external":
                        external_calls += 1
                    else:
                        unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": name,
                                                 "reason": "name is a scanned module, not callable"})
                elif name in BUILTINS:
                    external_calls += 1
                else:
                    unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": name,
                                             "reason": "unknown name (star import, dynamic definition, or missing)"})
            elif isinstance(fn, ast.Attribute):
                chain = dotted_chain(fn)
                if chain is None:
                    unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": ast.unparse(fn)[:80],
                                             "reason": "dynamic callee (call result, subscript, or other expression)"})
                    continue
                base = chain[0]
                if base == "self" and d["class_key"] and len(chain) == 2:
                    keys = class_methods.get(d["class_key"], {}).get(chain[1], [])
                    if len(keys) == 1:
                        add_call_edge(caller_key, keys[0], line)
                    else:
                        unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": ".".join(chain),
                                                 "reason": "self method not unique in its class (or inherited)"})
                    continue
                if base in here_b:
                    kind = here_b[base][0]
                    if kind == "external":
                        external_calls += 1
                        continue
                    if kind == "module":
                        full = here_b[base][1] + ("." + ".".join(chain[1:]) if len(chain) > 1 else "")
                    else:
                        full = here_b[base][1] + "." + here_b[base][2] + ("." + ".".join(chain[1:]) if len(chain) > 1 else "")
                    mod, _, fname = full.rpartition(".")
                    keys = module_funcs.get(mod, {}).get(fname, []) if mod in modules else []
                    if len(keys) == 1:
                        add_call_edge(caller_key, keys[0], line)
                    else:
                        unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": ".".join(chain),
                                                 "reason": "attribute call does not resolve to a unique function in the scanned set"})
                elif base in BUILTINS:
                    external_calls += 1
                else:
                    unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": ".".join(chain),
                                             "reason": "unbound receiver (instance attribute, local variable, or missing import)"})
            else:
                unresolved_calls.append({"from": d["qual"], "module": module, "line": line, "callee": ast.unparse(fn)[:80],
                                         "reason": "dynamic callee"})

    seen_imports = set()
    deduped = []
    for e in edges:
        if e["kind"] == "ImportsFrom":
            k = (e["from"], e["to"])
            if k in seen_imports:
                continue
            seen_imports.add(k)
        deduped.append(e)
    edges = deduped

    total_unresolved = len(unresolved_calls)
    print(json.dumps({
        "ok": True,
        "graph": {"nodes": nodes, "edges": edges},
        "files_scanned": len(modules),
        "parse_errors": parse_errors,
        "unresolved_imports": unresolved_imports[:MAX_LISTED],
        "unresolved_imports_total": len(unresolved_imports),
        "unresolved_calls": unresolved_calls[:MAX_LISTED],
        "unresolved_calls_total": total_unresolved,
        "external_or_builtin_calls": external_calls,
        "definitions": len(defs),
    }))


if __name__ == "__main__":
    main()
