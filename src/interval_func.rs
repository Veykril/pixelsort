use image::GenericImageView;

use inversion_list::InversionList;

pub fn mask(intervals: &mut [InversionList], mask: &image::GrayImage) {
    for (row, set) in mask.rows().zip(intervals) {
        let mut pixels = row.enumerate();
        while let Some((last_white, _)) = pixels.find(|(_, pixel)| **pixel == image::Luma([255])) {
            set.split(last_white);
            let first_white =
                if let Some((pos, _)) = pixels.find(|(_, pixel)| **pixel == image::Luma([0])) {
                    pos
                } else {
                    if let Some((_, to_remove)) = set.split(last_white) {
                        set.remove_range_at(to_remove);
                    }
                    break;
                };
            if let Some((to_remove, _)) = set.split(first_white) {
                set.remove_range_at(to_remove);
            }
        }
    }
}

#[cfg(feature = "rand")]
pub fn random(intervals: &mut [InversionList], lower: usize, upper: usize) {
    use rand::Rng;
    for set in intervals {
        let width = set.end().unwrap();
        let mut acc = 0;
        while acc < width {
            acc += rand::thread_rng().gen_range(lower, upper);
            set.split(acc);
        }
    }
}

pub fn threshold<P, I>(intervals: &mut [InversionList], image: &I, low: u8, high: u8)
where
    P: image::Pixel<Subpixel = u8>,
    I: GenericImageView<Pixel = P>,
{
    let mut gray = image::imageops::colorops::grayscale(image);
    for pixel in gray.pixels_mut() {
        if (low..high).contains(&pixel.0[0]) {
            *pixel = image::Luma([255]);
        } else {
            *pixel = image::Luma([0]);
        }
    }
    mask(intervals, &gray);
}

pub fn split_equal(intervals: &mut [InversionList], part_count: usize) {
    if let Some(width) = intervals.len().checked_div(part_count) {
        for set in intervals {
            for id in 0..part_count {
                set.split(id * width);
            }
        }
    }
}

#[cfg(feature = "imageproc")]
pub fn edges_canny<P, I>(
    intervals: &mut [InversionList],
    image: &I,
    low_thresh: f32,
    high_thresh: f32,
) where
    P: image::Pixel<Subpixel = u8>,
    I: GenericImageView<Pixel = P>,
{
    let gray = image::imageops::colorops::grayscale(image);
    let edges = imageproc::edges::canny(&gray, low_thresh, high_thresh);
    mask(intervals, &edges);
}
