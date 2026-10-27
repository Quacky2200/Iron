Platypus
====

Platypus is new game engine built (you guessed it) in rust. We hope you enjoy it.

Our goal is focusing on utilising Open Source toolkits to bring you the latest
game development from all the latest technology. We acknoledge Rust can be a
controversial yet it's also one of the most popular and growing languages today
due to its memory safety and the amount of projects created within it.

What Platypus has to offer:
=======================

One of the problems in the current game engine arena in recent history was
Unity's plan to introduce per-install licensing costs and had upset the
indie game industry, given that Unity is most popular engine, potentially
argued as a near-monopoly. Godot had been no-real comparison. There became
a panic about alternative game engines, especially for 3D with wide support.

Additionally when S&box came out in early testing and paid beta, there were
awful reviews (as seen in [https://sbox.game/news/rethinking](S&Box blogpost
- Rethinking by Garry). We hope that the next improvements will help their
platform tremendously.

In the immediate future this is partly why Platypus exists so that we can make
games immediately without licensing issues, with Open Source software and
with the same greatness as other game engines.

We also wish to provide a game engine that utilises some of the technology
frontiered by the web so that we can provide better experiences just as the
web had done, potentially allowing web designers/developers chances to work
on games for those who have the interest - following similar footsteps to
S&Box who impressed me with their Razor flexbox system.

Licence:
========

Platypus is licensed under the Apache License, Version 2.0 (Apache-2.0). The full
text is in [LICENSE](LICENSE). (It was private until there was a working demo;
there now is one.)

Every source file we write starts with the line
`SPDX-License-Identifier: Apache-2.0` in that file's comment syntax (`//` for
Rust, TypeScript, JavaScript and WGSL, `/* */` for CSS, `#` for Python, shell
and TOML), after a `#!` line if there is one. New files need it too: the
`M9-LIC-01` test (`tools/platypus_build/tests/spdx_headers.rs`) fails for a tracked
source file without it. Generated files, built fixtures and third-party code
are left out; the test's exclusion list says which.

Languages, dates and numbers (Intl):
====================================

Scripts written in TypeScript get the usual JavaScript Intl features natively
(string comparison, number and date formatting, time zones). For this we use
ICU4X, the Unicode Consortium's Rust library, instead of ICU4C, the C++
library the browsers use. The reasons are space and simplicity:

 - ICU4X is plain Rust, so it links into the game executable with the rest of
   Platypus: no C++ toolchain, no separate native library to ship, and the same
   build on Windows, Linux, macOS and (later) consoles and the web.
 - Its data is split by locale and by feature, so a game only carries the
   languages it lists in its package.json (a set of main markets such as US,
   UK and the EU languages is the default). ICU4C ships one big data file
   (tens of megabytes) for everything. Today the build still links ICU4X's
   data for every locale; trimming it to the game's list is planned.

The structural difference: ICU4C finds the best date pattern for any mix of
options ({ weekday: "long", hour: "numeric" } and so on) from a table called
availableFormats. ICU4X has no such table; it formats from a fixed set of
field combinations. The common cases give the same text as the browsers
(they are compared against Bun in the tests), but unusual combinations of date
options can differ or be refused. docs/differences.md lists them.

If a game needs the exact browser behaviour and every locale, an optional
build (the icu-full feature, using ICU4C through rust_icu) is planned. It needs
a native ICU installed at build time and makes the executable larger, so it
is off by default. The plan and its status are in
docs/research/platypus-ts-intl-datetime.md.

