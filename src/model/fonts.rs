
use super::{ Size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BitFontType {
    BuiltIn,
    _Library,
    _Custom
}


#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BitFont {
    pub name: String,
    pub size: Size<u8>,
    font_type: BitFontType,
    data_32: Option<Vec<u32>>,
    data_8: Vec<u8>
}

impl Default for BitFont {
    fn default() -> Self {
        BitFont::from_name(DEFAULT_FONT_NAME).unwrap()
    }
}

impl BitFont {
    pub fn from_name(font_name: &str) -> Option<Self>
    {
        if let Some(data) = get_font_data(font_name) {
            Some(BitFont {
                name: font_name.to_string(), 
                size: len_to_size(data.len()),
                font_type: BitFontType::BuiltIn,
                data_32: None,
                data_8: data.to_vec()
            })
        } else {
            None
        }
    }

    pub fn get_scanline(&self, ch: u16, y: usize) -> u32
    {
        if let Some(data_32) = &self.data_32 {
            data_32[ch as usize * self.size.height as usize + y]
        } else {
            self.data_8[ch as usize * self.size.height as usize + y] as u32
        }
    }
    
}

const IBM_CP437_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP437.F08");
const IBM_CP437_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP437.F14");
const IBM_CP437_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP437.F16");
const IBM_CP437_F19: &[u8] = include_bytes!("../../data/fonts/IBM/CP437.F19");

const IBM_CP737_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP737.F08");
const IBM_CP737_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP737.F14");
const IBM_CP737_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP737.F16");

const IBM_CP775_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP775.F08");
const IBM_CP775_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP775.F14");
const IBM_CP775_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP775.F16");

const IBM_CP850_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP850.F08");
const IBM_CP850_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP850.F14");
const IBM_CP850_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP850.F16");
const IBM_CP850_F19: &[u8] = include_bytes!("../../data/fonts/IBM/CP850.F19");

const IBM_CP852_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP852.F08");
const IBM_CP852_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP852.F14");
const IBM_CP852_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP852.F16");
const IBM_CP852_F19: &[u8] = include_bytes!("../../data/fonts/IBM/CP852.F19");

const IBM_CP855_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP855.F08");
const IBM_CP855_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP855.F14");
const IBM_CP855_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP855.F16");

const IBM_CP857_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP857.F08");
const IBM_CP857_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP857.F14");
const IBM_CP857_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP857.F16");

const IBM_CP860_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP860.F08");
const IBM_CP860_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP860.F14");
const IBM_CP860_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP860.F16");
const IBM_CP860_F19: &[u8] = include_bytes!("../../data/fonts/IBM/CP860.F19");

const IBM_CP861_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP861.F08");
const IBM_CP861_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP861.F14");
const IBM_CP861_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP861.F16");
const IBM_CP861_F19: &[u8] = include_bytes!("../../data/fonts/IBM/CP861.F19");

const IBM_CP862_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP862.F08");
const IBM_CP862_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP862.F14");
const IBM_CP862_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP862.F16");

const IBM_CP863_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP863.F08");
const IBM_CP863_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP863.F14");
const IBM_CP863_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP863.F16");
const IBM_CP863_F19: &[u8] = include_bytes!("../../data/fonts/IBM/CP863.F19");

const IBM_CP864_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP864.F08");
const IBM_CP864_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP864.F14");
const IBM_CP864_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP864.F16");

const IBM_CP865_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP865.F08");
const IBM_CP865_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP865.F14");
const IBM_CP865_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP865.F16");
const IBM_CP865_F19: &[u8] = include_bytes!("../../data/fonts/IBM/CP865.F19");

const IBM_CP866_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP866.F08");
const IBM_CP866_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP866.F14");
const IBM_CP866_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP866.F16");

const IBM_CP869_F08: &[u8] = include_bytes!("../../data/fonts/IBM/CP869.F08");
const IBM_CP869_F14: &[u8] = include_bytes!("../../data/fonts/IBM/CP869.F14");
const IBM_CP869_F16: &[u8] = include_bytes!("../../data/fonts/IBM/CP869.F16");

const AMIGA_TOPAZ_1 : &[u8] = include_bytes!("../../data/fonts/Amiga/Amiga Topaz 1.F16");
const AMIGA_TOPAZ_1P : &[u8] = include_bytes!("../../data/fonts/Amiga/Amiga Topaz 1+.F16");
const AMIGA_TOPAZ_2 : &[u8] = include_bytes!("../../data/fonts/Amiga/Amiga Topaz 2.F16");
const AMIGA_TOPAZ_2P : &[u8] = include_bytes!("../../data/fonts/Amiga/Amiga Topaz 2+.F16");
const AMIGA_P0T_NOODLE : &[u8] = include_bytes!("../../data/fonts/Amiga/Amiga P0T-NOoDLE.F16");
const AMIGA_MICROKNIGHT : &[u8] = include_bytes!("../../data/fonts/Amiga/Amiga MicroKnight.F16");
const AMIGA_MICROKNIGHTP : &[u8] = include_bytes!("../../data/fonts/Amiga/Amiga MicroKnight+.F16");
const AMIGA_MOSOUL : &[u8] = include_bytes!("../../data/fonts/Amiga/Amiga mOsOul.F16");

const C64_PETSCII_SHIFTED : &[u8] = include_bytes!("../../data/fonts/C64/C64 PETSCII shifted.F08");
const C64_PETSCII_UNSHIFTED : &[u8] = include_bytes!("../../data/fonts/C64/C64 PETSCII unshifted.F08");
const ATARI_ATASCII : &[u8] = include_bytes!("../../data/fonts/Atari/Atari ATASCII.F08");

pub const DEFAULT_FONT_NAME: &str = "IBM VGA";

pub const _SUPPORTED_FONTS: [&str;91] = [
    "IBM VGA",
    "IBM VGA50",
    "IBM VGA25G",
    "IBM EGA",
    "IBM EGA43",

    "IBM VGA 437",
    "IBM VGA50 437",
    "IBM VGA25G 437",
    "IBM EGA 437",
    "IBM EGA43 437",

    /* 
    "IBM VGA 720",
    "IBM VGA50 720",
    "IBM VGA25G 720",
    "IBM EGA 720",
    "IBM EGA43 720",*/
    
    "IBM VGA 737",
    "IBM VGA50 737",
    //"IBM VGA25G 737",
    "IBM EGA 737",
    "IBM EGA43 737",

    "IBM VGA 775",
    "IBM VGA50 775",
    //"IBM VGA25G 775",
    "IBM EGA 775",
    "IBM EGA43 775",

    /* "IBM VGA 819",
    "IBM VGA50 819",
    "IBM VGA25G 819",
    "IBM EGA 819",
    "IBM EGA43 819",*/

    "IBM VGA 850",
    "IBM VGA50 850",
    "IBM VGA25G 850",
    "IBM EGA 850",
    "IBM EGA43 850",

    "IBM VGA 852",
    "IBM VGA50 852",
    "IBM VGA25G 852",
    "IBM EGA 852",
    "IBM EGA43 852",

    "IBM VGA 855",
    "IBM VGA50 855",
    //"IBM VGA25G 855",
    "IBM EGA 855",
    "IBM EGA43 855",

    "IBM VGA 857",
    "IBM VGA50 857",
    //"IBM VGA25G 857",
    "IBM EGA 857",
    "IBM EGA43 857",/*

    "IBM VGA 858",
    "IBM VGA50 858",
    "IBM VGA25G 858",
    "IBM EGA 858",
    "IBM EGA43 858",*/

    "IBM VGA 860",
    "IBM VGA50 860",
    "IBM VGA25G 860",
    "IBM EGA 860",
    "IBM EGA43 860",

    "IBM VGA 861",
    "IBM VGA50 861",
    "IBM VGA25G 861",
    "IBM EGA 861",
    "IBM EGA43 861",

    "IBM VGA 862",
    "IBM VGA50 862",
    //"IBM VGA25G 862",
    "IBM EGA 862",
    "IBM EGA43 862",

    "IBM VGA 863",
    "IBM VGA50 863",
    "IBM VGA25G 863",
    "IBM EGA 863",
    "IBM EGA43 863",

    "IBM VGA 864",
    "IBM VGA50 864",
    //"IBM VGA25G 864",
    "IBM EGA 864",
    "IBM EGA43 864",

    "IBM VGA 865",
    "IBM VGA50 865",
    "IBM VGA25G 865",
    "IBM EGA 865",
    "IBM EGA43 865",

    "IBM VGA 866",
    "IBM VGA50 866",
    //"IBM VGA25G 866",
    "IBM EGA 866",
    "IBM EGA43 866",

    "IBM VGA 869",
    "IBM VGA50 869",
    //"IBM VGA25G 869",
    "IBM EGA 869",
    "IBM EGA43 869",

    /*"IBM VGA 872",
    "IBM VGA50 872",
    "IBM VGA25G 872",
    "IBM EGA 872",
    "IBM EGA43 872",

    "IBM VGA KAM",
    "IBM VGA50 KAM",
    "IBM VGA25G KAM",
    "IBM EGA KAM",
    "IBM EGA43 KAM",

    "IBM VGA MAZ",
    "IBM VGA50 MAZ",
    "IBM VGA25G MAZ",
    "IBM EGA MAZ",
    "IBM EGA43 MAZ",*/

    "IBM VGA MIK",
    "IBM VGA50 MIK",
    //"IBM VGA25G MIK",
    "IBM EGA MIK",
    "IBM EGA43 MIK",

    /* "IBM VGA 667",
    "IBM VGA50 667",
    "IBM VGA25G 667",
    "IBM EGA 667",
    "IBM EGA43 667",

    "IBM VGA 790",
    "IBM VGA50 790",
    "IBM VGA25G 790",
    "IBM EGA 790",
    "IBM EGA43 790",*/

    "IBM VGA 866",
    "IBM VGA50 866",
    //"IBM VGA25G 866",
    "IBM EGA 866",
    "IBM EGA43 866",

            /*
    "IBM VGA 867",
    "IBM VGA50 867",
    "IBM VGA25G 867",
    "IBM EGA 867",
    "IBM EGA43 867",

    "IBM VGA 895",
    "IBM VGA50 895",
    "IBM VGA25G 895",
    "IBM EGA 895",
    "IBM EGA43 895",

    "IBM VGA 991",
    "IBM VGA50 991",
    "IBM VGA25G 991",
    "IBM EGA 991",
    "IBM EGA43 991",*/
    
    "Amiga Topaz 1",
    "Amiga Topaz 1+",
    "Amiga Topaz 2",
    "Amiga Topaz 2+",
    "Amiga P0T-NOoDLE",
    "Amiga MicroKnight",
    "Amiga MicroKnight+",
    "Amiga mOsOul",

    "C64 PETSCII unshifted",
    "C64 PETSCII shifted",

    "Atari ATASCII",
];

fn len_to_size(len: usize) -> Size<u8>
{
    // only some variants are supported.
    match len / 256 {
        8 => Size::from(8, 8),
        14 => Size::from(8, 14),
        16 => Size::from(8, 16),
        19 => Size::from(8, 19),
        _ => panic!("unknown font")
    }
}

#[allow(clippy::match_same_arms)]
fn get_font_data(font_name: &str) -> Option<&[u8]>
{
    match font_name {
        "IBM VGA" | "IBM VGA 437" => Some(IBM_CP437_F16),
        "IBM VGA50" | "IBM VGA50 437" => Some(IBM_CP437_F08),
        "IBM VGA25G"| "IBM VGA25G 437"  => Some(IBM_CP437_F19),
        "IBM EGA" | "IBM EGA 437" => Some(IBM_CP437_F14),
        "IBM EGA43" | "IBM EGA43 437"=> Some(IBM_CP437_F08),

/* 
        "IBM VGA 720" => Some(IBM_CP720_F16),
        "IBM VGA50 720" => Some(IBM_CP720_F08),
        "IBM VGA25G 720" => Some(IBM_CP720_F19),
        "IBM EGA 720" => Some(IBM_CP720_F14),
        "IBM EGA43 720" => Some(IBM_CP720_F08),*/

        "IBM VGA 737" => Some(IBM_CP737_F16),
        "IBM VGA50 737" => Some(IBM_CP737_F08),
//        "IBM VGA25G 737" => Some(IBM_CP737_F19),
        "IBM EGA 737" => Some(IBM_CP737_F14),
        "IBM EGA43 737" => Some(IBM_CP737_F08),

        "IBM VGA 775" => Some(IBM_CP775_F16),
        "IBM VGA50 775" => Some(IBM_CP775_F08),
//        "IBM VGA25G 775" => Some(IBM_CP775_F19),
        "IBM EGA 775" => Some(IBM_CP775_F14),
        "IBM EGA43 775" => Some(IBM_CP775_F08),

/*         "IBM VGA 819" => Some(IBM_CP819_F16),
        "IBM VGA50 819" => Some(IBM_CP819_F08),
        "IBM VGA25G 819" => Some(IBM_CP819_F19),
        "IBM EGA 819" => Some(IBM_CP819_F14),
        "IBM EGA43 819" => Some(IBM_CP819_F08),*/

        "IBM VGA 850" => Some(IBM_CP850_F16),
        "IBM VGA50 850" => Some(IBM_CP850_F08),
        "IBM VGA25G 850" => Some(IBM_CP850_F19),
        "IBM EGA 850" => Some(IBM_CP850_F14),
        "IBM EGA43 850" => Some(IBM_CP850_F08),

        "IBM VGA 852" => Some(IBM_CP852_F16),
        "IBM VGA50 852" => Some(IBM_CP852_F08),
        "IBM VGA25G 852" => Some(IBM_CP852_F19),
        "IBM EGA 852" => Some(IBM_CP852_F14),
        "IBM EGA43 852" => Some(IBM_CP852_F08),

        "IBM VGA 855" => Some(IBM_CP855_F16),
        "IBM VGA50 855" => Some(IBM_CP855_F08),
//        "IBM VGA25G 855" => Some(IBM_CP855_F19),
        "IBM EGA 855" => Some(IBM_CP855_F14),
        "IBM EGA43 855" => Some(IBM_CP855_F08),

        "IBM VGA 857" => Some(IBM_CP857_F16),
        "IBM VGA50 857" => Some(IBM_CP857_F08),
//        "IBM VGA25G 857" => Some(IBM_CP857_F19),
        "IBM EGA 857" => Some(IBM_CP857_F14),
        "IBM EGA43 857" => Some(IBM_CP857_F08),/*

        "IBM VGA 858" => Some(IBM_CP858_F16),
        "IBM VGA50 858" => Some(IBM_CP858_F08),
        "IBM VGA25G 858" => Some(IBM_CP858_F19),
        "IBM EGA 858" => Some(IBM_CP858_F14),
        "IBM EGA43 858" => Some(IBM_CP858_F08),*/

        "IBM VGA 860" => Some(IBM_CP860_F16),
        "IBM VGA50 860" => Some(IBM_CP860_F08),
        "IBM VGA25G 860" => Some(IBM_CP860_F19),
        "IBM EGA 860" => Some(IBM_CP860_F14),
        "IBM EGA43 860" => Some(IBM_CP860_F08),

        "IBM VGA 861" => Some(IBM_CP861_F16),
        "IBM VGA50 861" => Some(IBM_CP861_F08),
        "IBM VGA25G 861" => Some(IBM_CP861_F19),
        "IBM EGA 861" => Some(IBM_CP861_F14),
        "IBM EGA43 861" => Some(IBM_CP861_F08),

        "IBM VGA 862" => Some(IBM_CP862_F16),
        "IBM VGA50 862" => Some(IBM_CP862_F08),
//        "IBM VGA25G 862" => Some(IBM_CP862_F19),
        "IBM EGA 862" => Some(IBM_CP862_F14),
        "IBM EGA43 862" => Some(IBM_CP862_F08),

        "IBM VGA 863" => Some(IBM_CP863_F16),
        "IBM VGA50 863" => Some(IBM_CP863_F08),
        "IBM VGA25G 863" => Some(IBM_CP863_F19),
        "IBM EGA 863" => Some(IBM_CP863_F14),
        "IBM EGA43 863" => Some(IBM_CP863_F08),

        "IBM VGA 864" => Some(IBM_CP864_F16),
        "IBM VGA50 864" => Some(IBM_CP864_F08),
//        "IBM VGA25G 864" => Some(IBM_CP864_F19),
        "IBM EGA 864" => Some(IBM_CP864_F14),
        "IBM EGA43 864" => Some(IBM_CP864_F08),

        "IBM VGA 865" => Some(IBM_CP865_F16),
        "IBM VGA50 865" => Some(IBM_CP865_F08),
        "IBM VGA25G 865" => Some(IBM_CP865_F19),
        "IBM EGA 865" => Some(IBM_CP865_F14),
        "IBM EGA43 865" => Some(IBM_CP865_F08),

        "IBM VGA 866" => Some(IBM_CP866_F16),
        "IBM VGA50 866" => Some(IBM_CP866_F08),
//        "IBM VGA25G 866" => Some(IBM_CP866_F19),
        "IBM EGA 866" => Some(IBM_CP866_F14),
        "IBM EGA43 866" => Some(IBM_CP866_F08),

        "IBM VGA 869" => Some(IBM_CP869_F16),
        "IBM VGA50 869" => Some(IBM_CP869_F08),
//        "IBM VGA25G 869" => Some(IBM_CP869_F19),
        "IBM EGA 869" => Some(IBM_CP869_F14),
        "IBM EGA43 869" => Some(IBM_CP869_F08),

/*        "IBM VGA 872" => Some(IBM_CP872_F16),
        "IBM VGA50 872" => Some(IBM_CP872_F08),
        "IBM VGA25G 872" => Some(IBM_CP872_F19),
        "IBM EGA 872" => Some(IBM_CP872_F14),
        "IBM EGA43 872" => Some(IBM_CP872_F08),

        "IBM VGA KAM" => Some(IBM_CP867_F16),
        "IBM VGA50 KAM" => Some(IBM_CP867_F08),
        "IBM VGA25G KAM" => Some(IBM_CP867_F19),
        "IBM EGA KAM" => Some(IBM_CP867_F14),
        "IBM EGA43 KAM" => Some(IBM_CP867_F08),

        "IBM VGA MAZ" => Some(IBM_CP667_F16),
        "IBM VGA50 MAZ" => Some(IBM_CP667_F08),
        "IBM VGA25G MAZ" => Some(IBM_CP667_F19),
        "IBM EGA MAZ" => Some(IBM_CP667_F14),
        "IBM EGA43 MAZ" => Some(IBM_CP667_F08),*/

        "IBM VGA MIK" => Some(IBM_CP866_F16),
        "IBM VGA50 MIK" => Some(IBM_CP866_F08),
//        "IBM VGA25G MIK" => Some(IBM_CP866_F19),
        "IBM EGA MIK" => Some(IBM_CP866_F14),
        "IBM EGA43 MIK" => Some(IBM_CP866_F08),

/*         "IBM VGA 667" => Some(IBM_CP667_F16),
        "IBM VGA50 667" => Some(IBM_CP667_F08),
        "IBM VGA25G 667" => Some(IBM_CP667_F19),
        "IBM EGA 667" => Some(IBM_CP667_F14),
        "IBM EGA43 667" => Some(IBM_CP667_F08),

        "IBM VGA 790" => Some(IBM_CP790_F16),
        "IBM VGA50 790" => Some(IBM_CP790_F08),
        "IBM VGA25G 790" => Some(IBM_CP790_F19),
        "IBM EGA 790" => Some(IBM_CP790_F14),
        "IBM EGA43 790" => Some(IBM_CP790_F08),*/

        /*
        "IBM VGA 867" => Some(IBM_CP867_F16),
        "IBM VGA50 867" => Some(IBM_CP867_F08),
        "IBM VGA25G 867" => Some(IBM_CP867_F19),
        "IBM EGA 867" => Some(IBM_CP867_F14),
        "IBM EGA43 867" => Some(IBM_CP867_F08),

        "IBM VGA 895" => Some(IBM_CP895_F16),
        "IBM VGA50 895" => Some(IBM_CP895_F08),
        "IBM VGA25G 895" => Some(IBM_CP895_F19),
        "IBM EGA 895" => Some(IBM_CP895_F14),
        "IBM EGA43 895" => Some(IBM_CP895_F08),

        "IBM VGA 991" => Some(IBM_CP991_F16),
        "IBM VGA50 991" => Some(IBM_CP991_F08),
        "IBM VGA25G 991" => Some(IBM_CP991_F19),
        "IBM EGA 991" => Some(IBM_CP991_F14),
        "IBM EGA43 991" => Some(IBM_CP991_F08),*/

        "Amiga Topaz 1" => Some(AMIGA_TOPAZ_1),
        "Amiga Topaz 1+" => Some(AMIGA_TOPAZ_1P),
        "Amiga Topaz 2" => Some(AMIGA_TOPAZ_2),
        "Amiga Topaz 2+" => Some(AMIGA_TOPAZ_2P),
        "Amiga P0T-NOoDLE" => Some(AMIGA_P0T_NOODLE),
        "Amiga MicroKnight" => Some(AMIGA_MICROKNIGHT),
        "Amiga MicroKnight+" => Some(AMIGA_MICROKNIGHTP),
        "Amiga mOsOul" => Some(AMIGA_MOSOUL),

        "C64 PETSCII unshifted" => Some(C64_PETSCII_SHIFTED),
        "C64 PETSCII shifted" => Some(C64_PETSCII_UNSHIFTED),

        "Atari ATASCII" => Some(ATARI_ATASCII),
        _ => None
    }
}