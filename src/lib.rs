pub struct Universe {
    generation_a: Option<usize>,
    generation_b: Option<usize>,
    buffer_a: Vec<Vec<u64>>,
    buffer_b: Vec<Vec<u64>>,
}


enum WhichBuffer { A, B }


impl Universe {
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

    pub fn latest_gen(&self) -> usize {
        match self.latest() {
            WhichBuffer::A => self.generation_a.unwrap(),
            WhichBuffer::B => self.generation_b.unwrap()
        }
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
}
