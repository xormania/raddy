# Hand-rolled binary parser misreads a section

**Class:** a one-off decoder of a binary format is treated as evidence about the file, but the decoder is wrong.

**Where it occurs:** ad-hoc wasm section walkers, ELF dumpers, any "I will just parse the bytes" check used to confirm a build.

**What you see:** `export  kind=13` (empty name) while `llvm-nm` shows `T raddy_execute`. The guest was fine; the parser was not.

**Covered by:** believe the toolchain's own dump (`llvm-nm`) over a script written in the same turn.

**Shape that fits the rest:** do not invent a parser to verify a compiler. Use the compiler's tools.
