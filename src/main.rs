use image::imageops;
use structopt::StructOpt;

use std::path::PathBuf;
use std::str;

use pixelsort::interval::{self, IntervalSet};
use pixelsort::sorting;

#[derive(StructOpt)]
#[structopt(author)]
struct Opt {
    #[structopt(parse(from_os_str), help = "the input image to sort")]
    input: PathBuf,

    #[structopt(
        short,
        long = "interval",
        default_value = "full",
        help = "Interval function used to seperate the image into intervals.",
        help = "Interval function used to seperate the image into intervals.\n\
                available functions: full, edges, random, threshold"
    )]
    interval_function: IntervalFunction,

    #[structopt(
        short,
        long,
        parse(from_os_str),
        help = "A file path to save the output image to."
    )]
    output: Option<PathBuf>,
    #[structopt(
        short,
        long,
        parse(from_os_str),
        help = "A file path to a gray image to mask parts of the input image.",
        long_help = "A file path to a gray image to mask parts of the input image.\n\
                     White pixels may be sorted, black pixels may not."
    )]
    mask: Option<PathBuf>,

    #[structopt(
        short = "u",
        default_value_if("interval_function", Some("full"), "0.0"),
        help = "The upper threshold used by some interval functions.",
        long_help = "The upper threshold used by some interval functions.\n\
                     \n\
                     Required by `edge` in the range of [0.0;1140.39), accepts floating point numbers.\n\
                     Required by `random`, defines the maximum possible size of the random intervals in integers.\n\
                     Required by `threshold`, defines the upper threshold a pixels lightness has to fall below to be sorted."
    )]
    upper: f32,
    #[structopt(
        short = "l",
        default_value_if("interval_function", Some("full"), "0.0"),
        help = "The lower threshold used by some interval functions.",
        long_help = "The lower threshold used by some interval functions.\n\
                     \n\
                     Required by `edge` in the range of [0.0;1140.39), accepts floating point numbers.\n\
                     Required by `random`, defines the minimum possible size of the random intervals in integers.\n\
                     Required by `threshold`, defines the lower threshold a pixels lightness has to surpass to be sorted."
    )]
    lower: f32,
    #[structopt(
        short,
        long,
        default_value = "0",
        help = "The rotation to apply to the image prior sorting.",
        long_help = "The rotation to apply to the image prior sorting.\n\
                     \n\
                     This value defines the angle at which pixels will be sorted. This may be any multiple of 90 degrees.\n\
                     To sort vertically instead of horizontally for example one would specifiy a rotation of 90 or 270 degrees."
    )]
    rotate: Rotation,
    #[structopt(
        short,
        required_if("interval_function", "split"),
        help = "The number of parts to split the intervals into.",
        long_help = "The number of parts to split the intervals into.\n\
                     \n\
                     Required by interval function `split`, splits the file into even intervals."
    )]
    num: usize,
    #[structopt(
        short,
        long,
        default_value = "lightness",
        help = "The function to use for sorting pixels.",
        long_help = "The function to use for sorting pixels.\n\
                     \n\
                     This mode defines how pixels are sorted, be it by lightness, intensity or min/maxmimum channel value of each pixel."
    )]
    sorting: SortingMode,
}

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

fn main() {
    let mut opt = Opt::from_args();
    let mut image = image::open(&opt.input).unwrap().to_rgba();
    let output = opt.output.take().unwrap_or_else(|| {
        let extension = opt
            .input
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("png");
        opt.input
            .with_extension(["sorted", ".", extension].concat())
    });

    //rotate
    match opt.rotate {
        Rotation::Quarter => image = imageops::rotate90(&image),
        Rotation::Half => image = imageops::rotate180(&image),
        Rotation::NegQuarter => image = imageops::rotate270(&image),
        Rotation::Zero => (),
    }

    let sorting_func = opt.sorting.function();
    let mut intervals = IntervalSet::intervals_from_image(&image);
    if let Some(mask_path) = opt.mask {
        let mut mask = image::open(mask_path).unwrap().to_luma();
        match opt.rotate {
            Rotation::Quarter => mask = imageops::rotate90(&mask),
            Rotation::Half => mask = imageops::rotate180(&mask),
            Rotation::NegQuarter => mask = imageops::rotate270(&mask),
            Rotation::Zero => (),
        }
        interval::mask(&mut intervals, &mask);
    }

    match opt.interval_function {
        IntervalFunction::Full => (),
        IntervalFunction::SplitEqual => interval::split_equal(&mut intervals, opt.num),
        #[cfg(feature = "imageproc")]
        IntervalFunction::Edges => {
            interval::edges_canny(&mut intervals, &image, opt.lower, opt.upper)
        }
        #[cfg(feature = "rand")]
        IntervalFunction::Random => {
            interval::random(&mut intervals, opt.lower as usize, opt.upper as usize)
        }
        IntervalFunction::Threshold => {
            interval::threshold(&mut intervals, &image, opt.lower as u8, opt.upper as u8)
        }
    };
    pixelsort::sort_image(&mut image, intervals, sorting_func);
    // rotate back
    match opt.rotate {
        Rotation::Quarter => image = imageops::rotate270(&image),
        Rotation::Half => image = imageops::rotate180(&image),
        Rotation::NegQuarter => image = imageops::rotate90(&image),
        Rotation::Zero => (),
    }
    image.save(&output).unwrap();
}
