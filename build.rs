use lazy_static::lazy_static;
use regex::Regex;
use semver::Version;
use std::env;
use std::ffi::OsStr;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::Command;

#[cfg(all(feature = "llvm-9", feature = "llvm-10"))]
std::compile_error!("llvm-sys-featured: Multiple LLVM versions selected. Please activate only one LLVM version feature.");

// Environment variables that can guide compilation

/// A single path to search for LLVM in (containing bin/llvm-config)
static ENV_LLVM_PREFIX: &str = "LLVM_SYS_FEATURED_PREFIX";

/// If set, enforce precise correspondence between crate and binary versions.
static ENV_STRICT_VERSIONING: &str = "LLVM_SYS_FEATURED_STRICT_VERSIONING";

/// If set, do not attempt to strip irrelevant options for llvm-config --cflags
static ENV_NO_CLEAN_CFLAGS: &str = "LLVM_SYS_FEATURED_NO_CLEAN_CFLAGS";

/// If set and targeting MSVC, force the debug runtime library
static ENV_USE_DEBUG_MSVCRT: &str = "LLVM_SYS_FEATURED_USE_DEBUG_MSVCRT";

/// If set, always link against libffi
static ENV_FORCE_FFI: &str = "LLVM_SYS_FEATURED_FFI_WORKAROUND";

lazy_static! {
    static ref SELECTED_VERSION: Version = {
        if cfg!(feature = "llvm-10") {
            Version::parse("10.0.0").unwrap()
        } else if cfg!(feature = "llvm-9") {
            Version::parse("9.0.0").unwrap()
        } else {
            panic!("llvm-sys-featured: Please select an LLVM version using a Cargo feature.")
        }
    };

    static ref LLVM_CONFIG_BINARY_NAMES: Vec<String> = {
        vec![
            "llvm-config".into(),
            format!("llvm-config-{}", SELECTED_VERSION.major),
            format!("llvm-config-{}.{}", SELECTED_VERSION.major, SELECTED_VERSION.minor),
            format!("llvm-config{}{}", SELECTED_VERSION.major, SELECTED_VERSION.minor),
        ]
    };

    /// Filesystem path to an llvm-config binary for the correct version.
    static ref LLVM_CONFIG_PATH: Option<PathBuf> = {
        // Did the user give us a binary path to use?
        if let Some(path) = env::var_os(ENV_LLVM_PREFIX) {
            // User gave us a path: try to use that, and fail if it doesn't work.
            for binary_name in LLVM_CONFIG_BINARY_NAMES.iter() {
                let mut pb: PathBuf = path.clone().into();
                pb.push("bin");
                pb.push(binary_name);

                let ver = llvm_version(&pb)
                    .unwrap_or_else(|_| panic!("Failed to execute {:?}", &pb));
                if is_compatible_llvm(&ver) {
                    return Some(pb);
                } else {
                    println!("LLVM binaries specified by {} are the wrong version.
                              (Found {}, need {}.)", ENV_LLVM_PREFIX, ver, *SELECTED_VERSION);
                }
            }
            None
        } else {
            // User didn't give us a path: try to find llvm-config via system PATH.
            for binary_name in LLVM_CONFIG_BINARY_NAMES.iter() {
                match llvm_version(binary_name) {
                    Ok(ref version) if is_compatible_llvm(version) => {
                        // Compatible version found. Nice.
                        return Some(binary_name.into());
                    }
                    Ok(version) => {
                        // Version mismatch. Will try further searches, but warn that
                        // we're not using the system one.
                        println!(
                            "Found LLVM version {} on PATH, but need {}.",
                            version, *SELECTED_VERSION
                        );
                    }
                    Err(ref e) if e.kind() == ErrorKind::NotFound => {
                        // Looks like we failed to execute any llvm-config. Keep
                        // searching.
                    }
                    // Some other error, probably a weird failure. Give up.
                    Err(e) => panic!("Failed to search PATH for llvm-config: {}", e),
                }
            }
            println!("Didn't find usable system-wide LLVM.");
            None
        }
    };
}

/// Check whether the given LLVM version is compatible with the one selected via
/// Cargo features.
fn is_compatible_llvm(llvm_version: &Version) -> bool {
    let strict =
        cfg!(feature = "strict-versioning") || env::var_os(ENV_STRICT_VERSIONING).is_some();
    if strict {
        llvm_version == &*SELECTED_VERSION
    } else {
        llvm_version >= &*SELECTED_VERSION
    }
}

/// Get the output from running `llvm-config` with the given argument.
///
/// Lazily searches for or compiles LLVM as configured by the environment
/// variables.
fn llvm_config(arg: &str) -> String {
    llvm_config_ex(&*LLVM_CONFIG_PATH.clone().unwrap(), arg)
        .expect("Surprising failure from llvm-config")
}

/// Invoke the specified binary as llvm-config.
///
/// Explicit version of the `llvm_config` function that bubbles errors
/// up.
fn llvm_config_ex(binary: impl AsRef<OsStr>, arg: &str) -> io::Result<String> {
    Command::new(binary)
        .arg(arg)
        .arg("--link-static") // Don't use dylib for >= 3.9
        .output()
        .map(|output| {
            String::from_utf8(output.stdout).expect("Output from llvm-config was not valid UTF-8")
        })
}

/// Get the LLVM version using llvm-config.
fn llvm_version(binary: impl AsRef<OsStr>) -> io::Result<Version> {
    let version_str = llvm_config_ex(binary.as_ref(), "--version")?;

    // LLVM isn't really semver and uses version suffixes to build
    // version strings like '3.8.0svn', so limit what we try to parse
    // to only the numeric bits.
    let re = Regex::new(r"^(?P<major>\d+)\.(?P<minor>\d+)(?:\.(?P<patch>\d+))??").unwrap();
    let c = re
        .captures(&version_str)
        .expect("Could not determine LLVM version from llvm-config.");

    // some systems don't have a patch number but Version wants it so we just append .0 if it isn't
    // there
    let s = match c.name("patch") {
        None => format!("{}.0", &c[0]),
        Some(_) => c[0].to_string(),
    };
    Ok(Version::parse(&s).unwrap())
}

/// Get the names of the dylibs required by LLVM, including the C++ standard
/// library.
fn get_system_libraries() -> Vec<String> {
    llvm_config("--system-libs")
        .split(&[' ', '\n'] as &[char])
        .filter(|s| !s.is_empty())
        .map(|flag| {
            if cfg!(target_env = "msvc") {
                // Same as --libnames, foo.lib
                assert!(flag.ends_with(".lib"));
                &flag[..flag.len() - 4]
            } else if cfg!(target_os = "macos") {
                // Linker flags style, -lfoo
                assert!(flag.starts_with("-l"));
                if flag.ends_with(".tbd") && flag.starts_with("-llib") {
                    &flag[5..flag.len() - 4]
                } else {
                    &flag[2..]
                }
            } else {
                // Linker flags style, -lfoo
                assert!(flag.starts_with("-l"));
                &flag[2..]
            }
        })
        .chain(get_system_libcpp())
        .map(str::to_owned)
        .collect::<Vec<String>>()
}

/// Get the library that must be linked for C++, if any.
fn get_system_libcpp() -> Option<&'static str> {
    if cfg!(target_env = "msvc") {
        // MSVC doesn't need an explicit one.
        None
    } else if cfg!(target_os = "macos") {
        // On OS X 10.9 and later, LLVM's libc++ is the default. On earlier
        // releases GCC's libstdc++ is default. Unfortunately we can't
        // reasonably detect which one we need (on older ones libc++ is
        // available and can be selected with -stdlib=lib++), so assume the
        // latest, at the cost of breaking the build on older OS releases
        // when LLVM was built against libstdc++.
        Some("c++")
    } else if cfg!(target_os = "freebsd") {
        Some("c++")
    } else {
        // Otherwise assume GCC's libstdc++.
        // This assumption is probably wrong on some platforms, but would need
        // testing on them.
        Some("stdc++")
    }
}

/// Get the names of libraries to link against.
fn get_link_libraries() -> Vec<String> {
    // Using --libnames in conjunction with --libdir is particularly important
    // for MSVC when LLVM is in a path with spaces, but it is generally less of
    // a hack than parsing linker flags output from --libs and --ldflags.
    llvm_config("--libnames")
        .split(&[' ', '\n'] as &[char])
        .filter(|s| !s.is_empty())
        .map(|name| {
            // --libnames gives library filenames. Extract only the name that
            // we need to pass to the linker.
            if cfg!(target_env = "msvc") {
                // LLVMfoo.lib
                assert!(name.ends_with(".lib"));
                &name[..name.len() - 4]
            } else {
                // libLLVMfoo.a
                assert!(name.starts_with("lib") && name.ends_with(".a"));
                &name[3..name.len() - 2]
            }
        })
        .map(str::to_owned)
        .collect::<Vec<String>>()
}

fn get_llvm_cflags() -> String {
    let output = llvm_config("--cflags");

    // llvm-config includes cflags from its own compilation with --cflags that
    // may not be relevant to us. In particularly annoying cases, these might
    // include flags that aren't understood by the default compiler we're
    // using. Unless requested otherwise, clean CFLAGS of options that are
    // known to be possibly-harmful.
    let no_clean = env::var_os(&*ENV_NO_CLEAN_CFLAGS).is_some();
    if no_clean || cfg!(target_env = "msvc") {
        // MSVC doesn't accept -W... options, so don't try to strip them and
        // possibly strip something that should be retained. Also do nothing if
        // the user requests it.
        return output;
    }

    llvm_config("--cflags")
        .split(&[' ', '\n'][..])
        .filter(|word| !word.starts_with("-W"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_llvm_debug() -> bool {
    // Has to be either Debug or Release
    llvm_config("--build-mode").contains("Debug")
}

fn main() {
    // Behavior can be significantly affected by these vars.
    println!("cargo:rerun-if-env-changed={}", &*ENV_LLVM_PREFIX);
    println!("cargo:rerun-if-env-changed={}", &*ENV_STRICT_VERSIONING);
    println!("cargo:rerun-if-env-changed={}", &*ENV_NO_CLEAN_CFLAGS);
    println!("cargo:rerun-if-env-changed={}", &*ENV_USE_DEBUG_MSVCRT);
    println!("cargo:rerun-if-env-changed={}", &*ENV_FORCE_FFI);

    if LLVM_CONFIG_PATH.is_none() {
        println!("cargo:rustc-cfg=LLVM_SYS_NOT_FOUND");
        return;
    }

    // Build the extra wrapper functions.
    if !cfg!(feature = "disable-alltargets-init") {
        std::env::set_var("CFLAGS", get_llvm_cflags());
        cc::Build::new()
            .file("wrappers/target.c")
            .compile("targetwrappers");
    }

    if cfg!(feature = "no-llvm-linking") {
        return;
    }

    let libdir = llvm_config("--libdir");

    // Export information to other crates
    println!(
        "cargo:config_path={}",
        LLVM_CONFIG_PATH.clone().unwrap().display()
    ); // will be DEP_LLVM_CONFIG_PATH
    println!("cargo:libdir={}", libdir); // DEP_LLVM_LIBDIR

    // Link LLVM libraries
    println!("cargo:rustc-link-search=native={}", libdir);
    for name in get_link_libraries() {
        println!("cargo:rustc-link-lib=static={}", name);
    }

    // Link system libraries
    for name in get_system_libraries() {
        println!("cargo:rustc-link-lib=dylib={}", name);
    }

    let use_debug_msvcrt = env::var_os(&*ENV_USE_DEBUG_MSVCRT).is_some();
    if cfg!(target_env = "msvc") && (use_debug_msvcrt || is_llvm_debug()) {
        println!("cargo:rustc-link-lib={}", "msvcrtd");
    }

    // Link libffi if the user requested this workaround.
    // See https://bitbucket.org/tari/llvm-sys.rs/issues/12/
    let force_ffi = env::var_os(&*ENV_FORCE_FFI).is_some();
    if force_ffi {
        println!("cargo:rustc-link-lib=dylib={}", "ffi");
    }
}
