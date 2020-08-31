# This crate is now deprecated

Originally, this crate was created as one solution to the problem of
using Cargo features to select an LLVM version for use with `llvm-sys`,
which wasn't possible at the time.

Since then, thanks to changes in `llvm-sys` (see
[this `llvm-sys` GitLab issue](https://gitlab.com/taricorp/llvm-sys.rs/-/issues/8)),
it is now possible to achieve the same goal much more simply, using
Cargo's [dependency renaming] feature.
For instance, as of this writing, [`llvm-ir`] uses the following in its
Cargo.toml:
```toml
[dependencies]
llvm-sys-80 = { package = "llvm-sys", version = "80.3.0", optional = true }
llvm-sys-90 = { package = "llvm-sys", version = "90.2.0", optional = true }
llvm-sys-100 = { package = "llvm-sys", version = "100.2.0", optional = true }

[features]
# Select the LLVM version to be compatible with.
# You _must_ enable exactly one of the following features.
llvm-8 = ["llvm-sys-80"]
llvm-9 = ["llvm-sys-90"]
llvm-10 = ["llvm-sys-100"]
```
and checks in its `build.rs` script that exactly one of those features was
enabled.

Then, in its actual library code, `llvm-ir` uses the following:
```rust
#[cfg(feature = "llvm-8")]
pub use llvm_sys_80 as llvm_sys;
#[cfg(feature = "llvm-9")]
pub use llvm_sys_90 as llvm_sys;
#[cfg(feature = "llvm-10")]
pub use llvm_sys_100 as llvm_sys;
```
which declares `llvm-sys` to represent the appropriate version of the
dependency.

Since this is now possible with `llvm-sys`, there is no more need for the
alternative solution represented by `llvm-sys-featured`.
This crate is now deprecated in favor of using `llvm-sys` directly, and will
not be maintained or updated for future LLVM releases.

[dependency renaming]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml
[`llvm-ir`]: https://crates.io/crates/llvm-ir

The original README for `llvm-sys-featured` follows.

-----

# llvm-sys-featured

Rust bindings to LLVM's C API.

This is essentially a mirror of [llvm-sys], except that instead of using
separate crates.io releases for each LLVM version, this crate uses Cargo
features. Currently, the goal is that with the `llvm-8` feature you get
exactly `llvm-sys` version 80.2.0, with the `llvm-9` feature you get exactly
`llvm-sys` version 90.1.0, and with the `llvm-10` feature you get exactly
`llvm-sys` version 100.1.1.

## Usage

Add this crate as a dependency in your `Cargo.toml`, selecting the feature
corresponding to the LLVM version you want:

```toml
[dependencies]
llvm-sys-featured = { version = "0.1.1", features = ["llvm-10"] }
```

Currently, the supported LLVM versions are `llvm-8`, `llvm-9`, and `llvm-10`.

There must be the corresponding LLVM version available on your system.
By default, `llvm-sys-featured` will look for `llvm-config` on `PATH` to find
a system-wide copy of LLVM and use that if it is compatible.
Alternately, you can set the environment variable `LLVM_SYS_FEATURED_PREFIX`
with the path (install prefix) to a compiled and installed copy of the
libraries, which will be used instead.

If you want to use `llvm-sys-featured` as a drop-in replacement for
`llvm-sys` (keeping the `llvm-sys` name in your code), you can use Cargo's
[dependency renaming] feature:

```toml
[dependencies]
llvm-sys = { package = "llvm-sys-featured", version = "0.1.1", features = ["llvm-10"] }
```

[dependency renaming]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml

## Documentation

Documentation for this crate's API can be found at
[docs.rs](https://docs.rs/llvm-sys-featured).
However, most of the interfaces do not have detailed descriptions there, as
they are simply bindings to the corresponding LLVM C API interfaces; refer to
the [LLVM documentation](https://llvm.org/docs/) for more information,
particularly the [generated API documentation](https://llvm.org/doxygen).

See the `examples` directory in this repository for API examples; these
are exactly the same as the examples provided by [llvm-sys].

## Safe bindings

We recommend that most users not use this crate directly, but instead use one
of the following safe, "Rusty" APIs for LLVM:
  * [llvm-ir](https://crates.io/crates/llvm-ir)
  * [Inkwell](https://github.com/TheDan64/inkwell)

## LLVM compatibility

Currently, this crate supports LLVM 8, LLVM 9, and LLVM 10. (See
[Usage](#usage).)

Like [llvm-sys], this crate checks that the LLVM version being used matches the
one selected via Cargo features (or crate version in `llvm-sys`'s case).
This is because the LLVM [C API stability guarantees][c-api-stability] are
relatively weak.

[c-api-stability]: http://llvm.org/docs/DeveloperPolicy.html#c-api-changes

As an exception, like [llvm-sys], this crate allows to use a newer LLVM
version with older bindings, which should be safe in almost all cases; for
more information, see comments in the [llvm-sys] README.
This behavior can be disabled by either selecting the `strict-versioning`
feature on this crate, or by setting the environment variable
`LLVM_SYS_FEATURED_STRICT_VERSIONING`; in either case, this crate will then
enforce that the LLVM version being used exactly matches the one selected via
Cargo features.

## Downloading LLVM

LLVM can be acquired from your system package manager on most systems; but if
you want a newer version of LLVM, you can download it from the LLVM [Download
page](https://releases.llvm.org/download.html). On Debian and Ubuntu systems,
you can also get the latest versions from `apt` by following [these
instructions](https://apt.llvm.org/).

## Building LLVM

For more information on building LLVM from source, see the LLVM docs or the
[llvm-sys] README.

## Credits

At least 99% of the code in this crate is taken directly from [llvm-sys],
so a big thanks to its author Peter Marheine, who deserves all the credit for
the excellent LLVM bindings.

This crate, like [llvm-sys], is licensed under the MIT License. That means
you're free to adapt and redistribute this crate with very few restrictions,
just like how this crate adapts and redistributes [llvm-sys].

[llvm-sys]: https://gitlab.com/taricorp/llvm-sys.rs
