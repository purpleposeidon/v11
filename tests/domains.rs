#![allow(dead_code)]

#[macro_use]
extern crate v11;
//extern crate v11_macros;


domain! { TEST }

domain! { COMPILE_TRUE_NAME ("FOO") }

mod add_domain {
    use v11::*;
    domain! { ADD_OLD }
    domain! { ADD_NEW }

    property! { static ADD_OLD/ADD_OLD_VAL: usize = 9; }
    property! { static ADD_NEW/ADD_NEW_VAL: usize = 10; }

    #[test]
    fn add_domain() {
        ADD_OLD.register();
        ADD_OLD_VAL.register();
        let mut universe = Universe::new(&[ADD_OLD]);
        assert_eq!(universe.get(ADD_OLD_VAL), 9);
        ADD_NEW.register();
        ADD_NEW_VAL.register();
        universe.add_domain(ADD_NEW);
        assert_eq!(universe.get(ADD_NEW_VAL), 10);
    }
}

mod missing_domain {
    use v11::*;
    domain! { MISSING_OLD }
    domain! { MISSING_NEW }

    property! { static MISSING_OLD/MISSING_OLD_VAL: usize = 9; }
    property! { static MISSING_NEW/MISSING_NEW_VAL: usize = 10; }

    #[test]
    #[should_panic]
    fn missing_domain() {
        MISSING_OLD.register();
        MISSING_OLD_VAL.register();
        MISSING_NEW.register();
        MISSING_NEW_VAL.register();
        let universe = Universe::new(&[MISSING_OLD]);
        universe.get(MISSING_NEW_VAL);
    }
}
