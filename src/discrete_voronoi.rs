use grid::{BoundingBox, Grid, GridIdx};
use metric::Metric;
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
    bounding: Option<BoundingBox>
}

impl<S, M> VoronoiBuilder<S, M>
where
    S: Site,
    M: Metric
{
    // Will remove all sites that have the same coordinates
    pub fn new(mut sites: Vec<S>) -> Self {
        sites.sort_unstable_by_key(|site| site.coordinates());
        sites.dedup_by_key(|site| site.coordinates());
        VoronoiBuilder {
            sites,
            metric: PhantomData,
            bounding: None
        }
    }

    pub fn metric<E>(self) -> VoronoiBuilder<S, E>
    where
        E: Metric
    {
        VoronoiBuilder {
            metric: PhantomData,
            sites: self.sites,
            bounding: self.bounding
        }
    }

    pub fn bounding(mut self, bounding: BoundingBox) -> Self {
        self.bounding = Some(bounding);

        self
    }

    pub fn build(self) -> VoronoiTesselation<S, M> {
        let bounding = if let Some(value) = self.bounding {
            value
        } else {
            BoundingBox::fit_to_sites(&self.sites)
        };

        let num_sites = self.sites.len();
        let sites_id_pars = self.sites.into_iter().zip(0..(num_sites as u32));
        let wrapped_sites = sites_id_pars
            .map(|(site, id)| SiteWrapper::new(id, site))
            .collect::<Vec<_>>();

        VoronoiTesselation {
            sites: wrapped_sites,
            metric: PhantomData,
            grid: Grid::new(bounding)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiteOwner(u32);

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

    pub fn bounding(&self) -> &BoundingBox {
        self.grid.bounding()
    }

    pub fn compute(&mut self) {
        // Claim initial particle locations
        for site_wrapper in self.sites.iter_mut() {
            let mut to_claim = vec![GridIdx::from(site_wrapper.site.coordinates())];
            let (claimed, contested) = self.grid.claim_cells(&to_claim, site_wrapper.id);

            debug_assert_eq!(claimed.len(), 1);
            debug_assert!(contested.is_empty());

            site_wrapper.newly_claimed.append(&mut to_claim);
        }

        while self.sum_newly_claimed() > 0 {
            for site_wrapper_idx in 0..self.sites.len() {
                let ref mut site_wrapper = self.sites[site_wrapper_idx];
                site_wrapper.boundary_chain.clear();
                site_wrapper.update_boundary_chain(self.grid.bounding());

                site_wrapper.newly_claimed.clear();
                let (mut claimed, contested) = self.grid.claim_cells(
                    &site_wrapper.boundary_chain,
                    site_wrapper.id
                );

                site_wrapper
                    .newly_claimed
                    .append(&mut claimed);

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
    }

    fn handle_conflicts(
        sites: &mut Vec<SiteWrapper<S>>,
        owner_idx: usize,
        contested: Vec<(GridIdx, SiteOwner)>,
        grid: &mut Grid
    ) -> Vec<GridIdx> {
        let (won, lost): (Vec<(GridIdx, SiteOwner)>, Vec<(GridIdx, SiteOwner)>) = contested.into_iter().partition(
            |&(constested_idx, old_owner)| {
                let our_distance = M::distance(&sites[owner_idx].site, &constested_idx);
                let their_distance = M::distance(&sites[old_owner.0 as usize].site, &constested_idx);

                their_distance <= our_distance
            }
        );

        for (idx, old_owner) in lost {
            grid[idx].set_owner(old_owner);
        }

        won.into_iter().map(|(idx, _)| idx).collect()
    }

    fn sum_newly_claimed(&self) -> usize {
        self.sites
            .iter()
            .map(|site_wrapper| site_wrapper.newly_claimed.len())
            .sum()
    }
}
