use gfx;
use gfx::handle::{RenderTargetView, DepthStencilView};
use gfx::traits::{Factory, FactoryExt};
use gfx::{Encoder, PipelineState};
use gfx_device_gl as gl;

use super::{ColorFormat, DepthFormat};

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "a_Pos",
        color: [f32; 4] = "a_Color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

pub trait Render {
    fn render_fan<V>(&mut self, iter: V)
    where V: ::std::iter::IntoIterator<Item=Vertex>;
}

pub struct Renderer {
    factory: gl::Factory,
    encoder: Encoder<gl::Resources, gl::CommandBuffer>,
    out_color: RenderTargetView<gl::Resources, ColorFormat>,
    pso: PipelineState<gl::Resources, pipe::Meta>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Renderer {
    pub fn new(
        mut factory: gl::Factory,
        encoder: Encoder<gl::Resources, gl::CommandBuffer>,
        out_color: RenderTargetView<gl::Resources, ColorFormat>
    ) -> Self {
        use gfx::state::{Rasterizer, MultiSample};

        let vs = factory.create_shader_vertex(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/plain_150.glslv"))
        ).unwrap();
        let ps = factory.create_shader_pixel(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/plain_150.glslf"))
        ).unwrap();

        let pso = factory.create_pipeline_state(
            &::gfx::ShaderSet::Simple(vs, ps),
            ::gfx::Primitive::TriangleList,
            Rasterizer {
                samples: Some(MultiSample),
                ..Rasterizer::new_fill()
            },
            pipe::new(),
        ).expect("Failed to create a PSO");

        Renderer {
            factory, encoder, pso, out_color,
            vertices: vec![],
            indices: vec![],
        }
    }

    pub fn update_views(&mut self, window: &::glutin::GlWindow, depth: &mut DepthStencilView<gl::Resources, DepthFormat>) {
        ::gfx_glutin::update_views(&window, &mut self.out_color, depth)
    }

    pub fn draw(&mut self, device: &mut gl::Device) {
        let (vbuf, sl) =
            self.factory.create_vertex_buffer_with_slice(&self.vertices, &*self.indices);
        let data = pipe::Data {
            vbuf,
            out: self.out_color.clone(),
        };

        self.encoder.clear(&data.out, BLACK);
        self.encoder.draw(&sl, &self.pso, &data);
        self.encoder.flush(device);

        self.vertices.clear();
        self.indices.clear();
    }
}

impl Render for Renderer {
    fn render_fan<V>(&mut self, iter: V)
    where V: ::std::iter::IntoIterator<Item=Vertex> {
        let i0 = self.vertices.len() as u16;
        let mut vs = iter.into_iter();
        self.vertices.push(vs.next().unwrap());
        self.vertices.push(vs.next().unwrap());
        for (i, v) in vs.enumerate() {
            let i = i as u16 + 1;
            self.vertices.push(v);
            self.indices.extend(&[i0, i0+i, i0+i+1]);
        }
    }
}
