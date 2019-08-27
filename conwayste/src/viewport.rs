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

use ggez::graphics::Rect;
use ggez::nalgebra::Point2;

use crate::utils;
use crate::constants::{
    DEFAULT_SCREEN_HEIGHT,
    DEFAULT_SCREEN_WIDTH,
    MAX_CELL_SIZE,
    MIN_CELL_SIZE,
    PIXELS_SCROLLED_PER_FRAME,
};

const NO_INPUT                  : (isize, isize) = (0, 0);
const PAN_LEFT                  : isize          = -1;
const PAN_RIGHT                 : isize          =  1;
const PAN_UP                    : isize          = -1;
const PAN_DOWN                  : isize          =  1;
const ZOOM_IN                   : f32            =  1.0;
const ZOOM_OUT                  : f32            = -1.0;

#[derive(Debug, PartialEq)]
pub struct Cell {
    pub col: usize,
    pub row: usize,
}

impl Cell {
    pub fn new(col: usize, row: usize) -> Cell {
        Cell {
            col,
            row,
        }
    }
}

pub struct Viewport {
    grid_view:           GridView,
}

#[derive(PartialEq)]
/// Whether the user is zooming in our out.
pub enum ZoomDirection {
    ZoomOut,
    ZoomIn
}

impl Viewport {

    /// Creates a new Viewport which manages how the how things
    /// are displayed within the Window.
    pub fn new(cell_size: f32, length: usize, width: usize) -> Viewport {
        Viewport {
            grid_view : GridView::new(cell_size, length, width),
        }
    }

    /// Adjusting the zoom level is a two step process:
    ///
    /// 1) The cell size controls the rectangle size of each cell.
    ///   Zooming in increments, out decrements.
    ///
    /// 2) The offset needs to be repositioned so that the center of the screen
    ///   holds after the cell size change.
    pub fn adjust_zoom_level(&mut self, direction : ZoomDirection) {
        if (direction == ZoomDirection::ZoomIn && self.grid_view.cell_size < MAX_CELL_SIZE) ||
           (direction == ZoomDirection::ZoomOut && self.grid_view.cell_size > MIN_CELL_SIZE) {

            let zoom_dir: f32 = match direction {
                ZoomDirection::ZoomIn => ZOOM_IN,
                ZoomDirection::ZoomOut => ZOOM_OUT,
            };

            let next_cell_size = self.grid_view.cell_size + zoom_dir;
            let old_cell_size = self.grid_view.cell_size;
            let cell_size_delta = next_cell_size - old_cell_size;

            let window_center = Point2::new(self.grid_view.rect.w/2.0, self.grid_view.rect.h/2.0);

            if let Some(cell) = self.grid_view.game_coords_from_window(window_center) {
                let (old_cell_center_x, old_cell_center_y) = (cell.row, cell.col);
                let delta_x = cell_size_delta * (old_cell_center_x as f32 * next_cell_size as f32 -
                                                 old_cell_center_x as f32 * old_cell_size as f32);
                let delta_y = cell_size_delta * (old_cell_center_y as f32 * next_cell_size as f32 -
                                                 old_cell_center_y as f32 * old_cell_size as f32);

                self.grid_view.cell_size = next_cell_size;

                let columns = self.grid_view.columns as u32;

                let phi = columns as i32 * old_cell_size as i32;
                let alpha = self.grid_view.rect.w as i32;

                if phi > alpha {
                    self.grid_view.grid_origin = utils::Graphics::point_offset(self.grid_view.grid_origin,
                                                                         -cell_size_delta * delta_x,
                                                                         -cell_size_delta * delta_y
                                                                         );
                }

                self.adjust_panning(true, NO_INPUT);

            }
        }
    }

    /// Panning moves the grid_origin around, if it can.
    /// We always keep a border of ten pixels on each side.
    /// This works by checking to see how much of the grid (or lack thereof) is
    /// displayed on the onscreen.
    ///
    /// We need to re-adjust the grid origin if the cell size changes or if
    /// the user moves around.
    ///
    /// The panning Left and Up cases are straightforward as the origin does not move.
    /// The Down and Right cases look at how much of the Grid is displayed on screen (`ϕ`, `phi`).
    /// This is compared against the size of the screen, `α`, `alpha`, to see if we can
    /// adjust the grid origin.
    fn adjust_panning(&mut self, recenter_after_zoom: bool, arrow_input: (isize, isize)) {
        let (columns, rows) = (self.grid_view.columns as u32, self.grid_view.rows as u32);

        //debug!("\n\nP A N N I N G:");
        //debug!("Columns, Rows = {:?}", (columns, rows));

        let (dx, dy) = arrow_input;
        let dx_in_pixels = -(dx as f32) * PIXELS_SCROLLED_PER_FRAME;
        let dy_in_pixels = -(dy as f32) * PIXELS_SCROLLED_PER_FRAME;

        let cur_origin_x = self.grid_view.grid_origin.x;
        let cur_origin_y = self.grid_view.grid_origin.y;

        let new_origin_x = cur_origin_x + dx_in_pixels;
        let new_origin_y = cur_origin_y + dy_in_pixels;

        let cell_size = self.grid_view.cell_size;
        let border_in_cells = 10.0;
        let border_in_px = border_in_cells * cell_size;

        //debug!("Cell Size: {:?}", (cell_size));

        let mut pan = true;
        let mut limit_x = self.grid_view.grid_origin.x;
        let mut limit_y = self.grid_view.grid_origin.y;
        // Here we check if we're at our limits. In all other cases, we'll fallthrough to the
        // bottom grid_origin offsetting.

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

    /// Parent viewport handler update. Currently we update the following, in-order:
    /// # Pan around the grid view.
    pub fn update(&mut self, direction: (isize, isize)) {
        self.adjust_panning(false, direction);
    }

    /// Set dimensions of the grid in window coordinates (pixels). This may cause unintended
    /// consequences if modified while a game is running.  Be mindful of the window size.
    pub fn set_dimensions(&mut self, w: u32, h: u32) {
        self.grid_view.set_width(w as f32);
        self.grid_view.set_height(h as f32);
    }

    /// Given a point, find the nearest Cell (game coordinates) specified by a point in window
    /// coordinates.
    pub fn get_cell(&self, point: Point2<f32>) -> Option<Cell> {
        self.grid_view.game_coords_from_window(point)
    }

    /// Gets the cell size in pixels.
    pub fn get_cell_size(&self) -> f32 {
        self.grid_view.cell_size
    }

    /// Gets a rectangle representing the grid in game coordinates.
    pub fn get_rect(&self) -> Rect {
        self.grid_view.rect
    }

    /// Returns the origin of the grid in window coordinates.
    pub fn get_origin(&self) -> Point2<f32> {
        self.grid_view.grid_origin
    }

    /// Returns the width of the grid in pixels.
    pub fn grid_width(&self) -> u32 {
        self.grid_view.columns as u32 * self.grid_view.cell_size as u32
    }

    /// Returns the height of the grid in pixels.
    pub fn grid_height(&self) -> u32 {
        self.grid_view.rows as u32 * self.grid_view.cell_size as u32
    }

    pub fn get_rect_from_origin(&self) -> Rect {
        let origin = self.get_origin();
        let full_width  = self.grid_width() as f32;
        let full_height = self.grid_height() as f32;

        Rect::new(origin.x, origin.y, full_width, full_height)
    }

    pub fn get_screen_area(&self, cell: Cell) -> Option<Rect> {
        self.grid_view.window_coords_from_game(cell)
    }
}

/// Controls the mapping between window and game coordinates.
/// This should always be sized with respect to the window, otherwise we'll
/// get black bars.
struct GridView {
    rect:        Rect,  // the area the game grid takes up on screen
    cell_size:   f32,   // zoom level in window coordinates
    columns:     usize, // width in game coords (should match bitmap/universe width)
    rows:        usize, // height in game coords (should match bitmap/universe height)
    // The grid origin point tells us where the top-left of the universe is with respect to the
    // window.
    grid_origin: Point2<f32>, // top-left corner of grid in window coords. (may be outside rect)
}


impl GridView {

    /// Creates a new Gridview which maintains control over how the conwayste universe is positioned with
    /// respect to the window.
    pub fn new(cell_size: f32, universe_width_in_cells: usize, universe_height_in_cells: usize) -> GridView {
        GridView {
            rect:        Rect::new(0.0, 0.0, DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT),
            cell_size:   cell_size,
            columns:     universe_width_in_cells,
            rows:        universe_height_in_cells,
            grid_origin: Point2::new(0.0, 0.0),
        }
    }

    /// Attempt to return a tuple of cell coordinates within the game space.
    /// Can be outside of the playble space, it is the responsibility of the caller
    /// to sanitize the output.
    fn game_coords_from_window_unchecked(&self, point: Point2<f32>) -> (isize, isize) {
        let col: isize = ((point.x - self.grid_origin.x) / self.cell_size) as isize;
        let row: isize = ((point.y - self.grid_origin.y) / self.cell_size) as isize;

        (col, row)
    }

    /// Given a window point in pixels, we'll determine the nearest intersecting
    /// row, column pair.
    // Given a Point2<f32>(x,y), we determine a col/row tuple in cell units
    fn game_coords_from_window(&self, point: Point2<f32>) -> Option<Cell> {
        let (col, row) = self.game_coords_from_window_unchecked(point);

        if col < 0 || col >= self.columns as isize || row < 0 || row >= self.rows as isize {
            return None;
        }
        Some( Cell::new(col as usize , row as usize) )
    }

    /// Attempt to return a rectangle for the on-screen area of the specified cell.
    /// If partially in view, will be clipped by the bounding rectangle.
    /// Caller must ensure that column and row are within bounds.
    fn window_coords_from_game_unchecked(&self, col: isize, row: isize) -> Option<Rect> {
        let left   = self.grid_origin.x + (col as f32)     * self.cell_size;
        let right  = self.grid_origin.x + (col + 1) as f32 * self.cell_size - 1.0;
        let top    = self.grid_origin.y + (row as f32)     * self.cell_size;
        let bottom = self.grid_origin.y + (row + 1) as f32 * self.cell_size - 1.0;

        assert!(left < right);
        assert!(top < bottom);

        // The 'minus one' above for right and bottom give it that grid-like feel :)
        let rect = Rect::new(left, top, right - left, bottom - top);
        utils::Graphics::intersection(rect, self.rect)
    }

    /// The column and row supplied lies is `None` outside of the grid.
    /// Otherwise we'll translate a row/column pair into its representative rectangle.
    fn window_coords_from_game(&self, cell: Cell) -> Option<Rect> {
        if cell.row < self.rows && cell.col < self.columns {
            return self.window_coords_from_game_unchecked( cell.col as isize, cell.row as isize);
        }
        return None;
    }

    /// Sets the width of the viewport in window coordinates (pixels).
    pub fn set_width(&mut self, width: f32) {
        self.rect.w = width;
    }

    /// Sets the height of the viewport in window coordinates (pixels).
    pub fn set_height(&mut self, height: f32) {
        self.rect.h = height;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn gen_default_gridview() -> GridView {
        let cell_size = 10.0;
        let universe_height_in_cells = 128;
        let universe_width_in_cells = 256;

        GridView::new(cell_size, universe_width_in_cells, universe_height_in_cells)
    }

    #[test]
    fn test_gridview_default_instantiation() {
        let gv = gen_default_gridview();
        let rect = Rect::new(0.0, 0.0, DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT);

        assert_eq!(gv.cell_size, 10.0);
        assert_eq!(gv.rect, rect);
        assert_eq!(gv.grid_origin, Point2::new(0.0, 0.0));
        assert_eq!(gv.columns, 256);
        assert_eq!(gv.rows, 128);
    }

    #[test]
    fn test_gridview_game_coords_unchecked() {
        let gv = gen_default_gridview();
        let inside = Point2::new(5.0, 5.0);
        let corner = Point2::new(DEFAULT_SCREEN_WIDTH as f32 * gv.cell_size, DEFAULT_SCREEN_HEIGHT as f32 * gv.cell_size);
        let outside = Point2::new(-10.0, -10.0);

        assert_eq!(gv.game_coords_from_window_unchecked(inside), (0, 0));
        assert_eq!(gv.game_coords_from_window_unchecked(corner), (1200, 800));
        assert_eq!(gv.game_coords_from_window_unchecked(outside), (-1, -1));
    }

    #[test]
    fn test_gridview_game_coords_checked() {
        let gv = gen_default_gridview();
        let inside = Point2::new(5.0, 5.0);
        let corner1 = Point2::new( (DEFAULT_SCREEN_WIDTH-1.0) * gv.cell_size, (DEFAULT_SCREEN_HEIGHT-1.0) * gv.cell_size);
        let corner2 = Point2::new( DEFAULT_SCREEN_WIDTH * gv.cell_size, DEFAULT_SCREEN_HEIGHT * gv.cell_size);
        let outside = Point2::new(-10.0, -10.0);
        let edge_point = Point2::new(1200.0, 800.0);

        assert_eq!(gv.game_coords_from_window(inside), Some(Cell::new(0, 0)));
        assert_eq!(gv.game_coords_from_window(corner1), None);
        assert_eq!(gv.game_coords_from_window(corner2), None);
        assert_eq!(gv.game_coords_from_window(outside), None);
        assert_eq!(gv.game_coords_from_window(edge_point), Some(Cell::new(120, 80)));
    }

    #[test]
    fn test_gridview_window_coords_from_game_unchecked() {
        let gv = gen_default_gridview();

        assert_eq!(gv.window_coords_from_game_unchecked(0, 0), Some(Rect::new(0.0, 0.0, 9.0, 9.0)));
        assert_eq!(gv.window_coords_from_game_unchecked(120, 80), None); // Creates a rectangle with 0 dimensions
        assert_eq!(gv.window_coords_from_game_unchecked(-1, -1), None);
    }

    #[test]
    fn test_gridview_window_coords_from_game_checked() {
        let gv = gen_default_gridview();
        let inside = Cell::new(0, 0);
        let corner = Cell::new(120, 80);
        let outside1 = Cell::new(121, 80);
        let outside2 = Cell::new(120, 81);

        assert_eq!(gv.window_coords_from_game(inside), Some(Rect::new(0.0, 0.0, 9.0, 9.0)) );
        assert_eq!(gv.window_coords_from_game(corner), None);
        assert_eq!(gv.window_coords_from_game(outside1), None);
        assert_eq!(gv.window_coords_from_game(outside2), None);
    }
}
