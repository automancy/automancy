use automancy_data::colors;
use yakui::{
    Color, Rect, TextureId, Vec2, Vec4,
    paint::{PaintDom, PaintMesh, Pipeline, Vertex},
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

pub struct PaintRectLerpedColor {
    pub rect: Rect,
    pub color: (Color, Color, Color, Color),
    pub texture: Option<(TextureId, Rect)>,
    pub pipeline: Pipeline,
}

fn lerp_color(uv: Vec2, color: (Color, Color, Color, Color)) -> Vec4 {
    let (x0, y0, x1, y1) = color;

    let x = x0.to_linear().lerp(x1.to_linear(), uv.x);
    let y = y0.to_linear().lerp(y1.to_linear(), uv.y);

    ((x + y) / 2.0).with_w(x.w + y.w * (1.0 - x.w))
}

impl PaintRectLerpedColor {
    /// Create a new `PaintRect` with the default pipeline, no texture, and the
    /// given rectangle.
    ///
    /// (0, 0) is the top-left corner of the screen, while (1, 1) is the
    /// bottom-right corner of the screen. Widgets must take the viewport into
    /// account.
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            color: (Color::RED, Color::GREEN, Color::BLUE, Color::WHITE),
            texture: None,
            pipeline: Pipeline::Main,
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
        mesh.pipeline = self.pipeline;

        output.add_mesh(mesh);
    }
}

pub struct RoundedRectLerpedColor {
    pub rect: Rect,
    pub radius: f32,
    pub color: (Color, Color, Color, Color),
    pub texture: Option<(TextureId, Rect)>,
}

impl RoundedRectLerpedColor {
    pub fn new(rect: Rect, radius: f32) -> Self {
        Self {
            rect,
            radius,
            color: (colors::BLACK, colors::BLACK, colors::BLACK, colors::BLACK),
            texture: None,
        }
    }

    pub fn add(&self, output: &mut PaintDom) {
        let rect = self.rect;

        // We are not prepared to let a corner's radius be bigger than a side's
        // half-length.
        let radius = self
            .radius
            .min(rect.size().x / 2.0)
            .min(rect.size().y / 2.0);

        // Fallback to a rectangle if the radius is too small.
        if radius < 1.0 {
            let mut p = PaintRectLerpedColor::new(rect);
            p.texture = self.texture;
            p.color = self.color;
            return p.add(output);
        }

        let color = self.color;

        let slices = f32::ceil(TAU / 8.0 / f32::acos(1.0 - 0.2 / radius)) as u32;

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

        let create_vertex = |pos, vert| Vertex::new(pos, calc_uv(pos), lerp_color(vert, color));

        let mut rectangle = |min: Vec2, max: Vec2| {
            let base_vertex = vertices.len();

            let size = max - min;
            let rect_vertices = RECT_POS.map(Vec2::from).map(|vert| {
                let pos = vert * size + min;
                create_vertex(pos, (pos - rect.pos()) / rect.size())
            });

            let rect_indices = RECT_INDEX.map(|index| index + base_vertex as u16);

            vertices.extend(rect_vertices);
            indices.extend(rect_indices);
        };

        rectangle(
            Vec2::new(rect.pos().x + radius, rect.pos().y),
            Vec2::new(rect.max().x - radius, rect.pos().y + radius),
        );
        rectangle(
            Vec2::new(rect.pos().x, rect.pos().y + radius),
            Vec2::new(rect.max().x, rect.max().y - radius),
        );
        rectangle(
            Vec2::new(rect.pos().x + radius, rect.max().y - radius),
            Vec2::new(rect.max().x - radius, rect.max().y),
        );

        let mut corner = |center: Vec2, start_angle: f32| {
            let pos = (center - rect.pos()) / rect.size();

            let center_vertex = vertices.len();
            vertices.push(create_vertex(center, pos));

            let first_offset = radius * Vec2::new(start_angle.cos(), -start_angle.sin());
            vertices.push(create_vertex(center + first_offset, pos));

            for i in 1..=slices {
                let percent = i as f32 / slices as f32;
                let angle = start_angle + percent * TAU / 4.0;
                let offset = radius * Vec2::new(angle.cos(), -angle.sin());
                let index = vertices.len();
                vertices.push(create_vertex(center + offset, pos));

                indices.extend_from_slice(&[
                    center_vertex as u16,
                    (index - 1) as u16,
                    index as u16,
                ]);
            }
        };

        corner(Vec2::new(rect.max().x - radius, rect.pos().y + radius), 0.0);
        corner(
            Vec2::new(rect.pos().x + radius, rect.pos().y + radius),
            TAU / 4.0,
        );
        corner(
            Vec2::new(rect.pos().x + radius, rect.max().y - radius),
            TAU / 2.0,
        );
        corner(
            Vec2::new(rect.max().x - radius, rect.max().y - radius),
            3.0 * TAU / 4.0,
        );

        let mut mesh = PaintMesh::new(vertices, indices);
        mesh.texture = self.texture;
        output.add_mesh(mesh);
    }
}
