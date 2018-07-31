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

const MAX_NUMBER: usize = 50000;
const NO_OP_CHAR: char = '"';

use bits::{BitGrid, CharGrid};
use std::collections::BTreeMap;
use std::str::FromStr;


/// This contains just the RLE pattern string. For example: "4bobo$7b3o!"
#[derive(Debug, PartialEq)]
pub struct Pattern(pub String);

/// Represents the contents of a RLE file.
#[derive(Debug, PartialEq)]
pub struct PatternFile {
    pub comment_lines: Vec<String>,
    header_line: HeaderLine,
    pub pattern: Pattern,
}

#[derive(Debug, PartialEq)]
struct HeaderLine {
    pub x: usize, // width (cols)
    pub y: usize, // height (rows)
    pub rule: Option<String>,
}

//TODO: module doc examples


impl PatternFile {
    #[inline]
    pub fn width(&self) -> usize {
        self.header_line.x
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.header_line.y
    }

    pub fn to_new_bit_grid(&self) -> Result<BitGrid, String> {
        self.pattern.to_new_bit_grid(self.width(), self.height())
    }

    pub fn to_grid<G: CharGrid>(&self, grid: &mut G, visibility: Option<usize>) -> Result<(), String> {
        self.pattern.to_grid(grid, visibility)
    }
}

impl FromStr for PatternFile {
    type Err = String;

    /// Generate a PatternFile from the contents of an RLE file.
    fn from_str(file_contents: &str) -> Result<Self, Self::Err> {
        let mut comment_lines: Vec<String> = vec![];
        let mut comments_ended = false;
        let mut opt_header_line: Option<HeaderLine> = None;
        let mut pattern_lines: Vec<&str> = vec![];
        for line in file_contents.lines() {
            if line.starts_with("#") {
                if comments_ended {
                    return Err("Found a comment line after a non-comment line".to_owned());
                }
                comment_lines.push(line.to_owned());
                continue;
            } else {
                comments_ended = true;
            }
            if opt_header_line.is_none() {
                // this line should be a header line
                opt_header_line = Some(HeaderLine::from_str(line)?);
                continue;
            }
            match line.find('!') {
                Some(idx) => {
                    pattern_lines.push(&line[0..=idx]);
                    break; // we don't care about anything after the '!'
                }
                None => pattern_lines.push(line),
            };
        }
        if opt_header_line.is_none() {
            return Err("missing header line".to_owned());
        }
        if pattern_lines.is_empty() {
            return Err("missing pattern lines".to_owned());
        }
        let mut pattern = "".to_owned();
        for line in pattern_lines {
            pattern.push_str(line);
        }
        Ok(PatternFile {
            comment_lines,
            header_line: opt_header_line.unwrap(),
            pattern: Pattern(pattern),
        })
    }
}


impl FromStr for HeaderLine {
    type Err = String;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut map = BTreeMap::new();
        for term in line.split(",") {
            let parts = term
                .split("=")
                .map(|part| part.trim())
                .collect::<Vec<&str>>();
            if parts.len() != 2 {
                return Err(format!("unexpected term in header line: {:?}", term));
            }
            map.insert(parts[0], parts[1]);
        }
        if !map.contains_key("x") || !map.contains_key("y") {
            return Err(format!("header line missing `x` and/or `y`: {:?}", line));
        }
        let x = usize::from_str(map.get("x").unwrap()).map_err(|e| format!("Error while parsing x: {}", e))?;
        let y = usize::from_str(map.get("y").unwrap()).map_err(|e| format!("Error while parsing y: {}", e))?;
        let rule =  map.get("rule").map(|s: &&str| (*s).to_owned());
        Ok(HeaderLine { x, y, rule })
    }
}


fn digits_to_number(digits: &Vec<char>) -> Result<usize, String> {
    let mut result = 0;
    for ch in digits {
        let d = ch.to_digit(10).unwrap();
        result = result * 10 + d as usize;
        if result > MAX_NUMBER {
            return Err(format!("Could not parse digits {:?} because larger than {}", digits, MAX_NUMBER));
        }
    }
    Ok(result)
}


impl Pattern {
    /// Creates a BitGrid out of this pattern. If there are no parse errors, the result contains
    /// the smallest BitGrid that fits a pattern `width` cells wide and `height` cells high.
    pub fn to_new_bit_grid(&self, width: usize, height: usize) -> Result<BitGrid, String> {
        let word_width = (width - 1)/64 + 1;
        let mut grid = BitGrid::new(word_width, height);
        self.to_grid(&mut grid, None)?;
        Ok(grid)
    }

    /// Writes the pattern to a BitGrid or GenState (that is, anything implementing CharGrid).  The
    /// characters in pattern must be valid for the grid, as determined by `::is_valid(ch)`, with
    /// one exception: `NO_OP_CHAR` (`"`). Cells are skipped with runs containing `NO_OP_CHAR`.
    ///
    /// # Panics
    ///
    /// This function will panic if an attempt is made to write out of bounds.
    ///
    /// # Note
    ///
    /// If there is a parsing error, `grid` may be in a partially written state. If this is a
    /// problem, then back up `grid` before calling this.
    pub fn to_grid<G: CharGrid>(&self, grid: &mut G, visibility: Option<usize>) -> Result<(), String> {
        let mut col: usize = 0;
        let mut row: usize = 0;
        let mut char_indices = self.0.char_indices();
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
                _ if G::is_valid(ch) => {
                    // cell
                    let number = if digits.len() > 0 {
                        digits_to_number(&digits)?
                    } else { 1 };
                    digits.clear();
                    if ch != NO_OP_CHAR {
                        for _ in 0..number {
                            grid.write_at_position(col, row, ch, visibility);
                            col += 1;
                        }
                    } else {
                        col += number;
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
                        digits_to_number(&digits)?
                    } else { 1 };
                    digits.clear();
                    col = 0;
                    row += number;
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
            return Err("Premature termination".to_owned());
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // Glider gun
    #[test]
    fn one_line_parsing_works1() {
        let gun = Pattern("24bo$22bobo$12b2o6b2o12b2o$11bo3bo4b2o12b2o$2o8bo5bo3b2o$2o8bo3bob2o4bobo$10bo5bo7bo$11bo3bo$12b2o!".to_owned()).to_new_bit_grid(36, 9).unwrap();
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
        let gun = Pattern("24bo$22bobo$12b2o6b2o\r\n12b2o$\r\n11bo3bo4b2o12b2o$2o8b\ro5bo3b2o$2o8bo3bob2o4b\nobo$10bo5bo7bo$11bo3bo$12b2o!".to_owned()).to_new_bit_grid(36, 9).unwrap();
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
        let pattern = Pattern("27bo$28bo$29bo$28bo$27bo$29b3o20$oo$bbo$bbo$3b4o!".to_owned()).to_new_bit_grid(32, 29).unwrap();
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


    #[test]
    fn parse_whole_file_works() {
        let pat: PatternFile = PatternFile::from_str("#N Gosper glider gun\n#C This was the first gun discovered.\n#C As its name suggests, it was discovered by Bill Gosper.\nx = 36, y = 9, rule = B3/S23\n24bo$22bobo$12b2o6b2o12b2o$11bo3bo4b2o12b2o$2o8bo5bo3b2o$2o8bo3bob2o4b\nobo$10bo5bo7bo$11bo3bo$12b2o!\n").unwrap();
        assert_eq!(pat.comment_lines.len(), 3);
        assert_eq!(pat.header_line, HeaderLine{x: 36, y: 9, rule: Some("B3/S23".to_owned())});
        assert_eq!(pat.pattern.0, "24bo$22bobo$12b2o6b2o12b2o$11bo3bo4b2o12b2o$2o8bo5bo3b2o$2o8bo3bob2o4bobo$10bo5bo7bo$11bo3bo$12b2o!");
    }

    #[test]
    fn parse_whole_file_works_with_crap_at_the_end() {
        let pat: PatternFile = PatternFile::from_str("#N Gosper glider gun\n#C This was the first gun discovered.\n#C As its name suggests, it was discovered by Bill Gosper.\nx = 36, y = 9, rule = B3/S23\n24bo$22bobo$12b2o6b2o12b2o$11bo3bo4b2o12b2o$2o8bo5bo3b2o$2o8bo3bob2o4b\nobo$10bo5bo7bo$11bo3bo$12b2o!blah\n\nyaddayadda\n").unwrap();
        assert_eq!(pat.comment_lines.len(), 3);
        assert_eq!(pat.header_line, HeaderLine{x: 36, y: 9, rule: Some("B3/S23".to_owned())});
        assert_eq!(pat.pattern.0, "24bo$22bobo$12b2o6b2o12b2o$11bo3bo4b2o12b2o$2o8bo5bo3b2o$2o8bo3bob2o4bobo$10bo5bo7bo$11bo3bo$12b2o!");
    }

    #[test]
    fn parse_whole_file_works_vacuum_cleaner() {
        let pat: PatternFile = PatternFile::from_str("#N Vacuum (gun)\r\n#O Dieter Leithner\r\n#C A true period 46 double-barreled gun found on February 21, 1997.\r\n#C www.conwaylife.com/wiki/index.php?title=Vacuum_(gun)\r\nx = 49, y = 43, rule = b3/s23\r\nb2o23b2o21b$b2o23bo22b$24bobo22b$15b2o7b2o23b$2o13bobo31b$2o13bob2o30b\r\n$16b2o31b$16bo32b$44b2o3b$16bo27b2o3b$16b2o31b$2o13bob2o13bo3bo12b$2o\r\n13bobo13bo5bo7b2o2b$15b2o14bo13b2o2b$31b2o3bo12b$b2o30b3o13b$b2o46b$\r\n33b3o13b$31b2o3bo12b$31bo13b2o2b$31bo5bo7b2o2b$32bo3bo12b2$44b2o3b$44b\r\n2o3b5$37b2o10b$37bobo7b2o$39bo7b2o$37b3o9b$22bobo24b$21b3o25b$21b3o25b\r\n$21bo15b3o9b$25bobo11bo9b$21b2o4bo9bobo9b$16b2o4bo3b2o9b2o10b$15bobo6b\r\no24b$15bo33b$14b2o!").unwrap();
        assert_eq!(pat.comment_lines.len(), 4);
        assert_eq!(pat.header_line, HeaderLine{x: 49, y: 43, rule: Some("B3/S23".to_owned().to_lowercase())});
        assert_eq!(pat.pattern.0, "b2o23b2o21b$b2o23bo22b$24bobo22b$15b2o7b2o23b$2o13bobo31b$2o13bob2o30b$16b2o31b$16bo32b$44b2o3b$16bo27b2o3b$16b2o31b$2o13bob2o13bo3bo12b$2o13bobo13bo5bo7b2o2b$15b2o14bo13b2o2b$31b2o3bo12b$b2o30b3o13b$b2o46b$33b3o13b$31b2o3bo12b$31bo13b2o2b$31bo5bo7b2o2b$32bo3bo12b2$44b2o3b$44b2o3b5$37b2o10b$37bobo7b2o$39bo7b2o$37b3o9b$22bobo24b$21b3o25b$21b3o25b$21bo15b3o9b$25bobo11bo9b$21b2o4bo9bobo9b$16b2o4bo3b2o9b2o10b$15bobo6bo24b$15bo33b$14b2o!");
        assert_eq!(pat.to_new_bit_grid().unwrap(), BitGrid(vec![
            vec![0b0110000000000000000000000011000000000000000000000000000000000000],
            vec![0b0110000000000000000000000010000000000000000000000000000000000000],
            vec![0b0000000000000000000000001010000000000000000000000000000000000000],
            vec![0b0000000000000001100000001100000000000000000000000000000000000000],
            vec![0b1100000000000001010000000000000000000000000000000000000000000000],
            vec![0b1100000000000001011000000000000000000000000000000000000000000000],
            vec![0b0000000000000000110000000000000000000000000000000000000000000000],
            vec![0b0000000000000000100000000000000000000000000000000000000000000000],
            vec![0b0000000000000000000000000000000000000000000011000000000000000000],
            vec![0b0000000000000000100000000000000000000000000011000000000000000000],
            vec![0b0000000000000000110000000000000000000000000000000000000000000000],
            vec![0b1100000000000001011000000000000010001000000000000000000000000000],
            vec![0b1100000000000001010000000000000100000100000001100000000000000000],
            vec![0b0000000000000001100000000000000100000000000001100000000000000000],
            vec![0b0000000000000000000000000000000110001000000000000000000000000000],
            vec![0b0110000000000000000000000000000001110000000000000000000000000000],
            vec![0b0110000000000000000000000000000000000000000000000000000000000000],
            vec![0b0000000000000000000000000000000001110000000000000000000000000000],
            vec![0b0000000000000000000000000000000110001000000000000000000000000000],
            vec![0b0000000000000000000000000000000100000000000001100000000000000000],
            vec![0b0000000000000000000000000000000100000100000001100000000000000000],
            vec![0b0000000000000000000000000000000010001000000000000000000000000000],
            vec![0b0000000000000000000000000000000000000000000000000000000000000000],
            vec![0b0000000000000000000000000000000000000000000011000000000000000000],
            vec![0b0000000000000000000000000000000000000000000011000000000000000000],
            vec![0b0000000000000000000000000000000000000000000000000000000000000000],
            vec![0b0000000000000000000000000000000000000000000000000000000000000000],
            vec![0b0000000000000000000000000000000000000000000000000000000000000000],
            vec![0b0000000000000000000000000000000000000000000000000000000000000000],
            vec![0b0000000000000000000000000000000000000110000000000000000000000000],
            vec![0b0000000000000000000000000000000000000101000000011000000000000000],
            vec![0b0000000000000000000000000000000000000001000000011000000000000000],
            vec![0b0000000000000000000000000000000000000111000000000000000000000000],
            vec![0b0000000000000000000000101000000000000000000000000000000000000000],
            vec![0b0000000000000000000001110000000000000000000000000000000000000000],
            vec![0b0000000000000000000001110000000000000000000000000000000000000000],
            vec![0b0000000000000000000001000000000000000111000000000000000000000000],
            vec![0b0000000000000000000000000101000000000001000000000000000000000000],
            vec![0b0000000000000000000001100001000000000101000000000000000000000000],
            vec![0b0000000000000000110000100011000000000110000000000000000000000000],
            vec![0b0000000000000001010000001000000000000000000000000000000000000000],
            vec![0b0000000000000001000000000000000000000000000000000000000000000000],
            vec![0b0000000000000011000000000000000000000000000000000000000000000000]]));
    }
}
