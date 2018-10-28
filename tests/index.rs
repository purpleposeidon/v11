#[macro_use] extern crate v11;
#[macro_use] extern crate v11_macros;

use v11::Universe;

domain! { TEST }

fn make_universe() -> Universe {
    // Prevent lock clobbering breaking tests w/ threading.
    use std::sync::{Once, ONCE_INIT};
    static REGISTER: Once = ONCE_INIT;
    REGISTER.call_once(|| {
        TEST.register();
        orchard::register();
    });
    Universe::new(&[TEST])
}


#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum TreeType {
    Apple, Orange, Melon
}

table! {
    #[kind = "consistent"]
    [TEST/orchard] {
        #[index]
        variety: [TreeType; VecCol<TreeType>],
        branch_count: [usize; VecCol<usize>],
    }
}

#[test]
fn indexing() {
    let universe = &make_universe();
    let mut orchard = orchard::write(universe);
    orchard.push(orchard::Row {
        variety: TreeType::Orange,
        branch_count: 237,
    });
    orchard.push(orchard::Row {
        variety: TreeType::Apple,
        branch_count: 42,
    });
    orchard.push(orchard::Row {
        variety: TreeType::Apple,
        branch_count: 1337,
    });
    orchard.live_flush(universe, ::v11::event::CREATE);
    let orchard = orchard.as_read();
    assert_eq!(
        Some(orchard::FIRST),
        orchard.variety.find(TreeType::Orange).next(),
    );
    let mut index = orchard.variety.find(TreeType::Apple);
    assert_eq!(
        Some(orchard::RowId::new(1)),
        index.next(),
    );
    assert_eq!(
        Some(orchard::RowId::new(2)),
        index.next(),
    );
    assert_eq!(
        None,
        index.next(),
    );
    assert_eq!(
        None,
        orchard.variety.find(TreeType::Melon).next(),
    );
}
