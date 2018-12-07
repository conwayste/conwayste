/*  Copyright 2017-2018 the Conwayste Developers.
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

use std::ops::{Index, IndexMut};
use std::cmp;
use universe::Region;
use rle::Pattern;


#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum BitOperation {
    Clear,
    Set,
    Toggle
}


#[derive(Debug, PartialEq, Clone)]
pub struct BitGrid(pub Vec<Vec<u64>>);

impl BitGrid {
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

    // Sets or clears a rectangle of bits. Panics if Region is out of range.
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
        let region = {
            self.region()
        };
        self.modify_region(region, BitOperation::Clear);
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
                self[row][word_col] &= !(1 << shift)
            }
            'o' => {
                self[row][word_col] |=   1 << shift
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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_works() {
        let grid = BitGrid::new(2, 5);
        assert_eq!(grid.height(), 5);
    }

    #[test]
    fn width_works() {
        let grid = BitGrid::new(2, 5);
        assert_eq!(grid.width(), 2*64);
    }

    #[test]
    fn create_valid_empty_bitgrid() {
        let height = 11;
        let width_in_words = 10;
        let grid = BitGrid::new(width_in_words, height);

        assert_eq!(grid[0][0], 0);
        assert_eq!(grid[height-1][width_in_words-1], 0);

        for x in 0..height {
            for y in 0..width_in_words {
                assert_eq!(grid[x][y], 0);
            }
        }
    }

    #[test]
    #[should_panic]
    fn create_bitgrid_with_invalid_dims() {
        let height = 0;
        let width_in_words = 0;
        let _ = BitGrid::new(width_in_words, height);
    }

    #[test]
    fn set_cell_bits_within_a_bitgrid() {
        let height = 10;
        let width_in_words = 10;
        let mut grid = BitGrid::new(width_in_words, height);

        for x in 0..height {
            for y in 0..width_in_words {
                assert_eq!(grid[x][y], 0);
            }
        }

        grid.modify_bits_in_word(height/2, width_in_words/2, 1<<63, BitOperation::Set);
        assert_eq!(grid[height/2][width_in_words/2] >> 63, 1);
        
        grid.modify_bits_in_word(height-1, width_in_words-1, 1<<63, BitOperation::Set);
        assert_eq!(grid[height-1][width_in_words-1] >> 63, 1);
    }

    #[test]
    fn clear_cell_bits_within_a_bitgrid() {
        let height = 10;
        let width_in_words = 10;
        let mut grid = BitGrid::new(width_in_words, height);

        for x in 0..height {
            for y in 0..width_in_words {
                grid[x][y] = u64::max_value();
            }
        }

        grid.modify_bits_in_word(height/2, width_in_words/2, 1<<63, BitOperation::Clear);
        assert_eq!(grid[height/2][width_in_words/2] >> 63, 0);
        
        grid.modify_bits_in_word(height-1, width_in_words-1, 1<<63, BitOperation::Clear);
        assert_eq!(grid[height-1][width_in_words-1] >> 63, 0);
    }

    #[test]
    fn toggle_cell_bits_within_a_bitgrid() {
        let height = 10;
        let width_in_words = 10;
        let mut grid = BitGrid::new(width_in_words, height);

        for x in 0..height {
            for y in 0..width_in_words {
                grid[x][y] = u64::max_value();
            }
        }

        grid.modify_bits_in_word(height/2, width_in_words/2, 1<<63, BitOperation::Toggle);
        assert_eq!(grid[height/2][width_in_words/2] >> 63, 0);
        
        grid.modify_bits_in_word(height/2, width_in_words/2, 1<<63, BitOperation::Toggle);
        assert_eq!(grid[height/2][width_in_words/2] >> 63, 1);
    }

    #[test]
    fn fill_region_within_a_bit_grid() {
        let height = 10;
        let width_in_words = 10;

        let region1_w = 7;
        let region1_h = 7;
        let region2_w = 3;
        let region2_h = 3;
        let region3_h = 4;
        let region3_w = 4;

        let mut grid = BitGrid::new(width_in_words, height);
        let region1 = Region::new(0, 0, region1_w, region1_h);
        let region2 = Region::new(0, 0, region2_w, region2_h);
        let region3 = Region::new(region2_w as isize, region2_h as isize, region3_w, region3_h);

        grid.modify_region(region1, BitOperation::Set);

        for y in 0..region1_w {
            assert_eq!(grid[y][0], 0xFE00000000000000);
        }

        grid.modify_region(region2, BitOperation::Clear);
        for y in 0..region2_w {
            assert_eq!(grid[y][0], 0x1E00000000000000);
        }

        grid.modify_region(region3, BitOperation::Toggle);
        for x in region2_w..region3_w {
            for y in region2_h..region3_h {
                assert_eq!(grid[x][y], 0);
            }
        }
    }

    #[test]
    fn bounding_box_of_empty_bit_grid_is_none() {
        let grid = BitGrid::new(1, 1);
        assert_eq!(grid.bounding_box(), None);
    }

    #[test]
    fn bounding_box_of_empty_bit_grid_is_none2() {
        let grid = BitGrid::new(2, 5);
        assert_eq!(grid.bounding_box(), None);
    }

    #[test]
    fn bounding_box_for_single_off() {
        let grid = Pattern("b!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.bounding_box(), None);
    }

    #[test]
    fn bounding_box_for_single_on() {
        let grid = Pattern("o!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(0, 0, 1, 1)));
    }

    #[test]
    fn bounding_box_for_single_line_complicated() {
        let grid = Pattern("15b87o!".to_owned()).to_new_bit_grid(15+87, 1).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(15, 0, 87, 1)));
    }

    #[test]
    fn bounding_box_for_single_line_complicated2() {
        let grid = Pattern("82b87o!".to_owned()).to_new_bit_grid(82+87, 1).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(82, 0, 87, 1)));
    }

    #[test]
    fn bounding_box_for_multi_line_complicated() {
        let grid = Pattern("4$15b87o!".to_owned()).to_new_bit_grid(15+87, 5).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(15, 4, 87, 1)));
    }

    #[test]
    fn bounding_box_for_multi_line_complicated2() {
        let grid = Pattern("4$15b87o$10b100o$25b3o!".to_owned()).to_new_bit_grid(110, 7).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(10, 4, 100, 3)));
    }


    #[test]
    fn copy_simple() {
        let grid = Pattern("64o$64o!".to_owned()).to_new_bit_grid(64, 2).unwrap();
        let mut grid2 = BitGrid::new(1, 2); // 64x2
        let dst_region = Region::new(0, 0, 32, 3);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "32o$32o!".to_owned());
    }

    #[test]
    fn copy_out_of_bounds() {
        let grid = Pattern("64o$64o!".to_owned()).to_new_bit_grid(64, 2).unwrap();
        let mut grid2 = BitGrid::new(1, 2); // 64x2
        let dst_region = Region::new(17, 3, 32, 5);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "!".to_owned());
    }

    #[test]
    fn copy_simple2() {
        let grid = Pattern("64o$64o!".to_owned()).to_new_bit_grid(64, 2).unwrap();
        let mut grid2 = BitGrid::new(1, 2); // 64x2
        let dst_region = Region::new(16, 1, 32, 3);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "$16b32o!".to_owned());
    }

    #[test]
    fn copy_for_multi_line_complicated() {
        let grid = Pattern("4$b87o$3b100o$3o!".to_owned()).to_new_bit_grid(110, 7).unwrap();
        let mut grid2 = BitGrid::new(2, 10); // 128x10
        let dst_region = Region::new(2, 3, 64, 5);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "7$3b63o!".to_owned());
    }

    #[test]
    fn copy_for_multi_line_complicated2() {
        let grid = Pattern("4$b87o$3b100o$3o!".to_owned()).to_new_bit_grid(110, 7).unwrap();
        let mut grid2 = BitGrid::new(2, 10); // 128x10
        let dst_region = Region::new(2, 3, 65, 5);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "7$3b64o!".to_owned());
    }

    #[test]
    fn copy_for_multi_line_complicated3() {
        let grid = Pattern("4$b87o$3b100o$3o!".to_owned()).to_new_bit_grid(110, 7).unwrap();
        let mut grid2 = BitGrid::new(1, 10); // 64x10
        let dst_region = Region::new(2, 3, 5, 5);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "7$3b4o!".to_owned());
    }


    #[test]
    #[should_panic]
    fn modify_region_with_a_negative_region_panics() {
        let height = 10;
        let width_in_words = 10;

        let mut grid = BitGrid::new(width_in_words, height);
        let region_neg = Region::new(-1, -1, 1, 1);
        grid.modify_region(region_neg, BitOperation::Set);
    }

    #[test]
    fn get_run_for_single_on() {
        let grid = Pattern("o!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.get_run(0, 0, None), (1, 'o'));
    }

    #[test]
    fn get_run_for_single_off_then_on() {
        let grid = Pattern("bo!".to_owned()).to_new_bit_grid(2, 1).unwrap();
        assert_eq!(grid.get_run(0, 0, None), (1, 'b'));
    }

    #[test]
    fn get_run_for_single_off() {
        let grid = Pattern("b!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.get_run(0, 0, None), (64, 'b'));
    }

    #[test]
    fn get_run_nonzero_col_for_single_off() {
        let grid = Pattern("b!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.get_run(23, 0, None), (64 - 23, 'b'));
    }

    #[test]
    fn get_run_nonzero_col_complicated() {
        let grid = Pattern("15b87o!".to_owned()).to_new_bit_grid(15+87, 1).unwrap();
        assert_eq!(grid.get_run(15, 0, None), (87, 'o'));
    }

    #[test]
    fn get_run_nonzero_col_multiline_complicated() {
        let grid = Pattern("3$15b87o!".to_owned()).to_new_bit_grid(15+87, 4).unwrap();
        assert_eq!(grid.get_run(15, 3, None), (87, 'o'));
    }



    #[test]
    fn to_pattern_simple() {
        let original_pattern = Pattern("bo!".to_owned());
        let grid = original_pattern.to_new_bit_grid(2, 1).unwrap();
        let pattern = grid.to_pattern(None);
        assert_eq!(pattern, original_pattern);
    }

    #[test]
    fn to_pattern_nonzero_col_3empty_then_complicated() {
        let original_pattern = Pattern("3$15b87o!".to_owned());
        let grid = original_pattern.to_new_bit_grid(15+87, 4).unwrap();
        let pattern = grid.to_pattern(None);
        assert_eq!(pattern, original_pattern);
    }

    #[test]
    fn to_pattern_nonzero_col_veryverycomplicated() {
        let original_pattern = Pattern("b2o23b2o21b$b2o23bo22b$24bobo22b$15b2o7b2o23b$2o13bobo31b$2o13bob2o30b$16b2o31b$16bo32b$44b2o3b$16bo27b2o3b$16b2o31b$2o13bob2o13bo3bo12b$2o13bobo13bo5bo7b2o2b$15b2o14bo13b2o2b$31b2o3bo12b$b2o30b3o13b$b2o46b$33b3o13b$31b2o3bo12b$31bo13b2o2b$31bo5bo7b2o2b$32bo3bo12b2$44b2o3b$44b2o3b5$37b2o10b$37bobo7b2o$39bo7b2o$37b3o9b$22bobo24b$21b3o25b$21b3o25b$21bo15b3o9b$25bobo11bo9b$21b2o4bo9bobo9b$16b2o4bo3b2o9b2o10b$15bobo6bo24b$15bo33b$14b2o!".to_owned());
        let grid = original_pattern.to_new_bit_grid(49, 43).unwrap();
        let pattern = grid.to_pattern(None);
        assert_eq!(pattern.0.as_str(), "b2o23b2o$b2o23bo$24bobo$15b2o7b2o$2o13bobo$2o13bob2o$16b2o$16bo$44b2o$\r\n16bo27b2o$16b2o$2o13bob2o13bo3bo$2o13bobo13bo5bo7b2o$15b2o14bo13b2o$\r\n31b2o3bo$b2o30b3o$b2o$33b3o$31b2o3bo$31bo13b2o$31bo5bo7b2o$32bo3bo2$\r\n44b2o$44b2o5$37b2o$37bobo7b2o$39bo7b2o$37b3o$22bobo$21b3o$21b3o$21bo\r\n15b3o$25bobo11bo$21b2o4bo9bobo$16b2o4bo3b2o9b2o$15bobo6bo$15bo$14b2o!");
        for line in pattern.0.as_str().lines() {
            assert!(line.len() <= 70);
        }
    }

    #[test]
    fn get_run_empty() {
        let grid = BitGrid::new(1,1);
        let pattern = grid.to_pattern(None);
        assert_eq!(pattern.0.as_str(), "!");
    }
}
