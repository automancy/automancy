use crate::{
    id::{ItemId, ModelId, TileId},
    math::{Matrix4, Quat, Vec3, consts},
};

pub mod colors;
pub mod draw;
pub mod view;

pub mod util {
    use crate::math::{Float, Matrix4, Vec2, Vec3, vec2_to_radians};

    pub const LINE_DEPTH: Float = 0.075;

    /// Produces a line shape.
    #[inline]
    pub fn make_line(a: Vec2, b: Vec2, z: Float) -> Matrix4 {
        let mid = Vec2::lerp(a, b, 0.5);
        let d = a.distance(b);
        let theta = vec2_to_radians(b - a);

        Matrix4::translation_3d(Vec3::new(mid.x, mid.y, z))
            * Matrix4::rotation_z(theta)
            * Matrix4::scaling_3d(Vec3::new(d.max(0.001), 0.1, LINE_DEPTH))
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum GenericModel {
    #[default]
    None,
    Plain(ModelId),
    Item(ItemId),
    Tile(TileId),
}

impl GenericModel {
    pub fn view_matrix(&self) -> Matrix4 {
        match self {
            GenericModel::None => Matrix4::default(),
            GenericModel::Plain(..) => Matrix4::default(),
            GenericModel::Item(..) => Matrix4::default(),
            GenericModel::Tile(..) => {
                let rot = Quat::rotation_x(0.4);
                let eye = rot * Vec3::new(0.0, 0.0, 3.0);

                view::camera_projection(consts::FRAC_PI_4, 1.0)
                    * Matrix4::look_at_rh(eye, eye + rot * Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.0, 0.0))
            },
        }
    }
}

pub mod deserialize {
    use serde::Deserialize;

    use crate::{
        id::{
            IdInterner, ItemId, ModelId, TileId,
            deserialize::{StrId, StrIdParseError},
        },
        rendering::GenericModel,
    };

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
    pub enum StrGenericModel {
        Plain(StrId),
        Item(StrId),
        Tile(StrId),
    }

    impl StrGenericModel {
        pub fn into_icon(self, interner: &mut IdInterner, fallback_namespace: Option<&str>) -> Result<GenericModel, StrIdParseError> {
            Ok(match self {
                StrGenericModel::Plain(id) => GenericModel::Plain(ModelId(interner.get_or_intern(id, fallback_namespace)?)),
                StrGenericModel::Item(id) => GenericModel::Item(ItemId(interner.get_or_intern(id, fallback_namespace)?)),
                StrGenericModel::Tile(id) => GenericModel::Tile(TileId(interner.get_or_intern(id, fallback_namespace)?)),
            })
        }
    }
}
