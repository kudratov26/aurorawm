use image::GenericImageView;

pub struct WallpaperData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn load_wallpaper(path: Option<&str>) -> Option<WallpaperData> {
    let path = path?;
    let img = image::open(path).ok()?;
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8().into_raw();
    Some(WallpaperData {
        width,
        height,
        rgba,
    })
}
