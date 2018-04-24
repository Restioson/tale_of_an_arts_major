use ggez::{self, error::GameResult, graphics};
use std::clone::Clone;
use std::marker::PhantomData;

pub trait Gui {
    fn update(&mut self, mouse_x: f32, mouse_y: f32) -> GameResult<()>;

    fn draw(&mut self, ctx: &mut ggez::Context, font: &graphics::Font, mouse_x: f32, mouse_y: f32) -> GameResult<()>;

    fn mouse_pressed(&mut self, mouse_x: f32, mouse_y: f32);

    fn mouse_released(&mut self, mouse_x: f32, mouse_y: f32);
}

pub trait ButtonType<S: GuiState>: Clone {
    fn perform(&self, state: &mut S);
}

pub trait GuiState: Clone {}

pub struct GuiComponents<S: GuiState, T: ButtonType<S>> {
    _phantom: PhantomData<S>,
    buttons: Vec<Button<S, T>>,
}

impl<S: GuiState, T: ButtonType<S>> GuiComponents<S, T> {
    pub fn new(buttons: Vec<Button<S, T>>) -> Self {
        GuiComponents {
            _phantom: PhantomData,
            buttons,
        }
    }

    pub fn draw(
        &self,
        ctx: &mut ggez::Context,
        mouse_x: f32,
        mouse_y: f32,
    ) -> GameResult<()> {
        for button in self.buttons.iter() {
            button.draw(ctx, mouse_x, mouse_y)?;
        }
        Ok(())
    }

    pub fn mouse_pressed(&self, state: &mut S, mouse_x: f32, mouse_y: f32) {
        let button_types: Vec<Box<T>> = self.buttons
            .iter()
            .filter(|button| button.is_selected(mouse_x, mouse_y))
            .map(|button| button.button_type.clone())
            .collect();
        for button_type in button_types {
            button_type.perform(state);
        }
    }
}

pub struct Button<S: GuiState, T: ButtonType<S>> {
    _phantom: PhantomData<S>,
    button_type: Box<T>,
    pub pos: graphics::Point2,
    pub size: graphics::Point2,
    icon: Option<graphics::Image>,
    fill_color: graphics::Color,
    hover_color: graphics::Color,
}

impl<S: GuiState, T: ButtonType<S>> Button<S, T> {
    pub fn new(
        button_type: T,
        pos: graphics::Point2,
        size: graphics::Point2,
        icon: Option<graphics::Image>,
        fill_color: graphics::Color,
        hover_color: graphics::Color,
    ) -> Self {
        Button {
            _phantom: PhantomData,
            button_type: Box::new(button_type),
            pos,
            size,
            icon,
            fill_color,
            hover_color,
        }
    }

    pub fn draw(&self, ctx: &mut ggez::Context, mouse_x: f32, mouse_y: f32) -> GameResult<()> {
        draw_rectangle(
            ctx,
            self.pos,
            self.size,
            if self.is_selected(mouse_x, mouse_y) {
                self.hover_color
            } else {
                self.fill_color
            },
        )?;

        if let Some(icon) = self.icon.as_ref() {
            draw_texture(
                ctx,
                icon,
                graphics::Point2::new(
                    self.pos.x + (self.size.x - icon.width() as f32) / 2.0,
                    self.pos.y + (self.size.y - icon.height() as f32) / 2.0,
                ),
            )?;
        }

        Ok(())
    }

    pub fn is_selected(&self, mouse_x: f32, mouse_y: f32) -> bool {
        mouse_x >= self.pos.x && mouse_y >= self.pos.y && mouse_x < self.pos.x + self.size.x
            && mouse_y < self.pos.y + self.size.y
    }
}

pub fn draw_rectangle(
    ctx: &mut ggez::Context,
    pos: graphics::Point2,
    size: graphics::Point2,
    color: graphics::Color,
) -> GameResult<()> {
    graphics::set_color(ctx, color)?;
    graphics::rectangle(
        ctx,
        graphics::DrawMode::Fill,
        graphics::Rect::new(
            pos.x * ::GLOBAL_SCALE,
            pos.y * ::GLOBAL_SCALE,
            size.x * ::GLOBAL_SCALE,
            size.y * ::GLOBAL_SCALE,
        ),
    )?;

    graphics::set_color(ctx, graphics::Color::new(1.0, 1.0, 1.0, 1.0))?;

    Ok(())
}

pub fn draw_texture(
    ctx: &mut ggez::Context,
    icon: &graphics::Image,
    pos: graphics::Point2,
) -> GameResult<()> {
    graphics::draw_ex(
        ctx,
        icon,
        graphics::DrawParam {
            src: graphics::Rect::one(),
            dest: graphics::Point2::new(pos.x * ::GLOBAL_SCALE, pos.y * ::GLOBAL_SCALE),
            rotation: 0.0,
            scale: graphics::Point2::new(::GLOBAL_SCALE, ::GLOBAL_SCALE),
            offset: graphics::Point2::new(0.0, 0.0),
            shear: graphics::Point2::new(0.0, 0.0),
            color: None,
        },
    )
}
