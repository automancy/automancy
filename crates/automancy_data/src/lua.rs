use core::marker::PhantomData;

use bytemuck::TransparentWrapper;

use crate::lua::doc::LuaTypeName;

#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    derive_more::PartialEq,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
    derive_more::AsMut,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct TypedTable<K, V>(
    mlua::Table,
    #[deref(ignore)]
    #[deref_mut(ignore)]
    PhantomData<(K, V)>,
);

impl<K, V> From<mlua::Table> for TypedTable<K, V> {
    fn from(value: mlua::Table) -> Self {
        Self::new(value)
    }
}

impl<K, V> From<TypedTable<K, V>> for mlua::Table {
    fn from(val: TypedTable<K, V>) -> Self {
        val.0
    }
}

impl<K, V> TypedTable<K, V> {
    pub fn new(table: mlua::Table) -> Self {
        Self(table, PhantomData)
    }

    pub fn into_inner(self) -> mlua::Table {
        self.0
    }
}

impl<K, V> PartialEq<mlua::Table> for TypedTable<K, V> {
    fn eq(&self, other: &mlua::Table) -> bool {
        &self.0 == other
    }
}

impl<K, V> PartialEq<TypedTable<K, V>> for mlua::Table {
    fn eq(&self, other: &TypedTable<K, V>) -> bool {
        self == &other.0
    }
}

unsafe impl<K, V> TransparentWrapper<mlua::Table> for TypedTable<K, V> {}

impl<K: mlua::FromLua + mlua::IntoLua, V: mlua::FromLua + mlua::IntoLua> TypedTable<K, V> {
    pub fn get(&self, key: K) -> mlua::Result<V> {
        self.0.get(key)
    }

    pub fn raw_get(&self, key: K) -> mlua::Result<V> {
        self.0.raw_get(key)
    }

    pub fn set(&self, key: K, value: V) -> mlua::Result<()> {
        self.0.set(key, value)
    }

    pub fn raw_set(&self, key: K, value: V) -> mlua::Result<()> {
        self.0.raw_set(key, value)
    }

    pub fn remove(&self, key: K) -> mlua::Result<()> {
        self.0.remove(key)
    }

    pub fn raw_remove(&self, key: K) -> mlua::Result<()> {
        self.0.raw_remove(key)
    }

    pub fn pop(&self) -> mlua::Result<V> {
        self.0.pop()
    }

    pub fn raw_pop(&self) -> mlua::Result<V> {
        self.0.raw_pop()
    }

    pub fn push(&self, value: V) -> mlua::Result<()> {
        self.0.push(value)
    }

    pub fn raw_push(&self, value: V) -> mlua::Result<()> {
        self.0.raw_push(value)
    }

    pub fn metatable(&self) -> Option<mlua::Table> {
        self.0.metatable()
    }

    pub fn set_metatable(&self, metatable: Option<mlua::Table>) -> mlua::Result<()> {
        self.0.set_metatable(metatable)
    }

    pub fn sequence_values(&self) -> mlua::table::TableSequence<'_, V> {
        self.0.sequence_values()
    }

    pub fn pairs(&self) -> mlua::table::TablePairs<'_, K, V> {
        self.0.pairs()
    }

    pub fn equals(&self, other: &mlua::Table) -> mlua::Result<bool> {
        self.0.equals(other)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> mlua::Result<mlua::Integer> {
        self.0.len()
    }

    pub fn raw_len(&self) -> usize {
        self.0.raw_len()
    }

    pub fn clear(&self) -> mlua::Result<()> {
        self.0.clear()
    }
}

impl<K, V> mlua::IntoLua for TypedTable<K, V> {
    fn into_lua(self, _lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Table(self.0))
    }
}

impl<K: LuaTypeName, V: LuaTypeName> mlua::FromLua for TypedTable<K, V> {
    fn from_lua(value: mlua::Value, _lua: &mlua::prelude::Lua) -> mlua::Result<Self> {
        let Some(v) = value.as_table() else {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: Self::lua_name().to_string(),
                message: None,
            });
        };

        Ok(Self::new(v.clone()))
    }
}

pub mod doc {
    use core::any::TypeId;
    use std::sync::{Arc, LazyLock, RwLock};

    use hashbrown::HashMap;
    use mlua::MetaMethod;

    use crate::{
        game::coord::TileCoord,
        id::IdLike,
        id_map::{IdCoordMap, IdMap, IdSet, ImmutableIdMap, ImmutableIdSet},
        lua::TypedTable,
    };

    /// const trait impls don't allow generics or associated types to be called in const context :(
    ///
    /// kill this thing with fire whenever const trait is stable and good
    #[macro_export]
    macro_rules! fake_static {
        ($e:expr) => {{
            use core::cell::Cell;

            thread_local! {
                static NAME: Cell<Option<&'static str>> = const { Cell::new(None) };
            }

            if NAME.get().is_none() {
                let s: String = $e;
                NAME.set(Some(s.leak()));
            }

            NAME.get().unwrap()
        }};
    }

    static STATIC_NAMES: LazyLock<Arc<RwLock<HashMap<TypeId, &'static str>>>> = LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

    pub fn generic_fake_static<T: 'static>(name: impl FnOnce() -> String) -> &'static str {
        if let Some(name) = STATIC_NAMES.read().unwrap().get(&TypeId::of::<T>()) {
            name
        } else {
            let new_name = name().leak();

            STATIC_NAMES.write().unwrap().insert(TypeId::of::<T>(), new_name);

            new_name
        }
    }

    #[macro_export]
    macro_rules! gen_alias_doc {
        ($ty:ty) => {
            $crate::fake_static!(format!("---@alias {} {}", Self::lua_name(), <$ty>::lua_name()))
        };
    }

    #[macro_export]
    macro_rules! gen_field_doc {
        () => {
            $crate::fake_static!(
                Self::fields()
                    .map(|v| format!("---@field {}{} {}", v.name, if v.optional { "?" } else { "" }, v.ty))
                    .join("\n")
            )
        };
    }

    #[macro_export]
    macro_rules! gen_method_doc {
        () => {
            $crate::fake_static!(
                Self::methods()
                    .map(|v| {
                        format!(
                            "---@field {} fun({}){}",
                            v.name,
                            v.args
                                .into_iter()
                                .filter(|arg| !arg.name.is_empty() && !arg.ty.is_empty())
                                .map(|arg| format!("{}: {}", arg.name, arg.ty))
                                .collect::<Vec<_>>()
                                .join(", "),
                            if v.return_ty.is_empty() {
                                String::new()
                            } else {
                                format!(": {}", v.return_ty)
                            }
                        )
                    })
                    .join("\n")
            )
        };
    }

    #[macro_export]
    macro_rules! gen_operator_doc {
        () => {
            $crate::fake_static!(
                Self::operators()
                    .map(|v| {
                        format!(
                            "---@operator {}({}): {}",
                            v.name,
                            v.args
                                .into_iter()
                                .filter(|arg| !arg.is_empty())
                                .collect::<Vec<_>>()
                                .join(", "),
                            v.return_ty
                        )
                    })
                    .join("\n")
            )
        };
    }

    #[macro_export]
    macro_rules! gen_object_doc {
        () => {
            $crate::fake_static!(format!("---@class (exact) {}", Self::lua_name(),))
        };
        (fields) => {
            $crate::fake_static!(format!("---@class (exact) {}\n{}", Self::lua_name(), Self::lua_field_doc(),))
        };
        (methods) => {
            $crate::fake_static!(format!("---@class (exact) {}\n{}", Self::lua_name(), Self::lua_method_doc(),))
        };
        (operators) => {
            $crate::fake_static!(format!(
                "---@class (exact) {}\n{}",
                Self::lua_name(),
                Self::lua_operator_doc(),
            ))
        };
        (fields, methods) => {
            $crate::fake_static!(format!(
                "---@class (exact) {}\n{}\n{}",
                Self::lua_name(),
                Self::lua_field_doc(),
                Self::lua_method_doc()
            ))
        };
        (methods, operators) => {
            $crate::fake_static!(format!(
                "---@class (exact) {}\n{}\n{}",
                Self::lua_name(),
                Self::lua_method_doc(),
                Self::lua_operator_doc()
            ))
        };
        (fields, operators) => {
            $crate::fake_static!(format!(
                "---@class (exact) {}\n{}\n{}",
                Self::lua_name(),
                Self::lua_field_doc(),
                Self::lua_operator_doc()
            ))
        };
        (fields, methods, operators) => {
            $crate::fake_static!(format!(
                "---@class (exact) {}\n{}\n{}\n{}",
                Self::lua_name(),
                Self::lua_field_doc(),
                Self::lua_method_doc(),
                Self::lua_operator_doc()
            ))
        };
    }

    pub struct LuaField {
        pub name: &'static str,
        pub ty: &'static str,
        pub optional: bool,
    }

    impl LuaField {
        pub const fn empty() -> Self {
            Self {
                name: "",
                ty: "",
                optional: false,
            }
        }
    }

    pub struct LuaArg {
        pub name: &'static str,
        pub ty: &'static str,
    }

    impl LuaArg {
        pub const fn empty() -> Self {
            Self {
                name: "",
                ty: "",
            }
        }
    }

    pub struct LuaMethod<const ARGS_SIZE: usize = 0> {
        pub name: &'static str,
        pub args: [LuaArg; ARGS_SIZE],
        pub return_ty: &'static str,
    }

    impl<const ARGS_SIZE: usize> LuaMethod<ARGS_SIZE> {
        pub const fn empty() -> Self {
            Self {
                name: "",
                args: [const { LuaArg::empty() }; ARGS_SIZE],
                return_ty: "",
            }
        }
    }

    pub struct LuaOperator<const ARGS_SIZE: usize = 0> {
        pub name: &'static str,
        pub args: [&'static str; ARGS_SIZE],
        pub return_ty: &'static str,
    }

    impl<const ARGS_SIZE: usize> LuaOperator<ARGS_SIZE> {
        pub const fn empty() -> Self {
            Self {
                name: "",
                args: [const { "" }; ARGS_SIZE],
                return_ty: "",
            }
        }
    }

    pub const trait LuaTypeName: 'static {
        fn lua_name() -> &'static str;
    }

    pub trait LuaAliasDoc: LuaTypeName {
        fn lua_alias_doc() -> &'static str;
    }

    pub trait LuaFieldDoc<const SIZE: usize>: LuaTypeName {
        fn fields() -> [LuaField; SIZE];

        fn lua_field_doc() -> &'static str;
    }

    pub trait LuaMethodDoc<const ARG_SIZE: usize, const SIZE: usize>: LuaTypeName {
        fn methods() -> [LuaMethod<ARG_SIZE>; SIZE];

        fn lua_method_doc() -> &'static str;
    }

    pub trait LuaOperatorDoc<const ARG_SIZE: usize, const SIZE: usize> {
        fn operators() -> [LuaOperator<ARG_SIZE>; SIZE];

        fn lua_operator_doc() -> &'static str;
    }

    pub trait LuaObjectDoc: LuaTypeName {
        fn lua_object_doc() -> &'static str;
    }

    const impl LuaTypeName for mlua::Integer {
        fn lua_name() -> &'static str {
            "integer"
        }
    }

    const impl LuaTypeName for mlua::Number {
        fn lua_name() -> &'static str {
            "number"
        }
    }

    const impl LuaTypeName for bool {
        fn lua_name() -> &'static str {
            "boolean"
        }
    }

    const impl LuaTypeName for i32 {
        fn lua_name() -> &'static str {
            "i32"
        }
    }

    impl LuaAliasDoc for i32 {
        fn lua_alias_doc() -> &'static str {
            gen_alias_doc!(mlua::Integer)
        }
    }

    const impl LuaTypeName for u32 {
        fn lua_name() -> &'static str {
            "u32"
        }
    }

    impl LuaAliasDoc for u32 {
        fn lua_alias_doc() -> &'static str {
            gen_alias_doc!(mlua::Integer)
        }
    }

    const impl LuaTypeName for u8 {
        fn lua_name() -> &'static str {
            "u8"
        }
    }

    impl LuaAliasDoc for u8 {
        fn lua_alias_doc() -> &'static str {
            gen_alias_doc!(mlua::Integer)
        }
    }

    const impl LuaTypeName for f32 {
        fn lua_name() -> &'static str {
            "f32"
        }
    }

    impl LuaAliasDoc for f32 {
        fn lua_alias_doc() -> &'static str {
            gen_alias_doc!(mlua::Number)
        }
    }

    const impl LuaTypeName for Vec<i32> {
        fn lua_name() -> &'static str {
            "i32[]"
        }
    }

    const impl LuaTypeName for Vec<u32> {
        fn lua_name() -> &'static str {
            "u32[]"
        }
    }

    const impl LuaTypeName for Vec<f32> {
        fn lua_name() -> &'static str {
            "f32[]"
        }
    }

    const impl LuaTypeName for mlua::BorrowedStr {
        fn lua_name() -> &'static str {
            "string"
        }
    }

    const impl LuaTypeName for mlua::LuaString {
        fn lua_name() -> &'static str {
            "string"
        }
    }

    const impl LuaTypeName for mlua::Table {
        fn lua_name() -> &'static str {
            "table<unknown, unknown>"
        }
    }

    impl<K, V> LuaTypeName for TypedTable<K, V>
    where
        K: LuaTypeName,
        V: LuaTypeName,
    {
        fn lua_name() -> &'static str {
            generic_fake_static::<Self>(|| format!("table<{}, {}>", K::lua_name(), V::lua_name()))
        }
    }

    impl<K, V> LuaTypeName for IdMap<K, V>
    where
        K: LuaTypeName + IdLike,
        V: LuaTypeName,
    {
        fn lua_name() -> &'static str {
            generic_fake_static::<Self>(|| format!("table<{}, {}>", K::lua_name(), V::lua_name()))
        }
    }

    impl<K> LuaTypeName for IdSet<K>
    where
        K: LuaTypeName + IdLike,
    {
        fn lua_name() -> &'static str {
            generic_fake_static::<Self>(|| format!("table<{}, boolean>", K::lua_name()))
        }
    }

    impl<V> LuaTypeName for IdCoordMap<V>
    where
        V: LuaTypeName + IdLike,
    {
        fn lua_name() -> &'static str {
            generic_fake_static::<Self>(|| format!("table<{}, {}>", TileCoord::lua_name(), V::lua_name()))
        }
    }

    impl<K, V> LuaTypeName for ImmutableIdMap<K, V>
    where
        K: LuaTypeName + IdLike,
        V: LuaTypeName,
    {
        fn lua_name() -> &'static str {
            generic_fake_static::<Self>(|| format!("table<{}, {}>", K::lua_name(), V::lua_name()))
        }
    }

    impl<K> LuaTypeName for ImmutableIdSet<K>
    where
        K: LuaTypeName + IdLike,
    {
        fn lua_name() -> &'static str {
            generic_fake_static::<Self>(|| format!("table<{}, boolean>", K::lua_name()))
        }
    }

    pub const trait MetaMethodExt {
        fn doc_name(self) -> &'static str;
    }

    const impl MetaMethodExt for MetaMethod {
        fn doc_name(self) -> &'static str {
            match self {
                MetaMethod::Add => "add",
                MetaMethod::Sub => "sub",
                MetaMethod::Mul => "mul",
                MetaMethod::Div => "div",
                MetaMethod::Mod => "mod",
                MetaMethod::Pow => "pow",
                MetaMethod::Unm => "unm",

                MetaMethod::Concat => "concat",
                MetaMethod::Len => "len",
                MetaMethod::Eq => "eq",
                MetaMethod::Lt => "lt",
                MetaMethod::Le => "le",
                MetaMethod::Index => "index",
                MetaMethod::NewIndex => "newindex",
                MetaMethod::Call => "call",
                MetaMethod::ToString => "tostring",
                MetaMethod::ToDebugString => "todebugstring",

                MetaMethod::Type => "name",

                _ => unreachable!(),
            }
        }
    }
}
