use crate::*;
use bary_core::prelude::*;

#[derive(Debug, Clone)]
pub struct OrbitalController {
    last_update: Nanotime,
    current: Option<GlobalOrbit>,
    destination: Option<GlobalOrbit>,
    plan: Option<ManeuverPlan>,
}

impl OrbitalController {
    pub fn idle() -> Self {
        OrbitalController {
            last_update: Nanotime::zero(),
            current: None,
            destination: None,
            plan: None,
        }
    }

    pub fn clear(&mut self) {
        self.destination = None;
        self.plan = None;
    }

    pub fn is_idle(&self) -> bool {
        self.destination.is_none()
    }

    pub fn needs_update(&self, stamp: Nanotime) -> bool {
        stamp - self.last_update > Nanotime::secs(1)
    }

    pub fn update(&mut self, stamp: Nanotime, orbit: GlobalOrbit) -> Result<(), &'static str> {
        self.last_update = stamp;

        self.current = Some(orbit);

        if self.destination.is_none() {
            return Ok(());
        }

        if let Some((c, d)) = self.current.zip(self.destination)
            && c.1.is_similar(&d.1) {
                self.destination = None;
                self.plan = None;
                return Ok(());
            }

        let nominal = self
            .plan
            .as_ref()
            .and_then(|m| m.segment_at(stamp))
            .map(|s| s.orbit);

        if let Some(nominal) = nominal
            && orbit.1.is_similar(&nominal) {
                return Ok(());
            }

        if self.current.is_some() && self.destination.is_some() {
            self.reroute(stamp)
        } else {
            Ok(())
        }
    }

    pub fn set_destination(
        &mut self,
        destination: GlobalOrbit,
        stamp: Nanotime,
    ) -> Result<(), &'static str> {
        self.destination = Some(destination);
        self.reroute(stamp)
    }

    pub fn reroute(&mut self, stamp: Nanotime) -> Result<(), &'static str> {
        let c = self.current.as_ref().ok_or("No current orbit")?;
        let d = self.destination.as_ref().ok_or("No destination orbit")?;
        if c.0 != d.0 {
            return Err("Cannot path between bodies");
        }
        let p = best_maneuver_plan(&c.1, &d.1, stamp)?;
        self.plan = Some(p);
        Ok(())
    }

    pub fn destination(&self) -> Option<&GlobalOrbit> {
        self.destination.as_ref()
    }

    pub fn plan(&self) -> Option<&ManeuverPlan> {
        self.plan.as_ref()
    }
}

impl std::fmt::Display for OrbitalController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = self
            .current
            .map(|c| format!("{}", c))
            .unwrap_or("None".into());
        let d = self
            .destination
            .map(|d| format!("{}", d))
            .unwrap_or("None".into());
        write!(f, "CTRL {} {} {}", self.last_update, c, d)?;

        if let Some(p) = &self.plan {
            write!(f, "\n{}", p)?;
        }

        Ok(())
    }
}
