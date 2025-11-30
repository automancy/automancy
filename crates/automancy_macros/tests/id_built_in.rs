use automancy_data::id::{Id, IdInterner, InternSymbol};

#[test]
pub fn test_id_built_in() {
    let mut interner = IdInterner::new();

    assert_eq!(Id::none().to_usize(), interner.get_or_intern_static("automancy:none").to_usize());
    assert_eq!(Id::none().to_usize(), interner.get_or_intern_static("core:none").to_usize());
    assert_eq!(Id::any().to_usize(), interner.get_or_intern_static("automancy:any").to_usize());
    assert_eq!(Id::any().to_usize(), interner.get_or_intern_static("core:any").to_usize());

    assert_ne!(
        Id::none().to_usize(),
        interner.get_or_intern_static("automancy:tile/none").to_usize()
    );
    assert_ne!(Id::none().to_usize(), interner.get_or_intern_static("core:tile/none").to_usize());

    assert_ne!(Id::any().to_usize(), interner.get_or_intern_static("babababa:wawawawa").to_usize());
    assert_ne!(Id::any().to_usize(), interner.get_or_intern_static("deez:deez/nuts").to_usize());
}
