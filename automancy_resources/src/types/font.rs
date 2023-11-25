use crate::{LoadResourceError, ResourceManager, COULD_NOT_GET_FILE_STEM, FONT_EXT};
use automancy_defs::flexstr::ToSharedStr;
use automancy_defs::log;
use std::ffi::OsStr;
use std::fs::{read_dir, File};
use std::io::Read;
use std::path::Path;
use ttf_parser::{name_id, Face};

pub struct Font {
    pub name: String,
    pub data: Vec<u8>,
}
impl ResourceManager {
    pub fn load_fonts(&mut self, dir: &Path) -> anyhow::Result<()> {
        let fonts = dir.join("fonts");
        if let Ok(fonts) = read_dir(fonts) {
            let mut fonts = fonts
                .into_iter()
                .flatten()
                .map(|v| v.path())
                .filter(|v| {
                    v.extension().is_some()
                        && FONT_EXT.contains(&v.extension().unwrap().to_str().unwrap())
                })
                .collect::<Vec<_>>();
            for file in fonts {
                log::info!("loading font {file:?}");
                let mut data: Vec<u8> = Vec::new();
                File::open(&file)?.read_to_end(&mut data)?;
                let file_stem = file.file_stem().unwrap().to_str().unwrap().to_string();
                let name = Face::parse(data.as_slice(), 0)?
                    .tables()
                    .name
                    .expect("Failed to get name table (likely malformed font file)")
                    .names
                    .into_iter()
                    .filter_map(|n| n.to_string())
                    .find(|n| n.to_lowercase()[..2] == file_stem.to_lowercase()[..2])
                    .unwrap_or(file_stem);
                self.fonts.insert(
                    file.file_name()
                        .ok_or_else(|| {
                            LoadResourceError::InvalidFileError(
                                file.clone(),
                                COULD_NOT_GET_FILE_STEM,
                            )
                        })?
                        .to_str()
                        .ok_or_else(|| LoadResourceError::OsStringError(file.clone()))?
                        .to_string()
                        .to_shared_str(),
                    Font { name, data },
                );
            }
        }
        Ok(())
    }
}
