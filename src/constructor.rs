/**
 * Registers a function to be called before main (if an executable) or when loaded (if a dynamic
 * library).
 *
 * This isn't exactly portable, though the implementation is quite simple.
 * Doing anything particularly complicated, such IO or loading libraries, may cause problems
 * on certain operating systems.
 *
 * Beware, for some say that these techniques can unleash a terrible evil.
 * [lazy_static](https://crates.io/crates/lazy_static) may be a more appropriate tool.
 *
 *
 * Example
 * =======
 *
 * ```
 * # #[macro_use] extern crate v11;
 * pub static mut x: usize = 0;
 * 
 * pub extern fn init() {
 *     unsafe { x = 5; }
 * }
 * constructor! { init }
 * 
 * 
 * fn main() {
 *    assert_eq!(unsafe { x }, 5);
 *    println!("x = {}", unsafe { x });
 * }
 * ```
 * */
#[macro_export]
macro_rules! constructor {
    ($($NAME:ident)*) => {
        #[allow(dead_code)]
        $(mod $NAME {
            // http://stackoverflow.com/questions/35428834/how-to-specify-which-elf-section-to-use-in-a-rust-dylib
            // https://msdn.microsoft.com/en-us/library/bb918180.aspx
            // Help given by WindowsBunny!

            #[cfg(target_os = "linux")]
            #[link_section = ".ctors"]
            pub static CONSTRUCTOR_LINUX: extern fn() = super::$NAME;

            // TODO: macos untested
            #[cfg(target_os = "macos")]
            #[link_section = "__DATA,__mod_init_func"]
            pub static CONSTRUCTOR_MACOS: extern fn() = super::$NAME;
            // 

            // TODO: windows untested; may require something more complicated for certain target
            // triples?
            #[cfg(target_os = "windows")]
            #[link_section = ".CRT$XCU"]
            pub static CONSTRUCTOR_WINDOWS: extern fn() = super::$NAME;
        })*
    };
}

