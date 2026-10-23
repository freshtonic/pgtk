#!/usr/bin/env python3
"""Makes the meta-command token patterns of `src/ast.rs`.

Each psql meta-command is one token: the backslash, the name, and the
argument region that psql reads for the command. The patterns follow the
states of `src/bin/psql/psqlscanslash.l`, and the argument type of each command
comes from `exec_command` in `src/bin/psql/command.c`
(packages/pgtk-sql/docs/adr/0013-keep-most-meta-command-arguments-as-text.md).

Usage:

    scripts/meta-command-patterns.py          # print each enum variant
    scripts/meta-command-patterns.py --check  # check that src/ast.rs has them

Change the tables here, then copy the printed variants into `src/ast.rs`.
"""

import pathlib
import sys


def parts(vt):
    """The parts of each pattern. `vt` is true from 17, where `space` of
    psqlscanslash.l holds the vertical tab.

    - `namech`: a character of a command name (`xslashcmd`), which ends at
      `space` or a backslash. psql reads one line, so a line break also ends it.
    - `piece`: one part of an argument (`xslasharg`): an unquoted character, or
      `\x27...\x27` with backslash escapes (`xslashquote`), `"..."`
      (`xslashdquote`) or `` `...` `` (`xslashbackquote`).
    - `normal`: `OT_NORMAL` arguments, the space after them, and a `\\\\`
      (`xslashend`).
    - `whole`: an `OT_WHOLE_LINE` argument (`xslashwholeline`).
    - `filepipe`: an `OT_FILEPIPE` argument, which is the rest of the line
      when it starts with `|`.
    - `g`: the arguments of `\\g` and `\\gx`: an option list that starts with
      `(`, then a file or pipe.
    """
    ws = r" \t\r\f\x0b" if vt else r" \t\r\f"
    sp = f"[{ws}]"
    namech = f"[^{ws}\\n\\\\]"
    argch = f"[^{ws}\\n\\\\'\"`]"
    quoted = r"'(?:[^'\\\n]|\\[^\n])*'"
    dquoted = r'"[^"\n]*"'
    bquoted = r"`[^`\n]*`"
    piece = f"(?:{argch}|{quoted}|{dquoted}|{bquoted})"
    arg = f"{piece}+"
    normal = f"(?:{sp}+{arg})*{sp}*(?:\\\\\\\\)?"
    whole = f"(?:(?:{sp}|\\\\)[^\\n]*)?"
    filepipe = f"(?:{sp}+\\|[^\\n]*|{normal})"
    paren = f"(?:{sp}+\\({piece}*(?:{sp}+{arg})*)?"
    g = f"{paren}{filepipe}"
    unknown_first = f"[^{ws}\\n\\\\;:]"
    return dict(sp=sp, namech=namech, normal=normal, whole=whole,
                filepipe=filepipe, g=g, unknown_first=unknown_first)


def names(alternatives, p):
    out = []
    for a in alternatives:
        if a == "d*":
            out.append("d" + p["namech"] + "*")
        elif a == "lo_*":
            out.append("lo_" + p["namech"] + "*")
        elif a == "copy":
            out.append("(?i:copy)")
        else:
            out.append(a.replace("+", r"\+").replace("?", r"\?"))
    return "(?:" + "|".join(out) + ")" if len(out) > 1 else out[0]


def pattern(alternatives, tail, vt):
    p = parts(vt)
    return "\\\\" + names(alternatives, p) + p[tail]


def unknown(vt):
    p = parts(vt)
    return "\\\\(?:" + p["unknown_first"] + p["namech"] + "*)?"


# The command names of each token, and how `exec_command` in command.c reads
# their arguments. `d*` and `lo_*` are every name with that start.
TOKENS = {
    "set": (["set"], "normal"),
    "unset": (["unset"], "normal"),
    "if": (["if"], "normal"),
    "elif": (["elif"], "normal"),
    "else": (["else"], "normal"),
    "endif": (["endif"], "normal"),
    "reset": (["r", "reset"], "normal"),
    "include": (["i", "include", "ir", "include_relative"], "normal"),
    "quit": (["q", "quit"], "normal"),
    "other": (["a", "C", "c", "connect", "cd", "conninfo", "copyright", "d*", "e", "edit",
               "echo", "qecho", "warn", "encoding", "errverbose", "f", "H", "html", "l",
               "list", "l+", "list+", "lo_*", "p", "print", "password", "prompt", "pset",
               "restrict", "s", "setenv", "t", "T", "timing", "x", "z", "?"], "normal"),
    "other15": (["getenv"], "normal"),
    "other16": (["bind", "zS"], "normal"),
    "other18": (["bind_named", "lx", "listx", "lx+", "listx+", "l+x", "list+x", "zx", "zSx",
                 "zxS"], "normal"),
    "wholeline": (["copy", "ef", "ev", "h", "help", "sf", "sf+", "sv", "sv+", "unrestrict",
                   "!"], "whole"),
    "output": (["o", "out", "w", "write"], "filepipe"),
    # Send commands.
    "g": (["g"], "g"),
    "gx": (["gx"], "g"),
    "gdesc": (["gdesc"], "normal"),
    "gexec": (["gexec"], "normal"),
    "gset": (["gset"], "normal"),
    "crosstabview": (["crosstabview"], "normal"),
    "watch": (["watch"], "normal"),
    "parse": (["parse"], "normal"),
    "sendpipeline": (["sendpipeline"], "normal"),
    "close_prepared": (["close_prepared"], "normal"),
    "endpipeline": (["endpipeline"], "normal"),
    "flushrequest": (["flushrequest"], "normal"),
    "flush": (["flush"], "normal"),
    "getresults": (["getresults"], "normal"),
    "startpipeline": (["startpipeline"], "normal"),
    "syncpipeline": (["syncpipeline"], "normal"),
}

def raw(s):
    return 'r#"' + s + '"#'


def variant(variant_name, type_name, key, cfg=None, priority=2, indent="        "):
    """Rust source for one enum variant whose field is a lexed token."""
    if key == "unknown":
        p17, p14 = unknown(True), unknown(False)
    else:
        alternatives, tail = TOKENS[key]
        p17, p14 = pattern(alternatives, tail, True), pattern(alternatives, tail, False)
    pr = f", priority = {priority}" if priority else ""
    lines = []
    if cfg:
        lines.append(f'{indent}#[cfg(feature = "{cfg}")]')
    lines.append(f"{indent}{variant_name}(")
    if cfg == "since-pg18":
        lines.append(f"{indent}    #[lex(pattern = {raw(p17)}{pr})]")
    else:
        lines.append(f'{indent}    #[cfg_attr(feature = "since-pg17", lex(pattern = {raw(p17)}{pr}))]')
        lines.append(f'{indent}    #[cfg_attr(not(feature = "since-pg17"), lex(pattern = {raw(p14)}{pr}))]')
    lines.append(f"{indent}    {type_name},")
    lines.append(f"{indent}),")
    return "\n".join(lines)


# Each enum variant of src/ast.rs: the variant, its token type, its key in
# TOKENS, the feature that gates it, and its priority (2 wins the tie with the
# `Unknown` catch-all, which has none).
ROWS = [
    ("Set", "SetCommandText", "set", None),
    ("Unset", "UnsetCommandText", "unset", None),
    ("If", "IfCommandText", "if", None),
    ("Elif", "ElifCommandText", "elif", None),
    ("Else", "ElseCommandText", "else", None),
    ("Endif", "EndifCommandText", "endif", None),
    ("Include", "IncludeCommandText", "include", None),
    ("Quit", "QuitCommandText", "quit", None),
    ("WholeLine", "WholeLineCommandText", "wholeline", None),
    ("Output", "OutputCommandText", "output", None),
    ("Other", "OtherCommandText", "other", None),
    ("OtherSince15", "OtherSince15CommandText", "other15", "since-pg15"),
    ("OtherSince16", "OtherSince16CommandText", "other16", "since-pg16"),
    ("OtherSince18", "OtherSince18CommandText", "other18", "since-pg18"),
    ("Unknown", "UnknownCommandText", "unknown", None, None),
    ("Crosstabview", "SendCrosstabview", "crosstabview", None),
    ("Parse", "SendParse", "parse", "since-pg18"),
    ("Sendpipeline", "SendSendpipeline", "sendpipeline", "since-pg18"),
    ("Gdesc", "SendGdesc", "gdesc", None),
    ("Gexec", "SendGexec", "gexec", None),
    ("Gset", "SendGset", "gset", None),
    ("Watch", "SendWatch", "watch", None),
    ("Gx", "SendGx", "gx", None),
    ("G", "SendG", "g", None),
    ("ClosePrepared", "DiscardClosePrepared", "close_prepared", "since-pg18"),
    ("Endpipeline", "DiscardEndpipeline", "endpipeline", "since-pg18"),
    ("Flushrequest", "DiscardFlushrequest", "flushrequest", "since-pg18"),
    ("Flush", "DiscardFlush", "flush", "since-pg18"),
    ("Getresults", "DiscardGetresults", "getresults", "since-pg18"),
    ("Startpipeline", "DiscardStartpipeline", "startpipeline", "since-pg18"),
    ("Syncpipeline", "DiscardSyncpipeline", "syncpipeline", "since-pg18"),
    ("Reset", "ResetCommandText", "reset", None),
]



def variants():
    """Each variant name and its Rust source, in the order of ROWS."""
    for row in ROWS:
        name, type_name, key, cfg = row[:4]
        priority = row[4] if len(row) > 4 else 2
        yield name, variant(name, type_name, key, cfg, priority=priority)


def main():
    if sys.argv[1:] == ["--check"]:
        ast = (pathlib.Path(__file__).resolve().parent.parent / "src" / "ast.rs").read_text()
        missing = [name for name, source in variants() if source not in ast]
        if missing:
            print("src/ast.rs does not hold the patterns of: " + ", ".join(missing))
            sys.exit(1)
        print("src/ast.rs holds every meta-command pattern")
        return
    for name, source in variants():
        print(f"// --- {name}")
        print(source)


if __name__ == "__main__":
    main()
