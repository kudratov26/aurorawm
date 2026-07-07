use anyhow::Result;
use smithay::backend::renderer::gles2::Gles2Renderer;
use smithay::utils::{Rectangle, Size, Transform, Physical};
use crate::output::OutputManager;

pub struct AuroraRenderer {
    pub renderer: Option<Gles2Renderer>,
    pub clear_color: [f32; 4],
}

impl AuroraRenderer {
    pub fn new(output_manager: &OutputManager) -> Result<Self> {
        // Get the first GPU's renderer
        let renderer = output_manager.get_renderer().cloned();
        
        Ok(Self {
            renderer,
            clear_color: [0.1, 0.1, 0.1, 1.0], // Dark background
        })
    }
    
    pub fn render(&mut self, output_size: Size<u32, Physical>) -> Result<()> {
        if let Some(renderer) = &mut self.renderer {
            // Start a new frame
            let mut frame = renderer.render(
                output_size,
                Transform::Normal,
            )?;
            
            // Clear the frame
            frame.clear(
                self.clear_color[0],
                self.clear_color[1],
                self.clear_color[2],
                self.clear_color[3],
            )?;
            
            // TODO: Render windows, borders, shadows, etc.
            
            // Submit the frame
            frame.submit()?;
        }
        
        Ok(())
    }
    
    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color = [r, g, b, a];
    }
    
    pub fn is_available(&self) -> bool {
        self.renderer.is_some()
    }
}

// Animation system for smooth transitions
pub struct AnimationEngine {
    pub animations: Vec<Animation>,
    pub enabled: bool,
    pub duration_ms: u32,
}

pub struct Animation {
    pub window_id: u32,
    pub start: Rectangle<i32, i32>,
    pub end: Rectangle<i32, i32>,
    pub progress: f64,
    pub easing: EasingFunction,
}

#[derive(Debug, Clone, Copy)]
pub enum EasingFunction {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
}

impl AnimationEngine {
    pub fn new(enabled: bool, duration_ms: u32) -> Self {
        Self {
            animations: Vec::new(),
            enabled,
            duration_ms,
        }
    }
    
    pub fn add_animation(&mut self, animation: Animation) {
        if self.enabled {
            self.animations.push(animation);
        }
    }
    
    pub fn update(&mut self, delta_ms: u32) {
        if !self.enabled {
            return;
        }
        
        let progress_step = delta_ms as f64 / self.duration_ms as f64;
        
        self.animations.retain(|anim| {
            anim.progress += progress_step;
            anim.progress <= 1.0
        });
    }
    
    pub fn get_current_geometry(&self, animation: &Animation) -> Rectangle<i32, i32> {
        let t = animation.easing.apply(animation.progress);
        
        let x = animation.start.loc.x as f64 + (animation.end.loc.x - animation.start.loc.x) as f64 * t;
        let y = animation.start.loc.y as f64 + (animation.end.loc.y - animation.start.loc.y) as f64 * t;
        let w = animation.start.size.w as f64 + (animation.end.size.w - animation.start.size.w) as f64 * t;
        let h = animation.start.size.h as f64 + (animation.end.size.h - animation.start.size.h) as f64 * t;
        
        Rectangle::from_loc_and_size(
            smithay::utils::Point::from((x as i32, y as i32)),
            smithay::utils::Size::from((w as i32, h as i32)),
        )
    }
}

impl EasingFunction {
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseInQuad => t * t,
            EasingFunction::EaseOutQuad => t * (2.0 - t),
            EasingFunction::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            EasingFunction::EaseInCubic => t * t * t,
            EasingFunction::EaseOutCubic => {
                let t = t - 1.0;
                t * t * t + 1.0
            }
            EasingFunction::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let t = t - 1.0;
                    4.0 * t * t * t + 1.0
                }
            }
        }
    }
}
