use ron::extensions::Extensions;

pub const RON_EXTENSIONS: Extensions = Extensions::UNWRAP_NEWTYPES
    .union(Extensions::IMPLICIT_SOME)
    .union(Extensions::UNWRAP_VARIANT_NEWTYPES);

pub fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(RON_EXTENSIONS)
}

pub type PrettyConfig = ron::ser::PrettyConfig;

pub fn to_string_pretty<T>(value: &T) -> ron::Result<String>
where
    T: ?Sized + serde::Serialize,
{
    ron_options().to_string_pretty(value, PrettyConfig::new())
}
