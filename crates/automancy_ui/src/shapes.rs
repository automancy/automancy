pub use prelude::*;
mod prelude {
    pub use yakui::shapes::cross;
}

use automancy_data::math::consts;
use yakui::{
    Border, BorderRadius, Color, Rect, TextureId, Vec2, Vec4,
    paint::{PaintDom, PaintMesh, Pipeline, Vertex},
    util::auto_builders,
};

#[rustfmt::skip]
const RECT_POS: [[f32; 2]; 4] = [
    [0.0, 0.0],
    [0.0, 1.0],
    [1.0, 1.0],
    [1.0, 0.0]
];

#[rustfmt::skip]
const RECT_INDEX: [u16; 6] = [
    0, 1, 2,
    3, 0, 2,
];

fn lerp_color(uv: Vec2, color: (Color, Color, Color, Color)) -> Vec4 {
    let (x0, x1, y0, y1) = color;

    let x = x0.to_linear().lerp(x1.to_linear(), uv.x);
    let y = y0.to_linear().lerp(y1.to_linear(), uv.x);

    x.lerp(y, uv.y) /*.with_w(x.w + y.w * (1.0 - x.w)) */
}

/// See [`RoundRect`](crate::components::RoundRect).
pub struct PaintRect {
    pub rect: Rect,
    /// component order: (x0, x1, y0, y1)
    ///
    /// blended like this:
    /// ```txt
    /// x0－x1
    /// ｜　｜
    /// y0－y1
    /// ```
    pub color: (Color, Color, Color, Color),
    pub texture: Option<(TextureId, Rect)>,
}

auto_builders!(PaintRect {
    texture: Option<(TextureId, Rect)>,
});

impl PaintRect {
    /// Create a new `PaintRect` with the default pipeline, no texture, and the
    /// given rectangle.
    ///
    /// (0, 0) is the top-left corner of the screen, while (1, 1) is the
    /// bottom-right corner of the screen. Widgets must take the viewport into
    /// account.
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            color: (Color::WHITE, Color::WHITE, Color::WHITE, Color::WHITE),
            texture: None,
        }
    }

    pub fn color_xy(self, color: (Color, Color, Color, Color)) -> Self {
        Self {
            color,
            ..self
        }
    }

    pub fn color_x(self, (x0, x1): (Color, Color)) -> Self {
        Self {
            color: (x0, x1, x0, x1),
            ..self
        }
    }

    pub fn color_y(self, (y0, y1): (Color, Color)) -> Self {
        Self {
            color: (y0, y0, y1, y1),
            ..self
        }
    }

    pub fn color(self, color: Color) -> Self {
        Self {
            color: (color, color, color, color),
            ..self
        }
    }

    /// Add this rectangle to the PaintDom to be drawn this frame.
    pub fn add(&self, output: &mut PaintDom) {
        let size = self.rect.size();
        let pos = self.rect.pos();
        let color = self.color;
        let texture_rect = match self.texture {
            Some((_index, rect)) => rect,
            None => Rect::from_pos_size(Vec2::ZERO, Vec2::ONE),
        };

        let vertices = RECT_POS.map(Vec2::from).map(|vert| {
            Vertex::new(
                vert * size + pos,
                vert * texture_rect.size() + texture_rect.pos(),
                lerp_color(vert, color),
            )
        });

        let mut mesh = PaintMesh::new(vertices, RECT_INDEX);
        mesh.texture = self.texture;
        mesh.pipeline = Pipeline::Main;

        output.add_mesh(mesh);
    }
}

pub struct PaintRoundRect {
    pub rect: Rect,
    pub color: (Color, Color, Color, Color),
    pub radius: BorderRadius,
    pub texture: Option<(TextureId, Rect)>,
    pub border: Option<Border>,
}

auto_builders!(PaintRoundRect {
    radius: BorderRadius,
    texture: Option<(TextureId, Rect)>,
    border: Option<Border>,
});

impl PaintRoundRect {
    pub fn new<T: Into<BorderRadius>>(rect: Rect, radius: T) -> Self {
        Self {
            rect,
            color: (Color::WHITE, Color::WHITE, Color::WHITE, Color::WHITE),
            texture: None,
            radius: radius.into(),
            border: None,
        }
    }

    pub fn color_xy(self, color: (Color, Color, Color, Color)) -> Self {
        Self {
            color,
            ..self
        }
    }

    pub fn color_x(self, (x0, x1): (Color, Color)) -> Self {
        Self {
            color: (x0, x1, x0, x1),
            ..self
        }
    }

    pub fn color_y(self, (y0, y1): (Color, Color)) -> Self {
        Self {
            color: (y0, y0, y1, y1),
            ..self
        }
    }

    pub fn color(self, color: Color) -> Self {
        Self {
            color: (color, color, color, color),
            ..self
        }
    }

    pub fn add(&self, output: &mut PaintDom) {
        // Draw border background first if there's a border
        // TODO copied from yakui, ???, draws the border twice
        if let Some(border) = &self.border {
            self.draw_border(output, border);
        }

        let (rect, radius) = if let Some(border) = &self.border {
            let border_width = border.width;
            let inner_rect = Rect::from_pos_size(
                self.rect.pos() + Vec2::new(border_width, border_width),
                self.rect.size() - Vec2::new(border_width * 2.0, border_width * 2.0),
            );
            let inner_radius = BorderRadius {
                top_left: (self.radius.top_left - border_width).max(0.0),
                top_right: (self.radius.top_right - border_width).max(0.0),
                bottom_left: (self.radius.bottom_left - border_width).max(0.0),
                bottom_right: (self.radius.bottom_right - border_width).max(0.0),
            };
            (inner_rect, inner_radius)
        } else {
            (self.rect, self.radius)
        };

        let BorderRadius {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        } = radius;

        // We are not prepared to let a corner's radius be bigger than a side's
        // half-length.
        let max_radius = (rect.size().x / 2.0).min(rect.size().y / 2.0);

        let top_left = top_left.min(max_radius);
        let top_right = top_right.min(max_radius);
        let bottom_left = bottom_left.min(max_radius);
        let bottom_right = bottom_right.min(max_radius);

        // Fallback to a rectangle if the radius is too small.
        if top_left < 1.0 && top_right < 1.0 && bottom_left < 1.0 && bottom_right < 1.0 {
            let mut p = PaintRect::new(rect);
            p.texture = self.texture;
            p.color = self.color;
            return p.add(output);
        }

        let color = self.color;

        let max_radius = top_left.max(top_right).max(bottom_left).max(bottom_right);
        let slices = f32::ceil(consts::TAU / 8.0 / f32::acos(1.0 - 0.2 / max_radius)) as u32;

        // 3 rectangles and 4 corners
        let mut vertices = Vec::with_capacity(4 * 3 + (slices + 2) as usize * 4);
        let mut indices = Vec::with_capacity(6 * 3 + slices as usize * (3 * 4));

        let (uv_offset, uv_factor) = self
            .texture
            .map(|(_, texture_rect)| (texture_rect.pos(), texture_rect.size() / rect.size()))
            .unwrap_or((Vec2::ZERO, Vec2::ZERO));

        let calc_uv = |position| {
            if self.texture.is_none() {
                return Vec2::ZERO;
            }
            (position - rect.pos()) * uv_factor + uv_offset
        };

        let create_vertex = |pos| Vertex::new(pos, calc_uv(pos), lerp_color((pos - rect.pos()) / rect.size(), color));

        let mut rectangle = |min: Vec2, max: Vec2| {
            let base_vertex = vertices.len();

            let size = max - min;
            let rect_vertices = RECT_POS.map(Vec2::from).map(|vert| {
                let pos = vert * size + min;
                create_vertex(pos)
            });

            let rect_indices = RECT_INDEX.map(|index| index + base_vertex as u16);

            vertices.extend(rect_vertices);
            indices.extend(rect_indices);
        };

        // Top
        rectangle(
            Vec2::new(rect.pos().x + top_left, rect.pos().y),
            Vec2::new(rect.max().x - top_right, rect.pos().y + top_left.max(top_right)),
        );

        // Left
        rectangle(
            Vec2::new(rect.pos().x, rect.pos().y + top_left),
            Vec2::new(rect.pos().x + bottom_left.max(top_left), rect.max().y - bottom_left),
        );

        // Center
        rectangle(
            Vec2::new(rect.pos().x + bottom_left.max(top_left), rect.pos().y + top_left.max(top_right)),
            Vec2::new(
                rect.max().x - bottom_right.max(top_right),
                rect.max().y - bottom_left.max(bottom_right),
            ),
        );

        // Right
        rectangle(
            Vec2::new(rect.max().x - bottom_right.max(top_right), rect.pos().y + top_right),
            Vec2::new(rect.max().x, rect.max().y - bottom_right),
        );

        // Bottom
        rectangle(
            Vec2::new(rect.pos().x + bottom_left, rect.max().y - bottom_left.max(bottom_right)),
            Vec2::new(rect.max().x - bottom_right, rect.max().y),
        );

        let mut corner = |center: Vec2, radius: f32, start_angle: f32| {
            if radius < 1.0 {
                return;
            }

            let center_vertex = vertices.len();
            vertices.push(create_vertex(center));

            let first_offset = radius * Vec2::new(start_angle.cos(), -start_angle.sin());
            vertices.push(create_vertex(center + first_offset));

            for i in 1..=slices {
                let percent = i as f32 / slices as f32;
                let angle = start_angle + percent * consts::TAU / 4.0;
                let offset = radius * Vec2::new(angle.cos(), -angle.sin());
                let index = vertices.len();
                vertices.push(create_vertex(center + offset));

                indices.extend_from_slice(&[center_vertex as u16, (index - 1) as u16, index as u16]);
            }
        };

        corner(Vec2::new(rect.max().x - top_right, rect.pos().y + top_right), top_right, 0.0);
        corner(
            Vec2::new(rect.pos().x + top_left, rect.pos().y + top_left),
            top_left,
            consts::TAU / 4.0,
        );
        corner(
            Vec2::new(rect.pos().x + bottom_left, rect.max().y - bottom_left),
            bottom_left,
            consts::TAU / 2.0,
        );
        corner(
            Vec2::new(rect.max().x - bottom_right, rect.max().y - bottom_right),
            bottom_right,
            3.0 * consts::TAU / 4.0,
        );

        // TODO copied from yakui, ???, draws the border twice
        if let Some(border) = &self.border {
            self.draw_border(output, border);
        }

        let mut mesh = PaintMesh::new(vertices, indices);
        mesh.texture = self.texture;
        output.add_mesh(mesh);
    }

    fn draw_border(&self, output: &mut PaintDom, border: &Border) {
        PaintRoundRect::new(self.rect, self.radius).color(border.color).add(output);
    }
}
