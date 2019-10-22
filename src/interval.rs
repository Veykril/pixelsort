use image::GenericImageView;

use std::ops::{Bound, Range, RangeBounds};

#[derive(Debug)]
pub struct IntervalSet(Vec<Range<usize>>);

impl IntervalSet {
    pub unsafe fn from_vec(vec: Vec<Range<usize>>) -> Self {
        IntervalSet(vec)
    }

    pub fn new(size: usize) -> Self {
        IntervalSet(vec![0..size])
    }

    pub fn intervals_from_image<I: GenericImageView>(image: &I) -> Vec<IntervalSet> {
        (0..image.height())
            .map(|_| IntervalSet::new(image.width() as usize))
            .collect()
    }

    pub fn pop_index(&mut self, idx: usize) -> Option<Range<usize>> {
        if idx < self.0.len() {
            Some(self.0.remove(idx))
        } else {
            None
        }
    }

    // FIXME: wont work for ranges whose start and end point are inside of
    // self.full_range but which arent inside of any of the contained ranges
    pub fn remove_range<R: RangeBounds<usize>>(&mut self, range: R) {
        let start = match range.start_bound() {
            Bound::Unbounded => self.start(),
            Bound::Included(&x) => x,
            Bound::Excluded(x) => x + 1,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => self.end(),
            Bound::Included(x) => x + 1,
            Bound::Excluded(&x) => x,
        };
        if end <= start {
            return;
        }
        let remove_range = match (self.split_at(start), self.split_at(end)) {
            (Some((_, pos)), Some((_, end_pos))) => pos..end_pos,
            (Some((_, pos)), None) => pos..self.0.len(),
            (None, Some((_, pos))) => 0..pos,
            // FIXME: this case is the problem
            (None, None) => {
                let full_range = self.full_range();
                if full_range.start < start && end < full_range.end {
                    // range encloses the set
                    0..self.0.len()
                } else {
                    // range is outside the set
                    return;
                }
            }
        };
        self.0.drain(remove_range);
    }

    // returns the index of the right split off part
    pub fn split_at(&mut self, at: usize) -> Option<(usize, usize)> {
        let (idx, to_split) = self
            .0
            .iter_mut()
            .enumerate()
            .filter(|(_, range)| range.start <= at)
            .last()
            .filter(|(_, range)| at < range.end)?;
        if to_split.start != at {
            let end = std::mem::replace(&mut to_split.end, at);
            self.0.insert(idx + 1, at..end);
            Some((idx, idx + 1))
        } else {
            Some((idx, idx))
        }
    }

    pub fn end(&self) -> usize {
        self.0.last().map(|r| r.end).unwrap_or(0)
    }

    pub fn start(&self) -> usize {
        self.0.first().map(|r| r.start).unwrap_or(0)
    }

    pub fn full_range(&self) -> Range<usize> {
        self.start()..self.end()
    }

    pub fn iter<'this>(&'this self) -> impl Iterator<Item = Range<usize>> + 'this {
        self.0.iter().cloned()
    }
}

pub fn mask(intervals: &mut [IntervalSet], mask: &image::GrayImage) {
    for (row, set) in mask.rows().zip(intervals) {
        let mut pixels = row.enumerate();
        while let Some((last_white, _)) = pixels.find(|(_, pixel)| **pixel == image::Luma([255])) {
            set.split_at(last_white);
            let first_white =
                if let Some((pos, _)) = pixels.find(|(_, pixel)| **pixel == image::Luma([0])) {
                    pos
                } else {
                    if let Some((_, to_remove)) = set.split_at(last_white) {
                        set.pop_index(to_remove);
                    }
                    break;
                };
            if let Some((to_remove, _)) = set.split_at(first_white) {
                set.pop_index(to_remove);
            }
        }
    }
}

#[cfg(feature = "rand")]
pub fn random(intervals: &mut [IntervalSet], lower: usize, upper: usize) {
    use rand::Rng;
    for set in intervals {
        let width = set.end();
        let mut acc = 0;
        while acc < width {
            acc += rand::thread_rng().gen_range(lower, upper);
            set.split_at(acc);
        }
    }
}

pub fn threshold<P, I>(intervals: &mut [IntervalSet], image: &I, low: u8, high: u8)
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

pub fn split_equal(intervals: &mut [IntervalSet], part_count: usize) {
    if let Some(width) = intervals.len().checked_div(part_count) {
        for set in intervals {
            for id in 0..part_count {
                set.split_at(id * width);
            }
        }
    }
}

#[cfg(feature = "imageproc")]
pub fn edges_canny<P, I>(
    intervals: &mut [IntervalSet],
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
