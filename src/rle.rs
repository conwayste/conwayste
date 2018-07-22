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

use bits::BitGrid;
use std::collections::BTreeMap;
use std::str::FromStr;


#[derive(Debug, PartialEq)]
pub struct Pattern(String);

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
    pub fn to_new_bit_grid(&self, width: usize, height: usize) -> Result<BitGrid, String> {
        let word_width = (width - 1)/64 + 1;
        let mut grid = BitGrid::new(word_width, height);
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
                'b' | 'o' => {
                    // cell
                    let number = if digits.len() > 0 {
                        digits_to_number(&digits)?
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
        Ok(grid)
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
}
