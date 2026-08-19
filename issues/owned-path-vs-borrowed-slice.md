# Owned path vs borrowed slice

**Class:** a helper is typed for `&[&str]` while the only dynamic producer has `Vec<String>`.

**Where it occurs:** any "path through a nested map" function (`insert_path`, URL builders, figment Dict walks) that is first written against static CLI keys and later fed parsed env keys.

**What you see:** `expected `&[&str]`, found `&Vec<String>``. Compiles until the second caller exists.

**Covered by:** the env overlay now copies to `Vec<&str>` at the call site. The deeper fix, if a third caller appears, is to take `impl IntoIterator<Item = impl AsRef<str>>`.

**Shape that fits the rest:** type the helper for the dynamic case. Static slices coerce; owned strings do not.
