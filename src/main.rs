use anyhow::{
    Context,
    Result,
};
use clap::Parser;
use image::{
    Rgb,
    Rgba,
};
use itertools::Itertools;
use rayon::iter::{
    IntoParallelIterator,
    ParallelIterator,
};
use rusttype::{
    Font,
    Scale,
};
use std::{
    env::current_exe,
    fs::DirEntry,
    path::PathBuf,
};
static PREFIX: &str = "ze-stopka-";

fn footer() -> String {
    format!(
        "FotoBudka Maciej Smykowski {}",
        chrono::Local::now().format("%Y-%m-%d")
    )
}

static FONT_BYTES: &[u8] = include_bytes!("../resources/static/Inconsolata/Inconsolata-Bold.ttf");
lazy_static::lazy_static! {
    static ref FONT: Font<'static> = {
        let font = Vec::from(FONT_BYTES);
        let font = Font::try_from_vec(font).expect("BAD FONT");
        font
    };
}
const FONT_SCALE: f32 = 0.58;

use image::{
    DynamicImage,
    GenericImage,
    GenericImageView,
};
use std::io::{
    BufWriter,
    Seek,
    Write,
};

fn add_footer(bytes: &[u8]) -> Result<Vec<u8>> {
    let format = image::guess_format(bytes).context("odgadywanie formatu")?;
    let mut parsed = image::load(std::io::Cursor::new(bytes), format).context("ładowanie pliku")?;
    let (width, height) = (parsed.width() as f32, parsed.height() as f32);
    // let height = 12.4;
    let text = footer();
    let scale = width / (text.len() as f32 * FONT_SCALE);
    let scale = Scale { x: scale, y: scale };
    static COLOR: Rgba<u8> = Rgba([255u8, 255u8, 255u8, u8::MAX]);
    let margin = (width * 0.05) as i32;
    imageproc::drawing::draw_text_mut(
        &mut parsed,
        COLOR,
        margin,
        (height - scale.y - (margin as f32 * 0.5)) as i32,
        scale,
        &FONT,
        &text,
    );
    let mut buffer = vec![];
    let mut writer = std::io::Cursor::new(&mut buffer);
    parsed
        .write_to(&mut writer, format)
        .with_context(|| format!("zapisywanie pliku do oryginalnego formatu ({format:?})"))?;
    Ok(buffer)
}

fn add_footer_to_file(dir_entry: &DirEntry) -> Result<PathBuf> {
    let path = dir_entry.path();
    let file_name = {
        path.file_name()
            .context("plik nie ma nazwy")?
            .to_string_lossy()
            .to_string()
    };
    let new_path = path.with_file_name(format!("{PREFIX}__{file_name}"));
    if new_path.exists() {
        return Ok(new_path);
    }

    let bytes = std::fs::read(&path).with_context(|| format!("otwieranie pliku [{path:?}]"))?;
    let bytes_with_footer = add_footer(&bytes).context("dodawanie nagłówka")?;
    std::fs::write(&new_path, bytes_with_footer).context("zapisywanie pliku")?;
    Ok(new_path)
}

#[derive(Debug, Parser)]
#[clap(author, version)]
struct Args {
    /// folder który ma być zmodyfikowany
    #[clap(long, short)]
    pub folder: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::try_init().ok();
    let current_dir = match args.folder {
        Some(dir) => dir,
        None => {
            let current_exe = current_exe().context("odczytywanie ścieżki tego pliku na dysku")?;

            current_exe
                .parent()
                .with_context(|| format!("znajdowanie folderu który ma być wzbogacony o ściezki (katalog w którym znajduje się plik [{current_exe:?}])"))?.into()
        }
    };

    let entries = std::fs::read_dir(&current_dir)
        .with_context(|| format!("czytanie plików ze ścieżki :: {current_dir:?}"))?;
    let file_paths = entries
        .into_iter()
        .filter_map(|file| file.ok())
        .filter(|file| {
            file.file_type()
                .map(|kind| kind.is_file())
                .unwrap_or_default()
        })
        .filter(|file| {
            !file
                .path()
                .file_name()
                .map(|file_name| file_name.to_string_lossy().starts_with(PREFIX))
                .unwrap_or(true)
        })
        .collect_vec();
    file_paths.into_par_iter().for_each(|file| {
        log::info!(
            "{} :: {:#?}",
            file.path().display(),
            add_footer_to_file(&file).with_context(|| format!("dodawanie stopki do {file:?}"))
        );
    });

    Ok(())
}
