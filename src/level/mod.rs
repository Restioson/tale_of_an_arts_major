use cgmath::Point2;
use collision::Aabb2;
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::{self, DrawParam, Drawable, Image, Rect};
use ggez::{self, GameResult};
use std::fs::File;
use std::io::BufReader;
use tiled::{self, Map, Object};
use util;

pub struct Level {
    tilesets: Vec<LoadedTileset>,
    pub collision_rects: Vec<Aabb2<f32>>,
    pub easel_rect: Aabb2<f32>,
    pub player_spawn: Point2<f32>,
    pub guards: Vec<GuardInfo>,
    pub guard_jump_boxes: Vec<GuardJumpBox>,
    pub guard_turn_around: Vec<Aabb2<f32>>,
}

#[derive(Debug, Clone)]
pub struct GuardInfo {
    pub spawn: Point2<f32>,
}

#[derive(Clone)]
pub enum GuardJumpBox {
    Left(Aabb2<f32>),
    Right(Aabb2<f32>),
}

impl GuardJumpBox {
    pub fn get_aabb2(&self) -> &Aabb2<f32> {
        match self {
            GuardJumpBox::Left(ref a) => a,
            GuardJumpBox::Right(ref a) => a,
        }
    }

    pub fn get_direction(&self) -> ::entity::components::Direction {
        match self {
            GuardJumpBox::Left(_) => ::entity::components::Direction::Left,
            GuardJumpBox::Right(_) => ::entity::components::Direction::Right,
        }
    }
}

impl Level {
    /// Loads a level from a tiled map
    ///
    /// ## Panicking
    ///
    /// Panics if an object named `easel` is not found in the object layer named `objects`
    pub fn load_from(file: File, ctx: &mut ggez::Context) -> GameResult<Self> {
        use tiled::ObjectShape;

        let reader = BufReader::new(file);
        let map = tiled::parse(reader).expect("Error reading map file!");

        // Load tilesets
        let tilesets = map.tilesets
            .iter()
            .map(|set| {
                let set_source = &set.images[0];
                let image = Image::new(ctx, format!("/{}", set_source.source))
                    .expect("Error reading tileset image!");
                LoadedTileset {
                    batch: Level::build_batch(&map, image.clone(), &set, &set_source),
                }
            })
            .collect();

        // Get objects
        let collision_rects = map.object_groups
            .iter()
            .filter(|group| group.name == "collision")
            .flat_map(|group| &group.objects)
            .filter_map(|object| match object.shape {
                ObjectShape::Rect { width, height } => Some(Aabb2::new(
                    Point2::new(object.x / 16.0, object.y / 16.0),
                    Point2::new((object.x + width) / 16.0, (object.y + height) / 16.0),
                )),
                _ => None,
            })
            .collect();

        let player_spawn = util::take(Level::find_object_points(&map, "objects", "player_spawn"))
            .unwrap_or_else(|| Point2::new(0.0, 0.0));

        let guard_spawns = Level::find_objects_by_type(&map, "objects", "guard_spawn");

        let mut guards = Vec::with_capacity(guard_spawns.len());

        for spawn in guard_spawns {
            guards.push(GuardInfo {
                spawn: Point2::new(spawn.x / 16.0, spawn.y / 16.0),
            });
        }

        let guard_turn_around = Level::find_objects_by_type(&map, "objects", "turn_around")
            .iter()
            .filter_map(|object| match object.shape {
                ObjectShape::Rect { width, height } => Some(Aabb2::new(
                    Point2::new(object.x / 16.0, object.y / 16.0),
                    Point2::new((object.x + width) / 16.0, (object.y + height) / 16.0),
                )),
                _ => None,
            })
            .collect::<Vec<Aabb2<f32>>>();

        let mut guard_jump_boxes: Vec<_> =
            Level::find_objects_by_type(&map, "objects", "jump_left")
                .into_iter()
                .map(|o| (false, o))
                .collect();

        guard_jump_boxes.append(
            &mut Level::find_objects_by_type(&map, "objects", "jump_right")
                .into_iter()
                .map(|o| (false, o))
                .collect(),
        );

        let guard_jump_boxes = guard_jump_boxes
            .into_iter()
            .filter_map(|(is_right, object)| match object.shape {
                ObjectShape::Rect { width, height } => Some({
                    let aabb = Aabb2::new(
                        Point2::new(object.x / 16.0, object.y / 16.0),
                        Point2::new((object.x + width) / 16.0, (object.y + height) / 16.0),
                    );
                    if is_right {
                        GuardJumpBox::Right(aabb)
                    } else {
                        GuardJumpBox::Left(aabb)
                    }
                }),
                _ => None,
            })
            .collect::<Vec<GuardJumpBox>>();

        let easel_rect = util::take(Level::find_objects(&map, "objects", "easel"))
            .and_then(|object| match object.shape {
                ObjectShape::Rect { width, height } => Some(Aabb2::new(
                    Point2::new(object.x / 16.0, object.y / 16.0),
                    Point2::new((object.x + width) / 16.0, (object.y + height) / 16.0),
                )),
                _ => None,
            })
            .expect("Level requires an object named `easel` in the `objects` layer!");

        Ok(Level {
            tilesets,
            collision_rects,
            easel_rect,
            player_spawn,
            guards,
            guard_jump_boxes,
            guard_turn_around,
        })
    }

    fn find_object_points_by_type(
        map: &Map,
        group_name: &'static str,
        object_type: &'static str,
    ) -> Vec<Point2<f32>> {
        Level::find_objects_by_type(map, group_name, object_type)
            .into_iter()
            .map(|object| Point2::new(object.x / 16.0, object.y / 16.0))
            .collect()
    }

    fn find_object_points(
        map: &Map,
        group_name: &'static str,
        object_type: &'static str,
    ) -> Vec<Point2<f32>> {
        Level::find_objects(map, group_name, object_type)
            .into_iter()
            .map(|object| Point2::new(object.x / 16.0, object.y / 16.0))
            .collect()
    }

    fn find_objects<'a>(
        map: &'a Map,
        group_name: &'static str,
        object_name: &'static str,
    ) -> Vec<&'a Object> {
        map.object_groups
            .iter()
            .filter(|group| group.name == group_name)
            .flat_map(|group| &group.objects)
            .filter(|object| object.name == object_name)
            .collect()
    }

    fn find_objects_by_type<'a>(
        map: &'a Map,
        group_name: &'static str,
        object_type: &'static str,
    ) -> Vec<&'a Object> {
        map.object_groups
            .iter()
            .filter(|group| group.name == group_name)
            .flat_map(|group| &group.objects)
            .filter(|object| object.obj_type == object_type)
            .collect()
    }

    fn build_batch(
        map: &tiled::Map,
        image: Image,
        tileset: &tiled::Tileset,
        set_source: &tiled::Image,
    ) -> SpriteBatch {
        let mut batch = SpriteBatch::new(image);
        let source_width = set_source.width as f32;
        let source_height = set_source.height as f32;
        let source_tile_width = set_source.width as u32 / (tileset.tile_width + tileset.spacing);

        for layer in &map.layers {
            for x in 0..map.width {
                for y in 0..map.height {
                    let tile_idx = layer.tiles[y as usize][x as usize];
                    if tile_idx == 0 {
                        continue;
                    }

                    // Get the tile coordinates within the tile map source image
                    let src_tile_x = (tile_idx - 1) % source_tile_width;
                    let src_tile_y = (tile_idx - 1) / source_tile_width;

                    // Get the U/V which the tile should be sampled from
                    let src_u =
                        (src_tile_x * (tileset.tile_width + tileset.spacing)) as f32 / source_width;
                    let src_v = (src_tile_y * (tileset.tile_height + tileset.spacing)) as f32
                        / source_height;
                    let src_rect = Rect::new(
                        src_u,
                        src_v,
                        tileset.tile_width as f32 / source_width,
                        tileset.tile_height as f32 / source_height,
                    );

                    batch.add(DrawParam {
                        src: src_rect,
                        dest: graphics::Point2::new(
                            (x * tileset.tile_width) as f32,
                            (y * tileset.tile_height) as f32,
                        ),
                        rotation: 0.0,
                        scale: graphics::Point2::new(1.0, 1.0),
                        offset: graphics::Point2::new(0.0, 0.0),
                        shear: graphics::Point2::new(0.0, 0.0),
                        color: None,
                    });
                }
            }
        }

        batch
    }

    pub fn render(&mut self, ctx: &mut ggez::Context) {
        for tileset in &self.tilesets {
            tileset
                .batch
                .draw_ex(
                    ctx,
                    DrawParam {
                        src: Rect::one(),
                        dest: graphics::Point2::new(0.0, 0.0),
                        rotation: 0.0,
                        scale: graphics::Point2::new(::GLOBAL_SCALE, ::GLOBAL_SCALE),
                        offset: graphics::Point2::new(0.0, 0.0),
                        shear: graphics::Point2::new(0.0, 0.0),
                        color: None,
                    },
                )
                .expect("Error drawing tileset!");
        }
    }
}

struct LoadedTileset {
    pub batch: SpriteBatch,
}
