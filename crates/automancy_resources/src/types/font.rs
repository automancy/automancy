use crate::{LoadResourceError, ResourceManager, FONT_EXT};
use automancy_defs::{
    log,
    ttf_parser::{
        name::Names,
        name_id::{FAMILY, TYPOGRAPHIC_FAMILY},
        Face, Language,
    },
};
use std::ffi::OsStr;
use std::fs::{read_dir, File};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

pub struct Font {
    pub name: String,
    pub data: Arc<Vec<u8>>,
}

fn collect_families(name_id: u16, names: &Names) -> Vec<(String, Language)> {
    let mut families = Vec::new();
    for name in names.into_iter() {
        if name.name_id == name_id {
            if let Some(family) = name.to_string() {
                families.push((family, name.language()));
            }
        }
    }

    families
}

fn parse_name(names: Names) -> Option<String> {
    let mut families = collect_families(TYPOGRAPHIC_FAMILY, &names);

    // We have to fallback to Family Name when no Typographic Family Name was set.
    if families.is_empty() {
        families = collect_families(FAMILY, &names);
    }

    // Make English US the first one.
    if families.len() > 1 {
        if let Some(index) = families
            .iter()
            .position(|f| f.1 == Language::English_UnitedStates)
        {
            if index != 0 {
                families.swap(0, index);
            }
        }
    }

    if families.is_empty() {
        return None;
    }

    families.into_iter().next().map(|v| v.0)
}

impl ResourceManager {
    pub fn load_fonts(&mut self, dir: &Path) -> anyhow::Result<()> {
        let fonts = dir.join("fonts");

        if let Ok(fonts) = read_dir(fonts) {
            for file in fonts.into_iter().flatten().map(|v| v.path()).filter(|v| {
                v.extension()
                    .and_then(OsStr::to_str)
                    .is_some_and(|v| FONT_EXT.contains(&v))
            }) {
                log::info!("Loading font {file:?}");

                let mut data: Vec<u8> = Vec::new();
                File::open(&file)?.read_to_end(&mut data)?;
                let data = Arc::new(data);

                let name = parse_name(Face::parse(&data, 0)?.names())
                    .ok_or_else(|| LoadResourceError::CouldNotGetFontName(file.clone()))?;

                log::info!("Loaded font '{name}'!");

                self.fonts.insert(name.clone(), Font { name, data });
            }
        }

        Ok(())
    }
}
