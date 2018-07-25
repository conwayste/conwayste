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
use universe::Region;


#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum BitOperation {
    Clear,
    Set,
    Toggle
}


#[derive(Debug)]
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

    /// Width in cells
    pub fn width(&self) -> usize {
        self.width_in_words() * 64
    }

    pub fn width_in_words(&self) -> usize {
        if self.height() > 0 {
            self.0[0].len()
        } else {
            0
        }
    }

    /// Height in cells
    pub fn height(&self) -> usize {
        self.0.len()
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
    /// Panics if `col` or `row` are out of range, or if `char` is invalid for this type.
    fn write_at_position(&mut self, col: usize, row: usize, ch: char, visibility: Option<usize>);

    fn is_valid(ch: char) -> bool;
}


const VALID_BIT_GRID_CHARS: [char; 2] = ['b', 'o'];

impl CharGrid for BitGrid {
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
            _ => unimplemented!()
        }
    }

    fn is_valid(ch: char) -> bool {
        VALID_BIT_GRID_CHARS.contains(&ch)
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
    #[should_panic]
    fn fill_grid_with_a_negative_region_panics() {
        let height = 10;
        let width_in_words = 10;

        let mut grid = BitGrid::new(width_in_words, height);
        let region_neg = Region::new(-1, -1, 1, 1);
        grid.modify_region(region_neg, BitOperation::Set);
    }
}
