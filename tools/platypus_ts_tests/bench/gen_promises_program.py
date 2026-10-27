# SPDX-License-Identifier: Apache-2.0

# Builds bench/promises.ts from bench/promises_src.ts: every `async function t_NAME(): Promise<string>`
# gets an exported `NAME()` that starts it and stores the result, and `NAME_result()` that returns it.
#   python bench/gen_promises_program.py   (then add_module.py promises pr 900 <id>, see the notes)
import os
import re

here = os.path.dirname(os.path.abspath(__file__))
src = open(os.path.join(here, "promises_src.ts"), encoding="utf8").read()
names = re.findall(r"^async function t_(\w+)\(\): Promise<string>", src, re.M)
out = src + "\n// ---- glue: start each test and keep its result ----\n"
for n in names:
    out += f"""
let r_{n} = "";
export function {n}(): string {{
  r_{n} = "";
  t_{n}()
    .then((v) => {{
      r_{n} = v;
    }})
    .catch((e) => {{
      r_{n} = "rejected:" + e.message;
    }});
  return "";
}}
export function {n}_result(): string {{
  return r_{n};
}}
"""
open(os.path.join(here, "promises.ts"), "w", encoding="utf8").write(out)
print(len(names), "tests:", ", ".join(names))
