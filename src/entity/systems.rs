#![allow(unknown_lints)]

use cgmath::{InnerSpace, Point2, Vector2};
use collision::{dbvt::query_ray, Discrete, Ray2};
use ggez;
use ggez::graphics::{self, Color, DrawMode, DrawParam};
use rhusics_core::{ContactEvent, Pose};
use rhusics_ecs::DeltaTime;
use rhusics_ecs::physics2d::{BodyPose2, DynamicBoundingVolumeTree2, RigidBodyParts2};
use shrev::{EventChannel, ReaderId};
use specs::{Entity, Fetch, FetchMut, Join, ReadStorage, System, WriteStorage};
use specs::Entities;
use std;
use super::components::*;
use super::resources::*;

/// The acceleration of gravity
const GRAVITY_ACCEL: f32 = 30.0;
const GUARD_BASE_SPEED: f32 = 3.0;
/// How much faster a guard is while alerted (as a multiplier)
const GUARD_ALERTED_MULTIPLIER: f32 = 1.5;

pub struct SpriteSystem<'a> {
    ctx: &'a mut ggez::Context,
}

impl<'a> SpriteSystem<'a> {
    pub fn new(ctx: &'a mut ggez::Context) -> Self {
        SpriteSystem { ctx }
    }
}

impl<'a> System<'a> for SpriteSystem<'a> {
    type SystemData = (
        RigidBodyParts2<'a, f32, BodyPose2<f32>, ()>,
        ReadStorage<'a, Sprite>,
    );

    fn run(&mut self, (rigid_body_parts, sprite): Self::SystemData) {
        for (body, sprite) in (&rigid_body_parts.poses, &sprite).join() {
            let pos = body.position();
            graphics::draw_ex(
                self.ctx,
                &sprite.image,
                DrawParam {
                    src: sprite.clip,
                    dest: graphics::Point2::new(
                        ((pos.x * 16.0) - sprite.image.width() as f32 / 2.0) * ::GLOBAL_SCALE,
                        ((pos.y * 16.0) - sprite.image.height() as f32 / 2.0) * ::GLOBAL_SCALE,
                    ),
                    rotation: sprite.rotation,
                    scale: graphics::Point2::new(
                        sprite.scale.x * ::GLOBAL_SCALE,
                        sprite.scale.y * ::GLOBAL_SCALE,
                    ),
                    offset: graphics::Point2::new(0.0, 0.0),
                    shear: graphics::Point2::new(0.0, 0.0),
                    color: None,
                },
            ).expect("Error drawing!");
        }
    }
}

pub struct DebugRenderSystem<'a> {
    ctx: &'a mut ggez::Context,
}

impl<'a> DebugRenderSystem<'a> {
    pub fn new(ctx: &'a mut ggez::Context) -> Self {
        DebugRenderSystem { ctx }
    }
}

impl<'a> System<'a> for DebugRenderSystem<'a> {
    type SystemData = (RigidBodyParts2<'a, f32, BodyPose2<f32>, ()>, );

    fn run(&mut self, (rigid_body_parts, ): Self::SystemData) {
        for (shape, pose) in (&rigid_body_parts.shapes, &rigid_body_parts.poses).join() {
            graphics::set_color(self.ctx, Color::new(1.0, 1.0, 1.0, 1.0))
                .expect("Error setting color!");
            let pos = pose.position();
            let bound = shape.bound();
            let min = bound.min;
            let max = bound.max;
            let final_scale = ::GLOBAL_SCALE * 16.0;
            graphics::rectangle(
                self.ctx,
                DrawMode::Line(3.0),
                graphics::Rect::new(
                    min.x * final_scale,
                    min.y * final_scale,
                    (max.x - min.x) * final_scale,
                    (max.y - min.y) * final_scale,
                ),
            ).expect("Error drawing entity bounds!");

            graphics::set_color(self.ctx, Color::new(1.0, 0.0, 0.0, 1.0))
                .expect("Error setting color!");
            let draw_x = pos.x * final_scale;
            let draw_y = pos.y * final_scale;
            graphics::rectangle(
                self.ctx,
                DrawMode::Fill,
                graphics::Rect::new(draw_x - 2.0, draw_y - 2.0, 4.0, 4.0),
            ).expect("Error drawing entity origin!");
        }
    }
}

/// Updates the player's rectangle
pub struct CollisionStateSystem {
    pub contact_reader: ReaderId<ContactEvent<Entity, Point2<f32>>>,
}

impl<'a> System<'a> for CollisionStateSystem {
    #![allow(clippy)]
    type SystemData = (
        WriteStorage<'a, CollisionState>,
        Fetch<'a, EventChannel<ContactEvent<Entity, Point2<f32>>>>,
        Fetch<'a, DeltaTime<f32>>
    );

    fn run(&mut self, (mut collision_state, contacts, delta_time): Self::SystemData) {
        for (mut collision_state, ) in (&mut collision_state, ).join() {
            collision_state.ground = false;
            if collision_state.jump_cooldown > 0.0 {
                collision_state.jump_cooldown = (collision_state.jump_cooldown - delta_time.delta_seconds).max(0.0);
            }
        }
        for event in contacts.read(&mut self.contact_reader) {
            if let Some(state) = collision_state.get_mut(event.bodies.0) {
                if event.contact.normal.dot(Vector2::new(0.0, 1.0)) > 0.25 {
                    state.ground = true;
                }
            }
        }
    }
}

/// Updates the player's rectangle
pub struct PlayerSystem;

impl<'a> System<'a> for PlayerSystem {
    #![allow(clippy)]
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, CollisionState>,
        RigidBodyParts2<'a, f32, BodyPose2<f32>, ()>,
        FetchMut<'a, GlobalPlayerState>,
        Fetch<'a, GameInput>,
    );

    fn run(
        &mut self,
        (
            entities,
            player,
            mut collision_state,
            mut rigid_body_parts,
            mut player_state,
            input,
        ): Self::SystemData,
    ) {
        // FIXME player isn't used?
        for (entity, _player, mut collision_state, body, shape, mut forces) in (
            &*entities,
            &player,
            &mut collision_state,
            &rigid_body_parts.poses,
            &rigid_body_parts.shapes,
            &mut rigid_body_parts.forces,
        ).join()
            {
                let pos = body.position();
                player_state.pos = pos;
                player_state.bounds = shape.bound().clone();
                player_state.id = entity.id();

                let walk_speed = if collision_state.ground { 50.0 } else { 10.0 };
                forces.add_force(Vector2::new(input.move_horizontal * walk_speed, 0.0)); // TODO: constant for speed

                if collision_state.ground && input.jumping && collision_state.jump_cooldown <= std::f32::EPSILON {
                    forces.add_force(Vector2::new(0.0, -1000.0));
                    collision_state.jump_cooldown = 0.25;
                }
            }
    }
}

pub struct PhysicsExtras;

impl<'a> System<'a> for PhysicsExtras {
    type SystemData = (
        RigidBodyParts2<'a, f32, BodyPose2<f32>, ()>,
        ReadStorage<'a, CollisionState>,
        Fetch<'a, DeltaTime<f32>>,
    );

    fn run(&mut self, (mut rigid_body_parts, collision_state, delta_time): Self::SystemData) {
        for (mut forces, mut velocity, mass, collision_state) in (
            &mut rigid_body_parts.forces,
            &mut rigid_body_parts.velocities,
            &rigid_body_parts.masses,
            &collision_state,
        ).join()
            {
                let mass_value = mass.mass();
                forces.add_force(Vector2::new(0.0, GRAVITY_ACCEL * mass_value)); // TODO: constant for speed

                let friction =
                    if collision_state.ground { 0.1 } else { 0.02 } / delta_time.delta_seconds;

                let linear = velocity.linear();
                forces.add_force(Vector2::new(
                    -linear.x * friction * mass_value,
                    -linear.y * friction * mass_value,
                ));
            }
    }
}

pub struct GuardAiSystem;

impl<'a> System<'a> for GuardAiSystem {
    #![allow(clippy)]
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, CollisionState>,
        WriteStorage<'a, GuardAi>,
        WriteStorage<'a, Directional>,
        RigidBodyParts2<'a, f32, BodyPose2<f32>, ()>,
        FetchMut<'a, GlobalPlayerState>,
        Fetch<'a, GuardJumpBoxes>,
        Fetch<'a, GuardTurnAroundBoxes>,
        Fetch<'a, DynamicBoundingVolumeTree2<f32>>,
        Fetch<'a, DeltaTime<f32>>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut collision_state,
            mut ai,
            mut directional,
            mut rigid_body_parts,
            mut player_state,
            jump_boxes,
            turn_around_boxes,
            tree,
            delta_time,
        ): Self::SystemData,
    ) {
        //        use entity::components::GuardAi::*;
        for (entity, mut collision_state, mut ai, mut directional, mut pose, mut shape, mut forces) in
            (
                &*entities,
                &mut collision_state,
                &mut ai,
                &mut directional,
                &mut rigid_body_parts.poses,
                &mut rigid_body_parts.shapes,
                &mut rigid_body_parts.forces,
            ).join()
            {
                let position = pose.position();

                let ray_dir = player_state.pos - position;

                let mut found_player = false;

                if ray_dir.x * directional.multiplier() > 0.0 || ai.state != GuardAiState::Patrolling {
                    let ray = Ray2::new(
                        Point2::new(position.x, position.y - ::GUARD_SIZE.1 * 0.25),
                        ray_dir,
                    );
                    //            let angle = ray_dir.angle().0; // FIXME: make sure it actually check if in bounds of vision (60deg)

                    use ord_subset::OrdSubsetIterExt;
                    let collision = query_ray(&*tree, ray)
                        .into_iter()
                        .filter(|&(hit, _point)| hit.value.id() != entity.id())
                        .ord_subset_min_by_key(|&(_hit, point)| (point - ray.origin).dot(ray.direction));
                    match collision {
                        None => (),
                        Some((hit, _point)) => found_player = hit.value.id() == player_state.id,
                    }
                }

                if ai.turn_around_cooldown > 0.0 {
                    ai.turn_around_cooldown = (ai.turn_around_cooldown - delta_time.delta_seconds).max(0.0);
                }

                let player_direction = if player_state.pos.x > pose.position().x {
                    ::entity::components::Direction::Right
                } else {
                    ::entity::components::Direction::Left
                };

                let bound = shape.bound();

                ai.state = match ai.state {
                    GuardAiState::Searching => {
                        if found_player {
                            GuardAiState::Chasing
                        } else {
                            if collision_state.ground {
                                for (direction_box, aabb) in
                                    jump_boxes.0.iter().map(|b| (b.get_direction(), b.get_aabb2()))
                                    {
                                        if bound.intersects(aabb) {
                                            if player_direction == direction_box && collision_state.jump_cooldown <= std::f32::EPSILON {
                                                forces.add_force(Vector2::new(0.0, -1000.0));
                                                collision_state.jump_cooldown = 0.25;
                                            }
                                        }
                                    }
                            }

                            GuardAiState::Searching
                        }
                    }
                    GuardAiState::Chasing => {
                        if !found_player {
                            GuardAiState::Searching
                        } else {
                            if bound.intersects(&player_state.bounds) {
                                player_state.captured = true;
                            }

                            let player_above = player_state.pos.y < pose.position().y;

                            let walk_speed = if collision_state.ground { 25.0 } else { 10.0 };
                            forces.add_force(Vector2::new(player_direction.multiplier() * walk_speed, 0.0));

                            if collision_state.ground {
                                for (direction_box, aabb) in
                                    jump_boxes.0.iter().map(|b| (b.get_direction(), b.get_aabb2()))
                                    {
                                        if bound.intersects(aabb) {
                                            // TODO embed at map level
                                            // FIXME for good measure
                                            // TODO check if should jump always - MUST DO THIS ACTUALLY PROPERLY
                                            if player_direction == direction_box && player_above && collision_state.jump_cooldown <= std::f32::EPSILON {
                                                forces.add_force(Vector2::new(0.0, -1000.0));
                                                collision_state.jump_cooldown = 0.25;
                                            }
                                        }
                                    }
                            }

                            GuardAiState::Chasing
                        }
                    }
                    GuardAiState::Patrolling => {
                        if found_player {
                            GuardAiState::Chasing
                        } else {
                            let walk_speed = if collision_state.ground { 7.0 } else { 5.0 };
                            forces.add_force(Vector2::new(
                                directional.direction.multiplier() * walk_speed,
                                0.0,
                            ));

                            let bound = shape.bound();

                            for aabb in turn_around_boxes.0.iter() {
                                if bound.intersects(aabb) && ai.turn_around_cooldown <= std::f32::EPSILON {
                                    directional.direction = directional.direction.invert();
                                    ai.turn_around_cooldown = 0.75;
                                }
                            }

                            GuardAiState::Patrolling
                        }
                    }
                }
            }
    }
}
