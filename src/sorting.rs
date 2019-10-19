use image::Pixel;

#[inline]
pub fn lightness<P>(pixel: &P) -> u32
where
    P: Pixel<Subpixel = u8>,
{
    pixel.to_luma()[0] as u32
}

#[inline]
pub fn intensity<P>(pixel: &P) -> u32
where
    P: Pixel<Subpixel = u8>,
{
    pixel.channels().iter().map(|c| *c as u32).sum()
}

#[inline]
pub fn chan_min<P>(pixel: &P) -> u32
where
    P: Pixel<Subpixel = u8>,
{
    pixel.channels().iter().copied().min().unwrap_or(0) as u32
}

#[inline]
pub fn chan_max<P>(pixel: &P) -> u32
where
    P: Pixel<Subpixel = u8>,
{
    pixel.channels().iter().copied().max().unwrap_or(255) as u32
}
