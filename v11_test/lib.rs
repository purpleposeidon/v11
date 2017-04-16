#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

extern crate rustc_serialize;


#[cfg(test)]
pub mod addons;

#[cfg(test)]
pub mod tables;

// FIXME: Can this go into "tests/" as is standard? Previous usage of build.rs probably made that
// difficult.
