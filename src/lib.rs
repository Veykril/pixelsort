use image::{GenericImage, GenericImageView, Pixel};

pub mod sorting;

pub mod interval;
use self::interval::IntervalSet;

pub fn sort_image<I, P, SF>(image: &mut I, intervals: Vec<IntervalSet>, sorting_function: SF)
where
    I: GenericImage + GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = u8>,
    SF: FnMut(&P) -> u32 + Clone,
{
    // allocate buffer outside to prevent frequent reallocations
    let mut scratch = Vec::new();
    for (row, set) in intervals
        .into_iter()
        .enumerate()
        .take(image.height() as usize)
    {
        for range in set.iter() {
            let mut sub = image.sub_image(
                range.start as u32,
                row as u32,
                range.end as u32 - range.start as u32,
                1,
            );
            scratch.extend(sub.pixels().map(|(_, _, pixel)| pixel));
            scratch.sort_by_key(sorting_function.clone());
            for (x, pixel) in scratch.drain(..).enumerate() {
                // SAFETY: if we were to put a pixel outside of its bounds we would've panicked at the pixels() collection
                unsafe { sub.unsafe_put_pixel(x as u32, 0, pixel) };
                //unsafe { sub.unsafe_put_pixel(x as u32, 0, Pixel::from_channels(255, 0, 0, 255)) };
            }
        }
    }
}
