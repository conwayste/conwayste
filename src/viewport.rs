/*  Copyright 2017-2018 the Conwayste Developers.
 *
 *  This file is part of conwayste.
 *
 *  conwayste is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  conwayste is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with conwayste.  If not, see
 *  <http://www.gnu.org/licenses/>. */

extern crate ggez;

use ggez::graphics::{Point2, Rect};

use utils;
use config;

const MAX_CELL_SIZE             : f32   = 20.0;
const MIN_CELL_SIZE             : f32   = 5.0;
const PIXELS_SCROLLED_PER_FRAME : i32   = 50;
const NO_INPUT                  : (i32, i32) = (0, 0);
const PAN_LEFT                  : i32   = -1;
const PAN_RIGHT                 : i32   =  1;
const PAN_UP                    : i32   = -1;
const PAN_DOWN                  : i32   =  1;
const ZOOM_IN                   : f32   =  1.0;
const ZOOM_OUT                  : f32   = -1.0;

pub struct Cell {
    pub row: usize,
    pub col: usize,
}

pub struct Viewport {
    grid_view:           GridView,
}

/*
#[derive(Debug, Clone)]
struct WindowCornersInGameCoords {
    top_left : Point2,
    bottom_right: Point2,
}
*/

#[derive(PartialEq)]
pub enum ZoomDirection {
    ZoomOut,
    ZoomIn
}

impl Viewport {

    pub fn new(cell_size: f32, length: usize, width: usize) -> Viewport {
        Viewport {
            grid_view : GridView::new(cell_size, length, width),
        }
    }

    pub fn adjust_zoom_level(&mut self, direction : ZoomDirection) {
        if (direction == ZoomDirection::ZoomIn && self.grid_view.cell_size < MAX_CELL_SIZE) ||
           (direction == ZoomDirection::ZoomOut && self.grid_view.cell_size > MIN_CELL_SIZE) {

            let zoom_dir: f32;
            match direction {
                ZoomDirection::ZoomIn => zoom_dir = ZOOM_IN,
                ZoomDirection::ZoomOut => zoom_dir = ZOOM_OUT,
            }

//            debug!("Window Size: ({}, {})", self.grid_view.rect.w, self.grid_view.rect.h);
//            debug!("Origin Before: ({},{})", self.grid_view.grid_origin.x, self.grid_view.grid_origin.y);
//            debug!("Cell Size Before: {},", self.grid_view.cell_size);

            let next_cell_size = self.grid_view.cell_size + zoom_dir;
            let old_cell_size = self.grid_view.cell_size;

            let window_center = Point2::new(self.grid_view.rect.w/2.0, self.grid_view.rect.h/2.0);

            if let Some(cell) = self.grid_view.game_coords_from_window(window_center) {
                let (old_cell_count_for_x, old_cell_count_for_y) = (cell.row, cell.col);
                let delta_x = zoom_dir * (old_cell_count_for_x as f32 * next_cell_size as f32 - old_cell_count_for_x as f32 * old_cell_size as f32);
                let delta_y = zoom_dir * (old_cell_count_for_y as f32 * next_cell_size as f32 - old_cell_count_for_y as f32 * old_cell_size as f32);

//                debug!("current cell count: {}, {}", old_cell_count_for_x, old_cell_count_for_x);
//                debug!("delta in win coords: {}, {}", delta_x, delta_y);

                self.grid_view.cell_size = next_cell_size;

                let columns = self.grid_view.columns as u32;

                let phi = columns as i32 * old_cell_size as i32;
                let alpha = self.grid_view.rect.w as i32;

                if phi > alpha {
                    self.grid_view.grid_origin = utils::Graphics::point_offset(self.grid_view.grid_origin,
                                                                         -zoom_dir * delta_x,
                                                                         -zoom_dir * delta_y
                                                                         );
                }

                self.adjust_panning(true, NO_INPUT);

//                debug!("Origin After: ({},{})\n", self.grid_view.grid_origin.x, self.grid_view.grid_origin.y);
//                debug!("Cell Size After: {},", self.grid_view.cell_size);
            }
        }
    }

    fn adjust_panning(&mut self, recenter_after_zoom: bool, arrow_input: (i32, i32)) {
        let (columns, rows) = (self.grid_view.columns as u32, self.grid_view.rows as u32);

//        debug!("\n\nP A N N I N G:");
//        debug!("Columns, Rows = {:?}", (columns, rows));

        let (dx, dy) = arrow_input;
        let dx_in_pixels = (-dx * PIXELS_SCROLLED_PER_FRAME) as f32;
        let dy_in_pixels = (-dy * PIXELS_SCROLLED_PER_FRAME) as f32;

        let cur_origin_x = self.grid_view.grid_origin.x;
        let cur_origin_y = self.grid_view.grid_origin.y;

        let new_origin_x = cur_origin_x + dx_in_pixels;
        let new_origin_y = cur_origin_y + dy_in_pixels;

        let cell_size = self.grid_view.cell_size;
        let border_in_cells = 10.0;
        let border_in_px = border_in_cells * cell_size;

//        debug!("Cell Size: {:?}", (cell_size));

        let mut pan = true;
        let mut limit_x = self.grid_view.grid_origin.x;
        let mut limit_y = self.grid_view.grid_origin.y;
        // Here we check if we're at our limits. In all other cases, we'll fallthrough to the
        // bottom grid_origin offsetting

        // Panning left
        if dx == PAN_LEFT || recenter_after_zoom {
            if new_origin_x > 0.0 {
                if new_origin_x > border_in_px {
                    pan = false;
                    limit_x = border_in_px;
                }
            }
        }

        // Panning right
        //
        //  /      α     \
        //                  v------ includes the border
        //  |------------|----|
        //  |            |    |
        //  |            |    |
        //  |            |    |
        //  |------------|----|
        //
        //  \        ϕ        /
        //
        if dx == PAN_RIGHT || recenter_after_zoom {
            let phi = (border_in_cells + columns as f32)*(cell_size);
            let alpha = self.grid_view.rect.w;

            if phi > alpha && f32::abs(new_origin_x) >= phi - alpha {
                pan = false;
                limit_x = -(phi - alpha);
            }

            if phi < alpha {
                pan = false;
            }
        }

        // Panning up
        if dy == PAN_UP || recenter_after_zoom {
            if new_origin_y > 0.0 && new_origin_y > border_in_px {
                pan = false;
                limit_y = border_in_px;
            }
        }

        // Panning down
        if dy == PAN_DOWN || recenter_after_zoom {
            let phi = (border_in_cells + rows as f32)*(cell_size);
            let alpha = self.grid_view.rect.h;

            if phi > alpha && f32::abs(new_origin_y) >= phi - alpha {
                pan = false;
                limit_y = -(phi - alpha);
            }

            if phi < alpha {
                pan = false;
            }
        }

        if pan {
            self.grid_view.grid_origin = utils::Graphics::point_offset(self.grid_view.grid_origin, dx_in_pixels, dy_in_pixels);
        }
        else {
            // We cannot pan as we are out of bounds, but let us ensure we maintain a border
            self.grid_view.grid_origin = Point2::new(limit_x as f32, limit_y as f32);
        }

    }

    pub fn update(&mut self, direction: (i32, i32)) {
        self.adjust_panning(false, direction);
    }

    pub fn set_dimensions(&mut self, w: f32, h: f32) {
        self.grid_view.set_width(w);
        self.grid_view.set_height(h);
        self.grid_view.rect.w = w;
        self.grid_view.rect.h = h;
    }

    pub fn get_cell(&self, point: Point2) -> Option<Cell> {
        self.grid_view.game_coords_from_window(point)
    }

    pub fn get_cell_size(&self) -> f32 {
        self.grid_view.cell_size
    }

    pub fn get_viewport(&self) -> Rect {
        self.grid_view.rect
    }

    pub fn get_origin(&self) -> Point2 {
        self.grid_view.grid_origin
    }

    pub fn grid_width(&self) -> u32 {
        self.grid_view.columns as u32 * self.grid_view.cell_size as u32
    }

    pub fn grid_height(&self) -> u32 {
        self.grid_view.rows as u32 * self.grid_view.cell_size as u32
    }

    /*
    // TODO reevaluate necessity
     fn _get_all_window_coords_in_game_coords(&mut self) -> Option<WindowCornersInGameCoords> {
        let resolution = self.video_settings.get_active_resolution();
        let win_width_px = resolution.0 as f32;
        let win_height_px = resolution.1 as f32;

        debug!("\tWindow: {:?} px", (win_width_px, win_height_px));

        let result : Option<WindowCornersInGameCoords>;

        let (origin_x, origin_y) = self.grid_view.game_coords_from_window_unchecked(Point2::new(0.0, 0.0));
        {
            let (win_width_px, win_height_px) = self.grid_view.game_coords_from_window_unchecked(Point2::new(win_width_px, win_height_px));
            {
                result = Some(WindowCornersInGameCoords {
                    top_left : Point2::new(origin_x as f32, origin_y as f32),
                    bottom_right : Point2::new(win_width_px as f32, win_height_px as f32),
                });
                debug!("\tReturning... {:?}", result);
            }
        }

        result
    }
    */

    pub fn get_screen_area(&self, col: usize, row: usize) -> Option<Rect> {
        self.grid_view.window_coords_from_game(col, row)
    }
}

// Controls the mapping between window and game coordinates
struct GridView {
    rect:        Rect,  // the area the game grid takes up on screen
    cell_size:   f32,   // zoom level in window coordinates
    columns:     usize, // width in game coords (should match bitmap/universe width)
    rows:        usize, // height in game coords (should match bitmap/universe height)
    // The grid origin point tells us where the top-left of the universe is with respect to the
    // window.
    grid_origin: Point2, // top-left corner of grid in window coords. (may be outside rect)
}


impl GridView {

    pub fn new(cell_size: f32, universe_width_in_cells: usize, universe_height_in_cells: usize) -> GridView {
        GridView {
            rect:        Rect::new(0.0, 0.0, config::DEFAULT_SCREEN_WIDTH, config::DEFAULT_SCREEN_HEIGHT),
            cell_size:   cell_size,
            columns:     universe_width_in_cells,
            rows:        universe_height_in_cells,
            grid_origin: Point2::new(0.0, 0.0),
        }
    }

    fn game_coords_from_window_unchecked(&self, point: Point2) -> (isize, isize) {
        let col: isize = ((point.x - self.grid_origin.x) / self.cell_size) as isize;
        let row: isize = ((point.y - self.grid_origin.y) / self.cell_size) as isize;
        
        (col , row )
    }

    // Given a Point2(x,y), we determine a col/row tuple in cell units
    fn game_coords_from_window(&self, point: Point2) -> Option<Cell> {
        let (col, row) = self.game_coords_from_window_unchecked(point);

        if col < 0 || col >= self.columns as isize || row < 0 || row >= self.rows as isize {
            return None;
        }
        Some(Cell {col: col as usize , row: row as usize })
    }

    // Attempt to return a rectangle for the on-screen area of the specified cell.
    // If partially in view, will be clipped by the bounding rectangle.
    // Caller must ensure that col and row are within bounds.
    fn window_coords_from_game_unchecked(&self, col: usize, row: usize) -> Option<Rect> {
        let left   = self.grid_origin.x + (col as f32)     * self.cell_size;
        let right  = self.grid_origin.x + (col + 1) as f32 * self.cell_size - 1.0;
        let top    = self.grid_origin.y + (row as f32)     * self.cell_size;
        let bottom = self.grid_origin.y + (row + 1) as f32 * self.cell_size - 1.0;

        assert!(left < right);
        assert!(top < bottom);
        // The 'minus one' for right and bottom give it that grid-like feel :)
        let rect = Rect::new(left, top, (right - left), (bottom - top));
        utils::Graphics::intersection(rect, self.rect)
    }

    fn window_coords_from_game(&self, col: usize, row: usize) -> Option<Rect> {
        if row < self.rows && col < self.columns {
            return self.window_coords_from_game_unchecked(col, row);
        }
        return None;
    }

    pub fn set_width(&mut self, width: f32) {
        self.rect.w = width;
    }

    pub fn set_height(&mut self, height: f32) {
        self.rect.h = height;
    }
}
