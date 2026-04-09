use std::sync::Arc;

use super::window::Window;

#[derive(Default)]
pub struct WindowManager {
    pub windows: Vec<Arc<Window>>,
}

impl WindowManager {
    /// Push a new window to the top of the stack.
    pub fn push(&mut self, window: Arc<Window>) {
        tracing::info!("Pushing window: ???");
        self.windows.push(window);
    }
}

// /// Rebuilds each window's queued render items from its descendant geometry and
// /// sprite components. Windows without tiled geometry fall back to a simple
// /// border-only chrome item.
// pub fn update_window_render_items(
//     mut windows: Query<(Entity, &Rect, &mut WindowRenderItems), With<Window>>,
//     hierarchy: Query<&Children>,
//     tiled_geometry: Query<(&GeometryTiled, Option<&ZIndex>)>,
//     sprites: Query<&SpriteRender>,
//     mut descendants: Local<Vec<Entity>>,
// ) {
//     for (window, rect, mut window_render_items) in windows.iter_mut() {
//         // Clear out old renders.
//         window_render_items.clear();
//         descendants.clear();
//         descendants.extend(hierarchy.iter_descendants::<Children>(window));

//         // Tiled Geometry
//         let mut tiled_geometries = descendants
//             .iter()
//             .filter_map(|&entity| {
//                 tiled_geometry
//                     .get(entity)
//                     .ok()
//                     .map(|(tiled_geometry, z_index)| {
//                         let z_index = z_index.map(|i| i.0).unwrap_or(0);

//                         (
//                             z_index,
//                             tiled_geometry.tiled_geometry_handle,
//                             tiled_geometry.alpha,
//                         )
//                     })
//             })
//             .collect::<Vec<_>>();

//         if tiled_geometries.is_empty() {
//             // Render a default window background.
//             window_render_items.render_border(rect.position, rect.size, 2, Vec4::ONE);
//         } else {
//             tiled_geometries.sort_by_key(|(z_index, _, _)| *z_index);

//             tiled_geometries
//                 .drain(..)
//                 .for_each(|(_, tiled_geometry_handle, alpha)| {
//                     window_render_items.render_tiled_geometry(tiled_geometry_handle, alpha);
//                 });
//         }

//         // Widgets
//         for &entity in &descendants {
//             if let Ok(sprite_render) = sprites.get(entity) {
//                 window_render_items.render_sprite(
//                     sprite_render.position,
//                     sprite_render.sprite,
//                     sprite_render.frame,
//                     sprite_render.alpha,
//                 );
//             }
//         }
//     }
// }

// pub fn spawn_window<'a>(commands: &'a mut Commands, rect: Rect) -> EntityCommands<'a> {
//     commands.spawn((Window, rect))
// }

// pub fn spawn_window_geometries(
//     commands: &mut Commands,
//     window_renderer: &mut WindowRenderer,
//     images: &Images,
//     window_entity: Entity,
//     geometries: &[Geometry],
// ) -> Result<Vec<Entity>, AssetError> {
//     use crate::game::config::windows::GeometryKind;
//     use crate::game::windows::ecs::geometry::GeometryTiled;

//     let mut result = Vec::with_capacity(geometries.len());

//     for (i, geometry) in geometries.iter().enumerate() {
//         match geometry.kind {
//             GeometryKind::Normal(ref _geometry_normal) => todo!(),
//             GeometryKind::Tiled(ref tiled) => {
//                 let path = PathBuf::from("textures")
//                     .join("interface")
//                     .join(&tiled.jpg_name)
//                     .with_extension("jpg");

//                 let image_handle = images.load(&path)?;
//                 let image = images.get(image_handle).expect("just created");

//                 let tiled_geometry_handle = window_renderer
//                     .create_tiled_geometry(
//                         image_handle,
//                         IVec2::from(tiled.dimensions).as_uvec2(),
//                         IVec2::from(tiled.chunk_dimensions).as_uvec2(),
//                     )
//                     .ok_or(AssetError::FileNotFound(path))?;

//                 let z_index = -(i as i32);

//                 let geometry_entity = commands
//                     .spawn((
//                         GeometryTiled {
//                             tiled_geometry_handle,
//                             alpha: 1.0,
//                             size: image.size,
//                         },
//                         ZIndex(z_index),
//                         ChildOf(window_entity),
//                     ))
//                     .id();

//                 result.push((z_index, geometry_entity));
//             }
//         }
//     }

//     // Sort the results according to z_index.
//     result.sort_by_key(|e| e.0);

//     Ok(result.iter().map(|e| e.1).collect())
// }
