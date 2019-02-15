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

const MAX_NUMBER: usize = 50000;
pub const NO_OP_CHAR: char = '"';

use grids::{BitGrid, CharGrid};
use std::collections::BTreeMap;
use std::str::FromStr;


/// This contains just the RLE pattern string. For example: "4bobo$7b3o!"
#[derive(Debug, PartialEq, Clone)]
pub struct Pattern(pub String);

/// Represents the contents of a RLE file.
#[derive(Debug, PartialEq, Clone)]
pub struct PatternFile {
    pub comment_lines: Vec<String>,
    pub header_line: HeaderLine,
    pub pattern: Pattern,
}

#[derive(Debug, PartialEq, Clone)]
pub struct HeaderLine {
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
