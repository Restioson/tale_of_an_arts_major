use cgmath::Point2;
use collision::Aabb2;

/// The rectangle of the player. Used in the guard ai.
pub struct GlobalPlayerState {
    pub pos: Point2<f32>,
    pub bounds: Aabb2<f32>,
    pub id: u32,
    pub captured: bool,
}

pub struct GuardJumpBoxes(pub Vec<::level::GuardJumpBox>);

pub struct GuardTurnAroundBoxes(pub Vec<::collision::Aabb2<f32>>);

#[derive(Clone)]
pub struct GameInput {
    pub move_horizontal: f32,
    pub jumping: bool,
}

impl GameInput {
    pub fn new() -> Self {
        GameInput {
            move_horizontal: 0.0,
            jumping: false,
        }
    }
}
