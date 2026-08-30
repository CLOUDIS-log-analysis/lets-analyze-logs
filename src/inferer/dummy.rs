use crate::{BugLocation, Ctxt, StartingLocation};

pub fn dummy_inferer(_ctx: &Ctxt, sps: &[StartingLocation]) -> anyhow::Result<Vec<BugLocation>> {
    let mut locs = Vec::new();
    for StartingLocation { loc, reliability } in sps.iter().cloned() {
        locs.push(BugLocation { loc, reliability });
    }

    Ok(locs)
}
