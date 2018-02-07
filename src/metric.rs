use site::{Point, Site};

type OR = f32;
type IR = f64;

pub trait Metric {
    type Output;
    fn distance<S, X>(a: &S, b: &X) -> Self::Output
    where
        S: Site,
        X: Point;
}

pub struct Euclidean;

impl Euclidean {
    fn magnitude<A, B>(a: &A, b: &B) -> IR
    where
        A: Point,
        B: Point,
    {
        let (a_x, a_y) = a.coordinates();
        let (b_x, b_y) = b.coordinates();

        let mag_x = (a_x as IR - b_x as IR)
            * (a_x as IR - b_x as IR);
        let mag_y = (a_y as IR - b_y as IR)
            * (a_y as IR - b_y as IR);

        mag_x + mag_y
    }
}

impl Metric for Euclidean {
    type Output = OR;

    fn distance<S, X>(a: &S, b: &X) -> Self::Output
    where
        S: Site,
        X: Point,
    {
        Euclidean::magnitude(a, b).sqrt() as Self::Output
    }
}

pub struct MultWeightedEuclidean;

impl Metric for MultWeightedEuclidean {
    type Output = OR;

    fn distance<S, X>(a: &S, b: &X) -> Self::Output
    where
        S: Site,
        X: Point,
    {
        (1 as OR / a.weight()) * Euclidean::distance(a, b)
    }
}

pub struct AdditiveWeightedEuclidean;

impl Metric for AdditiveWeightedEuclidean {
    type Output = OR;

    fn distance<S, X>(a: &S, b: &X) -> Self::Output
    where
        S: Site,
        X: Point,
    {
        Euclidean::distance(a, b) - a.weight()
    }
}

pub struct PowerEuclidean;

impl Metric for PowerEuclidean {
    type Output = OR;

    fn distance<S, X>(a: &S, b: &X) -> Self::Output
    where
        S: Site,
        X: Point,
    {
        Euclidean::magnitude(a, b) as Self::Output
    }
}

pub struct Manhattan;

impl Metric for Manhattan {
    type Output = OR;

    fn distance<S, X>(a: &S, b: &X) -> Self::Output
    where
        S: Site,
        X: Point,
    {
        let (a_x, a_y) = a.coordinates();
        let (b_x, b_y) = b.coordinates();

        let mag_x = (a_x as IR - b_x as IR).abs();
        let mag_y = (a_y as IR - b_y as IR).abs();
        let magnitude = mag_x + mag_y;

        magnitude as Self::Output
    }
}
