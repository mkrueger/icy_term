use egui_extras::RetainedImage;

lazy_static::lazy_static! {
    pub static ref CALL_END_SVG: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "call_end.svg",
        include_bytes!("../../data/icons/call_end.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref UPLOAD_SVG: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "upload.svg",
        include_bytes!("../../data/icons/upload.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref DOWNLOAD_SVG: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "download.svg",
        include_bytes!("../../data/icons/download.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref KEY_SVG: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "key.svg",
        include_bytes!("../../data/icons/key.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref DELETE_SVG: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "delete.svg",
        include_bytes!("../../data/icons/delete.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref ADD_SVG: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "add.svg",
        include_bytes!("../../data/icons/add.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref SETTINGS_SVG: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "settings.svg",
        include_bytes!("../../data/icons/settings.svg"),
    )
    .unwrap();
}
