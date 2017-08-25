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
    gen_states:      Vec<GenState>,             // circular buffer of generational states
    player_writable: Vec<Region>,               // writable region (indexed by player_id)
}

type BitGrid = Vec<Vec<u64>>;

// Describes the state of the universe for a particular generation
// This includes any cells alive, known, and each player's own gen states
// for this current session
struct GenState {
    gen_or_none:   Option<usize>,        // Some(generation number) (redundant info); if None, this is an unused buffer
    cells:         BitGrid,              // 1 = cell is known to be Alive
    wall_cells:    BitGrid,              // 1 = is a wall cell (should this just be fixed for the universe?)
    known:         BitGrid,              // 1 = cell is known (always 1 if this is server)
    player_states: Vec<PlayerGenState>,  // player-specific info (indexed by player_id)
}

struct PlayerGenState {
    cells:     BitGrid,   // cells belonging to this player (if 1 here, must be 1 in GenState cells)
    fog:       BitGrid,   // cells that the player is currently invisible to the player
}


#[derive(Eq,PartialEq,Ord,PartialOrd,Copy,Clone,Debug)]
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
    assert!(width_in_words != 0);
    assert!(height != 0);

    let mut result: BitGrid = Vec::new();
    for _ in 0 .. height {
        let row: Vec<u64> = vec![0; width_in_words];
        result.push(row);
    }
    result
}

#[derive(Eq,PartialEq,Debug, Clone, Copy)]
enum BitOperation {
    Clear,
    Set,
    Toggle
}

impl fmt::Display for BitOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        let string = match *self {
            BitOperation::Clear => "Clear",
            BitOperation::Set => "Set",
            BitOperation::Toggle => "Toggle",
        };

        try!(write!(f, "{}", string));
        Ok(())
    }
}

#[inline]
fn modify_cell_bits(bit_grid: &mut BitGrid, row: usize, word_col: usize, mask: u64, op: BitOperation) {

    //debug!("Enter Modify ({}).... [{}][{}] = {}", op, row, word_col, bit_grid[row][word_col] & mask);
    
    match op {
        BitOperation::Set => bit_grid[row][word_col] |= mask,
        BitOperation::Clear => bit_grid[row][word_col] &= !mask,
        BitOperation::Toggle => bit_grid[row][word_col] ^= mask,
    }

    //debug!("...Modified [{}][{}] = {:b}", row, word_col, bit_grid[row][word_col]);
}

// Sets or clears a rectangle of bits. Panics if Region is out of range.
fn fill_region(grid: &mut BitGrid, region: Region, op: BitOperation) {
    for y in region.top() .. region.bottom() + 1 {
        assert!(y >= 0);
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
                modify_cell_bits(grid, y as usize, word_col, mask, op);
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


impl Universe {

    pub fn get_cell_state(&mut self, col: usize, row: usize, player_id: Option<usize>) -> CellState {
        let gen_state = &mut self.gen_states[self.state_index];
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1)); // translate literal col (ex: 134) to bit index in word_col
        let mask  = 1 << shift;     // cell to set

        if let Some(opt_player_id) = player_id {
            let cell = (gen_state.player_states[opt_player_id].cells[row][word_col] & mask) >> shift;
            if cell == 1 {CellState::Alive(player_id)} else {CellState::Dead}
        }
        else {
            let cell = (gen_state.cells[row][word_col] & mask) >> shift;
            if cell == 1 {CellState::Alive(None)} else {CellState::Dead}
        }
    }

    // sets the state of a cell, with minimal checking
    // doesn't support setting CellState::Fog
    pub fn set_unchecked(&mut self, col: usize, row: usize, new_state: CellState) {
        let gen_state = &mut self.gen_states[self.state_index];
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1)); // translate literal col (ex: 134) to bit index in word_col
        let mask  = 1 << shift;     // cell to set

        // panic if not known
        let known_cell_word = gen_state.known[row][word_col];
        if known_cell_word & mask == 0 {
            panic!("Tried to set unknown cell at ({}, {})", col, row);
        }

        // clear all player cell bits, so that this cell is unowned by any player (we'll set
        // ownership further down)
        {
            for player_id in 0 .. self.num_players {
                let ref mut grid = gen_state.player_states[player_id].cells;
                modify_cell_bits(grid, row, word_col, mask, BitOperation::Clear);
            }
        }

        let cells = &mut gen_state.cells;
        let walls  = &mut gen_state.wall_cells;
        match new_state {
            CellState::Dead => {
                modify_cell_bits(cells, row, word_col, mask, BitOperation::Clear);
                modify_cell_bits(walls, row, word_col, mask, BitOperation::Clear);
            }
            CellState::Alive(opt_player_id) => {
                modify_cell_bits(cells, row, word_col, mask, BitOperation::Set);
                modify_cell_bits(walls, row, word_col, mask, BitOperation::Clear);

                if let Some(player_id) = opt_player_id {
                    let ref mut player = gen_state.player_states[player_id];
                    modify_cell_bits(&mut player.cells, row, word_col, mask, BitOperation::Set);
                    modify_cell_bits(&mut player.fog, row, word_col, mask, BitOperation::Clear);
                }
            }
            CellState::Wall => {
                modify_cell_bits(cells, row, word_col, mask, BitOperation::Clear);
                modify_cell_bits(walls, row, word_col, mask, BitOperation::Set);
            }
            _ => unimplemented!()
        }
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
            let mask  = 1 << shift;     // bit to set for cell represented by (row,col)

            let cells = &mut gen_state.cells;
            let wall  = &mut gen_state.wall_cells;
            let cells_word = cells[row][word_col];
            let walls_word = wall [row][word_col];

            if walls_word & mask > 0 {
                return;
            }

            if !self.player_writable[player_id].contains(col as isize, row as isize) { return;
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
    //
    // This operation works in three steps
    //  1. Toggle alive/dead cell in the current generation state cell grid
    //  2. Clear all players' cell
    //  3. If general cell transitioned Dead->Alive, then set requested player's cell
    //  ..
    pub fn toggle_unchecked(&mut self, col: usize, row: usize, opt_player_id: Option<usize>) -> CellState {
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1));
        let mask = 1 << shift;

        let word =
        {
            let cells = &mut self.gen_states[self.state_index].cells;
            modify_cell_bits(cells, row, word_col, mask, BitOperation::Toggle);
            cells[row][word_col]
        };

        // Cell transitioned Dead -> Alive 
        let next_cell = (word & mask) > 0;
        //debug!("Word/Mask: => {:b} | {:b}", word, mask);
        //debug!("Next Cell: {}", next_cell);

        // clear all player cell bits
        for player_id in 0 .. self.num_players {
            let ref mut player_cells = self.gen_states[self.state_index].player_states[player_id].cells;
            modify_cell_bits(player_cells, row, word_col, mask, BitOperation::Clear);
        }

        if next_cell {
            // set this player's cell bit, if needed, and clear fog
            if let Some(player_id) = opt_player_id {
                let ref mut player = self.gen_states[self.state_index].player_states[player_id];
                modify_cell_bits(&mut player.cells, row, word_col, mask, BitOperation::Set);
                modify_cell_bits(&mut player.fog, row, word_col, mask, BitOperation::Clear);
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
        if !self.player_writable[player_id].contains(col as isize, row as isize) {
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
        if width % 64 != 0 {
            return Err("Width must be a multiple of 64");
        } else if width == 0 {
            return Err("Width must be positive");
        }

        // Initialize all generational states with the default appropriate bitgrids
        let mut gen_states = Vec::new();
        for i in 0 .. history {
            let mut player_states = Vec::new();
            for player_id in 0 .. num_players {

                let mut pgs = PlayerGenState {
                    cells:     new_bitgrid(width_in_words, height),
                    fog:       new_bitgrid(width_in_words, height),
                };

                // unless writable region, the whole grid is player fog
                fill_region(&mut pgs.fog, Region::new(0, 0, width, height), BitOperation::Set);

                // clear player fog on writable regions
                fill_region(&mut pgs.fog, player_writable[player_id], BitOperation::Clear);

                player_states.push(pgs);
            }

            // Known cells describe what the current operative (player, server)
            // visibility reaches. For example, a Server has total visibility as
            // it needs to know all.
            let mut known = new_bitgrid(width_in_words, height);
            
            if is_server && i == 0 {
                // could use fill_region but its much cheaper this way
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

                    // any known cells with at least one unknown neighbor will become unknown in
                    // the next generation
                    known_next[row_idx][col_idx] = Universe::contagious_zero(known_nw, known_n, known_ne, known_w, known_cen, known_e, known_sw, known_s, known_se);

                    cells_cen_next &= known_next[row_idx][col_idx];
                    cells_cen_next &= !wall_row_c[col_idx];

                    // assign to the u64 element in the next generation
                    cells_next[row_idx][col_idx] = cells_cen_next;

                    let mut in_multiple: u64 = 0;
                    let mut seen_before: u64 = 0;
                    for player_id in 0 .. self.num_players {
                        // Any unknown cell with 
                        //
                        // A cell which would have belonged to 2+ players in the next
                        // generation will belong to no one. These are unowned cells.
                        //
                        // Unowned cells follow the same rules of life.
                        //
                        // Any unowned cells are influenced by their neighbors, and if players,
                        // can be acquired by the player, just as long as no two players are
                        // fighting over those cells
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
                            let mut state = CellState::Wall;
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
    pub fn each_non_dead_full(&self, visibility: Option<usize>, callback: &mut FnMut(usize, usize, CellState)) {
        self.each_non_dead(self.region(), visibility, callback);
    }


    /// Get a Region of the same size as the universe
    pub fn region(&self) -> Region {
        Region::new(0, 0, self.width, self.height)
    }
}


#[derive(Eq,PartialEq,Ord,PartialOrd,Copy,Clone,Debug)]
pub struct Region {
    left:   isize,
    top:    isize,
    width:  usize,
    height: usize,
}

impl Region {
    // A region is described in game coordinates
    pub fn new(left: isize, top: isize, width: usize, height: usize) -> Self {
        assert!(width != 0);
        assert!(height != 0);

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

    pub fn contains(&self, col: isize, row: isize) -> bool {
        self.left    <= col &&
        col <= self.right() &&
        self.top     <= row &&
        row <= self.bottom()
    }
}


#[cfg(test)]
mod universe_tests {
    use super::*;

    fn generate_test_universe_with_default_params() -> Universe {
        let player0_writable = Region::new(100, 70, 34, 16);   // used for the glider gun and predefined patterns
        let player1_writable = Region::new(0, 0, 80, 80);
        let writable_regions = vec![player0_writable, player1_writable];
 
        Universe::new(256,
                      128,   // height
                      true, // server_mode
                      16,   // history
                      2,    // players
                      writable_regions
                      ).unwrap()
    }

    #[test]
    fn new_universe_with_valid_dims() {
        let uni = generate_test_universe_with_default_params();
        let universe_as_region = Region::new(0, 0, 256, 128);

        assert_eq!(uni.width(), 256);
        assert_eq!(uni.height(), 128);
        assert_eq!(uni.region(), universe_as_region);
    }

    #[test]
    fn new_universe_with_bad_dims() {

        let player0_writable = Region::new(100, 70, 34, 16);   // used for the glider gun and predefined patterns
        let player1_writable = Region::new(0, 0, 80, 80);
        let writable_regions = vec![player0_writable, player1_writable];

        let uni_result1 = Universe::new(255,   // width
                                        128,   // height
                                        true,  // server_mode
                                        16,    // history
                                        2,     // players
                                        writable_regions.clone()
                                      );
        assert!(uni_result1.is_err());

        let uni_result2 = Universe::new(256,  // width
                                        0,    // height
                                        true, // server_mode
                                        16,   // history
                                        2,    // players
                                        writable_regions.clone()
                                      );
        assert!(uni_result2.is_err());

        let uni_result3 = Universe::new(0,   // width
                                        256,   // height
                                        true,  // server_mode
                                        16,    // history
                                        2,     // players
                                        writable_regions.clone()
                                      );
        assert!(uni_result3.is_err());
    }

    #[test]
    fn new_universe_first_gen_is_one() {
        let uni = generate_test_universe_with_default_params();
        assert_eq!(uni.latest_gen(), 1);
    }

    #[test]
    #[should_panic]
    fn universe_with_no_gens_panics() {
        let player0_writable = Region::new(100, 70, 34, 16);   // used for the glider gun and predefined patterns
        let player1_writable = Region::new(0, 0, 80, 80);
        let writable_regions = vec![player0_writable, player1_writable];


        let mut uni = Universe::new(128,  // width
                      64,   // height
                      true, // server_mode
                      16,   // history
                      2,    // players
                      writable_regions
                      ).unwrap();

        uni.generation = 0;
        uni.latest_gen();
    }

    #[test]
    fn next_single_gen_test_data1_with_wrapping() {
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
        let mut uni = generate_test_universe_with_default_params();

        // r-pentomino
        let _ = uni.toggle(16, 15, 0);
        let _ = uni.toggle(17, 15, 0);
        let _ = uni.toggle(15, 16, 0);
        let _ = uni.toggle(16, 16, 0);
        let _ = uni.toggle(16, 17, 0);

        let gens = 1000;
        for _ in 0..gens {
            uni.next();
        }
        assert_eq!(uni.latest_gen(), gens + 1);
    }

    #[test]
    fn set_unchecked_with_valid_rows_and_cols() {
        let mut uni = generate_test_universe_with_default_params();
        let max_width = uni.width()-1;
        let max_height = uni.height()-1;
        let mut cell_state;
        
        for x in 0.. max_width {
            for y in 0..max_height {
                cell_state = uni.get_cell_state(x,y, None);
                assert_eq!(cell_state, CellState::Dead);
            }
        }

        uni.set_unchecked(0, 0, CellState::Alive(None));
        cell_state = uni.get_cell_state(0,0, None);
        assert_eq!(cell_state, CellState::Alive(None));

        uni.set_unchecked(max_width, max_height, CellState::Alive(None));
        assert_eq!(cell_state, CellState::Alive(None));

        uni.set_unchecked(55, 55, CellState::Alive(None));
        assert_eq!(cell_state, CellState::Alive(None));
   }

    #[test]
    #[should_panic]
    fn set_unchecked_with_invalid_rols_and_cols_panics() {
        let mut uni = generate_test_universe_with_default_params();
        uni.set_unchecked(257, 129, CellState::Alive(None));
    }

    #[test]
    fn universe_cell_states_are_dead_on_creation() {
        let mut uni = generate_test_universe_with_default_params();
        let max_width = uni.width()-1;
        let max_height = uni.height()-1;
        
        for x in 0..max_width {
            for y in 0..max_height {
                let cell_state = uni.get_cell_state(x,y, None);
                assert_eq!(cell_state, CellState::Dead);
            }
        }
    }

    #[test]
    fn set_checked_verify_players_remain_within_writable_regions() {
        let mut uni = generate_test_universe_with_default_params();
        let max_width = uni.width()-1;
        let max_height = uni.height()-1;
        let player_id = 1; // writing into player 1's regions
        let alive_player_cell = CellState::Alive(Some(player_id));
        let mut cell_state;

        // Writable region OK, Transitions to Alive
        uni.set(0, 0, alive_player_cell, player_id);
        cell_state = uni.get_cell_state(0,0, Some(player_id));
        assert_eq!(cell_state, alive_player_cell);

        // This should be dead as it is outside the writable region
        uni.set(max_width, max_height, alive_player_cell, player_id);
        cell_state = uni.get_cell_state(max_width, max_height, Some(player_id));
        assert_eq!(cell_state, CellState::Dead);

        // Writable region OK, transitions to Alive
        uni.set(55, 55, alive_player_cell, player_id);
        cell_state = uni.get_cell_state(55, 55, Some(player_id));
        assert_eq!(cell_state, alive_player_cell);

        // Outside of player_id's writable region which will remain unchanged
        uni.set(81, 81, alive_player_cell, player_id);
        cell_state = uni.get_cell_state(81, 81, Some(player_id));
        assert_eq!(cell_state, CellState::Dead);
    }

    #[test]
    fn set_checked_cannot_set_a_fog_cell() {
        let mut uni = generate_test_universe_with_default_params();
        let player_id = 1; // writing into player 1's regions
        let alive_player_cell = CellState::Alive(Some(player_id));
        let state_index = uni.state_index;

        // Let's hardcode this and try to set a fog'd cell
        // within what was a players writable region.
        uni.gen_states[state_index].player_states[player_id].fog[0][0] |= 1<<63;

        uni.set(0, 0, alive_player_cell, player_id);
        let cell_state = uni.get_cell_state(0,0, Some(player_id));
        assert_eq!(cell_state, CellState::Dead);
    }


    #[test]
    fn toggle_unchecked_cell_toggled_is_owned_by_player() {
        let mut uni = generate_test_universe_with_default_params();
        let state_index = uni.state_index;
        let row = 0;
        let col = 0;
        let bit = 63;
        let player_one_opt = Some(0);
        let player_two_opt = Some(1);

        // Should transition from dead to alive. Player one will have their cell set, player two
        // will not
        assert_eq!(uni.toggle_unchecked(row, col, player_one_opt), CellState::Alive(player_one_opt));
        assert_eq!(uni.gen_states[state_index].player_states[player_one_opt.unwrap()].cells[row][col] >> bit, 1);
        assert_eq!(uni.gen_states[state_index].player_states[player_two_opt.unwrap()].cells[row][col] >> bit, 0);
    }

    #[test]
    fn toggle_unchecked_cell_toggled_by_both_players_repetatively() {
        let mut uni = generate_test_universe_with_default_params();
        let state_index = uni.state_index;
        let row = 0;
        let col = 0;
        let bit = 63;
        let player_one_opt = Some(0);
        let player_two_opt = Some(1);

        // Should transition from dead to alive. Player one will have their cell set, player two
        // will not
        assert_eq!(uni.toggle_unchecked(row, col, player_one_opt), CellState::Alive(player_one_opt));
        assert_eq!(uni.gen_states[state_index].player_states[player_one_opt.unwrap()].cells[row][col] >> bit, 1);
        assert_eq!(uni.gen_states[state_index].player_states[player_two_opt.unwrap()].cells[row][col] >> bit, 0);

        // Player two will now toggle the cell, killing it as it was previously alive.
        // Player one will be cleared as a result, the cell will not be set at all.
        // Notice we are not checking for writable regions here (unchecked doesn't care) so this
        // runs through
        assert_eq!(uni.toggle_unchecked(row, col, player_two_opt), CellState::Dead);
        assert_eq!(uni.gen_states[state_index].player_states[player_one_opt.unwrap()].cells[row][col] >> bit, 0);
        assert_eq!(uni.gen_states[state_index].player_states[player_two_opt.unwrap()].cells[row][col] >> bit, 0);
    }

    #[test]
    fn toggle_checked_outside_a_player_writable_region_fails() {
        let mut uni = generate_test_universe_with_default_params();
        let player_one = 0;
        let player_two = 1;
        let row = 0;
        let col = 0;

        assert_eq!(uni.toggle(row, col, player_one), Err(()));
        assert_eq!(uni.toggle(row, col, player_two).unwrap(), CellState::Alive(Some(player_two)));
    }

    #[test]
    fn toggle_checked_players_cannot_toggle_a_wall_cell() {
        let mut uni = generate_test_universe_with_default_params();
        let player_one = 0;
        let player_two = 1;
        let row = 0;
        let col = 0;
        let state_index = uni.state_index;

        modify_cell_bits(&mut uni.gen_states[state_index].wall_cells, row, col, 1<<63, BitOperation::Set);

        assert_eq!(uni.toggle(row, col, player_one), Err(()));
        assert_eq!(uni.toggle(row, col, player_two), Err(()));
    }

    #[test]
    fn toggle_checked_players_can_toggle_an_known_cell_if_writable() {
        let mut uni = generate_test_universe_with_default_params();
        let player_one = 0;
        let player_two = 1;
        let row = 0;
        let col = 0;
        let state_index = uni.state_index;

        modify_cell_bits(&mut uni.gen_states[state_index].known, row, col, 1<<63, BitOperation::Set);

        assert_eq!(uni.toggle(row, col, player_one), Err(()));
        assert_eq!(uni.toggle(row, col, player_two), Ok(CellState::Alive(Some(player_two))));
    }

    #[test]
    fn toggle_checked_players_cannot_toggle_an_unknown_cell() {
        let mut uni = generate_test_universe_with_default_params();
        let player_one = 0;
        let player_two = 1;
        let row = 0;
        let col = 0;
        let state_index = uni.state_index;

        modify_cell_bits(&mut uni.gen_states[state_index].known, row, col, 1<<63, BitOperation::Clear);

        assert_eq!(uni.toggle(row, col, player_one), Err(()));
        assert_eq!(uni.toggle(row, col, player_two), Err(()));
    }

    #[test]
    fn contagious_one_with_all_neighbors_set() {
        let north = u64::max_value();
        let northwest = u64::max_value();
        let northeast = u64::max_value();
        let west = u64::max_value();
        let mut center = u64::max_value();
        let east = u64::max_value();
        let southwest = u64::max_value();
        let south = u64::max_value();
        let southeast = u64::max_value();


        let mut output = Universe::contagious_one(northwest, north, northeast, west, center, east, southwest, south, southeast);
        assert_eq!(output, u64::max_value());

        center &= !(0x0000000F00000000);

        output = Universe::contagious_one(northwest, north, northeast, west, center, east, southwest, south, southeast);
        // 1 bit surrounding 'F', and inclusive, are cleared
        assert_eq!(output, 0xFFFFFFFFFFFFFFFF);
    }

    #[test]
    fn contagious_zero_with_all_neighbors_set() {
        let north = u64::max_value();
        let northwest = u64::max_value();
        let northeast = u64::max_value();
        let west = u64::max_value();
        let mut center = u64::max_value();
        let east = u64::max_value();
        let southwest = u64::max_value();
        let south = u64::max_value();
        let southeast = u64::max_value();


        let mut output = Universe::contagious_zero(northwest, north, northeast, west, center, east, southwest, south, southeast);
        assert_eq!(output, u64::max_value());

        center &= !(0x0000000F00000000);

        output = Universe::contagious_zero(northwest, north, northeast, west, center, east, southwest, south, southeast);
        // 1 bit surrounding 'F', and inclusive, are cleared
        assert_eq!(output, 0xFFFFFFE07FFFFFFF);
    }
}

#[cfg(test)]
mod region_tests {
    use super::*;

    #[test]
    fn region_with_valid_dims() {
        let region = Region::new(1, 10, 100, 200);

        assert_eq!(region.left(), 1);
        assert_eq!(region.top(), 10);
        assert_eq!(region.height(), 200);
        assert_eq!(region.width(), 100);
        assert_eq!(region.right(), 100);
        assert_eq!(region.bottom(), 209);
    }
    
    #[test]
    fn region_with_valid_dims_negative_top_and_left() {
        let region = Region::new(-1, -10, 100, 200);

        assert_eq!(region.left(), -1);
        assert_eq!(region.top(), -10);
        assert_eq!(region.height(), 200);
        assert_eq!(region.width(), 100);
        assert_eq!(region.right(), 98);
        assert_eq!(region.bottom(), 189);
    }

    #[test]
    #[should_panic]
    fn region_with_bad_dims_panics() {
        Region::new(0, 0, 0, 0);
    }

    #[test]
    fn region_contains_a_valid_sub_region() {
        let region1 = Region::new(1, 10, 100, 200);
        let region2 = Region::new(-100, -200, 100, 200);

        assert!(region1.contains(50, 50));
        assert!(region2.contains(-50, -50));
    }
    
    #[test]
    fn region_does_not_contain_sub_region() {
        let region1 = Region::new(1, 10, 100, 200);
        let region2 = Region::new(-100, -200, 100, 200);

        assert!(!region1.contains(-50, -50));
        assert!(!region2.contains(50, 50));
    }
}

#[cfg(test)]
mod cellstate_tests {
    use super::*;

    #[test]
    fn cell_states_as_char() {
        let dead = CellState::Dead;
        let alive = CellState::Alive(None);
        let player1 = CellState::Alive(Some(1));
        let player2 = CellState::Alive(Some(2));
        let wall = CellState::Wall;
        let fog = CellState::Fog;

        assert_eq!(dead.to_char(), 'b');
        assert_eq!(alive.to_char(), 'o');
        assert_eq!(player1.to_char(), 'B');
        assert_eq!(player2.to_char(), 'C');
        assert_eq!(wall.to_char(), 'W');
        assert_eq!(fog.to_char(), '?');
    }
}

#[cfg(test)]
mod bitgrid_tests {
    use super::*;

    #[test]
    fn create_valid_empty_bitgrid() {
        let height = 11;
        let width_in_words = 10;
        let grid = new_bitgrid(width_in_words, height);

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
        let _ = new_bitgrid(width_in_words, height);
    }

    #[test]
    fn set_cell_bits_within_a_bitgrid() {
        let height = 10;
        let width_in_words = 10;
        let mut grid = new_bitgrid(width_in_words, height);

        for x in 0..height {
            for y in 0..width_in_words {
                assert_eq!(grid[x][y], 0);
            }
        }

        modify_cell_bits(&mut grid, height/2, width_in_words/2, 1<<63, BitOperation::Set);
        assert_eq!(grid[height/2][width_in_words/2] >> 63, 1);
        
        modify_cell_bits(&mut grid, height-1, width_in_words-1, 1<<63, BitOperation::Set);
        assert_eq!(grid[height-1][width_in_words-1] >> 63, 1);
    }

    #[test]
    fn clear_cell_bits_within_a_bitgrid() {
        let height = 10;
        let width_in_words = 10;
        let mut grid = new_bitgrid(width_in_words, height);

        for x in 0..height {
            for y in 0..width_in_words {
                grid[x][y] = u64::max_value();
            }
        }

        modify_cell_bits(&mut grid, height/2, width_in_words/2, 1<<63, BitOperation::Clear);
        assert_eq!(grid[height/2][width_in_words/2] >> 63, 0);
        
        modify_cell_bits(&mut grid, height-1, width_in_words-1, 1<<63, BitOperation::Clear);
        assert_eq!(grid[height-1][width_in_words-1] >> 63, 0);
    }

    #[test]
    fn toggle_cell_bits_within_a_bitgrid() {
        let height = 10;
        let width_in_words = 10;
        let mut grid = new_bitgrid(width_in_words, height);

        for x in 0..height {
            for y in 0..width_in_words {
                grid[x][y] = u64::max_value();
            }
        }

        modify_cell_bits(&mut grid, height/2, width_in_words/2, 1<<63, BitOperation::Toggle);
        assert_eq!(grid[height/2][width_in_words/2] >> 63, 0);
        
        modify_cell_bits(&mut grid, height/2, width_in_words/2, 1<<63, BitOperation::Toggle);
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

        let mut grid = new_bitgrid(width_in_words, height);
        let region1 = Region::new(0, 0, region1_w, region1_h);
        let region2 = Region::new(0, 0, region2_w, region2_h);
        let region3 = Region::new(region2_w as isize, region2_h as isize, region3_w, region3_h);

        fill_region(&mut grid, region1, BitOperation::Set);

        for y in 0..region1_w {
            assert_eq!(grid[y][0], 0xFE00000000000000);
        }

        fill_region(&mut grid, region2, BitOperation::Clear);
        for y in 0..region2_w {
            assert_eq!(grid[y][0], 0x1E00000000000000);
        }

        fill_region(&mut grid, region3, BitOperation::Toggle);
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

        let mut grid = new_bitgrid(width_in_words, height);
        let region_neg = Region::new(-1, -1, 1, 1);
        fill_region(&mut grid, region_neg, BitOperation::Set);
    }

}
