use grid::{BoundingBox, Cell, Grid, GridIdx};
use metric::{Euclidean, Metric};
use site::Site;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use rayon::prelude::*;

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
        let wrapped_sites = sites_id_pars.map(|(site, id)| (SiteOwner(id), SiteWrapper::new(id, site)));

        let mut sites_map = HashMap::with_capacity(num_sites);
        sites_map.extend(wrapped_sites);
        let mut tesselation = VoronoiTesselation {
            sites: sites_map,
            metric: PhantomData,
            grid: Grid::new(bounds)
        };

        tesselation.init_sites();

        tesselation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        self.boundary_chain.par_extend(
            self.newly_claimed
                .par_iter()
                .flat_map(|idx| idx.neighbors(bounds).collect::<Vec<_>>())
        )
    }
}

impl<S> PartialEq for SiteWrapper<S>
where
    S: Site
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<S> Eq for SiteWrapper<S>
where
    S: Site
{
}

impl<S> Hash for SiteWrapper<S>
where
    S: Site
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct VoronoiTesselation<S, M>
where
    S: Site,
    M: Metric
{
    sites: HashMap<SiteOwner, SiteWrapper<S>>,
    metric: PhantomData<M>,
    grid: Grid
}

impl<S, M> VoronoiTesselation<S, M>
where
    S: Site,
    M: Metric
{
    pub fn sites(&self) -> Vec<&S> {
        self.sites
            .iter()
            .map(|(_, wrapper)| &wrapper.site)
            .collect()
    }

    pub fn bounds(&self) -> &BoundingBox {
        self.grid.bounds()
    }

    pub fn init_sites(&mut self) {
        for (_, site_wrapper) in self.sites.iter_mut() {
            let mut to_claim = vec![GridIdx::from(site_wrapper.site.coordinates())];
            let (claimed, contested) = self.grid.claim_cells(&to_claim, site_wrapper.id);

            debug_assert_eq!(claimed.len(), 1);
            debug_assert!(contested.is_empty());

            site_wrapper.newly_claimed.append(&mut to_claim);
        }
    }

    pub fn reset_grid(&mut self) {
        self.grid.clear()
    }

    pub fn compute(&mut self) {
        while self.sum_newly_claimed() > 0 {
            self.step();
        }
    }

    pub fn step(&mut self) {
        let keys: Vec<SiteOwner> = self.sites.keys().cloned().collect();
        for site_wrapper_idx in keys {
            let site_wrapper = self.sites.get_mut(&site_wrapper_idx).unwrap();

            site_wrapper.boundary_chain.clear();
            site_wrapper.update_boundary_chain(self.grid.bounds());

            site_wrapper.newly_claimed.clear();
            let (mut claimed, contested) = self.grid
                .claim_cells(&site_wrapper.boundary_chain, site_wrapper.id);

            site_wrapper.newly_claimed.append(&mut claimed);

            let mut claimed_won =
                VoronoiTesselation::<S, M>::handle_conflicts(&self.sites, &site_wrapper_idx, contested, &mut self.grid);

            self.sites
                .get_mut(&site_wrapper_idx)
                .unwrap()
                .newly_claimed
                .append(&mut claimed_won);
        }
    }

    fn handle_conflicts(
        sites: &HashMap<SiteOwner, SiteWrapper<S>>,
        owner_idx: &SiteOwner,
        contested: Vec<(GridIdx, SiteOwner)>,
        grid: &mut Grid
    ) -> Vec<GridIdx> {
        let mut claimed = Vec::new();
        for (idx, old_owner) in contested.into_iter() {
            let ref new_site_wrapper = sites[owner_idx];
            let ref old_site_wrapper = sites[&old_owner];

            let our_distance = M::distance(&new_site_wrapper.site, &idx);
            let their_distance = M::distance(&old_site_wrapper.site, &idx);

            if their_distance > our_distance {
                claimed.push(idx);
                grid[idx].set_owner(new_site_wrapper.id);
            } else if their_distance == our_distance {

            } else {
                grid[idx].set_owner(old_owner)
            }
        }

        claimed
    }

    fn sum_newly_claimed(&self) -> usize {
        self.sites
            .iter()
            .map(|(_, site_wrapper)| site_wrapper.newly_claimed.len())
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
                &Some(owner) => map(cell, Some(&sites[&owner].site)),
                &None => map(cell, None)
            })
            .collect()
    }

    pub fn into_regions(self) -> HashMap<S, Vec<Cell>>
    where
        S: Eq + Hash + Clone
    {
        let mut regions = HashMap::new();

        let cells: Vec<Cell> = From::from(self.grid.into_raw());
        for cell in cells.into_iter() {
            if cell.owner().is_some() {
                let owner = cell.owner().as_ref().unwrap();
                let ref site_wrapper = self.sites[owner];
                if !regions.contains_key(&site_wrapper.site) {
                    regions.insert(site_wrapper.site.clone(), Vec::new());
                }
                regions.get_mut(&site_wrapper.site).unwrap().push(cell);
            }
        }

        regions
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
