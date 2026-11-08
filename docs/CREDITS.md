# Credits

CalcDex is written in **Rust** and stands on a deliberately
small dependency set:

- **[egui / eframe](https://github.com/emilk/egui)** — the
  immediate-mode GUI and its application framework (glow
  renderer), including the bundled default fonts. MIT /
  Apache-2.0 dual license.
- The **Rust standard library** and toolchain (MIT /
  Apache-2.0).

Everything else — the calculator registry, the log, the
problem-set tooling, the installers and these documents —
is part of this repository, MIT-licensed (see
[../LICENSE](../LICENSE)). Reference values in the
constants table are the CODATA 2018 recommended values.

## RPN mode

The RPN mode follows the behaviour of the HP-42S calculator as
described in HP's published owner's manual and in the public
documentation of Free42 / Plus42 by Thomas Okken (GPLv2). No code
from those projects is used (clean-room); the names and
conventions are theirs and HP's.
