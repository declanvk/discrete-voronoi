use grid::{BoundingBox, Cell, Grid, GridIdx};
use metric::{Euclidean, Metric};
use site::Site;

use std::marker::PhantomData;

#[derive(Debug)]
pub struct VoronoiBuilder<S, M>
where
    S: Site,
    M: Metric
{
    sites: Vec<S>,
    metric: PhantomData<M>,
    bounds: Option<BoundingBox>
}

impl<S> VoronoiBuilder<S, Euclidean>
where
    S: Site
{
    // Will remove all sites that have the same coordinates
    pub fn new(mut sites: Vec<S>) -> Self {
        sites.sort_unstable_by_key(|site| site.coordinates());
        sites.dedup_by_key(|site| site.coordinates());
        VoronoiBuilder {
            sites,
            metric: PhantomData,
            bounds: None
        }
    }
}

impl<S, M> VoronoiBuilder<S, M>
where
    S: Site,
    M: Metric
{
    pub fn metric<E: Metric>(self) -> VoronoiBuilder<S, E> {
        VoronoiBuilder {
            metric: PhantomData,
            sites: self.sites,
            bounds: self.bounds
        }
    }

    pub fn bounds(mut self, bounds: BoundingBox) -> Self {
        self.bounds = Some(bounds);

        self
    }

    pub fn build(self) -> VoronoiTesselation<S, M> {
        let bounds = if let Some(value) = self.bounds {
            value
        } else {
            BoundingBox::fit_to_sites(&self.sites)
        };

        let num_sites = self.sites.len();
        let sites_id_pars = self.sites
            .into_iter()
            .filter(|site| {
                let idx = GridIdx::from(site.coordinates());

                idx.inside(&bounds)
            })
            .zip(0..(num_sites as u32));
        let wrapped_sites = sites_id_pars
            .map(|(site, id)| SiteWrapper::new(id, site))
            .collect::<Vec<_>>();

        let mut tesselation = VoronoiTesselation {
            sites: wrapped_sites,
            metric: PhantomData,
            grid: Grid::new(bounds)
        };

        tesselation.init_sites();

        tesselation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiteOwner(pub u32);

#[derive(Debug)]
struct SiteWrapper<S>
where
    S: Site
{
    id: SiteOwner,
    newly_claimed: Vec<GridIdx>,
    boundary_chain: Vec<GridIdx>,
    site: S
}

impl<S> SiteWrapper<S>
where
    S: Site
{
    fn new(id: u32, site: S) -> Self {
        SiteWrapper {
            id: SiteOwner(id),
            site,
            newly_claimed: Vec::new(),
            boundary_chain: Vec::new()
        }
    }

    fn update_boundary_chain(&mut self, bounds: &BoundingBox) {
        for idx in &self.newly_claimed {
            self.boundary_chain.extend(idx.neighbors(bounds));
        }
    }
}

pub struct VoronoiTesselation<S, M>
where
    S: Site,
    M: Metric
{
    sites: Vec<SiteWrapper<S>>,
    metric: PhantomData<M>,
    grid: Grid
}

impl<S, M> VoronoiTesselation<S, M>
where
    S: Site,
    M: Metric
{
    pub fn sites(&self) -> Vec<&S> {
        self.sites.iter().map(|s| &s.site).collect()
    }

    pub fn bounds(&self) -> &BoundingBox {
        self.grid.bounds()
    }

    pub fn init_sites(&mut self) {
        for site_wrapper in self.sites.iter_mut() {
            let mut to_claim = vec![GridIdx::from(site_wrapper.site.coordinates())];
            let (claimed, contested) = self.grid.claim_cells(&to_claim, site_wrapper.id);

            debug_assert_eq!(claimed.len(), 1);
            debug_assert!(contested.is_empty());

            site_wrapper.newly_claimed.append(&mut to_claim);
        }
    }

    pub fn compute(&mut self) {
        while self.sum_newly_claimed() > 0 {
            self.step();
        }
    }

    pub fn step(&mut self) {
        for site_wrapper_idx in 0..self.sites.len() {
            let ref mut site_wrapper = self.sites[site_wrapper_idx];

            site_wrapper.boundary_chain.clear();
            site_wrapper.update_boundary_chain(self.grid.bounds());

            site_wrapper.newly_claimed.clear();
            let (mut claimed, contested) = self.grid
                .claim_cells(&site_wrapper.boundary_chain, site_wrapper.id);

            site_wrapper.newly_claimed.append(&mut claimed);

            let mut claimed_won = VoronoiTesselation::<S, M>::handle_conflicts(
                &mut self.sites,
                site_wrapper_idx,
                contested,
                &mut self.grid
            );

            self.sites[site_wrapper_idx]
                .newly_claimed
                .append(&mut claimed_won);
        }
    }

    fn handle_conflicts(
        sites: &mut Vec<SiteWrapper<S>>,
        owner_idx: usize,
        contested: Vec<(GridIdx, SiteOwner)>,
        grid: &mut Grid
    ) -> Vec<GridIdx> {
        let mut claimed = Vec::new();
        for (idx, old_owner) in contested.into_iter() {
            let our_distance = M::distance(&sites[owner_idx].site, &idx);
            let their_distance = M::distance(&sites[old_owner.0 as usize].site, &idx);

            if their_distance > our_distance {
                claimed.push(idx);
                grid[idx].set_owner(sites[owner_idx].id);
            // } else if their_distance == our_distance {

            } else {
                grid[idx].set_owner(old_owner)
            }
        }

        claimed
    }

    fn sum_newly_claimed(&self) -> usize {
        self.sites
            .iter()
            .map(|site_wrapper| site_wrapper.newly_claimed.len())
            .sum()
    }

    pub fn into_buffer<F, T>(self, mut map: F) -> Vec<T>
    where
        F: FnMut(&Cell, Option<&S>) -> T
    {
        let sites = self.sites;
        self.grid
            .into_raw()
            .into_iter()
            .map(|cell| match cell.owner() {
                &Some(owner) => map(cell, Some(&sites[owner.0 as usize].site)),
                &None => map(cell, None)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use metric::MultWeightedEuclidean;

    #[test]
    fn build_voronoi_tesselation() {
        let sites: Vec<(isize, isize, f32)> = vec![
            (1, 1, 1f32),
            (2, 2, 2f32),
            (3, 3, 3f32),
            (4, 4, 4f32),
            (5, 5, 5f32),
            (6, 6, 6f32),
        ];

        let tess = VoronoiBuilder::new(sites).build();

        assert_eq!(tess.sites().len(), 6);
    }

    #[test]
    fn build_voronoi_clip_sites() {
        let sites: Vec<(isize, isize, f32)> = vec![
            (1, 1, 1f32),
            (2, 2, 2f32),
            (3, 3, 3f32),
            (4, 4, 4f32),
            (5, 5, 5f32),
            (6, 6, 6f32),
        ];

        let builder = VoronoiBuilder::new(sites).bounds(BoundingBox::new(2, 2, 3, 3));

        let tess = builder.build();

        assert_eq!(tess.sites().len(), 3);
    }

    #[test]
    fn compute_discrete_voronoi() {
        let sites: Vec<(isize, isize, f32)> = vec![
            (0, 0, 1f32),
            (1, 1, 1f32),
            (2, 2, 1f32),
            (3, 3, 1f32),
            (4, 4, 1f32),
            (5, 5, 1f32),
            (6, 6, 1f32),
        ];

        let mut tess = VoronoiBuilder::new(sites).build();

        tess.compute();
    }

    #[test]
    fn compute_large_bounding_box_voronoi() {
        let sites: Vec<(isize, isize, f32)> = vec![(2, 4, 8f32), (9, 11, 1f32), (4, 9, 8f32), (9, 4, 1f32)];

        let mut tess = VoronoiBuilder::new(sites)
            .metric::<MultWeightedEuclidean>()
            .bounds(BoundingBox::new(0, 0, 14, 14))
            .build();

        tess.compute();
    }

}
