use std::fmt;

/// Represents a wrapping universe in Conway's game of life.
pub struct Universe {
    width:        usize,           // width in u64 elements, _not_ width in cells!
    height:       usize,
    generation_a: Option<usize>,
    generation_b: Option<usize>,
    buffer_a:     Vec<Vec<u64>>,
    buffer_b:     Vec<Vec<u64>>,
}


#[derive(Eq,PartialEq,Ord,PartialOrd)]
pub enum CellState {
    Dead,
    Alive,              // TODO: Alive(Option<u8>) (player number)
    Wall,
    Fog,
}


#[derive(Eq,PartialEq)]
enum WhichBuffer { A, B }

impl fmt::Display for Universe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let latest = self.latest();
        let buffer_cur = if latest == WhichBuffer::A { &self.buffer_a } else { &self.buffer_b };
        for row in buffer_cur.iter() {
            for &word in row.iter() {
                let mut s = String::with_capacity(64);
                for shift in (0..64).rev() {
                    if (word>>shift)&1 == 1 {
                        s.push('*');
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
    // sets the state of a cell
    // TODO: unit tests
    // TODO: when multiple bitmaps are supported, adjust this
    pub fn set(&mut self, col: usize, row: usize, cell: CellState) {
        //XXX bounds checks
        let latest = self.latest();
        let buffer_cur = if latest == WhichBuffer::A { &mut self.buffer_a } else { &mut self.buffer_b };
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1));
        let mut word = buffer_cur[row][word_col];
        match cell {
            CellState::Dead => {
                word &= !(1 << shift);
            }
            CellState::Alive => {
                word |= 1 << shift;
            }
            _ => unimplemented!()
        }
        buffer_cur[row][word_col] = word;
    }


    // switches any non-dead state to CellState::Dead
    // switches CellState::Dead to CellState::Alive
    // TODO: unit tests
    pub fn toggle(&mut self, col: usize, row: usize) -> CellState {
        //XXX bounds checks
        let latest = self.latest();
        let buffer_cur = if latest == WhichBuffer::A { &mut self.buffer_a } else { &mut self.buffer_b };
        let word_col = col/64;
        let shift = 63 - (col & (64 - 1));
        let mut word = buffer_cur[row][word_col];
        word ^= 1 << shift;
        buffer_cur[row][word_col] = word;
        if (word >> shift) & 1 == 1 {
            // TODO: when multiple bitmaps are supported, adjust the XOR and the return value computation
            CellState::Alive
        } else {
            CellState::Dead
        }
    }


    /// Instantiate a new blank universe with the given width and height, in cells.
    /// The universe is at generation 1.
    pub fn new(width: usize, height: usize) -> Result<Universe, &'static str> {
        if height == 0 {
            return Err("Height must be positive");
        }

        if width != width/64*64 {
            return Err("Width must be a multiple of 64");
        } else if width == 0 {
            return Err("Width must be positive");
        }

        let mut buffer_a: Vec<Vec<u64>> = Vec::new();
        for _ in 0 .. height {
            let row: Vec<u64> = vec![0; width/64];
            buffer_a.push(row);
        }

        let mut buffer_b: Vec<Vec<u64>> = Vec::new();
        for _ in 0 .. height {
            let row: Vec<u64> = vec![0; width/64];
            buffer_b.push(row);
        }

        Ok(Universe {
            width:  width/64,
            height: height,
            generation_a: Some(1),
            generation_b: None,
            buffer_a: buffer_a,
            buffer_b: buffer_b,
        })
    }


    /// Return width in cells.
    pub fn width(&self) -> usize {
        return self.width * 64;
    }


    /// Return height in cells.
    pub fn height(&self) -> usize {
        return self.height;
    }


    fn latest(&self) -> WhichBuffer {
        if let Some(gen_a) = self.generation_a {
            match self.generation_b {
                Some(gen_b) => {
                    if gen_a < gen_b {
                        assert!(gen_a + 1 == gen_b);
                        WhichBuffer::B
                    } else if gen_a > gen_b {
                        assert!(gen_a - 1 == gen_b);
                        WhichBuffer::A
                    } else {
                        panic!("The generations are equal")
                    }
                },
                None => WhichBuffer::A
            }
        } else {
            assert!(self.generation_b.is_some());
            WhichBuffer::B
        }
    }

    /// Get the latest generation number (1-based).
    pub fn latest_gen(&self) -> usize {
        match self.latest() {
            WhichBuffer::A => self.generation_a.unwrap(),
            WhichBuffer::B => self.generation_b.unwrap()
        }
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

    /// Compute the next generation. Returns the new latest generation number.
    pub fn next(&mut self) -> usize {
        let latest     = self.latest();
        let latest_gen = self.latest_gen();
        let buffer_cur;
        let buffer_next;
        if latest == WhichBuffer::A {
            buffer_cur  =     &self.buffer_a;
            buffer_next = &mut self.buffer_b;
        } else {
            buffer_cur  =     &self.buffer_b;
            buffer_next = &mut self.buffer_a;
        }
        for row_idx in 0 .. self.height {
            let row_n = &buffer_cur[(row_idx + self.height - 1) % self.height];
            let row_c = &buffer_cur[ row_idx ];
            let row_s = &buffer_cur[(row_idx + 1) % self.height];
            // These will be shifted over at the beginning of the loop
            let mut nw;
            let mut w;
            let mut sw;
            let mut n   = row_n[self.width - 1];
            let mut cen = row_c[self.width - 1];
            let mut s   = row_s[self.width - 1];
            let mut ne  = row_n[0];
            let mut e   = row_c[0];
            let mut se  = row_s[0];
            for col_idx in 0 .. self.width {
                // shift over
                nw  = n;
                n   = ne;
                w   = cen;
                cen = e;
                sw  = s;
                s   = se;
                ne  = row_n[(col_idx + 1) % self.width];
                e   = row_c[(col_idx + 1) % self.width];
                se  = row_s[(col_idx + 1) % self.width];
                let result = Universe::next_single_gen(nw, n, ne, w, cen, e, sw, s, se);

                // assign to the u64 element in the next generation
                buffer_next[row_idx][col_idx] = result;
            }
        }
        let new_latest_gen = latest_gen + 1;
        if latest == WhichBuffer::A {
            self.generation_b = Some(new_latest_gen);
        } else {
            self.generation_a = Some(new_latest_gen);
        }
        new_latest_gen
    }


    /// Iterate over every non-dead cell in the universe for the current generation.
    /// Callback receives (x, y, cell_state).
    //TODO: unit test
    //TODO: other CellStates
    pub fn each_non_dead(&self, callback: &mut FnMut(usize, usize, CellState), region: Region) {
        let latest = self.latest();
        let buffer_cur = if latest == WhichBuffer::A { &self.buffer_a } else { &self.buffer_b };
        let mut x;
        let mut y = 0;
        for row in buffer_cur.iter() {
            if (y as isize) >= region.top() && (y as isize) < (region.top() + region.height() as isize) {
                x = 0;
                for &word in row.iter() {
                    for shift in (0..64).rev() {
                        if (x as isize) >= region.left() &&
                            (x as isize) < (region.left() + region.width() as isize) {
                            if (word>>shift)&1 == 1 {
                                callback(x, y, CellState::Alive);
                            }
                        }
                        x += 1;
                    }
                }
            }
            y += 1;
        }
    }


    /// Iterate over every non-dead cell in the universe for the current generation.
    /// Callback receives (x, y, cell_state).
    //TODO: unit test
    pub fn each_non_dead_full(&self, callback: &mut FnMut(usize, usize, CellState)) {
        self.each_non_dead(callback, self.region());
    }


    /// Get a Region of the same size as the universe
    pub fn region(&self) -> Region {
        Region::new(0, 0, self.width*64, self.height)
    }
}


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

    pub fn top(&self) -> isize {
        self.top
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
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
