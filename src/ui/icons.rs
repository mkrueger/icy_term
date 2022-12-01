use egui_extras::RetainedImage;

lazy_static::lazy_static! {
    pub static ref call_svg: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "call.svg",
        include_bytes!("../../data/icons/call.svg"),
    )
    .unwrap(); 
}

lazy_static::lazy_static! {
    pub static ref call_end_svg: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "call_end.svg",
        include_bytes!("../../data/icons/call_end.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref upload_svg: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "upload.svg",
        include_bytes!("../../data/icons/upload.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref download_svg: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "download.svg",
        include_bytes!("../../data/icons/download.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref key_svg: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "key.svg",
        include_bytes!("../../data/icons/key.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref delete_svg: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "delete.svg",
        include_bytes!("../../data/icons/delete.svg"),
    )
    .unwrap();
}

lazy_static::lazy_static! {
    pub static ref add_svg: RetainedImage = egui_extras::RetainedImage::from_svg_bytes(
        "add.svg",
        include_bytes!("../../data/icons/add.svg"),
    )
    .unwrap();
}