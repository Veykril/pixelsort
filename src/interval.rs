use image::GenericImageView;

use std::ops::{Bound, Range};

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

    pub fn pop_at(&mut self, idx: usize) -> Option<Range<usize>> {
        if idx < self.0.len() {
            Some(self.0.remove(idx))
        } else {
            None
        }
    }

    pub fn remove_range<R: std::ops::RangeBounds<usize>>(&mut self, range: R) {
        let start = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(&x) => x,
            Bound::Excluded(x) => x + 1,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => self.0.last().map(|range| range.end).unwrap_or(0),
            Bound::Included(x) => x + 1,
            Bound::Excluded(&x) => x,
        };
        if end <= start {
            return;
        }
        if let Some(left) = self
            .split(start)
            .map(|(_, pos)| pos)
            .or_else(|| self.0.iter().position(|range| range.start > start))
        {
            if let Some(right) = self.split(end).map(|(_, pos)| pos).or_else(|| {
                self.0
                    .iter()
                    .rev()
                    .position(|range| range.end < end)
                    .map(|pos| self.0.len() - pos)
            }) {
                self.0.drain(left..right).for_each(|_| ());
            }
        }
    }

    // returns the index of the right split off part
    pub fn split(&mut self, at: usize) -> Option<(usize, usize)> {
        let (idx, to_split) = self
            .0
            .iter_mut()
            .enumerate()
            .find(|(_, range)| range.contains(&at))?;
        let (first, second) = (to_split.start..at, at..to_split.end);
        // do not insert empty ranges
        if first.start != first.end {
            *to_split = first;
            if second.start != second.end {
                self.0.insert(idx + 1, second);
                return Some((idx, idx + 1));
            }
        } else if second.start != second.end {
            *to_split = second;
        }
        Some((idx, idx))
    }

    pub fn end(&self) -> usize {
        self.0.last().map(|r| r.end).unwrap_or(0)
    }

    pub fn iter<'this>(&'this self) -> impl Iterator<Item = Range<usize>> + 'this {
        self.0.iter().cloned()
    }
}

pub fn mask(intervals: &mut Vec<IntervalSet>, mask: &image::GrayImage) {
    for (row, set) in mask.rows().zip(intervals) {
        let mut pixels = row.enumerate();
        while let Some((last_white, _)) = pixels.find(|(_, pixel)| **pixel == image::Luma([255])) {
            set.split(last_white);
            let first_white =
                if let Some((pos, _)) = pixels.find(|(_, pixel)| **pixel == image::Luma([0])) {
                    pos
                } else {
                    if let Some((_, to_remove)) = set.split(last_white) {
                        set.pop_at(to_remove);
                    }
                    break;
                };
            if let Some((to_remove, _)) = set.split(first_white) {
                set.pop_at(to_remove);
            }
        }
    }
}

#[cfg(feature = "rand")]
pub fn random(intervals: &mut Vec<IntervalSet>, lower: usize, upper: usize) {
    use rand::Rng;
    for set in intervals {
        let width = set.end();
        let mut acc = 0;
        while acc < width {
            acc += rand::thread_rng().gen_range(lower, upper);
            set.split(acc);
        }
    }
}

pub fn threshold<P, I>(intervals: &mut Vec<IntervalSet>, image: &I, low: u8, high: u8)
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

pub fn split_equal(intervals: &mut Vec<IntervalSet>, part_count: usize) {
    let width = intervals.len() / part_count;
    for set in intervals {
        for id in 0..part_count {
            set.split(id * width);
        }
    }
}

#[cfg(feature = "imageproc")]
pub fn edges_canny<P, I>(
    intervals: &mut Vec<IntervalSet>,
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
