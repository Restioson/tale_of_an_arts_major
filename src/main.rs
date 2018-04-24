#![feature(nll)]
#![feature(specialization)]
#![feature(match_default_bindings)]
#![feature(custom_attribute)]
#![allow(unused_attributes)]

extern crate cgmath;
extern crate collision;
extern crate ggez;
extern crate image;
extern crate itertools;
extern crate ord_subset;
extern crate rand;
extern crate rhusics_core;
extern crate rhusics_ecs;
extern crate shrev;
extern crate specs;
extern crate tiled;

use canvas::PaintingCanvas;
use cgmath::{Basis2, One, Point2, Vector2};
use collision::Contains;
use entity::components::*;
use entity::resources::*;
use entity::systems::*;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler, Keycode, Mod};
use ggez::graphics::{self, FilterMode};
use level::Level;
use rhusics_core::{Pose, RigidBody};
use rhusics_core::ContactEvent;
use rhusics_ecs::{DeltaTime, WithRigidBody};
use rhusics_ecs::collide2d::*;
use rhusics_ecs::physics2d::{ContactResolutionSystem2, CurrentFrameUpdateSystem2,
                             Mass2, NextFrameSetupSystem2, register_physics,
                             SpatialCollisionSystem2, SpatialSortingSystem2, Velocity2};
use shrev::EventChannel;
use specs::{Dispatcher, DispatcherBuilder, Entity, RunNow, World};
use std::fs::File;
use std::path::Path;

mod canvas;
mod capture;
mod entity;
mod gui;
mod level;
mod util;

// TODO gegy fight me, doesnt fit on my screen
// TODO dynamically scale this
const SCREEN_SIZE: (u32, u32) = (800, 600);

const GLOBAL_SCALE: f32 = 2.0;
const SCALED_SIZE: (f32, f32) = (
    SCREEN_SIZE.0 as f32 / GLOBAL_SCALE,
    SCREEN_SIZE.1 as f32 / GLOBAL_SCALE,
);

const PLAYER_SIZE: (f32, f32) = (0.9, 1.8);
const GUARD_SIZE: (f32, f32) = (0.9, 1.8);

struct GameState<'a> {
    level_state: LevelState<'a>,
    render_state: RenderState,
    font: graphics::Font,
}

impl<'a> GameState<'a> {
    fn new(ctx: &mut Context) -> GameResult<GameState<'a>> {
        // TODO: Terrible code but works!
        let mut levels = vec![Level::load_from(
            File::open(&Path::new("resources/levels/level_1.tmx"))
                .expect("Failed to locate map file"),
            ctx,
        )?, Level::load_from(
            File::open(&Path::new("resources/levels/level_2.tmx"))
                .expect("Failed to locate map file"),
            ctx,
        )?];

        let font = graphics::Font::new(ctx, "/arial.ttf", 16)?;

        Ok(GameState {
            level_state: LevelState::new(ctx, levels.remove(0))?,
            render_state: RenderState::new(),
            font,
        })
    }
}

impl<'a> EventHandler for GameState<'a> {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.level_state.update(ctx, &mut self.render_state);

        self.render_state.gui.as_mut().map(|gui| {
            let mouse_pos = ggez::mouse::get_position(ctx).unwrap();
            let mouse_x = mouse_pos.x / ::GLOBAL_SCALE;
            let mouse_y = mouse_pos.y / ::GLOBAL_SCALE;
            gui.update(mouse_x, mouse_y).expect("Failed to update gui!");
        });

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx);

        graphics::set_color(ctx, graphics::Color::new(1.0, 1.0, 1.0, 1.0))?;

        self.level_state.render(ctx, &mut self.render_state);

        // FIXME
        let font = self.font.clone();
        self.render_state.gui.as_mut().map(|gui| {
            let mouse_pos = ggez::mouse::get_position(ctx).unwrap();
            let mouse_x = mouse_pos.x / ::GLOBAL_SCALE;
            let mouse_y = mouse_pos.y / ::GLOBAL_SCALE;
            gui.draw(ctx, &font, mouse_x, mouse_y)
                .expect("Failed to draw gui!");
        });

        graphics::present(ctx);
        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut ggez::Context,
        button: ggez::event::MouseButton,
        x: i32,
        y: i32,
    ) {
        if button == ggez::event::MouseButton::Left {
            self.render_state.gui.as_mut().map(|gui| {
                gui.mouse_pressed(x as f32 / ::GLOBAL_SCALE, y as f32 / ::GLOBAL_SCALE);
            });
        }
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut ggez::Context,
        button: ggez::event::MouseButton,
        x: i32,
        y: i32,
    ) {
        if button == ggez::event::MouseButton::Left {
            self.render_state.gui.as_mut().map(|gui| {
                gui.mouse_released(x as f32 / ::GLOBAL_SCALE, y as f32 / ::GLOBAL_SCALE);
            });
        }
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool) {
        use Keycode::*;
        match keycode {
            Escape => ctx.quit().expect("Failed to quit"),
            F12 => self.render_state.debug = !self.render_state.debug,
            _ => {
                self.level_state.key_pressed(ctx, &mut self.render_state, keycode);
                ()
            }
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool) {
        match keycode {
            _ => {
                self.level_state.key_released(keycode);
                ()
            }
        }
    }
}

struct RenderState {
    debug: bool,
    gui: Option<Box<gui::Gui>>,
}

impl RenderState {
    fn new() -> Self {
        RenderState {
            debug: false,
            gui: None,
        }
    }
}

struct LevelState<'a> {
    world: World,
    level: Level,
    update_dispatcher: Dispatcher<'a, 'a>,
    locked: bool,
}

impl<'a> LevelState<'a> {
    fn new(ctx: &mut Context, level: Level) -> GameResult<Self> {
        let player_image = graphics::Image::new(ctx, "/player_right.png")?;
        let guard_image = graphics::Image::new(ctx, "/guard.png")?;

        let mut world = World::new();

        entity::components::register_components(&mut world);
        register_physics::<f32, (), BodyPose2<f32>>(&mut world);

        let resolution_contact_reader = world
            .write_resource::<EventChannel<ContactEvent<Entity, Point2<f32>>>>()
            .register_reader();

        let state_contact_reader = world
            .write_resource::<EventChannel<ContactEvent<Entity, Point2<f32>>>>()
            .register_reader();

        world.res.add(GlobalPlayerState {
            pos: Point2::new(0.0, 0.0),
            bounds: collision::Aabb2::new(Point2::new(0.0, 0.0), Point2::new(0.0, 0.0)),
            id: 0,
            captured: false,
        });
        world.res.add(GameInput::new());
        world
            .res
            .add(GuardJumpBoxes(level.guard_jump_boxes.clone()));
        world
            .res
            .add(GuardTurnAroundBoxes(level.guard_turn_around.clone()));

        world
            .create_entity()
            .with(Player)
            .with(CollisionState { ground: false, jump_cooldown: 0.0 })
            .with_dynamic_rigid_body(
                CollisionShape2::<f32, BodyPose2<f32>, ()>::new_simple(
                    CollisionStrategy::FullResolution,
                    CollisionMode::Discrete,
                    Rectangle::new(PLAYER_SIZE.0, PLAYER_SIZE.1).into(),
                ),
                BodyPose2::new(level.player_spawn, Basis2::one()),
                Velocity2::new(Vector2::new(0.0, 0.0), 0.0),
                RigidBody::default(),
                Mass2::new(1.0),
            )
            .with(Sprite::new(player_image))
            .build();

        for guard_info in level.guards.iter() {
            world
                .create_entity()
                .with(CollisionState { ground: false, jump_cooldown: 0.0 })
                .with(Directional {
                    direction: Direction::Left,
                })
                .with(GuardAi {
                    state: GuardAiState::Patrolling,
                    info: guard_info.clone(),
                    turn_around_cooldown: 0.0,
                })
                .with_dynamic_rigid_body(
                    CollisionShape2::<f32, BodyPose2<f32>, ()>::new_simple(
                        CollisionStrategy::FullResolution,
                        CollisionMode::Discrete,
                        Rectangle::new(GUARD_SIZE.0, GUARD_SIZE.1).into(),
                    ),
                    BodyPose2::new(guard_info.spawn, Basis2::one()),
                    Velocity2::new(Vector2::new(0.0, 0.0), 0.0),
                    RigidBody::default(),
                    Mass2::new(1.0),
                )
                .with(Sprite::new(guard_image.clone()))
                .build();
        }

        for collision_rect in &level.collision_rects {
            let Vector2 { x: w, y: h, .. } = collision_rect.max - collision_rect.min;

            world
                .create_entity()
                .with_static_rigid_body(
                    CollisionShape2::<f32, BodyPose2<f32>, ()>::new_simple(
                        CollisionStrategy::FullResolution,
                        CollisionMode::Discrete,
                        Rectangle::new(w, h).into(),
                    ),
                    BodyPose2::new(
                        Point2::new(
                            collision_rect.min.x + w / 2.0,
                            collision_rect.min.y + h / 2.0,
                        ),
                        Basis2::one(),
                    ),
                    RigidBody::default(),
                    Mass2::new(1.0),
                )
                .build();
        }

        let update_dispatcher = DispatcherBuilder::new()
            .add(
                CollisionStateSystem {
                    contact_reader: state_contact_reader,
                },
                "collision_state",
                &[],
            )
            .add(PlayerSystem, "player", &["collision_state"])
            .add(PhysicsExtras, "gravity", &["collision_state"])
            .add(GuardAiSystem, "guard_ai", &["collision_state"])
            .add(
                CurrentFrameUpdateSystem2::<f32, BodyPose2<f32>>::new(),
                "solver",
                &[],
            )
            .add(
                NextFrameSetupSystem2::<f32, BodyPose2<f32>>::new(),
                "next_frame",
                &["solver"],
            )
            .add(
                SpatialSortingSystem2::<f32, BodyPose2<f32>, ()>::new(),
                "sorting",
                &["next_frame"],
            )
            .add(
                SpatialCollisionSystem2::<f32, BodyPose2<f32>, ()>::new()
                    .with_broad_phase(BroadBruteForce2::default())
                    .with_narrow_phase(GJK2::new()),
                "collision",
                &["sorting"],
            )
            .add(
                ContactResolutionSystem2::<f32, BodyPose2<f32>>::new(resolution_contact_reader),
                "resolution",
                &["collision"],
            )
            .build();

        Ok(LevelState { world, level, update_dispatcher, locked: false })
    }

    fn update(&mut self, ctx: &mut Context, render_state: &mut RenderState) {
        // Δt = t_{last frame} - t_{now}
        let dt = ggez::timer::get_delta(ctx);
        let seconds =
            (dt.as_secs() as f32 + (dt.subsec_nanos() as f32 / 1_000_000_000.0)).min(1.0 / 20.0);
        self.world.write_resource::<DeltaTime<f32>>().delta_seconds = seconds;
        if self.world.read_resource::<GlobalPlayerState>().captured {
            render_state.gui = Some(Box::new(capture::CaptureGui));
            self.locked = true;
        }

        self.update_dispatcher.dispatch(&self.world.res);
        self.world.maintain();
    }

    fn render(&mut self, ctx: &mut Context, render_state: &mut RenderState) {
        let resources = &mut self.world.res;

        self.level.render(ctx);

        // Run rendering systems
        SpriteSystem::new(ctx).run_now(resources);

        if render_state.debug {
            DebugRenderSystem::new(ctx).run_now(resources);
        }
    }

    fn key_pressed(&mut self, ctx: &mut Context, render_state: &mut RenderState, keycode: Keycode) {
        if self.locked {
            return;
        }

        use rand::Rng;
        use std::fs;

        let mut input = self.world.write_resource::<GameInput>();

        use Keycode::*;
        match keycode {
            Right => input.move_horizontal = 1.0,
            Left => input.move_horizontal = -1.0,
            Up => input.jumping = true,
            C => {
                // check if player is in bounds of easel rect
                let pos = self.world.read_resource::<GlobalPlayerState>().pos;
                if self.level.easel_rect.contains(&pos) {
                    render_state.gui = match render_state.gui {
                        Some(_) => None,
                        None => {
                            let paintings = fs::read_dir("./resources/paintings").unwrap().map(|r| r.unwrap()).collect::<Vec<_>>();
                            let chosen_file = rand::thread_rng().choose(paintings.as_ref());
                            Some(Box::new(PaintingCanvas::from_path(
                                ctx,
                                chosen_file.unwrap().path(),
                            )))
                        }
                    }
                }
            }
            _ => (),
        }
    }

    fn key_released(&mut self, keycode: Keycode) {
        let mut input = self.world.write_resource::<GameInput>();

        use Keycode::*;
        match keycode {
            Right | Left => input.move_horizontal = 0.0,
            Up => input.jumping = false,
            _ => (),
        }
    }
}

fn main() {
    let ctx = &mut ContextBuilder::new("toam", "gegy1000, Restioson")
        .window_setup(
            WindowSetup::default()
                .title("Tale of an Arts Major")
                .icon("/icon.png"),
        )
        .window_mode(WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
        .build()
        .expect("Failed to build ggez context");

    graphics::set_background_color(
        ctx,
        graphics::Color {
            r: 0.75,
            g: 0.75,
            b: 1.0,
            a: 1.0,
        },
    );

    // We don't want antialiasing when scaling the textures up, we have pixelart!
    // Reply from Restioson: "muh beautiful pixel art D:"
    graphics::set_default_filter(ctx, FilterMode::Nearest);

    let state = &mut GameState::new(ctx).unwrap();
    event::run(ctx, state).unwrap();
}
