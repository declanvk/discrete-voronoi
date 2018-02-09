extern crate discrete_voronoi;
extern crate image;
extern crate rand;

use std::env;

use rand::distributions::{IndependentSample, Range};
use rand::thread_rng;

use discrete_voronoi::{BoundingBox, Point, Site, VoronoiBuilder, VoronoiTesselation};
use discrete_voronoi::metric::{Manhattan, Metric};

use image::GrayImage;

#[derive(Debug)]
struct ImageSite {
    x: isize,
    y: isize,
    id: usize,
    weight: f32
}

impl Point for ImageSite {
    fn coordinates(&self) -> (isize, isize) {
        (self.x, self.y)
    }
}

impl Site for ImageSite {
    fn weight(&self) -> f32 {
        self.weight
    }
}

fn main() {
    let (width, height, num_sites, num_steps, output_path) = parse_arguments();
    let sites = generate_sites(width, height, num_sites);

    let bounding_box = BoundingBox::new(0, 0, width, height);
    let mut tess = VoronoiBuilder::new(sites)
        .bounds(bounding_box)
        .metric::<Manhattan>()
        .build();

    if let Some(num_steps) = num_steps {
        for _ in 0..num_steps {
            tess.step();
        }
    } else {
        tess.compute();
    }

    let image = save_image(tess);

    image
        .expect("Image generation failed")
        .save(output_path)
        .expect("Image save failed");
}

fn parse_arguments() -> (usize, usize, usize, Option<usize>, String) {
    let mut args: Vec<String> = env::args().collect();

    let (width, height, num_sites): (usize, usize, usize) = match args.len() {
        x if x != 6 => panic!("usage: ./generate_image width height num_sites num_steps output_path"),
        _ => match (args[1].parse(), args[2].parse(), args[3].parse()) {
            (Ok(width), Ok(height), Ok(num_sites)) => (width, height, num_sites),
            _ => panic!("Pass non-negative integer values for width and height.")
        }
    };

    let num_steps: Option<usize> = args[4].parse().ok();

    (width, height, num_sites, num_steps, args.remove(5))
}

fn generate_sites(width: usize, height: usize, num_sites: usize) -> Vec<ImageSite> {
    let mut rng = thread_rng();
    let x_range = Range::new(0, width as isize);
    let y_range = Range::new(0, height as isize);

    let mut sites = Vec::new();
    for id in 0..num_sites {
        let x = x_range.ind_sample(&mut rng);
        let y = y_range.ind_sample(&mut rng);
        sites.push(ImageSite {
            x,
            y,
            id,
            weight: 1f32
        });
    }

    sites
}

fn save_image<M>(tesselation: VoronoiTesselation<ImageSite, M>) -> Option<GrayImage>
where
    M: Metric
{
    let (width, height) = tesselation.bounds().dimensions();

    let pixels = tesselation.into_buffer(|_, possible_site| match possible_site {
        None => 255,
        Some(site) => {
            let y = site.y as f32;
            let x = site.x as f32;
            let height = height as f32;
            let width = width as f32;
            let r = (width.powi(2) + height.powi(2)).sqrt() / 7f32;

            let value = (x - width / 2f32).powi(2) / r + (y - height / 2f32).powi(2) / r;

            (value as usize % 255) as u8
        }
    });

    GrayImage::from_vec(width as u32, height as u32, pixels)
}
