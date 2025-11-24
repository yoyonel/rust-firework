use crate::physic_engine::PhysicEngineIterator;

pub trait RendererEngine {
    fn render_frame<P: PhysicEngineIterator>(&mut self, physic: &P) -> usize;
    fn set_window_size(&mut self, width: i32, height: i32);
    fn recreate_buffers(&mut self, max_particles: usize);
    fn close(&mut self);
}
