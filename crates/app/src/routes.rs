//! Route inspection from the same validated planning request. No invented geometry.
use crate::{
    atlas::ProjectedPosition,
    planning::{self, PlanningReport},
    snapshots::SnapshotApplication,
};
use azeroth_core::{
    atlas::project_position,
    entities::TravelEdge,
    error::Result,
    travel::RouteHop,
    types::{DataContext, ImageId, MapKey, PixelExtent, UtcMillis},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RouteFrame {
    pub id: ImageId,
    pub map: MapKey,
    pub width_px: PixelExtent,
    pub height_px: PixelExtent,
}
#[derive(Debug, Serialize)]
pub struct InspectedHop {
    pub leg: usize,
    pub stopping: bool,
    pub hop: RouteHop,
    pub requirements: azeroth_core::observation::Observation<azeroth_core::predicate::Predicate>,
    pub from: ProjectedPosition,
    pub to: ProjectedPosition,
    pub schematic_connection: bool,
    pub geometry_gap: Option<&'static str>,
}
#[derive(Debug, Serialize)]
pub struct RouteInspection {
    pub context: DataContext,
    pub fixture: bool,
    pub report: PlanningReport,
    pub frames: Vec<RouteFrame>,
    pub hops: Vec<InspectedHop>,
}
impl SnapshotApplication {
    pub fn inspect_plan_route(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<RouteInspection> {
        self.with_planning(text, now, |request, snapshot, content, clock| {
            let report = planning::calculate(request, snapshot, content, clock)?;
            let dataset = &content.reference();
            let context = content.context();
            let mut hops = Vec::new();
            for (leg, (route, stopping)) in report.result.selected.iter().map(|s| (&s.travel, false))
                .chain(std::iter::once((&report.result.stopping, true))).enumerate() {
                for hop in &route.hops {
                    let from_node = dataset.nodes.iter().find(|n| n.id == hop.from.id).expect("validated route node");
                    let to_node = dataset.nodes.iter().find(|n| n.id == hop.to.id).expect("validated route node");
                    let project = |position: &azeroth_core::types::Position| match project_position(position, &context, dataset) {
                        Ok(p) => ProjectedPosition::Placed(p), Err(gap) => ProjectedPosition::Gap(gap),
                    };
                    let from = project(&from_node.position);
                    let to = project(&to_node.position);
                    let same_space = from_node.position.map == to_node.position.map;
                    let same_image = matches!((&from, &to), (ProjectedPosition::Placed(a), ProjectedPosition::Placed(b)) if a.image == b.image);
                    let geometry_gap = if !same_space { Some("Endpoints use different map, floor or instance spaces") }
                        else if !same_image { Some("Exact dataset projection unavailable; inspect endpoint gaps") } else { None };
                    let edge: &TravelEdge = dataset.edges.iter().find(|e| e.id == hop.edge).expect("validated route edge");
                    hops.push(InspectedHop { leg, stopping, hop: hop.clone(), requirements: edge.requirements.clone(), from, to,
                        schematic_connection: same_space && same_image, geometry_gap });
                }
            }
            let frames = dataset.map_images.iter().map(|i| RouteFrame { id: i.id.clone(), map: i.map.clone(), width_px: i.width_px, height_px: i.height_px }).collect();
            Ok(RouteInspection { context, fixture: snapshot.fixture, report, frames, hops })
        })
    }
}
