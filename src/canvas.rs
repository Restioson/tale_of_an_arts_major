use ggez::{self, graphics};
use ggez::error::GameResult;
use gui;
use image;
use image::{DynamicImage, Pixel, Rgb, Rgba, RgbaImage};
use itertools::Itertools;
use std::path::Path;

const IMAGE_SIZE: u16 = 128;
const SPACING: f32 = 4.0;
const BUTTON_SIZE: f32 = 20.0;
const IMAGE_DRAW_LEFT: f32 = (::SCALED_SIZE.0) / 2.0 - IMAGE_SIZE as f32 - SPACING;
const IMAGE_DRAW_RIGHT: f32 = (::SCALED_SIZE.0) / 2.0 + SPACING;
const IMAGE_DRAW_TOP: f32 = (::SCALED_SIZE.1 - IMAGE_SIZE as f32) / 2.0;
const IMAGE_DRAW_BOTTOM: f32 = (::SCALED_SIZE.1 + IMAGE_SIZE as f32) / 2.0;

// TODO: Don't really want to clone this...
#[derive(Clone)]
enum CanvasButton {
    ModSize(i32),
    ColorPalette(Rgba<u8>),
    Done,
}

impl gui::ButtonType<CanvasState> for CanvasButton {
    fn perform(&self, state: &mut CanvasState) {
        match *self {
            CanvasButton::ModSize(delta) => {
                state.brush_size = (state.brush_size as i32 + delta).min(5).max(1) as u8;
            }
            CanvasButton::ColorPalette(color) => {
                state.selected_color = color;
            }
            CanvasButton::Done => {
                // TODO: Restioson
            }
        }
    }
}

#[derive(Clone)]
pub struct CanvasState {
    selected_color: Rgba<u8>,
    brush_size: u8,
}

impl gui::GuiState for CanvasState {}

/// An in game painting canvas for drawing to
pub struct PaintingCanvas {
    original_gpu_image: graphics::Image,
    reproduction: RgbaImage,
    reproduction_gpu_image: Option<graphics::Image>,
    color_palette: Vec<Rgb<u8>>,
    changed: bool,
    component_holder: gui::GuiComponents<CanvasState, CanvasButton>,
    mouse_down: bool,
    last_draw_point: Option<(f32, f32)>,
    state: CanvasState,
}

impl PaintingCanvas {
    pub fn from_path<P: AsRef<Path>>(ctx: &mut ggez::Context, path: P) -> Self {
        let original = image::open(path).expect("Error opening image!");

        let color_palette: Vec<Rgb<u8>> = original.to_rgb().pixels().unique().cloned().collect();
        assert!(
            color_palette.len() <= 16,
            "Paintings to reproduce cannot have more than 16 colours!"
        );

        let mut buttons = vec![
            gui::Button::new(
                CanvasButton::ModSize(1),
                graphics::Point2::new(IMAGE_DRAW_LEFT, IMAGE_DRAW_BOTTOM + SPACING),
                graphics::Point2::new(BUTTON_SIZE, BUTTON_SIZE),
                Some(graphics::Image::new(ctx, "/plus_button.png").expect("Error loading image!")),
                graphics::Color::new(0.9, 0.9, 0.9, 1.0),
                graphics::Color::new(0.2, 0.6, 0.2, 1.0),
            ),
            gui::Button::new(
                CanvasButton::ModSize(-1),
                graphics::Point2::new(
                    IMAGE_DRAW_LEFT + BUTTON_SIZE + SPACING,
                    IMAGE_DRAW_BOTTOM + SPACING,
                ),
                graphics::Point2::new(BUTTON_SIZE, BUTTON_SIZE),
                Some(graphics::Image::new(ctx, "/minus_button.png").expect("Error loading image!")),
                graphics::Color::new(0.9, 0.9, 0.9, 1.0),
                graphics::Color::new(0.2, 0.6, 0.2, 1.0),
            ),
            gui::Button::new(
                CanvasButton::Done,
                graphics::Point2::new(
                    IMAGE_DRAW_RIGHT + IMAGE_SIZE as f32 - BUTTON_SIZE,
                    IMAGE_DRAW_BOTTOM + SPACING,
                ),
                graphics::Point2::new(BUTTON_SIZE, BUTTON_SIZE),
                Some(graphics::Image::new(ctx, "/done_button.png").expect("Error loading image!")),
                graphics::Color::new(0.9, 0.9, 0.9, 1.0),
                graphics::Color::new(0.2, 0.6, 0.2, 1.0),
            ),
        ];

        let center_index = (color_palette.len() as f32 - 1.0) / 2.0;
        for (index, color) in color_palette.iter().enumerate() {
            let draw_color = graphics::Color::new(
                color.data[0] as f32 / 255.0,
                color.data[1] as f32 / 255.0,
                color.data[2] as f32 / 255.0,
                1.0,
            );
            buttons.push(gui::Button::new(
                CanvasButton::ColorPalette(color.to_rgba()),
                graphics::Point2::new(
                    (::SCALED_SIZE.0 - BUTTON_SIZE) / 2.0
                        + (index as f32 - center_index) * (BUTTON_SIZE + SPACING),
                    IMAGE_DRAW_TOP - BUTTON_SIZE - SPACING,
                ),
                graphics::Point2::new(BUTTON_SIZE, BUTTON_SIZE),
                None,
                draw_color,
                graphics::Color::new(
                    draw_color.r * 0.7,
                    draw_color.g * 0.7,
                    draw_color.b * 0.7,
                    1.0,
                ),
            ));
        }

        let selected_color = color_palette[0].to_rgba();
        PaintingCanvas {
            original_gpu_image: graphics::Image::from_rgba8(
                ctx,
                IMAGE_SIZE,
                IMAGE_SIZE,
                &original.to_rgba().into_raw(),
            ).unwrap(),
            reproduction: DynamicImage::new_rgba8(IMAGE_SIZE as u32, IMAGE_SIZE as u32).to_rgba(),
            reproduction_gpu_image: None,
            color_palette,
            changed: false,
            component_holder: gui::GuiComponents::new(buttons),
            mouse_down: false,
            last_draw_point: None,
            state: CanvasState {
                selected_color,
                brush_size: 2,
            },
        }
    }

    fn set_pixel(&mut self, x: u32, y: u32, color: Rgba<u8>) {
        self.reproduction.put_pixel(x, y, color);
        self.changed = true;
    }

    pub fn paint_line(
        &mut self,
        (origin_x, origin_y): (f32, f32),
        (target_x, target_y): (f32, f32),
    ) {
        // Find the longest axis, which will be used for step between intermediate points
        let step_count = (target_x - origin_x)
            .abs()
            .max((target_y - origin_y).abs())
            .ceil();

        // If the two points we're given are the same, just draw a point on its own
        if step_count > 0.0 {
            let step_size = 1.0 / step_count;

            for step in 0..step_count as i32 {
                let intermediate = step as f32 * step_size;
                let intermediate_x = origin_x + (target_x - origin_x) * intermediate;
                let intermediate_y = origin_y + (target_y - origin_y) * intermediate;
                self.paint_point((intermediate_x, intermediate_y));
            }
        } else {
            self.paint_point((origin_x, origin_y));
        }
    }

    pub fn paint_point(&mut self, (x, y): (f32, f32)) {
        let (x, y) = ((x - IMAGE_DRAW_LEFT) as i32, (y - IMAGE_DRAW_TOP) as i32);
        let radius = self.state.brush_size as i32;
        let radius_squared = radius * radius;

        // Iterate through everything in a square around the point and then only add points that
        // would be in the circle
        for offset_x in -radius..radius + 1 {
            for offset_y in -radius..radius + 1 {
                // If point is in range of the circle
                if (offset_x * offset_x + offset_y * offset_y) <= radius_squared {
                    let global_x = x + offset_x;
                    let global_y = y + offset_y;
                    // Make sure we don't draw outside of the image bounds
                    if global_x >= 0 && global_y >= 0
                        && global_x < self.original_gpu_image.width() as i32
                        && global_y < self.original_gpu_image.height() as i32
                        {
                            self.set_pixel(global_x as u32, global_y as u32, self.state.selected_color);
                        }
                }
            }
        }
    }

    /// Returns a ggez `graphics::Image` for the reproduction (first in tuple) and the original
    /// (second in tuple)
    pub fn ggez_images<'b>(
        &'b mut self,
        ctx: &mut ggez::Context,
    ) -> (&'b graphics::Image, &'b graphics::Image) {
        (
            {
                if self.changed || self.reproduction_gpu_image.is_none() {
                    self.changed = false;
                    self.reproduction_gpu_image = Some(
                        graphics::Image::from_rgba8(
                            ctx,
                            IMAGE_SIZE,
                            IMAGE_SIZE,
                            &self.reproduction,
                        ).expect("Image invalid!"),
                    );
                }

                self.reproduction_gpu_image.as_ref().unwrap()
            },
            &self.original_gpu_image,
        )
    }

    pub fn in_drawing_canvas(mouse_x: f32, mouse_y: f32) -> bool {
        mouse_x >= IMAGE_DRAW_LEFT && mouse_x <= IMAGE_DRAW_RIGHT && mouse_y <= IMAGE_DRAW_BOTTOM
            && mouse_y >= IMAGE_DRAW_TOP
    }
}

impl gui::Gui for PaintingCanvas {
    fn update(&mut self, mouse_x: f32, mouse_y: f32) -> GameResult<()> {
        if self.mouse_down {
            if PaintingCanvas::in_drawing_canvas(mouse_x, mouse_y) {
                let current_point = (mouse_x, mouse_y);
                match self.last_draw_point {
                    Some(point) => self.paint_line(point, current_point),
                    None => self.paint_point(current_point),
                }
                self.last_draw_point = Some(current_point);
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut ggez::Context, _font: &graphics::Font, mouse_x: f32, mouse_y: f32) -> GameResult<()> {
        gui::draw_rectangle(ctx, graphics::Point2::new(0.0, 0.0), graphics::Point2::new(::SCALED_SIZE.0, ::SCALED_SIZE.1), graphics::Color::new(0.1, 0.1, 0.1, 0.8))?;

        let positions_of_canvases: [graphics::Point2; 2] = [
            graphics::Point2::new(IMAGE_DRAW_LEFT, IMAGE_DRAW_TOP),
            graphics::Point2::new(IMAGE_DRAW_RIGHT, IMAGE_DRAW_TOP),
        ];

        let (reproduction, original) = self.ggez_images(ctx);
        let both = [reproduction, original];

        for (image, pos) in both.iter().zip(positions_of_canvases.iter()) {
            graphics::rectangle(
                ctx,
                graphics::DrawMode::Fill,
                graphics::Rect::new(
                    (pos.x - 1.0) * ::GLOBAL_SCALE,
                    (pos.y - 1.0) * ::GLOBAL_SCALE,
                    (image.width() as f32 + 2.0) * ::GLOBAL_SCALE,
                    (image.height() as f32 + 2.0) * ::GLOBAL_SCALE,
                ),
            )?;

            graphics::draw_ex(
                ctx,
                *image,
                graphics::DrawParam {
                    src: graphics::Rect::one(),
                    dest: graphics::Point2::new(pos.x * ::GLOBAL_SCALE, pos.y * ::GLOBAL_SCALE),
                    rotation: 0.0,
                    scale: graphics::Point2::new(::GLOBAL_SCALE, ::GLOBAL_SCALE),
                    offset: graphics::Point2::new(0.0, 0.0),
                    shear: graphics::Point2::new(0.0, 0.0),
                    color: None,
                },
            )?;
        }

        self.component_holder.draw(ctx, mouse_x, mouse_y)?;

        Ok(())
    }

    fn mouse_pressed(&mut self, mouse_x: f32, mouse_y: f32) {
        use std::clone::Clone;
        // FIXME: Dirty hack...
        let mut state = self.state.clone();
        self.component_holder.mouse_pressed(&mut state, mouse_x, mouse_y);
        self.state = state;

        self.mouse_down = true;
    }

    fn mouse_released(&mut self, _mouse_x: f32, _mouse_y: f32) {
        self.last_draw_point = None;
        self.mouse_down = false;
    }
}
