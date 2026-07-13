use smithay::utils::{Logical, Point, Rectangle, Size};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    Dwindle,
    Master,
    Spiral,
    Columns,
    Grid,
    Floating,
}

pub struct LayoutEngine {
    pub config: crate::config::LayoutConfig,
    pub current_layout: LayoutType,
    pub window_count: usize,
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
            window_count: 0,
        }
    }

    pub fn set_layout(&mut self, layout: LayoutType) {
        self.current_layout = layout;
    }

    pub fn arrange(
        &self,
        windows: &[Rectangle<i32, Logical>],
        output_area: Rectangle<i32, Logical>,
    ) -> Vec<Rectangle<i32, Logical>> {
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
                (output_area.size.w - 2 * outer_gap).max(0),
                (output_area.size.h - 2 * outer_gap).max(0),
            )),
        );

        match self.current_layout {
            LayoutType::Dwindle => self.arrange_dwindle(windows, usable_area, inner_gap, border_width),
            LayoutType::Master => self.arrange_master(windows, usable_area, inner_gap, border_width),
            LayoutType::Spiral => self.arrange_spiral(windows, usable_area, inner_gap, border_width),
            LayoutType::Columns => self.arrange_columns(windows, usable_area, inner_gap, border_width),
            LayoutType::Grid => self.arrange_grid(windows, usable_area, inner_gap, border_width),
            LayoutType::Floating => windows.to_vec(),
        }
    }

    fn arrange_dwindle(
        &self,
        windows: &[Rectangle<i32, Logical>],
        area: Rectangle<i32, Logical>,
        gap: i32,
        _border: i32,
    ) -> Vec<Rectangle<i32, Logical>> {
        if windows.is_empty() {
            return vec![];
        }

        let mut result = Vec::with_capacity(windows.len());
        let mut remaining_area = area;
        let total = windows.len();

        for i in 0..total {
            let is_last = i == total - 1;

            let geo = if is_last {
                Rectangle::<i32, Logical>::new(
                    Point::from((remaining_area.loc.x, remaining_area.loc.y)),
                    Size::from((
                        (remaining_area.size.w).max(0),
                        (remaining_area.size.h).max(0),
                    )),
                )
            } else {
                let split_vertical = i % 2 == 0;
                if split_vertical {
                    let half_width = remaining_area.size.w / 2;
                    let geo = Rectangle::<i32, Logical>::new(
                        Point::from((remaining_area.loc.x, remaining_area.loc.y)),
                        Size::from(((half_width - gap / 2).max(0), (remaining_area.size.h).max(0))),
                    );
                    remaining_area = Rectangle::<i32, Logical>::new(
                        Point::from((
                            remaining_area.loc.x + half_width + gap / 2,
                            remaining_area.loc.y,
                        )),
                        Size::from((
                            (remaining_area.size.w - half_width - gap / 2).max(0),
                            remaining_area.size.h,
                        )),
                    );
                    geo
                } else {
                    let half_height = remaining_area.size.h / 2;
                    let geo = Rectangle::<i32, Logical>::new(
                        Point::from((remaining_area.loc.x, remaining_area.loc.y)),
                        Size::from((
                            (remaining_area.size.w).max(0),
                            (half_height - gap / 2).max(0),
                        )),
                    );
                    remaining_area = Rectangle::<i32, Logical>::new(
                        Point::from((
                            remaining_area.loc.x,
                            remaining_area.loc.y + half_height + gap / 2,
                        )),
                        Size::from((
                            remaining_area.size.w,
                            (remaining_area.size.h - half_height - gap / 2).max(0),
                        )),
                    );
                    geo
                }
            };
            result.push(geo);
        }

        result
    }

    fn arrange_master(
        &self,
        windows: &[Rectangle<i32, Logical>],
        area: Rectangle<i32, Logical>,
        gap: i32,
        _border: i32,
    ) -> Vec<Rectangle<i32, Logical>> {
        if windows.is_empty() {
            return vec![];
        }

        let mut result = Vec::with_capacity(windows.len());
        let master_ratio = 0.6;
        let master_width = (area.size.w as f64 * master_ratio) as i32;

        let master_geo = Rectangle::<i32, Logical>::new(
            Point::from((area.loc.x, area.loc.y)),
            Size::from(((master_width - gap / 2).max(0), area.size.h)),
        );
        result.push(master_geo);

        if windows.len() > 1 {
            let stack_count = windows.len() - 1;
            let stack_width = (area.size.w - master_width - gap / 2).max(0);
            let stack_height = (area.size.h - (stack_count - 1) as i32 * gap) / stack_count as i32;

            for i in 0..stack_count {
                let geo = Rectangle::<i32, Logical>::new(
                    Point::from((
                        area.loc.x + master_width + gap / 2,
                        area.loc.y + i as i32 * (stack_height + gap),
                    )),
                    Size::from((stack_width, stack_height.max(0))),
                );
                result.push(geo);
            }
        }

        result
    }

    fn arrange_spiral(
        &self,
        windows: &[Rectangle<i32, Logical>],
        area: Rectangle<i32, Logical>,
        gap: i32,
        border: i32,
    ) -> Vec<Rectangle<i32, Logical>> {
        self.arrange_dwindle(windows, area, gap, border)
    }

    fn arrange_columns(
        &self,
        windows: &[Rectangle<i32, Logical>],
        area: Rectangle<i32, Logical>,
        gap: i32,
        _border: i32,
    ) -> Vec<Rectangle<i32, Logical>> {
        if windows.is_empty() {
            return vec![];
        }

        let mut result = Vec::with_capacity(windows.len());
        let column_count = windows.len().min(3) as i32;
        let column_width = (area.size.w - (column_count - 1) * gap) / column_count;
        let windows_per_column = (windows.len() + (column_count as usize) - 1) / (column_count as usize);

        for (i, _window) in windows.iter().enumerate() {
            let col = i / windows_per_column;
            let row = i % windows_per_column;

            let rows_in_col =
                ((windows.len() - col * windows_per_column).min(windows_per_column)) as i32;
            let window_height = (area.size.h - (rows_in_col - 1) * gap) / rows_in_col;

            let geo = Rectangle::<i32, Logical>::new(
                Point::from((
                    area.loc.x + col as i32 * (column_width + gap),
                    area.loc.y + row as i32 * (window_height + gap),
                )),
                Size::from((column_width.max(0), window_height.max(0))),
            );
            result.push(geo);
        }

        result
    }

    fn arrange_grid(
        &self,
        windows: &[Rectangle<i32, Logical>],
        area: Rectangle<i32, Logical>,
        gap: i32,
        _border: i32,
    ) -> Vec<Rectangle<i32, Logical>> {
        if windows.is_empty() {
            return vec![];
        }

        let mut result = Vec::with_capacity(windows.len());
        let count = windows.len();
        let cols = ((count as f64).sqrt().ceil() as usize).max(1);
        let rows = ((count as f64 / cols as f64).ceil() as usize).max(1);

        let cell_width = (area.size.w - (cols as i32 - 1) * gap) / cols as i32;
        let cell_height = (area.size.h - (rows as i32 - 1) * gap) / rows as i32;

        for (i, _window) in windows.iter().enumerate() {
            let col = (i % cols) as i32;
            let row = (i / cols) as i32;

            let geo = Rectangle::<i32, Logical>::new(
                Point::from((
                    area.loc.x + col * (cell_width + gap),
                    area.loc.y + row * (cell_height + gap),
                )),
                Size::from((cell_width.max(0), cell_height.max(0))),
            );
            result.push(geo);
        }

        result
    }
}
