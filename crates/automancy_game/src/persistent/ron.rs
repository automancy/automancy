use ron::{Options, extensions::Extensions};

pub const RON_EXTENSIONS: Extensions = Extensions::UNWRAP_NEWTYPES
    .union(Extensions::IMPLICIT_SOME)
    .union(Extensions::UNWRAP_VARIANT_NEWTYPES);

pub fn ron_options() -> Options {
    Options::default().with_default_extension(RON_EXTENSIONS)
}
