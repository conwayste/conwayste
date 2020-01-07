/*  Copyright 2017-2019 the Conwayste Developers.
 *
 *  This file is part of libconway.
 *
 *  libconway is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  libconway is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with libconway.  If not, see <http://www.gnu.org/licenses/>. */

use std::error::Error;
use std::ops::{Index, IndexMut};
use std::cmp;
use crate::universe::Region;
use crate::rle::Pattern;


#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum BitOperation {
    Clear,
    Set,
    Toggle
}

/// Defines a rotation.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Rotation {
    CW,  // clockwise
    CCW, // counter-clockwise
}


#[derive(Debug, PartialEq, Clone)]
pub struct BitGrid(pub Vec<Vec<u64>>);

impl BitGrid {
    /// Creates a new zero-initialized BitGrid of given dimensions.
    ///
    /// # Panics
    ///
    /// This function will panic if `with_in_words` or `height` are zero.
    pub fn new(width_in_words: usize, height: usize) -> Self {
        assert!(width_in_words != 0);
        assert!(height != 0);

        let mut rows: Vec<Vec<u64>> = Vec::with_capacity(height);
        for _ in 0 .. height {
            let row: Vec<u64> = vec![0; width_in_words];
            rows.push(row);
        }
        BitGrid(rows)
    }

    pub fn width_in_words(&self) -> usize {
        if self.height() > 0 {
            self.0[0].len()
        } else {
            0
        }
    }

    #[inline]
    pub fn modify_bits_in_word(&mut self, row: usize, word_col: usize, mask: u64, op: BitOperation) {
        match op {
            BitOperation::Set    => self[row][word_col] |=  mask,
            BitOperation::Clear  => self[row][word_col] &= !mask,
            BitOperation::Toggle => self[row][word_col] ^=  mask,
        }
    }

    /// Sets, clears, or toggles a rectangle of bits.
    ///
    /// # Panics
    ///
    /// This function will panic if `region` is out of range.
    pub fn modify_region(&mut self, region: Region, op: BitOperation) {
        for y in region.top() .. region.bottom() + 1 {
            assert!(y >= 0);
            for word_col in 0 .. self[y as usize].len() {
                let x_left  = word_col * 64;
                let x_right = x_left + 63;

                if region.right() >= x_left as isize && region.left() <= x_right as isize {
                    let mut mask = u64::max_value();

                    for shift in (0..64).rev() {
                        let x = x_right - shift;
                        if (x as isize) < region.left() || (x as isize) > region.right() {
                            mask &= !(1 << shift);
                        }
                    }

                    // apply change to bitgrid based on mask and bit
                    self.modify_bits_in_word(y as usize, word_col, mask, op);
                }
            }
        }
    }

    /// Returns `Some(`smallest region containing every 1 bit`)`, or `None` if there are no 1 bits.
    pub fn bounding_box(&self) -> Option<Region> {
        let (width, height) = (self.width(), self.height());
        let mut first_row = None;
        let mut last_row = None;
        let mut first_col = None;
        let mut last_col = None;
        for row in 0..height {
            let mut col = 0;
            while col < width {
                let (rle_len, ch) = self.get_run(col, row, None);
                if ch == 'o' {
                    if let Some(_first_col) = first_col {
                        if _first_col > col {
                            first_col = Some(col);
                        }
                    } else {
                        first_col = Some(col);
                    }
                    if first_row.is_none() {
                        first_row = Some(row);
                    }
                    let run_last_col = col + rle_len - 1;
                    if let Some(_last_col) = last_col {
                        if _last_col < run_last_col {
                            last_col = Some(run_last_col);
                        }
                    } else {
                        last_col = Some(run_last_col);
                    }
                    last_row = Some(row);
                }
                col += rle_len;
            }
        }
        if first_row.is_some() {
            let width = last_col.unwrap() - first_col.unwrap() + 1;
            let height = last_row.unwrap() - first_row.unwrap() + 1;
            Some(Region::new(first_col.unwrap() as isize, first_row.unwrap() as isize, width, height))
        } else {
            None
        }
    }

    /// Copies `src` BitGrid into `dst` BitGrid, but fit into `dst_region` with clipping. If there
    /// is a 10x10 pattern in source starting at 0,0 and the dst_region is only 8x8 with top left
    /// corner at 3,3 then the resulting pattern will be clipped in a 2 pixel region on the bottom
    /// and right and extend from 3,3 to 10,10. If `dst_region` extends beyond `dst`, the pattern
    /// will be further clipped by the dimensions of `dst`.
    ///
    /// The bits are copied using an `|=` operation, so 1 bits in the destination will not ever be
    /// cleared.
    pub fn copy(src: &BitGrid, dst: &mut BitGrid, dst_region: Region) {
        let dst_left   = cmp::max(0, dst_region.left()) as usize;
        let dst_right  = cmp::min(dst.width() as isize - 1, dst_region.right()) as usize;
        let dst_top    = cmp::max(0, dst_region.top()) as usize;
        let dst_bottom = cmp::min(dst.height() as isize - 1, dst_region.bottom()) as usize;
        if dst_left > dst_right || dst_top > dst_bottom {
            // nothing to do because both dimensions aren't positive
            return;
        }

        for src_row in 0..src.height() {
            let dst_row = src_row + dst_top;
            if dst_row > dst_bottom {
                break;
            }
            let mut src_col = 0;   // word-aligned
            while src_col < src.width() {
                let dst_col = src_col + dst_left;    // not word-aligned
                if dst_col > dst_right {
                    break;
                }
                let dst_word_idx = dst_col / 64;
                let shift = dst_col - dst_word_idx*64;  // right shift amount
                let mut word = src[src_row][src_col/64];
                // clear bits that would be beyond dst_right
                if dst_right - dst_col + 1 < 64 {
                    let mask = !((1u64 << (64 - (dst_right - dst_col + 1))) - 1);
                    word &= mask;
                }
                dst[dst_row][dst_word_idx] |= word >> shift;
                if shift > 0 && dst_word_idx+1 < dst.width_in_words() {
                    dst[dst_row][dst_word_idx+1] |= word << (64 - shift);
                }
                src_col += 64;
            }
        }
    }

    /// Get a Region of the same size as the BitGrid.
    pub fn region(&self) -> Region {
        Region::new(0, 0, self.width(), self.height())
    }

    /// Clear this BitGrid.
    pub fn clear(&mut self) {
        for row in &mut self.0 {
            for col_idx in 0..row.len() {
                row[col_idx] = 0;
            }
        }
    }

    /// Calls callback on each bit that is set (1). Callback receives (col, row).
    pub fn each_set<F: FnMut(usize, usize)>(&self, mut callback: F) {
        for row in 0 .. self.height() {
            let mut col = 0;
            for col_idx in 0 .. self.width_in_words() {
                let word = self.0[row][col_idx];
                for shift in (0..64).rev() {
                    if (word>>shift)&1 == 1 {
                        callback(col, row);
                    }
                    col += 1;
                }
            }
        }
    }

    /// Rotates pattern with top-left corner at `(0,0)` in the grid and lower right corner at
    /// `(width - 1, height - 1)` in the specified direction. This may change the dimensions of the
    /// grid.
    ///
    /// # Errors
    ///
    /// An error is returned if the width or height are out of range.
    pub fn rotate(&mut self, width: usize, height: usize, rotation: Rotation) -> Result<(), Box<dyn Error>> {
        if width > self.width() || height > self.height() {
            return Err(format!("Expected passed-in width={} and height={} to be less than grid width={} and height={}",
                    width, height, self.width(), self.height()).into());
        }
        let new_width_in_words = (self.height() - 1)/64 + 1;   // number of words needed for this many cells
        let new_height = self.width();
        let mut new = BitGrid::new(new_width_in_words, new_height);
        for row in 0 .. height {
            let new_col = match rotation {
                Rotation::CCW => row,
                Rotation::CW => height - row - 1,
            };
            let new_col_idx = new_col/64; // the column index in the new grid
            let mut col = 0;
            'rowloop:
            for col_idx in 0 .. self.width_in_words() {
                let word = self.0[row][col_idx];
                for shift in (0..64).rev() {
                    if col >= width {
                        break 'rowloop;
                    }
                    let new_row = match rotation {
                        Rotation::CCW => width - col - 1,
                        Rotation::CW => col,
                    };
                    if (word>>shift)&1 == 1 {
                        let new_shift = 63 - (new_col - new_col_idx*64);
                        // copy this bit to new but rotated
                        new.0[new_row][new_col_idx] |= 1<<new_shift;
                    }
                    col += 1;
                }
            }
        }
        // replace self with new
        self.0 = new.0;
        Ok(())
    }
}


impl Index<usize> for BitGrid {
    type Output = Vec<u64>;

    fn index(&self, i: usize) -> &Vec<u64> {
        &self.0[i]
    }
}

impl IndexMut<usize> for BitGrid {
    fn index_mut(&mut self, i: usize) -> &mut Vec<u64> {
        &mut self.0[i]
    }
}


pub trait CharGrid {
    /// Write a char `ch` to (`col`, `row`).
    /// 
    /// # Panics
    /// 
    /// Panics if:
    /// * `col` or `row` are out of range
    /// * `char` is invalid for this type. Use `is_valid` to check first.
    /// * `visibility` is invalid. That is, it equals `Some(player_id)`, but there is no such `player_id`.
    fn write_at_position(&mut self, col: usize, row: usize, ch: char, visibility: Option<usize>);

    /// Is `ch` a valid character?
    fn is_valid(ch: char) -> bool;

    /// Width in cells
    fn width(&self) -> usize;

    /// Height in cells
    fn height(&self) -> usize;

    /// Returns a Pattern that describes this `CharGrid` as viewed by specified player if
    /// `visibility.is_some()`, or a fog-less view if `visibility.is_none()`.
    fn to_pattern(&self, visibility: Option<usize>) -> Pattern {

        fn push(result: &mut String, output_col: &mut usize, rle_len: usize, ch: char) {
            let what_to_add = if rle_len == 1 {
                let mut s = String::with_capacity(1);
                s.push(ch);
                s
            } else { format!("{}{}", rle_len, ch) };
            if *output_col + what_to_add.len() > 70 {
                result.push_str("\r\n");
                *output_col = 0;
            }
            result.push_str(what_to_add.as_str());
            *output_col += what_to_add.len();
        }

        let mut result = "".to_owned();
        let (mut col, mut row) = (0, 0);
        let mut line_ends_buffered = 0;
        let mut output_col = 0;
        while row < self.height() {
            while col < self.width() {
                let (rle_len, ch) = self.get_run(col, row, visibility);

                match ch {
                    'b' => {
                        // Blank
                        // TODO: if supporting diffs with this same algorithm, then need to allow
                        // other characters to serve this purpose.
                        if col + rle_len < self.width() {
                            if line_ends_buffered > 0 {
                                push(&mut result, &mut output_col, line_ends_buffered, '$');
                                line_ends_buffered = 0;
                            }
                            push(&mut result, &mut output_col, rle_len, ch);
                        }
                    }
                    _ => {
                        // Non-blank
                        if line_ends_buffered > 0 {
                            push(&mut result, &mut output_col, line_ends_buffered, '$');
                            line_ends_buffered = 0;
                        }
                        push(&mut result, &mut output_col, rle_len, ch);
                    }
                }

                col += rle_len;
            }

            row += 1;
            col = 0;
            line_ends_buffered += 1;
        }
        push(&mut result, &mut output_col, 1, '!');
        Pattern(result)
    }

    /// Given a starting cell at `(col, row)`, get the character at that cell, and the number of
    /// contiguous identical cells considering only this cell and the cells to the right of it.
    /// This is intended for exporting to RLE.
    ///
    /// The `visibility` parameter, if not `None`, is used to generate a run as observed by a
    /// particular player.
    ///
    /// # Returns
    ///
    /// `(run_length, ch)`
    ///
    /// # Panics
    ///
    /// This function will panic if `col`, `row`, or `visibility` (`Some(player_id)`) are out of bounds.
    fn get_run(&self, col: usize, row: usize, visibility: Option<usize>) -> (usize, char);
}


const VALID_BIT_GRID_CHARS: [char; 2] = ['b', 'o'];

impl CharGrid for BitGrid {
    /// Width in cells
    fn width(&self) -> usize {
        self.width_in_words() * 64
    }

    /// Height in cells
    fn height(&self) -> usize {
        self.0.len()
    }

    /// _visibility is ignored, since BitGrids have no concept of a player.
    fn write_at_position(&mut self, col: usize, row: usize, ch: char, _visibility: Option<usize>) {
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1));
        match ch {
            'b' => {
                self.modify_bits_in_word(row, word_col, 1 << shift, BitOperation::Clear)
            }
            'o' => {
                self.modify_bits_in_word(row, word_col, 1 << shift, BitOperation::Set)
            }
            _ => panic!("invalid character: {:?}", ch)
        }
    }

    fn is_valid(ch: char) -> bool {
        VALID_BIT_GRID_CHARS.contains(&ch)
    }

    /// Given a starting cell at `(col, row)`, get the character at that cell, and the number of
    /// contiguous identical cells considering only this cell and the cells to the right of it.
    /// This is intended for exporting to RLE.
    ///
    /// The `_visibility` argument is unused and should be `None`.
    ///
    /// # Returns
    ///
    /// `(run_length, ch)`
    ///
    /// # Panics
    ///
    /// This function will panic if `col` or `row` are out of bounds.
    fn get_run(&self, col: usize, row: usize, _visibility: Option<usize>) -> (usize, char) {
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1));
        let mut word = self.0[row][word_col];
        let mut mask = 1 << shift;
        let is_set = (word & (1 << shift)) > 0;
        let mut end_col = col + 1;
        let ch = if is_set { 'o' } else { 'b' };

        // go to end of current word
        mask >>= 1;
        while mask > 0 {
            if (word & mask > 0) != is_set {
                return (end_col - col, ch);
            }
            end_col += 1;
            mask >>= 1;
        }
        assert_eq!(end_col % 64, 0);

        // skip words
        let mut end_word_col = word_col + 1;
        while end_word_col < self.0[row].len() {
            word = self.0[row][end_word_col];
            if is_set && word < u64::max_value() {
                break;
            }
            if !is_set && word > 0 {
                break;
            }
            end_col += 64;
            end_word_col += 1;
        }
        // start from beginning of last word
        if end_word_col == self.0[row].len() {
            return (end_col - col, ch);
        }
        mask = 1 << 63;
        while mask > 0 {
            if (word & mask > 0) != is_set {
                break;
            }
            end_col += 1;
            mask >>= 1;
        }
        return (end_col - col, ch);
    }
}
