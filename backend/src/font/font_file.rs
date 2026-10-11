use std::{io::Cursor, path::PathBuf};

use derivative::Derivative;
use image::{imageops::FilterType, io::Reader, DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};

use super::{
    dimensions::{detect_dimensions, CharacterSize, FontType},
    error::FontFileError,
};

/// Font that is used when no font file has been loaded: the 1080p font of the Walksnail goggles.
const BUILT_IN_FONT: &[u8] = include_bytes!("../../../resources/fonts/WS_APC_Nexus_36.png");
const BUILT_IN_FONT_NAME: &str = "WS_APC_Nexus_36.png";

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct FontFile {
    pub file_path: PathBuf,
    /// The font is part of the program and not loaded from `file_path`.
    pub built_in: bool,
    pub character_count: u32,
    pub character_size: CharacterSize,
    pub font_type: FontType,
    #[derivative(Debug = "ignore")]
    characters: Vec<RgbaImage>,
}

impl FontFile {
    #[tracing::instrument(ret, err)]
    pub fn open(path: PathBuf) -> Result<Self, FontFileError> {
        let font_image = Reader::open(&path)?.decode()?;
        Self::from_image(path, false, &font_image)
    }

    /// The font that is part of the program, so rendering works without loading a font file first.
    pub fn built_in() -> Self {
        let font_image = Reader::new(Cursor::new(BUILT_IN_FONT))
            .with_guessed_format()
            .and_then(|reader| reader.decode().map_err(std::io::Error::other))
            .expect("The built-in font is a valid image");
        Self::from_image(PathBuf::from(BUILT_IN_FONT_NAME), true, &font_image)
            .expect("The built-in font has valid dimensions")
    }

    fn from_image(path: PathBuf, built_in: bool, font_image: &DynamicImage) -> Result<Self, FontFileError> {
        let (width, height) = font_image.dimensions();
        let (character_size, font_type, character_count) = detect_dimensions(width, height)?;

        let characters = split_characters(font_image, &character_size, &font_type, character_count);

        Ok(Self {
            file_path: path,
            built_in,
            character_count,
            character_size,
            font_type,
            characters,
        })
    }

    pub fn get_character(&self, index: usize, size: &CharacterSize) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.characters.get(index).map(|original_image| {
            if size.width() != self.character_size.width() || size.height() != self.character_size.height() {
                image::imageops::resize(original_image, size.width(), size.height(), FilterType::Lanczos3)
            } else {
                original_image.clone()
            }
        })
    }
}

fn split_characters(
    font_image: &DynamicImage,
    character_size: &CharacterSize,
    font_type: &FontType,
    character_count: u32,
) -> Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let pages = font_type.pages();
    let char_width = character_size.width();
    let char_height = character_size.height();

    let mut char_vec = Vec::with_capacity((character_count * pages) as usize);

    for page_idx in 0..pages {
        let x = page_idx * char_width;
        for char_idx in 0..character_count {
            let y = char_idx * char_height;
            let char = font_image.view(x, y, char_width, char_height).to_image();
            char_vec.push(char);
        }
    }

    char_vec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_font_is_valid() {
        let font = FontFile::built_in();
        assert!(font.built_in);
        assert_eq!(font.character_count, 256);
        assert_eq!(font.character_size, CharacterSize::Large);
        assert_eq!(font.font_type, FontType::Standard);
        assert!(font.get_character(65, &CharacterSize::Large).is_some());
    }
}
