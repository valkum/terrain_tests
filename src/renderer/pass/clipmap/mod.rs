pub use self::seperate::DrawClipmap;

use crate::{ActiveClipmap, Clipmap};
use amethyst::core::{
    specs::prelude::{Join, Read, ReadStorage},
    transform::GlobalTransform,
};

mod seperate;

static VERT_SRC: &[u8] = include_bytes!("../../shader/vertex/clipmap.glsl");
static FRAG_SRC: &[u8] = include_bytes!("../../shader/fragment/clipmap.glsl");

pub fn get_clipmap<'a>(
    active: Read<'a, ActiveClipmap>,
    clipmaps: &'a ReadStorage<'a, Clipmap>,
    globals: &'a ReadStorage<'a, GlobalTransform>,
) -> Option<(&'a Clipmap, &'a GlobalTransform)> {
    active
        .entity
        .and_then(|entity| {
            let cm = clipmaps.get(entity);
            let transform = globals.get(entity);
            cm.into_iter().zip(transform.into_iter()).next()
        })
        .or_else(|| (clipmaps, globals).join().next())
}
