#![macro_use]

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
 * constructor! { init }
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
 * If compiling with `--release`, the mechanism for invoking the function will be stripped out
 * unless the function is externally visible. (Eg, all crates up to the root must be `pub`.)
 *
 *
 *
 * Beware, for some say that these techniques can unleash a terrible evil.
 * [lazy_static](https://crates.io/crates/lazy_static) may be a more appropriate tool.
 *
 * */
#[macro_export]
macro_rules! constructor {
    ($($NAME:ident)*) => {
        #[allow(dead_code)]
        $(pub mod $NAME {
            // http://stackoverflow.com/questions/35428834/how-to-specify-which-elf-section-to-use-in-a-rust-dylib
            // https://msdn.microsoft.com/en-us/library/bb918180.aspx
            // Help given by WindowsBunny!

            #[cfg(target_os = "linux")]
            #[link_section = ".ctors"]
            static CONSTRUCTOR: extern fn() = super::$NAME;

            // TODO: macos untested
            #[cfg(target_os = "macos")]
            #[link_section = "__DATA,__mod_init_func"]
            static CONSTRUCTOR: extern fn() = super::$NAME;

            // TODO: windows untested; may require something more complicated for certain target
            // triples?
            #[cfg(target_os = "windows")]
            #[link_section = ".CRT$XCU"]
            static CONSTRUCTOR: extern fn() = super::$NAME;

            // And we'll have a compilation error if we don't know `target_os`.

            pub extern fn dont_strip() -> &'static extern "C" fn() {
                // This function seems to be the minimal function that keeps 'CONSTRUCTOR' from
                // being stripped. However this fn must be externally accessible! So it & all
                // parent modules must be public.
                // Making `super::$NAME` refer to CONSTRUCTOR does not seem to be sufficient to
                // keep the symbol around.
                &CONSTRUCTOR
            }
        })*
    };
}
