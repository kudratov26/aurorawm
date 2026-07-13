use smithay::utils::{Logical, Point, Rectangle, Size};
use std::collections::HashMap;

use crate::config::Config;

pub struct LayoutEngine {
    pub config: crate::config::LayoutConfig,
    pub current_layout: LayoutType,
    pub windows: HashMap<u32, Window>,
    pub focused_window: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    Dwindle,
    Master,
    Spiral,
    Columns,
    Grid,
    Floating,
}

pub struct Window {
    pub id: u32,
    pub geometry: Rectangle<i32, Logical>,
    pub is_floating: bool,
    pub is_fullscreen: bool,
    pub is_focused: bool,
}

impl LayoutEngine {
    pub fn new(config: &Config) -> Self {
        let current_layout = match config.layout.default.as_str() {
            "master" => LayoutType::Master,
            "spiral" => LayoutType::Spiral,
            "columns" => LayoutType::Columns,
            "grid" => LayoutType::Grid,
            "floating" => LayoutType::Floating,
            _ => LayoutType::Dwindle,
        };

        Self {
            config: config.layout.clone(),
            current_layout,
            windows: HashMap::new(),
            focused_window: None,
        }
    }

    pub fn add_window(&mut self, id: u32, geometry: Rectangle<i32, Logical>) {
        let window = Window {
            id,
            geometry,
            is_floating: false,
            is_fullscreen: false,
            is_focused: false,
        };
        self.windows.insert(id, window);
    }

    pub fn remove_window(&mut self, id: u32) {
        self.windows.remove(&id);
        if self.focused_window == Some(id) {
            self.focused_window = None;
        }
    }

    pub fn focus_window(&mut self, id: u32) {
        if let Some(window) = self.windows.get_mut(&id) {
            window.is_focused = true;
        }
        if let Some(prev_focused) = self.focused_window {
            if let Some(window) = self.windows.get_mut(&prev_focused) {
                window.is_focused = false;
            }
        }
        self.focused_window = Some(id);
    }

    pub fn set_layout(&mut self, layout: LayoutType) {
        self.current_layout = layout;
    }

    pub fn arrange(&mut self, output_area: Rectangle<i32, Logical>) {
        let gaps = &self.config.gaps;
        let border_width = self.config.borders.width as i32;

        let inner_gap = gaps.inner as i32;
        let outer_gap = gaps.outer as i32;

        let usable_area = Rectangle::<i32, Logical>::new(
            Point::from((
                output_area.loc.x + outer_gap,
                output_area.loc.y + outer_gap,
            )),
            Size::from((
                output_area.size.w - 2 * outer_gap,
                output_area.size.h - 2 * outer_gap,
            )),
        );

        let mut tiled_windows: Vec<_> = self.windows.values()
            .filter(|w| !w.is_floating && !w.is_fullscreen)
            .collect();

        tiled_windows.sort_by_key(|w| w.id);

        match self.current_layout {
            LayoutType::Dwindle => self.arrange_dwindle(&tiled_windows, usable_area, inner_gap, border_width),
            LayoutType::Master => self.arrange_master(&tiled_windows, usable_area, inner_gap, border_width),
            LayoutType::Spiral => self.arrange_spiral(&tiled_windows, usable_area, inner_gap, border_width),
            LayoutType::Columns => self.arrange_columns(&tiled_windows, usable_area, inner_gap, border_width),
            LayoutType::Grid => self.arrange_grid(&tiled_windows, usable_area, inner_gap, border_width),
            LayoutType::Floating => {},
        }
    }

    fn arrange_dwindle(&self, windows: &[&Window], area: Rectangle<i32, Logical>, gap: i32, _border: i32) {
        if windows.is_empty() {
            return;
        }

        let mut remaining_area = area;
        let total = windows.len();

        for (i, _window) in windows.iter().enumerate() {
            let is_last = i == total - 1;

            if is_last {
                let _geometry = Rectangle::<i32, Logical>::new(
                    Point::from((remaining_area.loc.x, remaining_area.loc.y)),
                    Size::from((remaining_area.size.w, remaining_area.size.h)),
                );
            } else {
                let half_width = remaining_area.size.w / 2;

                let _geometry = Rectangle::<i32, Logical>::new(
                    Point::from((remaining_area.loc.x, remaining_area.loc.y)),
                    Size::from((half_width - gap / 2, remaining_area.size.h)),
                );

                remaining_area = Rectangle::<i32, Logical>::new(
                    Point::from((remaining_area.loc.x + half_width + gap / 2, remaining_area.loc.y)),
                    Size::from((remaining_area.size.w - half_width - gap / 2, remaining_area.size.h)),
                );
            }
        }
    }

    fn arrange_master(&self, windows: &[&Window], area: Rectangle<i32, Logical>, gap: i32, _border: i32) {
        if windows.is_empty() {
            return;
        }

        let master_ratio = 0.6;
        let master_width = (area.size.w as f64 * master_ratio) as i32;

        if let Some(_master) = windows.first() {
            let _geometry = Rectangle::<i32, Logical>::new(
                Point::from((area.loc.x, area.loc.y)),
                Size::from((master_width - gap / 2, area.size.h)),
            );
        }

        if windows.len() > 1 {
            let stack_width = area.size.w - master_width - gap / 2;
            let stack_height = area.size.h / (windows.len() - 1) as i32;

            for (i, _window) in windows.iter().skip(1).enumerate() {
                let _geometry = Rectangle::<i32, Logical>::new(
                    Point::from((
                        area.loc.x + master_width + gap / 2,
                        area.loc.y + i as i32 * (stack_height + gap),
                    )),
                    Size::from((stack_width, stack_height - gap)),
                );
            }
        }
    }

    fn arrange_spiral(&self, windows: &[&Window], area: Rectangle<i32, Logical>, gap: i32, border: i32) {
        self.arrange_dwindle(windows, area, gap, border);
    }

    fn arrange_columns(&self, windows: &[&Window], area: Rectangle<i32, Logical>, gap: i32, _border: i32) {
        if windows.is_empty() {
            return;
        }

        let column_count = windows.len().min(3);
        let column_width = area.size.w / column_count as i32;
        let windows_per_column = (windows.len() + column_count - 1) / column_count;

        for (i, _window) in windows.iter().enumerate() {
            let col = i / windows_per_column;
            let row = i % windows_per_column;

            let window_height = area.size.h / ((windows.len() - col * windows_per_column).min(windows_per_column)) as i32;

            let _geometry = Rectangle::<i32, Logical>::new(
                Point::from((
                    area.loc.x + col as i32 * column_width,
                    area.loc.y + row as i32 * (window_height + gap),
                )),
                Size::from((column_width - gap, window_height - gap)),
            );
        }
    }

    fn arrange_grid(&self, windows: &[&Window], area: Rectangle<i32, Logical>, gap: i32, _border: i32) {
        if windows.is_empty() {
            return;
        }

        let count = windows.len();
        let cols = (count as f64).sqrt().ceil() as i32;
        let rows = (count as f64 / cols as f64).ceil() as i32;

        let cell_width = area.size.w / cols;
        let cell_height = area.size.h / rows;

        for (i, _window) in windows.iter().enumerate() {
            let col = (i % cols as usize) as i32;
            let row = (i / cols as usize) as i32;

            let _geometry = Rectangle::<i32, Logical>::new(
                Point::from((
                    area.loc.x + col * cell_width + gap / 2,
                    area.loc.y + row * cell_height + gap / 2,
                )),
                Size::from((cell_width - gap, cell_height - gap)),
            );
        }
    }
}
