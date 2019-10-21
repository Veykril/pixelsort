use clap::{App, Arg};
use image::imageops;

use std::path::{Path, PathBuf};
use std::str;

use pixelsort::interval::{self, IntervalSet};
use pixelsort::sorting;

#[derive(Clone, Copy)]
pub enum SortingMode {
    Lightness,
    Intensity,
    Minimum,
    Maximum,
}

impl SortingMode {
    pub fn function<P>(self) -> fn(&P) -> u32
    where
        P: image::Pixel<Subpixel = u8>,
    {
        match self {
            SortingMode::Lightness => sorting::lightness,
            SortingMode::Intensity => sorting::intensity,
            SortingMode::Minimum => sorting::chan_max,
            SortingMode::Maximum => sorting::chan_min,
        }
    }
}

impl str::FromStr for SortingMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lightness" => Ok(SortingMode::Lightness),
            "intensity" => Ok(SortingMode::Intensity),
            "minimum" => Ok(SortingMode::Minimum),
            "maximum" => Ok(SortingMode::Maximum),
            _ => Err(String::from(s)),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Rotation {
    Zero,
    Quarter,
    Half,
    NegQuarter,
}

impl str::FromStr for Rotation {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num = s
            .parse::<isize>()
            .map_err(|e| format!("{:?}", e))?
            .rem_euclid(360);
        match num {
            0 => Ok(Rotation::Zero),
            90 => Ok(Rotation::Quarter),
            180 => Ok(Rotation::Half),
            270 => Ok(Rotation::NegQuarter),
            _ => Err(String::from("rotation angle must be a multiple of 90")),
        }
    }
}

#[derive(Clone, Copy)]
pub enum IntervalFunction {
    Full,
    #[cfg(feature = "imageproc")]
    Edges,
    #[cfg(feature = "rand")]
    Random,
    Threshold,
    SplitEqual,
}

impl str::FromStr for IntervalFunction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "full" => Ok(IntervalFunction::Full),
            #[cfg(feature = "imageproc")]
            "edge" => Ok(IntervalFunction::Edges),
            #[cfg(feature = "rand")]
            "random" => Ok(IntervalFunction::Random),
            "threshold" => Ok(IntervalFunction::Threshold),
            "split" => Ok(IntervalFunction::SplitEqual),
            _ => Err(String::from(s)),
        }
    }
}

// FIXME: clean this mess up
fn main() {
    use std::str::FromStr;
    let matches = App::new("pixelsort")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .arg(
            Arg::with_name("input")
                .help("The input image to sort.")
                .required(true)
                .takes_value(true),
        )
        .args(&[
            arg_interval(),
            arg_output(),
            arg_mask(),
            arg_upper(),
            arg_lower(),
            arg_rotation(),
            arg_num(),
            arg_sorting(),
        ])
        .get_matches();
    let input = Path::new(matches.value_of_os("input").unwrap());
    let mut image = image::open(input)
        .expect("failed to read input image")
        .to_rgba();
    let output = matches
        .value_of_os("output")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let extension = input
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("png");
            input.with_extension(["sorted", ".", extension].concat())
        });
    let rotate = Rotation::from_str(matches.value_of("rotation").unwrap()).unwrap();

    //rotate
    match rotate {
        Rotation::Quarter => image = imageops::rotate90(&image),
        Rotation::Half => image = imageops::rotate180(&image),
        Rotation::NegQuarter => image = imageops::rotate270(&image),
        Rotation::Zero => (),
    }
    let sorting_func = SortingMode::from_str(matches.value_of("sorting").unwrap())
        .unwrap()
        .function();
    let interval_func =
        IntervalFunction::from_str(matches.value_of("interval_func").unwrap()).unwrap();

    let mut intervals = IntervalSet::intervals_from_image(&image);
    if let Some(mask_path) = matches.value_of_os("mask").map(Path::new) {
        let mut mask = image::open(mask_path).unwrap().to_luma();
        match rotate {
            Rotation::Quarter => mask = imageops::rotate90(&mask),
            Rotation::Half => mask = imageops::rotate180(&mask),
            Rotation::NegQuarter => mask = imageops::rotate270(&mask),
            Rotation::Zero => (),
        }
        interval::mask(&mut intervals, &mask);
    }

    let upper = matches.value_of("upper").unwrap_or_default();
    let lower = matches.value_of("lower").unwrap_or_default();

    match interval_func {
        IntervalFunction::Full => (),
        IntervalFunction::SplitEqual => interval::split_equal(
            &mut intervals,
            matches
                .value_of("num")
                .unwrap()
                .parse()
                .expect("num was not an integer"),
        ),
        #[cfg(feature = "imageproc")]
        IntervalFunction::Edges => interval::edges_canny(
            &mut intervals,
            &image,
            lower.parse().expect("lower was not an float"),
            upper.parse().expect("upper was not an float"),
        ),
        #[cfg(feature = "rand")]
        IntervalFunction::Random => interval::random(
            &mut intervals,
            lower.parse().expect("lower was not an integer"),
            upper.parse().expect("upper was not an integer"),
        ),
        IntervalFunction::Threshold => interval::threshold(
            &mut intervals,
            &image,
            lower.parse().expect("lower was not a byte integer"),
            upper.parse().expect("upper was not a byte integer"),
        ),
    };
    pixelsort::sort_image(&mut image, intervals, sorting_func);
    // rotate back
    match rotate {
        Rotation::Quarter => image = imageops::rotate270(&image),
        Rotation::Half => image = imageops::rotate180(&image),
        Rotation::NegQuarter => image = imageops::rotate90(&image),
        Rotation::Zero => (),
    }
    image.save(&output).unwrap();
}

fn arg_sorting() -> Arg<'static, 'static> {
    Arg::with_name("sorting")
        .short("s")
        .long("sorting")
        .help("The function to use for sorting pixels.")
        .long_help(
            "The function to use for sorting pixels.\n\
                \n\
                This mode defines how pixels are sorted, be it by lightness, intensity or min/maxmimum channel value of each pixel.",
        )
        .default_value("lightness")
        .takes_value(true)
}

fn arg_num() -> Arg<'static, 'static> {
    Arg::with_name("num")
        .short("n")
        .long("num")
        .help("The number of parts to split the intervals into.")
        .long_help(
            "The number of parts to split the intervals into.\n\
             \n\
             Required by interval function `split`, splits the file into even intervals.",
        )
        .required_if("interval_func", "split")
        .takes_value(true)
}

fn arg_rotation() -> Arg<'static, 'static> {
    Arg::with_name("rotation")
        .short("r")
        .long("rotation")
        .help("The rotation to apply to the image prior sorting.")
        .long_help(
            "The rotation to apply to the image(and mask) prior sorting.\n\
                \n\
                This value defines the angle at which pixels will be sorted. This may be any multiple of 90 degrees.\n\
                To sort vertically instead of horizontally for example one would specifiy a rotation of 90 or 270 degrees.",
        )
        .default_value("0")
        .takes_value(true)
}

fn arg_upper() -> Arg<'static, 'static> {
    Arg::with_name("upper")
        .short("u")
        .long("upper")
        .help("The upper threshold used by some interval functions.")
        .long_help(
            "The upper threshold used by some interval functions.\n\
                \n\
                Required by `edge` in the range of [0.0;1140.39), accepts floating point numbers.\n\
                Required by `random`, defines the maximum possible size of the random intervals in integers.\n\
                Required by `threshold`, defines the upper threshold a pixels lightness has to fall below to be sorted.",
        )
        .required_ifs(&[("interval_func", "edges"), ("interval_func", "threshold"), ("interval_func", "random")])
        .takes_value(true)
}

fn arg_lower() -> Arg<'static, 'static> {
    Arg::with_name("lower")
        .short("l")
        .long("lower")
        .help("The lower threshold used by some interval functions.")
        .long_help(
            "The lower threshold used by some interval functions.\n\
                \n\
                Required by `edge` in the range of [0.0;1140.39), accepts floating point numbers.\n\
                Required by `random`, defines the minimum possible size of the random intervals in integers.\n\
                Required by `threshold`, defines the lower threshold a pixels lightness has to surpass to be sorted.",
        )
        .required_ifs(&[("interval_func", "edges"), ("interval_func", "threshold"), ("interval_func", "random")])
        .takes_value(true)
}

fn arg_mask() -> Arg<'static, 'static> {
    Arg::with_name("mask")
        .short("m")
        .long("mask")
        .help("A file path to a gray image to mask parts of the input image.")
        .long_help(
            "A file path to a gray image to mask parts of the input image.\n\
             White pixels may be sorted, black pixels may not.",
        )
        .takes_value(true)
}

fn arg_output() -> Arg<'static, 'static> {
    Arg::with_name("output")
        .short("o")
        .long("output")
        .help("A file path to save the output image to.")
        .takes_value(true)
}

fn arg_interval() -> Arg<'static, 'static> {
    Arg::with_name("interval_func")
        .short("i")
        .long("interval")
        .help("Interval function used to seperate the image into intervals.")
        .possible_values(&["full", "edge", "random", "split", "threshold"])
        .default_value("full")
        .takes_value(true)
}
