use std::fmt;
use std::ops::{Add, Mul};

use image::{GenericImage, ImageBuffer};

#[derive(Clone, Copy, PartialEq, Debug)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }

    /// Exponentiate `self` by multiplying 'exp' times
    pub fn pow(self, exp: u32) -> Self {
        (0..exp).fold(Complex::new(1.0, 0.0), |acc, _| acc * self)
    }

    pub const MAX_ITER: u16 = 1000;

    // Returns `None` if stable, otherwise `Some(iter)` if it diverges after `iter` iterations.
    fn stability(self) -> Option<u16> {
        let mut num = Complex::new(0.0, 0.0);

        for i in 0..Self::MAX_ITER {
            num = num.pow(2) + self;

            if num.re * num.re + num.im * num.im > 2.0 * 2.0 {
                return Some(i);
            }
        }

        None
    }
}

impl fmt::Display for Complex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.im < 0.0 { "-" } else { "+" };

        write!(
            f,
            "{re} {sign} {im}i",
            re = self.re,
            sign = sign,
            im = self.im.abs()
        )
    }
}

impl Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Complex::new(self.re + rhs.re, self.im + rhs.im)
    }
}

impl Mul<Self> for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let re = self.re * rhs.re - self.im * rhs.im;
        let im = self.re * rhs.im + self.im * rhs.re;
        Complex::new(re, im)
    }
}

fn hue_to_rgb(rad: f64) -> image::Rgb<u8> {
    let third = std::f64::consts::FRAC_PI_3;

    let x = 1.0 - ((rad / third) % 2.0 - 1.0).abs();
    let x8 = (x * 255.0) as u8;

    let color = if rad < third {
        [255, x8, 0]
    } else if rad < third * 2.0 {
        [x8, 255, 0]
    } else if rad < third * 3.0 {
        [0, 255, x8]
    } else if rad < third * 4.0 {
        [0, x8, 255]
    } else {
        [255, 0, x8]
    };

    image::Rgb(color)
}

fn main() {
    let img_height = 1000;
    let img_width = 1000;

    let viewport_height = 4.0;
    let viewport_width = 4.0;

    let zoom = 200.0;

    let portion_size = 1000;

    let top_left = (-0.76, -0.05);

    let mut image = ImageBuffer::new(img_width, img_height);

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(24 - 1)
        .build()
        .expect("build thread pool");

    let (tx, rx) = std::sync::mpsc::channel();

    for column in 0..=(img_width / portion_size) {
        let start_horizontal = column * portion_size;
        let end_horizontal = img_width.min((column + 1) * portion_size);

        for row in 0..=(img_height / portion_size) {
            let start_vertical = row * portion_size;
            let end_vertical = img_height.min((row + 1) * portion_size);

            // Compute an image starting at (startHorizontal, startVertical) and ending at
            // (endHorizontal, endVertical)

            let tx = tx.clone();
            pool.spawn_fifo(move || {
                let part = ImageBuffer::from_fn(
                    end_horizontal - start_horizontal,
                    end_vertical - start_vertical,
                    |x, y| {
                        let true_x = x + start_horizontal;
                        let true_y = y + start_vertical;
                        let re =
                            top_left.0 + (true_x as f64 / img_width as f64) * viewport_width / zoom;
                        let im = top_left.1
                            + (true_y as f64 / img_height as f64) * viewport_height / zoom;

                        let num = Complex::new(re, im);
                        let stability = num.stability();

                        if let Some(iter) = stability {
                            hue_to_rgb(
                                (iter as f64 / Complex::MAX_ITER as f64)
                                    * std::f64::consts::PI
                                    * 2.0,
                            )
                        } else {
                            image::Rgb([0, 0, 0])
                        }
                    },
                );

                println!("Done computing ({}, {})", start_horizontal, start_vertical);
                tx.send((start_horizontal, start_vertical, part))
                    .expect("send image part");
            });
        }
    }

    let total = (img_height / portion_size + 1) * (img_width / portion_size + 1);
    let mut count = 0;

    while let Ok((start_horizontal, start_vertical, part)) =
        rx.recv_timeout(std::time::Duration::from_secs(5))
    {
        count += 1;
        println!(
            "Concatenating ({}, {}) ({}/{})",
            start_horizontal, start_vertical, count, total
        );
        image
            .copy_from(&part, start_horizontal, start_vertical)
            .expect("copy part in image")
    }

    println!("Received all, saving...");

    let file = std::fs::File::create(format!("mandelbrot-{}x{}.png", img_width, img_height))
        .expect("create img file");

    let encoder = image::png::PngEncoder::new_with_quality(
        file,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::Sub,
    );

    encoder
        .encode(
            image.as_raw(),
            img_width,
            img_height,
            image::ColorType::Rgb8,
        )
        .expect("encode image");

    println!("saved!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        let c1 = Complex::new(1.0, 2.0);
        let c2 = Complex::new(2.5, -3.0);

        assert_eq!(c1 + c2, Complex::new(3.5, -1.0));
    }

    #[test]
    fn display() {
        let num = Complex::new(1.5, -3.0);
        assert_eq!(&format!("{}", num), "1.5 - 3i")
    }

    #[test]
    fn mul() {
        let c1 = Complex::new(1.0, -2.0);
        let c2 = Complex::new(2.0, 4.0);
        let res = Complex::new(10.0, 0.0);

        println!("c1: {}, c2: {}", c1, c2);

        assert_eq!(c1 * c2, res);
    }

    #[test]
    fn pow() {
        let c1 = Complex::new(2.0, 2.0);

        assert_eq!(c1.pow(0), Complex::new(1.0, 0.0));
        assert_eq!(c1.pow(1), c1);
        assert_eq!(c1.pow(2), c1 * c1);
        assert_eq!(c1.pow(3), c1 * c1 * c1);
    }
}
