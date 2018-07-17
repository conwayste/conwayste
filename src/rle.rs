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

use bits::BitGrid;

fn write_at_position(grid: &mut BitGrid, col: usize, row: usize, ch: char) {
    let word_col = col/64;
    let shift = 63 - (col & (64 - 1));
    match ch {
        'b' => {
            grid[row][word_col] &= !(1 << shift)
        }
        'o' => {
            grid[row][word_col] |=   1 << shift
        }
        _ => unimplemented!()
    }
}


fn digits_to_number(digits: &Vec<char>) -> usize {
    let mut result = 0;
    for ch in digits {
        let d = ch.to_digit(10).unwrap();
        result = result * 10 + d as usize;
    }
    result
}


fn from_rle(pattern: &str, width: usize, height: usize) -> Result<BitGrid, String> {
    let word_width = (width - 1)/64 + 1;
    let mut grid: BitGrid = {
        let mut rows: Vec<Vec<u64>> = Vec::with_capacity(height);
        for _ in 0..height {
            let row: Vec<u64> = vec![0; word_width];
            rows.push(row);
        }
        BitGrid(rows)
    };
    let mut col: usize = 0;
    let mut row: usize = 0;
    let mut char_indices = pattern.char_indices();
    let mut ch;
    let mut i;
    let mut complete = false;
    let mut digits: Vec<char> = vec![];
    loop {
        match char_indices.next() {
            Some((_i, _ch)) => {
                ch = _ch;
                i = _i;
            }
            None => break
        };
        if digits.len() > 0 && ch == '!' {
            return Err(format!("Cannot have {} after number at {}", ch, i));
        }
        match ch {
            'b' | 'o' => {
                // cell
                let number = if digits.len() > 0 {
                    digits_to_number(&digits)
                } else { 1 };
                digits.clear();
                for _ in 0..number {
                    write_at_position(&mut grid, col, row, ch);
                    col += 1;
                }
            }
            '!' => {
                // end of input
                complete = true;
                break;
            }
            '$' => {
                // new line
                let number = if digits.len() > 0 {
                    digits_to_number(&digits)
                } else { 1 };
                digits.clear();
                col = 0;
                for _ in 0..number {
                    row += 1;
                }
            }
            '\r' | '\n' => {
                // ignore newlines
            }
            x if x.is_digit(10) => {
                digits.push(ch);
            }
            _ => {
                return Err(format!("Unrecognized character {} at {}", ch, i));
            }
        }
    }
    if !complete {
        return Err("Premature termination".to_string());
    }
    Ok(grid)
}


#[cfg(test)]
mod tests {
    use super::*;

    // Glider gun
    #[test]
    fn one_line_parsing_works1() {
        let gun = from_rle("24bo$22bobo$12b2o6b2o12b2o$11bo3bo4b2o12b2o$2o8bo5bo3b2o$2o8bo3bob2o4bobo$10bo5bo7bo$11bo3bo$12b2o!", 36, 9).unwrap();
        assert_eq!(gun[0][0], 0b0000000000000000000000001000000000000000000000000000000000000000);
        assert_eq!(gun[1][0], 0b0000000000000000000000101000000000000000000000000000000000000000);
        assert_eq!(gun[2][0], 0b0000000000001100000011000000000000110000000000000000000000000000);
        assert_eq!(gun[3][0], 0b0000000000010001000011000000000000110000000000000000000000000000);
        assert_eq!(gun[4][0], 0b1100000000100000100011000000000000000000000000000000000000000000);
        assert_eq!(gun[5][0], 0b1100000000100010110000101000000000000000000000000000000000000000);
        assert_eq!(gun[6][0], 0b0000000000100000100000001000000000000000000000000000000000000000);
        assert_eq!(gun[7][0], 0b0000000000010001000000000000000000000000000000000000000000000000);
        assert_eq!(gun[8][0], 0b0000000000001100000000000000000000000000000000000000000000000000);
    }


    // Glider gun with line break
    #[test]
    fn multi_line_parsing_works1() {
        let gun = from_rle("24bo$22bobo$12b2o6b2o\r\n12b2o$\r\n11bo3bo4b2o12b2o$2o8b\ro5bo3b2o$2o8bo3bob2o4b\nobo$10bo5bo7bo$11bo3bo$12b2o!", 36, 9).unwrap();
        assert_eq!(gun[0][0], 0b0000000000000000000000001000000000000000000000000000000000000000);
        assert_eq!(gun[1][0], 0b0000000000000000000000101000000000000000000000000000000000000000);
        assert_eq!(gun[2][0], 0b0000000000001100000011000000000000110000000000000000000000000000);
        assert_eq!(gun[3][0], 0b0000000000010001000011000000000000110000000000000000000000000000);
        assert_eq!(gun[4][0], 0b1100000000100000100011000000000000000000000000000000000000000000);
        assert_eq!(gun[5][0], 0b1100000000100010110000101000000000000000000000000000000000000000);
        assert_eq!(gun[6][0], 0b0000000000100000100000001000000000000000000000000000000000000000);
        assert_eq!(gun[7][0], 0b0000000000010001000000000000000000000000000000000000000000000000);
        assert_eq!(gun[8][0], 0b0000000000001100000000000000000000000000000000000000000000000000);
    }

    #[test]
    fn parsing_with_new_row_rle_works() {
        let pattern = from_rle("27bo$28bo$29bo$28bo$27bo$29b3o20$oo$bbo$bbo$3b4o!", 32, 29).unwrap();
        assert_eq!(pattern.0[0..=5],
           [[0b0000000000000000000000000001000000000000000000000000000000000000],
            [0b0000000000000000000000000000100000000000000000000000000000000000],
            [0b0000000000000000000000000000010000000000000000000000000000000000],
            [0b0000000000000000000000000000100000000000000000000000000000000000],
            [0b0000000000000000000000000001000000000000000000000000000000000000],
            [0b0000000000000000000000000000011100000000000000000000000000000000]]);
        for row in 6..=24 {
            assert_eq!(pattern.0[row][0], 0);
        }
        assert_eq!(pattern.0[25..=28],
            [[0b1100000000000000000000000000000000000000000000000000000000000000],
             [0b0010000000000000000000000000000000000000000000000000000000000000],
             [0b0010000000000000000000000000000000000000000000000000000000000000],
             [0b0001111000000000000000000000000000000000000000000000000000000000]]);
    }
}
