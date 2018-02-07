use discrete_voronoi::SiteOwner;
use site::{Point, Site};
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    height: usize,
    width: usize,
    x_offset: isize,
    y_offset: isize
}

impl BoundingBox {
    pub fn fit_to_sites<S: Site>(sites: &Vec<S>) -> BoundingBox {
        let mut min_x = 0;
        let mut max_x = 0;
        let mut min_y = 0;
        let mut max_y = 0;

        for site in sites {
            let (x, y) = site.coordinates();

            if x > max_x {
                max_x = x;
            }

            if x < min_x {
                min_x = x;
            }

            if y > max_y {
                max_y = y;
            }

            if y < min_y {
                min_y = y;
            }
        }

        let width = (max_x - min_x) as usize;
        let height = (max_y - min_y) as usize;

        let x_offset = min_x;
        let y_offset = min_y;

        BoundingBox {
            height,
            width,
            x_offset,
            y_offset
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridIdx(isize, isize);

impl GridIdx {
    pub fn neighbors<'a>(&'a self, bounds: &'a BoundingBox) -> GridIdxNeighborIter<'a> {
        GridIdxNeighborIter(self, 0, bounds)
    }

    pub fn inside(&self, bounds: &BoundingBox) -> bool {
        let adjusted_x = self.0 - bounds.x_offset;
        let adjusted_y = self.1 - bounds.y_offset;

        0 <= adjusted_x && adjusted_x < bounds.width as isize && 0 <= adjusted_y && adjusted_y <= bounds.height as isize
    }
}

impl Point for GridIdx {
    fn coordinates(&self) -> (isize, isize) {
        (self.0, self.1)
    }
}

impl From<(isize, isize)> for GridIdx {
    fn from(src: (isize, isize)) -> Self {
        GridIdx(src.0, src.1)
    }
}

const MAX_DIRECTION: u8 = 4;
#[derive(Debug)]
pub struct GridIdxNeighborIter<'a>(&'a GridIdx, u8, &'a BoundingBox);

impl<'a> Iterator for GridIdxNeighborIter<'a> {
    type Item = GridIdx;

    fn next(&mut self) -> Option<Self::Item> {
        if self.1 >= MAX_DIRECTION {
            None
        } else {
            let next_output = loop {
                let possible = match self.1 {
                    0 => GridIdx((self.0).0, (self.0).1 + 1), // north
                    1 => GridIdx((self.0).0 + 1, (self.0).1), // east
                    2 => GridIdx((self.0).0, (self.0).1 - 1), // south
                    3 => GridIdx((self.0).0 - 1, (self.0).1), // west
                    _ => unreachable!()
                };

                if possible.inside(self.2) {
                    break possible;
                } else {
                    self.1 += 1;
                }
            };

            Some(next_output)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    bounding: BoundingBox,
    data: Box<[Cell]>
}

impl Grid {
    pub fn new(bounding: BoundingBox) -> Self {
        Grid {
            bounding,
            data: vec![Cell::new(); bounding.width * bounding.height].into_boxed_slice()
        }
    }

    pub fn clear(&mut self) {
        self.data = vec![Cell::new(); self.bounding.width * self.bounding.height].into_boxed_slice();
    }

    pub fn bounding(&self) -> &BoundingBox {
        &self.bounding
    }

    pub fn claim_cells(&mut self, indices: &Vec<GridIdx>, claimee: SiteOwner) -> (Vec<GridIdx>, Vec<(GridIdx, SiteOwner)>) {
        let mut contested = Vec::new();
        let mut claimed = Vec::new();

        for idx in indices {
            match self[*idx] {
                ref mut cell @ Cell {
                    contested: false,
                    owner: None
                } => {
                    cell.owner = Some(claimee);

                    claimed.push(*idx);
                }
                ref mut cell @ Cell {
                    contested: false,
                    owner: Some(_)
                } => {
                    let old_owner = cell.owner;
                    cell.contested = true;
                    cell.owner = None;

                    contested.push((*idx, old_owner.unwrap()));
                }
                Cell {
                    contested: true, ..
                } => {}
            }
        }

        (claimed, contested)
    }
}

impl Index<GridIdx> for Grid {
    type Output = Cell;

    fn index(&self, idx: GridIdx) -> &Self::Output {
        let x = (idx.0 + self.bounding.x_offset) as usize;
        let y = (idx.1 + self.bounding.y_offset) as usize;

        &self.data[x + y * self.bounding.width]
    }
}

impl IndexMut<GridIdx> for Grid {
    fn index_mut(&mut self, idx: GridIdx) -> &mut Self::Output {
        let x = (idx.0 + self.bounding.x_offset) as usize;
        let y = (idx.1 + self.bounding.y_offset) as usize;

        &mut self.data[x + y * self.bounding.width]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    contested: bool,
    owner: Option<SiteOwner>
}

impl Cell {
    fn new() -> Self {
        Cell {
            contested: false,
            owner: None
        }
    }

    pub fn set_owner(&mut self, new_owner: SiteOwner) {
        self.owner = Some(new_owner);
    }
}
