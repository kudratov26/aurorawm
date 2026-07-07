use std::time::{Duration, Instant};

pub fn parse_color(hex: &str) -> Option<[f32; 4]> {
    let hex = hex.trim_start_matches('#');
    
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
        Some([r, g, b, 1.0])
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
        let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
        Some([r, g, b, a])
    } else {
        None
    }
}

pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

pub struct FpsCounter {
    frame_count: u32,
    last_update: Instant,
    fps: f32,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            last_update: Instant::now(),
            fps: 0.0,
        }
    }
    
    pub fn frame(&mut self) -> Option<f32> {
        self.frame_count += 1;
        
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        
        if elapsed >= Duration::from_secs(1) {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_update = now;
            Some(self.fps)
        } else {
            None
        }
    }
    
    pub fn fps(&self) -> f32 {
        self.fps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("#ff0000"), Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(parse_color("#00ff00"), Some([0.0, 1.0, 0.0, 1.0]));
        assert_eq!(parse_color("#0000ff"), Some([0.0, 0.0, 1.0, 1.0]));
        assert_eq!(parse_color("#ff000080"), Some([1.0, 0.0, 0.0, 0.5]));
    }
}
