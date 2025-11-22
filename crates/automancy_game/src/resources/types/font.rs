use std::{
    ffi::OsStr,
    fs::{File, read_dir},
    io::Read,
    path::Path,
    sync::Arc,
};

use log;

use crate::resources::{FONT_EXT, LoadResourceError, ResourceManager};

pub struct Font {
    pub name: String,
    pub data: Arc<Vec<u8>>,
}

fn collect_families(name_id: u16, names: &ttf_parser::name::Names) -> Vec<(String, ttf_parser::Language)> {
    let mut families = Vec::new();
    for name in names.into_iter() {
        if name.name_id == name_id
            && let Some(family) = name.to_string()
        {
            families.push((family, name.language()));
        }
    }

    families
}

pub fn get_font_family_name(names: ttf_parser::name::Names) -> Option<String> {
    let mut families = collect_families(ttf_parser::name_id::TYPOGRAPHIC_FAMILY, &names);

    // We have to fallback to Family Name when no Typographic Family Name was set.
    if families.is_empty() {
        families = collect_families(ttf_parser::name_id::FAMILY, &names);
    }

    // Make English US the first one.
    if families.len() > 1
        && let Some(index) = families.iter().position(|f| f.1 == ttf_parser::Language::English_UnitedStates)
        && index != 0
    {
        families.swap(0, index);
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
            for file in fonts
                .into_iter()
                .flatten()
                .map(|v| v.path())
                .filter(|v| v.extension().and_then(OsStr::to_str).is_some_and(|v| FONT_EXT.contains(&v)))
            {
                log::info!("Loading font {file:?}");

                let mut data: Vec<u8> = Vec::new();
                File::open(&file)?.read_to_end(&mut data)?;
                let data = Arc::new(data);

                let name = get_font_family_name(ttf_parser::Face::parse(&data, 0)?.names())
                    .ok_or_else(|| LoadResourceError::CouldNotGetFontName(file.clone()))?;

                log::info!("Loaded font '{name}'!");

                self.fonts.insert(name.clone(), Font { name, data });
            }
        }

        Ok(())
    }
}
