use v11::*;

domain! { COMPILE_TRUE_NAME ("FOO") }

domain! { OLD }
domain! { NEW }

property! { static OLD/OLD_VAL: usize = 9; }
property! { static NEW/NEW_VAL: usize = 10; }

#[test]
fn add_domain() {
    OLD.register();
    OLD_VAL.register();
    let mut universe = Universe::new(&[OLD]);
    assert_eq!(universe.get(OLD_VAL), 9);
    NEW.register();
    NEW_VAL.register();
    universe.add_domain(NEW);
    assert_eq!(universe.get(NEW_VAL), 10);
}

#[test]
#[should_panic]
fn missing_domain() {
    OLD.register();
    OLD_VAL.register();
    NEW.register();
    NEW_VAL.register();
    let universe = Universe::new(&[OLD]);
    universe.get(NEW_VAL);
}
