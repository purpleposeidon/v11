#![macro_use]
// FIXME: Separate crate. (Also we no longer want property! to use this.)

/**
 * Registers a function to be called before main (if an executable) or when loaded (if a dynamic
 * library).
 *
 * Example
 * =======
 *
 * ```
 * # #[macro_use] extern crate v11;
 * pub static mut x: usize = 0;
 * 
 * extern fn init() {
 *     unsafe { x = 5; }
 * }
 * constructor! { INIT_CONSTRUCTOR: init }
 * 
 * 
 * fn main() {
 *    assert_eq!(unsafe { x }, 5);
 * }
 * ```
 *
 * Caveats
 * =======
 * This isn't exactly portable, though the implementation is quite simple.
 *
 * Doing anything particularly complicated, such IO or loading libraries, may cause problems
 * on Windows. (?)
 *
 * Every parent module must be `pub`lic. If it is not, then it will be
 * stripped out by `--release`. At least the compiler gives a helpful warning.
 *
 *
 *
 * Beware, for some say that these techniques can unleash a terrible evil.
 * [lazy_static](https://crates.io/crates/lazy_static) may be a more appropriate tool.
 *
 * */
#[macro_export]
macro_rules! constructor {
    ($($NAME:ident: $FN:ident),*) => {
        $(pub mod $NAME {
            #![allow(non_snake_case)]
            #![allow(dead_code)]
            #![deny(private_no_mangle_statics /* Constructor won't run in release mode! */)]

            // http://stackoverflow.com/questions/35428834/how-to-specify-which-elf-section-to-use-in-a-rust-dylib
            // https://msdn.microsoft.com/en-us/library/bb918180.aspx
            // Help given by WindowsBunny!

            #[cfg(target_os = "linux")]
            #[link_section = ".ctors"]
            #[no_mangle]
            pub static $NAME: extern fn() = super::$FN;

            // FIXME: macos untested
            #[cfg(target_os = "macos")]
            #[link_section = "__DATA,__mod_init_func"]
            #[no_mangle]
            pub static $NAME: extern fn() = super::$FN;

            // FIXME: windows untested; may require something more complicated for certain target
            // triples?
            #[cfg(target_os = "windows")]
            #[link_section = ".CRT$XCU"]
            #[no_mangle]
            pub static $NAME: extern fn() = super::$FN;

            // We could also just ignore cfg(target_os) & have 1 item, but this way we'll have a compilation error if we don't know `target_os`.
        })*
    };
}

#[cfg(test)]
pub mod test {
    pub static mut RAN: bool = false;
    extern "C" fn set_ran() {
        unsafe { RAN = true }
    }
    constructor! { SET_RAN_CONSTRUCTOR: set_ran }

    #[test]
    fn works() {
        assert!(unsafe { RAN });
    }
}
