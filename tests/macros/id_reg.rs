use automancy_defs::id::{Id, Interner};
use automancy_macros::IdReg;

#[derive(IdReg)]
pub struct Foo {
    pub a: Id,
    #[namespace(core)]
    pub b: Id,
    #[name(ccccccccccc)]
    pub c: Id,
    #[namespace(deez)]
    #[name("deez/nuts")]
    pub d: Id,
}

#[test]
pub fn test_id_reg() {
    println!("Testing IdReg macro...");

    let mut interner = Interner::new();

    let bar = Foo::new(&mut interner);

    assert_eq!(bar.a, interner.get_or_intern("automancy:a"));
    assert_eq!(bar.b, interner.get_or_intern("core:b"));
    assert_eq!(bar.c, interner.get_or_intern("automancy:ccccccccccc"));
    assert_eq!(bar.d, interner.get_or_intern("deez:deez/nuts"));

    println!("Success!");
}
