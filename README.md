### shared-file: adapters for concurrent File reads.

**This crate is still under development.**

Rust's [`Read`] trait requires exclusive (`&mut`) access, which
can be a pain. This crate implements `SharedFile`, which allows
concurrent readers to coexist.

