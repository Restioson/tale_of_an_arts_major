use ggez::{self, GameResult, graphics};
use gui;

const CAPTURE_MESSAGE: &'static str = "You were captured!";

pub struct CaptureGui;

impl gui::Gui for CaptureGui {
    fn update(&mut self, _mouse_x: f32, _mouse_y: f32) -> GameResult<()> {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context, font: &graphics::Font, _mouse_x: f32, _mouse_y: f32) -> GameResult<()> {
        gui::draw_rectangle(ctx, graphics::Point2::new(0.0, 0.0), graphics::Point2::new(::SCALED_SIZE.0, ::SCALED_SIZE.1), graphics::Color::new(0.1, 0.1, 0.1, 0.8))?;

        let text = graphics::Text::new(ctx, CAPTURE_MESSAGE, font)?;
        let text_width = text.width() as f32;
        let text_height = text.height() as f32;
        graphics::draw_ex(ctx, &text, graphics::DrawParam {
            src: graphics::Rect::one(),
            dest: graphics::Point2::new((::SCREEN_SIZE.0 as f32 - text_width) / 2.0, (::SCREEN_SIZE.1 as f32 - text_height) / 2.0),
            rotation: 0.0,
            scale: graphics::Point2::new(1.0, 1.0),
            offset: graphics::Point2::new(0.0, 0.0),
            shear: graphics::Point2::new(0.0, 0.0),
            color: None,
        });

        Ok(())
    }

    fn mouse_pressed(&mut self, _mouse_x: f32, _mouse_y: f32) {}

    fn mouse_released(&mut self, _mouse_x: f32, _mouse_y: f32) {}
}
