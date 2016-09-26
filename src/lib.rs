/// Represents a wrapping universe in Conway's game of life.
pub struct Universe {
    width:        usize,           // width in u64 elements, _not_ width in cells!
    height:       usize,
    generation_a: Option<usize>,
    generation_b: Option<usize>,
    buffer_a:     Vec<Vec<u64>>,
    buffer_b:     Vec<Vec<u64>>,
}


enum WhichBuffer { A, B }


impl Universe {
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
}


//XXX pub struct Region

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
}
