use crate::physic_engine::PhysicEngineIterator;
use crate::renderer_engine::BloomPass;

pub trait RendererEngine {
    fn render_frame<P: PhysicEngineIterator>(&mut self, physic: &P) -> usize;
    fn set_window_size(&mut self, width: i32, height: i32);
    fn recreate_buffers(&mut self, max_particles: usize);
    fn reload_shaders(&mut self) -> Result<(), String>;
    fn close(&mut self);
    fn bloom_pass_mut(&mut self) -> &mut BloomPass;
}
