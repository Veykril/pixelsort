use image::imageops;
use pixelsort::interval::{self, IntervalSet};
use pixelsort::sorting;
use structopt::StructOpt;

use std::path::PathBuf;
use std::str;

#[derive(StructOpt)]
#[structopt(author)]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf,

    #[structopt(short, default_value = "row")]
    interval_function: IntervalFunction,

    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
    #[structopt(short, long, parse(from_os_str))]
    mask: Option<PathBuf>,

    #[structopt(short = "u", default_value = "1.0")]
    upper: f32,
    #[structopt(short = "l", default_value = "0.0")]
    lower: f32,
    #[structopt(short, long, default_value = "0")]
    rotate: Rotation,
    #[structopt(short, long, default_value = "lightness")]
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
        match s {
            "0" | "360" => Ok(Rotation::Zero),
            "90" => Ok(Rotation::Quarter),
            "180" => Ok(Rotation::Half),
            "270" | "-90" => Ok(Rotation::NegQuarter),
            _ => Err(String::from(s)),
        }
    }
}

#[derive(Clone, Copy)]
pub enum IntervalFunction {
    Row,
    #[cfg(feature = "imageproc")]
    Edges,
    #[cfg(feature = "rand")]
    Random,
    Threshold,
}

impl str::FromStr for IntervalFunction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "row" => Ok(IntervalFunction::Row),
            #[cfg(feature = "imageproc")]
            "edge" => Ok(IntervalFunction::Edges),
            #[cfg(feature = "rand")]
            "random" => Ok(IntervalFunction::Random),
            "threshold" => Ok(IntervalFunction::Threshold),
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
        let mask = image::open(mask_path).unwrap().to_luma();
        interval::mask(&mut intervals, &mask);
    }

    match opt.interval_function {
        IntervalFunction::Row => (),
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
