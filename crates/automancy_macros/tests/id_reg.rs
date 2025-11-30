use automancy_data::id::{Id, IdInterner};
use automancy_macros::IdReg;

#[derive(IdReg)]
pub struct Foo {
    pub a: Id,
    #[namespace("core")]
    pub b: Id,
    #[name("ccccccccccc")]
    pub c: Id,
    #[namespace("deez")]
    #[name("deez/nuts")]
    pub d: Id,
}

#[test]
pub fn test_id_reg() {
    let mut interner = IdInterner::new();

    let bar = Foo::new(&mut interner);

    assert_eq!(bar.a, interner.get_or_intern_static("automancy:a"));
    assert_eq!(bar.b, interner.get_or_intern_static("core:b"));
    assert_eq!(bar.c, interner.get_or_intern_static("automancy:ccccccccccc"));
    assert_eq!(bar.d, interner.get_or_intern_static("deez:deez/nuts"));
}
