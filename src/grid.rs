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
    pub fn new(x_offset: isize, y_offset: isize, width: usize, height: usize) -> Self {
        BoundingBox {
            x_offset,
            y_offset,
            height,
            width
        }
    }

    pub fn fit_to_sites<S: Site>(sites: &Vec<S>) -> Self {
        assert!(!sites.is_empty(), "Sites must not be empty");
        let mut min_x = isize::max_value();
        let mut max_x = isize::min_value();
        let mut min_y = isize::max_value();
        let mut max_y = isize::min_value();

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

        let width = (max_x - min_x + 1) as usize;
        let height = (max_y - min_y + 1) as usize;

        let x_offset = min_x;
        let y_offset = min_y;

        BoundingBox {
            height,
            width,
            x_offset,
            y_offset
        }
    }

    pub fn translate_idx(&self, idx: GridIdx) -> (usize, usize) {
        let x = (idx.0 - self.x_offset) as usize;
        let y = (idx.1 - self.y_offset) as usize;
        (x, y)
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
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

        0 <= adjusted_x && adjusted_x < bounds.width as isize && 0 <= adjusted_y && adjusted_y < bounds.height as isize
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
            loop {
                let possible = match self.1 {
                    0 => GridIdx((self.0).0, (self.0).1 + 1), // north
                    1 => GridIdx((self.0).0 + 1, (self.0).1), // east
                    2 => GridIdx((self.0).0, (self.0).1 - 1), // south
                    3 => GridIdx((self.0).0 - 1, (self.0).1), // west
                    x if x >= MAX_DIRECTION => break None,
                    _ => unreachable!()
                };

                self.1 += 1;
                if possible.inside(self.2) {
                    break Some(possible);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Grid {
    bounds: BoundingBox,
    data: Box<[Cell]>
}

impl Grid {
    pub fn new(bounds: BoundingBox) -> Self {
        Grid {
            bounds,
            data: vec![Cell::new(); bounds.width * bounds.height].into_boxed_slice()
        }
    }

    pub fn clear(&mut self) {
        self.data = vec![Cell::new(); self.bounds.width * self.bounds.height].into_boxed_slice();
    }

    pub fn bounds(&self) -> &BoundingBox {
        &self.bounds
    }

    pub fn claim_cells(
        &mut self,
        indices: &Vec<GridIdx>,
        claimant: SiteOwner
    ) -> (Vec<GridIdx>, Vec<(GridIdx, SiteOwner)>) {
        let mut contested_cells = Vec::new();
        let mut claimed_cells = Vec::new();

        for idx in indices {
            let ref mut cell = self[*idx];
            let same_owner = cell.owner.map_or(false, |cell| cell == claimant);
            let contested = cell.contested;
            let empty = cell.owner.is_none();

            if !same_owner {
                if !contested && empty {
                    cell.owner = Some(claimant);

                    claimed_cells.push(*idx);
                } else if !empty {
                    let old_owner = cell.owner.take().unwrap();
                    cell.contested = true;

                    contested_cells.push((*idx, old_owner));
                }
            }
        }

        (claimed_cells, contested_cells)
    }

    pub fn into_raw(self) -> Box<[Cell]> {
        self.data
    }
}

impl Index<GridIdx> for Grid {
    type Output = Cell;

    fn index(&self, idx: GridIdx) -> &Self::Output {
        let (x, y) = self.bounds.translate_idx(idx);
        &self.data[x + y * self.bounds.width]
    }
}

impl IndexMut<GridIdx> for Grid {
    fn index_mut(&mut self, idx: GridIdx) -> &mut Self::Output {
        let (x, y) = self.bounds.translate_idx(idx);
        &mut self.data[x + y * self.bounds.width]
    }
}

#[derive(Debug, Clone)]
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

    pub fn owner(&self) -> &Option<SiteOwner> {
        &self.owner
    }

    pub fn contested(&self) -> bool {
        self.contested
    }
}
