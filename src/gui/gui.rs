use ggez::{self, error::GameResult, graphics};

pub trait Gui {
    fn update(&mut self, mouse_x: f32, mouse_y: f32) -> GameResult<()>;

    fn draw(&mut self, ctx: &mut ggez::Context, mouse_x: f32, mouse_y: f32) -> GameResult<()>;

    fn mouse_pressed(&mut self, mouse_x: f32, mouse_y: f32);

    fn mouse_released(&mut self, mouse_x: f32, mouse_y: f32);
}

pub trait ButtonType<T: Gui> {
    fn perform(&self, gui: &mut T);
}

pub struct GuiComponents<G: Gui, T: ButtonType<G>> {
    buttons: Vec<Button<G, T>>,
}

impl<G: Gui, T: ButtonType<G>> GuiComponents<G, T> {
    pub fn new(buttons: Vec<Button<G, T>>) -> Self {
        GuiComponents { buttons }
    }

    pub fn draw(&self, gui: &mut G, ctx: &mut ggez::Context, mouse_x: f32, mouse_y: f32) -> GameResult<()> {
        for button in self.buttons {
            button.draw(ctx, mouse_x, mouse_y)?;
        }
        Ok(())
    }

    pub fn mouse_pressed(&self, gui: &mut G, mouse_x: f32, mouse_y: f32) {
        let button_types: Vec<ButtonType<G>> = self.buttons.iter()
            .filter(|button| button.is_selected(mouse_x, mouse_y))
            .map(|button| button.button_type.clone())
            .collect();
        for button_type in button_types {
            button_type.perform(gui);
        }
    }
}

pub struct Button<G: Gui, T: ButtonType<G>> {
    button_type: T,
    pub pos: graphics::Point2,
    pub size: graphics::Point2,
    icon: Option<graphics::Image>,
    fill_color: graphics::Color,
    hover_color: graphics::Color,
}

impl<G: Gui, T: ButtonType<G>> Button<G, T> {
    pub fn new(button_type: T, pos: graphics::Point2, size: graphics::Point2, icon: Option<graphics::Image>, fill_color: graphics::Color, hover_color: graphics::Color) -> Self {
        Button { button_type, pos, size, icon, fill_color, hover_color }
    }

    pub fn draw(&self, ctx: &mut ggez::Context, mouse_x: f32, mouse_y: f32) -> GameResult<()> {
        draw_rectangle(ctx, self.pos, self.size, if self.is_selected(mouse_x, mouse_y) {
            self.hover_color
        } else {
            self.fill_color
        })?;

        if let Some(icon) = self.icon.as_ref() {
            draw_texture(ctx, icon, graphics::Point2::new(
                self.pos.x + (self.size.x - icon.width() as f32) / 2.0,
                self.pos.y + (self.size.y - icon.height() as f32) / 2.0,
            ))?;
        }

        Ok(())
    }

    pub fn is_selected(&self, mouse_x: f32, mouse_y: f32) -> bool {
        mouse_x >= self.pos.x && mouse_y >= self.pos.y && mouse_x < self.pos.x + self.size.x
            && mouse_y < self.pos.y + self.size.y
    }
}

fn draw_rectangle(ctx: &mut ggez::Context, pos: graphics::Point2, size: graphics::Point2, color: graphics::Color) -> GameResult<()> {
    graphics::set_color(ctx, color)?;
    graphics::rectangle(ctx, graphics::DrawMode::Fill, graphics::Rect::new(
        pos.x * ::GLOBAL_SCALE,
        pos.y * ::GLOBAL_SCALE,
        size.x * ::GLOBAL_SCALE,
        size.y * ::GLOBAL_SCALE,
    ))?;

    graphics::set_color(ctx, graphics::Color::new(1.0, 1.0, 1.0, 1.0))?;

    Ok(())
}

fn draw_texture(ctx: &mut ggez::Context, icon: &graphics::Image, pos: graphics::Point2) -> GameResult<()> {
    graphics::draw_ex(
        ctx,
        icon,
        graphics::DrawParam {
            src: graphics::Rect::one(),
            dest: graphics::Point2::new(
                pos.x * ::GLOBAL_SCALE,
                pos.y * ::GLOBAL_SCALE,
            ),
            rotation: 0.0,
            scale: graphics::Point2::new(::GLOBAL_SCALE, ::GLOBAL_SCALE),
            offset: graphics::Point2::new(0.0, 0.0),
            shear: graphics::Point2::new(0.0, 0.0),
            color: None,
        },
    )
}
