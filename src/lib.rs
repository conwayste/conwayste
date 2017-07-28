/*  Copyright 2017 the ConWaysteTheEnemy Developers.
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

use std::fmt;


/// Represents a wrapping universe in Conway's game of life.
pub struct Universe {
    width:           usize,
    height:          usize,
    width_in_words:  usize,                     // width in u64 elements, _not_ width in cells!
    generation:      usize,                     // current generation (1-based)
    num_players:     usize,                     // number of players in the game (player numbers are 0-based)
    state_index:     usize,                     // index of GenState for current generation within gen_states
    gen_states:      Vec<GenState>,             // circular buffer
    player_writable: Vec<Region>,               // writable region (indexed by player_id)
}

type BitGrid = Vec<Vec<u64>>;

struct GenState {
    gen_or_none:   Option<usize>,        // Some(generation number) (redundant info); if None, this is an unused buffer
    cells:         BitGrid,              // 1 = cell is known to be Alive
    wall_cells:    BitGrid,              // 1 = is a wall cell (should this just be fixed for the universe?)
    known:         BitGrid,              // 1 = cell is known (always 1 if this is server)
    player_states: Vec<PlayerGenState>,  // player-specific info (indexed by player_id)
}

struct PlayerGenState {
    cells:     BitGrid,   // cells belonging to this player (if 1 here, must be 1 in GenState cells)
    fog:       BitGrid,   // cells that the player is not allowed to know
}


#[derive(Eq,PartialEq,Ord,PartialOrd,Copy,Clone)]
pub enum CellState {
    Dead,
    Alive(Option<usize>),    // Some(player_number) or alive but not belonging to any player
    Wall,
    Fog,
}


impl CellState {
    // Roughly follows RLE specification: http://www.conwaylife.com/wiki/Run_Length_Encoded
    pub fn to_char(self) -> char {
        match self {
            CellState::Alive(Some(player_id)) => {
                if player_id >= 23 {
                    panic!("Player IDs must be less than 23 to be converted to chars");
                }
                std::char::from_u32(player_id as u32 + 65).unwrap()
            }
            CellState::Alive(None) => 'o',
            CellState::Dead        => 'b',
            CellState::Wall        => 'W',
            CellState::Fog         => '?',
        }
    }
}


fn new_bitgrid(width_in_words: usize, height: usize) -> BitGrid {
    let mut result: BitGrid = Vec::new();
    for _ in 0 .. height {
        let row: Vec<u64> = vec![0; width_in_words];
        result.push(row);
    }
    result
}


// Sets or clears a rectangle of bits. Panics if Region is out of range.
fn fill_region(grid: &mut BitGrid, region: Region, bit: bool) {
    for y in region.top() .. region.bottom() + 1 {
        for word_col in 0 .. grid[y as usize].len() {
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
                if bit {
                    grid[y as usize][word_col] |=  mask;
                } else {
                    grid[y as usize][word_col] &= !mask;
                }
            }
        }
    }
}


impl fmt::Display for Universe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let cells = &self.gen_states[self.state_index].cells;
        let wall  = &self.gen_states[self.state_index].wall_cells;
        let known = &self.gen_states[self.state_index].known;
        for row_idx in 0 .. self.height {
            for col_idx in 0 .. self.width_in_words {
                let cell_cen  = cells[row_idx][col_idx];
                let wall_cen  = wall [row_idx][col_idx];
                let known_cen = known[row_idx][col_idx];
                let mut s = String::with_capacity(64);
                for shift in (0..64).rev() {
                    if (known_cen>>shift)&1 == 0 {
                        s.push('?');
                    } else if (cell_cen>>shift)&1 == 1 {
                        let mut is_player = false;
                        for player_id in 0 .. self.num_players {
                            let player_word = self.gen_states[self.state_index].player_states[player_id].cells[row_idx][col_idx];
                            if (player_word>>shift)&1 == 1 {
                                s.push(std::char::from_u32(player_id as u32 + 65).unwrap());
                                is_player = true;
                                break;
                            }
                        }
                        if !is_player { s.push('*'); }
                    } else if (wall_cen>>shift)&1 == 1 {
                        s.push('W');
                    } else {
                        s.push(' ');
                    }
                }
                try!(write!(f, "{}", s));
            }
            try!(write!(f, "\n"));
        }
        Ok(())
    }
}


// TODO: unit tests
impl Universe {
    // sets the state of a cell, with minimal checking
    // doesn't support setting CellState::Fog
    pub fn set_unchecked(&mut self, col: usize, row: usize, new_state: CellState) {
        let gen_state = &mut self.gen_states[self.state_index];
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1));
        let mask  = 1 << shift;     // cell to set

        // panic if not known
        // known_cell_word is the current word of 64 cells
        let known_cell_word = gen_state.known[row][word_col];
        if known_cell_word & mask == 0 {
            panic!("Tried to set unknown cell at ({}, {})", col, row);
        }

        // clear all player cell bits, so that this cell is unowned by any player (we'll set
        // ownership further down)
        {
            for player_id in 0 .. self.num_players {
                gen_state.player_states[player_id].cells[row][word_col] &= !mask;
            }
        }

        let cells = &mut gen_state.cells;
        let wall  = &mut gen_state.wall_cells;
        let mut cells_word = cells[row][word_col];
        let mut walls_word = wall [row][word_col];
        match new_state {
            CellState::Dead => {
                cells_word &= !mask;
                walls_word &= !mask;
            }
            CellState::Alive(opt_player_id) => {
                cells_word |=  mask;
                walls_word &= !mask;
                if let Some(player_id) = opt_player_id {
                    gen_state.player_states[player_id].cells[row][word_col] |=  mask;
                    gen_state.player_states[player_id].fog[row][word_col]   &= !mask;
                }
            }
            CellState::Wall => {
                cells_word &= !mask;
                walls_word |=  mask;
            }
            _ => unimplemented!()
        }
        cells[row][word_col] = cells_word;
        wall [row][word_col] = walls_word;
    }


    // Checked set - check for:
    //   :) current cell state (can't change wall)
    //   :) player writable region
    //   :) fog
    //   :) if current cell is alive, player_id matches player_id argument
    // if checks fail, do nothing
    // panic if player_id inside CellState does not match player_id argument
    pub fn set(&mut self, col: usize, row: usize, new_state: CellState, player_id: usize) {
        {
            let gen_state = &mut self.gen_states[self.state_index];
            let word_col = col/64;
            let shift = 63 - (col & (64 - 1));
            let mask  = 1 << shift;     // cell to set

            let cells = &mut gen_state.cells;
            let wall  = &mut gen_state.wall_cells;
            let cells_word = cells[row][word_col];
            let walls_word = wall [row][word_col];

            if walls_word & mask > 0 {
                return;
            }

            if !self.player_writable[player_id].contains(col, row) {
                return;
            }

            if gen_state.player_states[player_id].fog[row][word_col] & mask > 0 {
                return;
            }

            // If the current cell is alive but not owned by this player, do nothing
            if cells_word & mask > 0 && gen_state.player_states[player_id].cells[row][word_col] & mask == 0 {
                return;
            }

            if let CellState::Alive(Some(new_state_player_id)) = new_state {
                if new_state_player_id != player_id {
                    panic!("A player cannot set the cell state of another player");
                }
            }
        }
            
        self.set_unchecked(col, row, new_state)
    }


    // Switches any non-dead state to CellState::Dead.
    // Switches CellState::Dead to CellState::Alive(opt_player_id) and clears fog for that player,
    // if any.
    pub fn toggle_unchecked(&mut self, col: usize, row: usize, opt_player_id: Option<usize>) -> CellState {
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1));

        let mask = 1 << shift;
        let mut word;
        {
            let cells = &mut self.gen_states[self.state_index].cells;
            word = cells[row][word_col];
            word ^= mask;
            cells[row][word_col] = word;
        }
        let next_cell = (word & mask) > 0;

        // clear all player cell bits
        for player_id in 0 .. self.num_players {
            self.gen_states[self.state_index].player_states[player_id].cells[row][word_col] &= !mask;
        }

        if next_cell {
            // set this player's cell bit, if needed, and clear fog
            if let Some(player_id) = opt_player_id {
                self.gen_states[self.state_index].player_states[player_id].cells[row][word_col] |= mask;
                self.gen_states[self.state_index].player_states[player_id].fog[row][word_col]   &= !mask;
            }

            CellState::Alive(opt_player_id)
        } else {
            CellState::Dead
        }
    }


    // Checked toggle - switch between CellState::Alive and CellState::Dead.
    // Result is Err if trying to toggle outside player's writable area, or if
    // trying to toggle a wall or an unknown cell.
    pub fn toggle(&mut self, col: usize, row: usize, player_id: usize) -> Result<CellState, ()> {
        if !self.player_writable[player_id].contains(col, row) {
            return Err(());
        }

        let word_col = col/64;
        let shift = 63 - (col & (64 - 1));
        {
            let wall  = &self.gen_states[self.state_index].wall_cells;
            let known = &self.gen_states[self.state_index].known;
            if (wall[row][word_col] >> shift) & 1 == 1 || (known[row][word_col] >> shift) & 1 == 0 {
                return Err(());
            }
        }
        Ok(self.toggle_unchecked(col, row, Some(player_id)))
    }


    /// Instantiate a new blank universe with the given width and height, in cells.
    /// The universe is at generation 1.
    pub fn new(width:           usize,
               height:          usize,
               is_server:       bool,
               history:         usize,
               num_players:     usize,
               player_writable: Vec<Region>) -> Result<Universe, &'static str> {
        if height == 0 {
            return Err("Height must be positive");
        }
        let width_in_words = width/64;
        if width != width_in_words * 64 {
            return Err("Width must be a multiple of 64");
        } else if width == 0 {
            return Err("Width must be positive");
        }

        let mut gen_states = Vec::new();
        for i in 0 .. history {
            let mut player_states = Vec::new();
            for player_id in 0 .. num_players {
                let mut pgs = PlayerGenState {
                    cells:     new_bitgrid(width_in_words, height),
                    fog:       new_bitgrid(width_in_words, height),
                };
                // unless writable region, the whole grid is player fog
                fill_region(&mut pgs.fog, Region::new(0, 0, width, height), true);
                // clear player fog on writable regions
                fill_region(&mut pgs.fog, player_writable[player_id], false);
                player_states.push(pgs);
            }
            let mut known = new_bitgrid(width_in_words, height);
            if is_server && i == 0 {
                for y in 0 .. height {
                    for x in 0 .. width_in_words {
                        known[y][x] = u64::max_value();   // if server, all cells are known
                    }
                }
            }
            gen_states.push(GenState {
                gen_or_none:   if i == 0 { Some(1) } else { None },
                cells:         new_bitgrid(width_in_words, height),
                wall_cells:    new_bitgrid(width_in_words, height),
                known:         known,
                player_states: player_states,
            });
        }

        Ok(Universe {
            width:           width,
            height:          height,
            width_in_words:  width_in_words,
            generation:      1,
            num_players:     num_players,
            state_index:     0,
            gen_states:      gen_states,
            player_writable: player_writable,
        })
    }


    /// Return width in cells.
    pub fn width(&self) -> usize {
        return self.width;
    }


    /// Return height in cells.
    pub fn height(&self) -> usize {
        return self.height;
    }


    /// Get the latest generation number (1-based).
    pub fn latest_gen(&self) -> usize {
        self.generation
    }

    fn next_single_gen(nw: u64, n: u64, ne: u64, w: u64, center: u64, e: u64, sw: u64, s: u64, se: u64) -> u64 {
        let a  = (nw     << 63) | (n      >>  1);
        let b  =  n;
        let c  = (n      <<  1) | (ne     >> 63);
        let d  = (w      << 63) | (center >> 1);
        let y6 = center;
        let e  = (center <<  1) | (e      >> 63);
        let f  = (sw     << 63) | (s      >>  1);
        let g  =  s;
        let h  = (s      <<  1) | (se     >> 63);

        // full adder #1
        let b_xor_c = b^c;
        let y1 = (a & b_xor_c) | (b & c);
        let y2 = a ^ b_xor_c;

        // full adder #2
        let e_xor_f = e^f;
        let c2 = (d & e_xor_f) | (e & f);
        let s2 = d ^ e_xor_f;

        // half adder #1
        let c3 = g & h;
        let s3 = g ^ h;

        // half adder #2
        let c4 = s2 & s3;
        let y5 = s2 ^ s3;

        // full adder #3
        let c2_xor_c3 = c2 ^ c3;
        let y3 = (c4 & c2_xor_c3) | (c2 & c3);
        let y4 = c4 ^ c2_xor_c3;

        let int1 = !y3 & !y4;
        !y1&y6&(y2&int1&y5 | y4&!y5) | y1&int1&(!y2&(y5 | y6) | y2&!y5) | !y1&y4&(y2^y5)
    }

    /*
     * A B C
     * D   E
     * F G H
     */
    // a cell is 0 if itself or any of its neighbors are 0
    fn contagious_zero(nw: u64, n: u64, ne: u64, w: u64, center: u64, e: u64, sw: u64, s: u64, se: u64) -> u64 {
        let a  = (nw     << 63) | (n      >>  1);
        let b  =  n;
        let c  = (n      <<  1) | (ne     >> 63);
        let d  = (w      << 63) | (center >> 1);
        let e  = (center <<  1) | (e      >> 63);
        let f  = (sw     << 63) | (s      >>  1);
        let g  =  s;
        let h  = (s      <<  1) | (se     >> 63);
        a & b & c & d & center & e & f & g & h
    }


    // a cell is 1 if itself or any of its neighbors are 1
    fn contagious_one(nw: u64, n: u64, ne: u64, w: u64, center: u64, e: u64, sw: u64, s: u64, se: u64) -> u64 {
        let a  = (nw     << 63) | (n      >>  1);
        let b  =  n;
        let c  = (n      <<  1) | (ne     >> 63);
        let d  = (w      << 63) | (center >> 1);
        let e  = (center <<  1) | (e      >> 63);
        let f  = (sw     << 63) | (s      >>  1);
        let g  =  s;
        let h  = (s      <<  1) | (se     >> 63);
        a | b | c | d | center | e | f | g | h
    }


    /// Compute the next generation. Returns the new latest generation number.
    // TODO: write some good unit tests covering all features and cases, then optimize & rewrite (use macros?)
    pub fn next(&mut self) -> usize {
        // get the buffers and buffers_next
        assert!(self.gen_states[self.state_index].gen_or_none.unwrap() == self.generation);
        let history = self.gen_states.len();
        let next_state_index = (self.state_index + 1) % history;

        let (gen_state, gen_state_next) = if self.state_index < next_state_index {
            let (p0, p1) = self.gen_states.split_at_mut(next_state_index);
            (&p0[next_state_index - 1], &mut p1[0])
        } else {
            // self.state_index == history-1 and next_state_index == 0
            let (p0, p1) = self.gen_states.split_at_mut(next_state_index + 1);
            (&p1[history - 2], &mut p0[0])
        };

        {
            let cells      = &gen_state.cells;
            let wall       = &gen_state.wall_cells;
            let known      = &gen_state.known;
            let cells_next = &mut gen_state_next.cells;
            let wall_next  = &mut gen_state_next.wall_cells;
            let known_next = &mut gen_state_next.known;
            for row_idx in 0 .. self.height {
                let n_row_idx = (row_idx + self.height - 1) % self.height;
                let s_row_idx = (row_idx + 1) % self.height;
                let cells_row_n = &cells[n_row_idx];
                let cells_row_c = &cells[ row_idx ];
                let cells_row_s = &cells[s_row_idx];
                let wall_row_c  = &wall[ row_idx ];
                let known_row_n = &known[n_row_idx];
                let known_row_c = &known[ row_idx ];
                let known_row_s = &known[s_row_idx];

                // These will be shifted over at the beginning of the loop
                let mut cells_nw;
                let mut cells_w;
                let mut cells_sw;
                let mut cells_n   = cells_row_n[self.width_in_words - 1];
                let mut cells_cen = cells_row_c[self.width_in_words - 1];
                let mut cells_s   = cells_row_s[self.width_in_words - 1];
                let mut cells_ne  = cells_row_n[0];
                let mut cells_e   = cells_row_c[0];
                let mut cells_se  = cells_row_s[0];
                let mut known_nw;
                let mut known_w;
                let mut known_sw;
                let mut known_n   = known_row_n[self.width_in_words - 1];
                let mut known_cen = known_row_c[self.width_in_words - 1];
                let mut known_s   = known_row_s[self.width_in_words - 1];
                let mut known_ne  = known_row_n[0];
                let mut known_e   = known_row_c[0];
                let mut known_se  = known_row_s[0];

                for col_idx in 0 .. self.width_in_words {
                    // shift over
                    cells_nw  = cells_n;
                    cells_n   = cells_ne;
                    cells_w   = cells_cen;
                    cells_cen = cells_e;
                    cells_sw  = cells_s;
                    cells_s   = cells_se;
                    cells_ne  = cells_row_n[(col_idx + 1) % self.width_in_words];
                    cells_e   = cells_row_c[(col_idx + 1) % self.width_in_words];
                    cells_se  = cells_row_s[(col_idx + 1) % self.width_in_words];
                    known_nw  = known_n;
                    known_n   = known_ne;
                    known_w   = known_cen;
                    known_cen = known_e;
                    known_sw  = known_s;
                    known_s   = known_se;
                    known_ne  = known_row_n[(col_idx + 1) % self.width_in_words];
                    known_e   = known_row_c[(col_idx + 1) % self.width_in_words];
                    known_se  = known_row_s[(col_idx + 1) % self.width_in_words];

                    // apply BitGrid changes
                    let mut cells_cen_next = Universe::next_single_gen(cells_nw, cells_n, cells_ne, cells_w, cells_cen, cells_e, cells_sw, cells_s, cells_se);
                    known_next[row_idx][col_idx] = Universe::contagious_zero(known_nw, known_n, known_ne, known_w, known_cen, known_e, known_sw, known_s, known_se);

                    cells_cen_next &= known_next[row_idx][col_idx];
                    cells_cen_next &= !wall_row_c[col_idx];

                    // assign to the u64 element in the next generation
                    cells_next[row_idx][col_idx] = cells_cen_next;

                    let mut in_multiple: u64 = 0;
                    let mut seen_before: u64 = 0;
                    for player_id in 0 .. self.num_players {
                        let player_cell_next =
                            Universe::contagious_one(
                                gen_state.player_states[player_id].cells[n_row_idx][(col_idx + self.width_in_words - 1) % self.width_in_words],
                                gen_state.player_states[player_id].cells[n_row_idx][col_idx],
                                gen_state.player_states[player_id].cells[n_row_idx][(col_idx + 1) % self.width_in_words],
                                gen_state.player_states[player_id].cells[ row_idx ][(col_idx + self.width_in_words - 1) % self.width_in_words],
                                gen_state.player_states[player_id].cells[ row_idx ][col_idx],
                                gen_state.player_states[player_id].cells[ row_idx ][(col_idx + 1) % self.width_in_words],
                                gen_state.player_states[player_id].cells[s_row_idx][(col_idx + self.width_in_words - 1) % self.width_in_words],
                                gen_state.player_states[player_id].cells[s_row_idx][col_idx],
                                gen_state.player_states[player_id].cells[s_row_idx][(col_idx + 1) % self.width_in_words]
                            ) & cells_cen_next;
                        in_multiple |= player_cell_next & seen_before;
                        seen_before |= player_cell_next;
                        gen_state_next.player_states[player_id].cells[row_idx][col_idx] = player_cell_next;
                    }
                    for player_id in 0 .. self.num_players {
                        let mut cell_next = gen_state_next.player_states[player_id].cells[row_idx][col_idx];
                        cell_next &= !in_multiple; // if a cell would have belonged to multiple players, it belongs to none
                        gen_state_next.player_states[player_id].fog[row_idx][col_idx] = gen_state.player_states[player_id].fog[row_idx][col_idx] & !cell_next; // clear fog!
                        gen_state_next.player_states[player_id].cells[row_idx][col_idx] = cell_next;
                    }
                }

                // copy wall to wall_next
                wall_next[row_idx].copy_from_slice(wall_row_c);
            }
        }

        // increment generation in appropriate places
        self.generation += 1;
        self.state_index = next_state_index;
        gen_state_next.gen_or_none = Some(self.generation);
        self.generation
    }


    /// Iterate over every non-dead cell in the universe for the current generation. `region` is
    /// the rectangular area used for restricting results. `visibility` is an optional player_id;
    /// if specified, causes cells not visible to the player to be passed as CellState::Fog to the
    /// callback.
    /// 
    /// Callback receives (x, y, cell_state).
    //TODO: unit test
    pub fn each_non_dead(&self, region: Region, visibility: Option<usize>, callback: &mut FnMut(usize, usize, CellState)) {
        let cells = &self.gen_states[self.state_index].cells;
        let wall  = &self.gen_states[self.state_index].wall_cells;
        let known = &self.gen_states[self.state_index].known;
        let opt_player_state = if let Some(player_id) = visibility {
            Some(&self.gen_states[self.state_index].player_states[player_id])
        } else { None };
        let mut x;
        for y in 0 .. self.height {
            let cells_row = &cells[y];
            let wall_row  = &wall [y];
            let known_row = &known[y];
            if (y as isize) >= region.top() && (y as isize) < (region.top() + region.height() as isize) {
                x = 0;
                for col_idx in 0 .. self.width_in_words {
                    let cells_word = cells_row[col_idx];
                    let wall_word  = wall_row [col_idx];
                    let known_word = known_row[col_idx];
                    let opt_player_words;
                    if let Some(player_state) = opt_player_state {
                        let player_cells_word = player_state.cells[y][col_idx];
                        let player_fog_word   = player_state.fog[y][col_idx];
                        opt_player_words = Some((player_cells_word, player_fog_word));
                    } else {
                        opt_player_words = None;
                    }
                    for shift in (0..64).rev() {
                        if (x as isize) >= region.left() &&
                            (x as isize) < (region.left() + region.width() as isize) {
                            let mut state = CellState::Wall;  // TODO: is this needed? Avoiding error: 'possibly uninitialized'
                            let c = (cells_word>>shift)&1 == 1;
                            let w = (wall_word >>shift)&1 == 1;
                            let k = (known_word>>shift)&1 == 1;
                            if c && w {
                                panic!("Cannot be both cell and wall at ({}, {})", x, y);
                            }
                            if !k && ((c && !w) || (!c && w)) {
                                panic!("Unspecified invalid state at ({}, {})", x, y);
                            }
                            if c && !w && k {
                                // It's known and it's a cell; check cells + fog for every player
                                // (expensive step since this is per-bit).

                                let mut opt_player_id = None;
                                for player_id in 0 .. self.num_players {
                                    let player_state = &self.gen_states[self.state_index].player_states[player_id];
                                    let pc = (player_state.cells[y][col_idx] >> shift) & 1 == 1;
                                    let pf = (player_state.fog[y][col_idx] >> shift) & 1 == 1;
                                    if pc && pf {
                                        panic!("Player cell and player fog at ({}, {}) for player {}", x, y, player_id);
                                    }
                                    if pc {
                                        if let Some(other_player_id) = opt_player_id {
                                            panic!("Cell ({}, {}) belongs to player {} and player {}!", x, y, other_player_id, player_id);
                                        }
                                        opt_player_id = Some(player_id);
                                    }
                                }
                                state = CellState::Alive(opt_player_id);
                            } else {
                                // (B) other states
                                if !c && !w {
                                    state = if k { CellState::Dead } else { CellState::Fog };
                                } else if !c && w {
                                    state = CellState::Wall;
                                }
                            }
                            if let Some((player_cells_word, player_fog_word)) = opt_player_words {
                                let pc = (player_cells_word>>shift)&1 == 1;
                                let pf = (player_fog_word>>shift)&1 == 1;
                                if !k && pc {
                                    panic!("Player can't have cells where unknown, at ({}, {})", x, y);
                                }
                                if w && pc {
                                    panic!("Player can't have cells where wall, at ({}, {})", x, y);
                                }
                                if pf {
                                    state = CellState::Fog;
                                }
                            }
                            if state != CellState::Dead {
                                callback(x, y, state);
                            }
                        }
                        x += 1;
                    }
                }
            }
        }
    }


    /// Iterate over every non-dead cell in the universe for the current generation.
    /// `visibility` is an optional player_id, allowing filtering based on fog.
    /// Callback receives (x, y, cell_state).
    //TODO: unit test
    pub fn each_non_dead_full(&self, visibility: Option<usize>, callback: &mut FnMut(usize, usize, CellState)) {
        self.each_non_dead(self.region(), visibility, callback);
    }


    /// Get a Region of the same size as the universe
    pub fn region(&self) -> Region {
        Region::new(0, 0, self.width, self.height)
    }
}


#[derive(Eq,PartialEq,Ord,PartialOrd,Copy,Clone)]
pub struct Region {
    left:   isize,
    top:    isize,
    width:  usize,
    height: usize,
}

impl Region {
    pub fn new(left: isize, top: isize, width: usize, height: usize) -> Self {
        Region {
            left:   left,
            top:    top,
            width:  width,
            height: height,
        }
    }

    pub fn left(&self) -> isize {
        self.left
    }

    pub fn right(&self) -> isize {
        self.left + (self.width as isize) - 1
    }

    pub fn top(&self) -> isize {
        self.top
    }

    pub fn bottom(&self) -> isize {
        self.top + (self.height as isize) - 1
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn contains(&self, col: usize, row: usize) -> bool {
        self.left    <= col as isize &&
        col as isize <= self.right() &&
        self.top     <= row as isize &&
        row as isize <= self.bottom()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_universe_with_valid_dims() {
        Universe::new(128,64).unwrap();
    }

    #[test]
    fn new_universe_with_bad_dims() {
        let uni_result1 = Universe::new(123,64);
        assert!(uni_result1.is_err());

        let uni_result2 = Universe::new(0,64);
        assert!(uni_result2.is_err());

        let uni_result3 = Universe::new(128,0);
        assert!(uni_result3.is_err());
    }

    #[test]
    fn new_universe_latest_gen_is_one() {
        let uni = Universe::new(128,64).unwrap();
        assert_eq!(uni.latest_gen(), 1);
    }
    #[test]
    #[should_panic]
    fn universe_with_no_gens_panics() {
        let mut uni = Universe::new(128,64).unwrap();
        uni.generation_a = None;
        uni.generation_b = None;
        uni.latest();
    }

    #[test]
    fn next_single_gen_test_data1() {
        // glider, blinker, glider
        let nw = 0x0000000000000000;
        let n  = 0x0000000400000002;
        let ne = 0x8000000000000000;
        let w  = 0x0000000000000001;
        let cen= 0xC000000400000001;
        let e  = 0x8000000000000000;
        let sw = 0x0000000000000000;
        let s  = 0x8000000400000001;
        let se = 0x0000000000000000;
        let next_center = Universe::next_single_gen(nw, n, ne, w, cen, e, sw, s, se);
        assert_eq!(next_center, 0xC000000E00000002);
    }

    #[test]
    fn next_test_data1() {
        let mut uni = Universe::new(128,64).unwrap();
        // r-pentomino
        uni.buffer_a[0][0] = 0x0000000300000000;
        uni.buffer_a[1][0] = 0x0000000600000000;
        uni.buffer_a[2][0] = 0x0000000200000000;
        let gens = 1000;
        for _ in 0..gens {
            uni.next();
        }
        assert_eq!(uni.latest_gen(), gens + 1);
    }
}
