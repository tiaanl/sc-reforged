pub use render_snapshot::*;

use bevy_ecs::prelude::*;

mod camera;
mod environment;
mod gizmos;
mod models;
mod render_snapshot;
mod terrain;
mod ui;

pub fn create_extract_schedule() -> Schedule {
    let mut schedule = Schedule::default();

    schedule.add_systems(
        (
            // Do the camera and environment first, becuase the rest of the
            // systems rely on their data.
            (camera::extract_camera, environment::extract_environment),
            // These can be done in any order.
            (
                terrain::extract_terrain_snapshot,
                models::extract_models_to_prepare,
                models::extract_model_snapshot,
                ui::extract_ui_snapshot,
                gizmos::extract_gizmos,
            ),
        )
            .chain(),
    );

    schedule
}
