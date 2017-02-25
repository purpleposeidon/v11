#![recursion_limit = "1024"] // quote :D

extern crate syntex;
extern crate syntex_syntax;
#[macro_use]
extern crate quote;
extern crate walkdir;
extern crate rustfmt;

use std::path::Path;
use std::ffi::OsStr;
use std::path::PathBuf;
use walkdir::WalkDir;

mod parse;
mod table;
mod output;
mod expander;

/// Generate the table code for the given crate. The typical build script calls this.
pub fn process_crate(crate_name: &str, run_rust_format: bool) {
    let out_dir = ::std::env::var("OUT_DIR").unwrap();
    let dst = Path::new(&out_dir).join("v11_generated_tables");
    process(crate_name, &Path::new("src/"), &dst, run_rust_format);
}

/// Add the table expander to an existing `syntex::Registry`.
pub fn add_expander<P: AsRef<Path>>(registry: &mut syntex::Registry, output: P, run_rust_format: bool) {
    let expander = expander::TableExpander::new(output, run_rust_format);
    registry.add_macro("table", expander);
}

/// Recursively process each Rust source file in the input directory,
/// writing any table module definitions to the output directory.
pub fn process(crate_name: &str, input: &Path, output: &Path, run_rust_format: bool) {
    // We can't use `cargo:rerun-if-changed` because a new table could
    // be defined anywhere.
    assert!(input.is_dir(), "Input path is not a directory: {:?}", input);
    ::std::fs::create_dir_all(output).expect("Could not create output directory");
    let tmp = TmpFile::new();
    let dot_rs = Some(OsStr::new("rs"));
    // FIXME: temp file, remove? Hmm.
    // Kinda wasteful.
    for entry in WalkDir::new(input).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() { continue; }
        let source = entry.path();
        if source.extension() != dot_rs { continue; }
        let mut registry = syntex::Registry::new();
        add_expander(&mut registry, output, run_rust_format);
        registry.expand(crate_name, source, &tmp.path).ok();
    }
}


struct TmpFile {
    path: PathBuf,
}
impl TmpFile {
    fn new() -> Self {
        let path = ::std::env::temp_dir().join("v11_build_null");
        TmpFile {
            path: path,
        }
    }
}
impl Drop for TmpFile {
    fn drop(&mut self) {
        ::std::fs::remove_file(&self.path).ok();
    }
}



#[allow(dead_code)]
fn warn<S: AsRef<str>>(msg: S) {
    for line in msg.as_ref().split("\n") {
        println!("cargo:warning={}", line);
    }
}

#[allow(dead_code)]
fn error<S: AsRef<str> + Clone>(msg: S) -> ! {
    // How to give error? panic's very unfriendly.
    warn(msg.clone());
    panic!("{}", msg.as_ref())
}
