use ggez::graphics::{self, Image, Rect};
use specs::{Component, HashMapStorage, VecStorage, World};

/// Registers all the components from this module to the given world
pub fn register_components(world: &mut World) {
    world.register::<Player>();
    world.register::<Sprite>();
    world.register::<Directional>();
    world.register::<CollisionState>();
    world.register::<GuardAi>();
}

/// A component for the player only
pub struct Player;

impl Component for Player {
    type Storage = HashMapStorage<Self>;
}

// A component tracking the collision state such as whether it is on the ground or not
pub struct CollisionState {
    pub ground: bool,
    pub jump_cooldown: f32,
}

impl Component for CollisionState {
    type Storage = HashMapStorage<Self>;
}

pub struct GuardAi {
    pub state: GuardAiState,
    pub info: ::level::GuardInfo,
    pub turn_around_cooldown: f32,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum GuardAiState {
    Searching,
    Chasing,
    Patrolling,
}

impl Component for GuardAi {
    type Storage = HashMapStorage<Self>;
}

#[derive(Copy, Clone)]
pub struct Directional {
    pub direction: Direction,
}

impl Directional {
    /// What you need to multiply a speed by to go in this direction
    pub fn multiplier(&self) -> f32 {
        match self.direction {
            Direction::Left => -1.0,
            Direction::Right => 1.0,
        }
    }
}

impl Component for Directional {
    type Storage = VecStorage<Self>;
}

/// The direction something is loooking in or moving in
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    /// What you need to multiply a speed by to go in this direction
    pub fn multiplier(&self) -> f32 {
        match *self {
            Direction::Left => -1.0,
            Direction::Right => 1.0,
        }
    }

    pub fn invert(&self) -> Direction {
        match *self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

/// A component with a sprite to be drawn on screen
pub struct Sprite {
    pub image: Image,
    /// The rotation in radians
    pub rotation: f32,
    pub clip: Rect,
    pub scale: graphics::Point2,
}

impl Sprite {
    pub fn new(image: Image) -> Self {
        Sprite {
            image,
            rotation: 0.0,
            clip: Rect::one(),
            scale: graphics::Point2::new(1.0, 1.0),
        }
    }
}

impl Component for Sprite {
    type Storage = HashMapStorage<Self>;
}
