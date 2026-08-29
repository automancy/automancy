use std::{fs::File, io::Read, path::Path, sync::Arc};

use log;

use crate::resources::{FONT_EXTS, MutableResourceManager, ResourceError, read_recursively};

fn collect_families(name_id: u16, names: &ttf_parser::name::Names) -> Vec<(String, ttf_parser::Language)> {
    let mut families = Vec::new();
    for name in names.into_iter() {
        if name.name_id == name_id
            && let Some(family) = name.to_string()
        {
            families.push((family, name.language()));
        }
    }

    // Make English US the first one.
    if families.len() > 1
        && let Some(index) = families.iter().position(|f| f.1 == ttf_parser::Language::English_UnitedStates)
        && index != 0
    {
        families.swap(0, index);
    }

    families
}

fn font_family_name(names: ttf_parser::name::Names) -> Option<String> {
    let families = collect_families(ttf_parser::name_id::FAMILY, &names);

    families.into_iter().next().map(|v| v.0)
}

fn font_name(names: ttf_parser::name::Names) -> Option<String> {
    let families = collect_families(ttf_parser::name_id::TYPOGRAPHIC_FAMILY, &names);

    // We have to fallback to Family Name when no Typographic Family Name was set.
    if families.is_empty() {
        return font_family_name(names);
    }

    families.into_iter().next().map(|v| v.0)
}

#[derive(Debug)]
pub struct FontData {
    pub font_family: String,
    pub weight: ttf_parser::Weight,
    pub style: ttf_parser::Style,
    pub stretch: ttf_parser::Width,
    pub bytes: Arc<Vec<u8>>,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_font_file(&mut self, path: &Path) -> Result<(), ResourceError> {
        log::info!("Loading font {}.", path.display());

        let mut data: Vec<u8> = Vec::new();
        File::open(path)?.read_to_end(&mut data)?;
        let bytes = Arc::new(data);

        let face = ttf_parser::Face::parse(&bytes, 0).map_err(|err| ResourceError::CouldNotParseFont(path.to_path_buf(), err))?;

        let name = font_name(face.names()).ok_or_else(|| ResourceError::CouldNotGetFontName(path.to_path_buf()))?;

        let weight = face.weight();
        let style = face.style();
        let stretch = face.width();

        self.fonts.entry(name.clone()).or_default().push(FontData {
            font_family: name.clone(),
            weight,
            style,
            stretch,
            bytes,
        });

        log::info!("Loaded font '{name} ({weight:?}, {style:?})'!");

        Ok(())
    }

    pub fn load_font_files(dir: &Path) {
        MutableResourceManager::with(|resource_man| {
            let path = dir.join("fonts");

            for entry in read_recursively(&path, FONT_EXTS) {
                match resource_man.load_font_file(entry.path()) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }
}
