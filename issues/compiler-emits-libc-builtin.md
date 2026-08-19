# Compiler emits libc builtins as wasm imports

**Class:** a freestanding wasm compile still grows `env::strlen` (or `memcpy`)
imports because the optimizer rewrites loops into builtins.

**Where it occurs:** any `-nostdlib` guest built at `-O2` without
`-fno-builtin`. Here, `guest/apps/hello-symfony/guest.c`.

**What you see:** instantiate fails with `unknown import: env::strlen`.

**Covered by:** `-fno-builtin` on that clang line (build.rs and
`just artifact`).

**Shape that fits the rest:** guest C that must not import libc keeps
`-fno-builtin` next to `-nostdlib`.
