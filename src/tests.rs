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

mod universe_tests {
    use universe::*;
    use universe::test_helpers::*;
    use grids::CharGrid;
    use rle::Pattern;
    use error::ConwayError::*;


    #[test]
    fn new_universe_with_valid_dims() {
        let uni = generate_test_universe_with_default_params(UniType::Server);
        let universe_as_region = Region::new(0, 0, 256, 128);

        assert_eq!(uni.width(), 256);
        assert_eq!(uni.height(), 128);
        assert_eq!(uni.region(), universe_as_region);
    }

    #[test]
    fn new_universe_with_bad_dims() {

        let player0 = PlayerBuilder::new(Region::new(100, 70, 34, 16));   // used for the glider gun and predefined patterns
        let player1 = PlayerBuilder::new(Region::new(0, 0, 80, 80));
        let players = vec![player0, player1];

        let mut bigbang = BigBang::new()
            .width(256)
            .height(128)
            .server_mode(true)
            .history(16)
            .fog_radius(9)
            .add_players(players);

        bigbang = bigbang.width(255);

        let uni_result1 = bigbang.birth();
        assert!(uni_result1.is_err());

        bigbang = bigbang.width(256).height(0);
        let uni_result2 = bigbang.birth();
        assert!(uni_result2.is_err());

        bigbang = bigbang.width(0).height(256);
        let uni_result3 = bigbang.birth();
        assert!(uni_result3.is_err());
    }

    #[test]
    fn new_universe_first_gen_is_one() {
        let uni = generate_test_universe_with_default_params(UniType::Server);
        assert_eq!(uni.latest_gen(), 1);
    }

    #[test]
    fn next_test_data1() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);

        // r-pentomino
        let _ = uni.toggle(16, 15, 0);
        let _ = uni.toggle(17, 15, 0);
        let _ = uni.toggle(15, 16, 0);
        let _ = uni.toggle(16, 16, 0);
        let _ = uni.toggle(16, 17, 0);

        let gens = 20;
        for _ in 0..gens {
            uni.next();
        }
        assert_eq!(uni.latest_gen(), gens + 1);
    }

    #[test]
    fn set_unchecked_with_valid_rows_and_cols() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
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
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
        uni.set_unchecked(257, 129, CellState::Alive(None));
    }

    #[test]
    fn universe_cell_states_are_dead_on_creation() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
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
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
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
    fn toggle_checked_outside_a_player_writable_region_fails() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
        let player_one = 0;
        let player_two = 1;
        let row = 0;
        let col = 0;

        assert_eq!(uni.toggle(row, col, player_one),
                   Err(AccessDenied{reason: "outside writable area: col=0, row=0".to_owned()}));
        assert_eq!(uni.toggle(row, col, player_two).unwrap(), CellState::Alive(Some(player_two)));
    }

    #[test]
    fn generate_fog_circle_bitmap_fails_with_radius_zero() {
        let player0 = PlayerBuilder::new(Region::new(100, 70, 34, 16));   // used for the glider gun and predefined patterns
        let player1 = PlayerBuilder::new(Region::new(0, 0, 80, 80));
        let players = vec![player0, player1];

        let uni = BigBang::new()
            .width(256)
            .height(128)
            .server_mode(true)
            .history(16)
            .fog_radius(0)
            .add_players(players)
            .birth();

        assert!(uni.is_err());
    }

    #[test]
    fn each_non_dead_detects_some_cells() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
        let player1 = 1;

        // glider
        uni.toggle(16, 15, player1).unwrap();
        uni.toggle(17, 16, player1).unwrap();
        uni.toggle(15, 17, player1).unwrap();
        uni.toggle(16, 17, player1).unwrap();
        uni.toggle(17, 17, player1).unwrap();

        // just a wall, for no reason at all
        for col in 10..15 {
            uni.set_unchecked(col, 12, CellState::Wall);
        }

        let gens = 21;
        for _ in 0..gens {
            uni.next();
        }
        let mut cells_with_pos: Vec<(usize, usize, CellState)> = vec![];
        uni.each_non_dead(Region::new(0, 0, 80, 80), Some(player1), &mut |col, row, state| {
            cells_with_pos.push((col, row, state));
        });
        assert_eq!(cells_with_pos.len(), 10);
        assert_eq!(cells_with_pos, vec![(10, 12, CellState::Wall),
                                        (11, 12, CellState::Wall),
                                        (12, 12, CellState::Wall),
                                        (13, 12, CellState::Wall),
                                        (14, 12, CellState::Wall),
                                        (20, 21, CellState::Alive(Some(1))),
                                        (22, 21, CellState::Alive(Some(1))),
                                        (21, 22, CellState::Alive(Some(1))),
                                        (22, 22, CellState::Alive(Some(1))),
                                        (21, 23, CellState::Alive(Some(1)))]);

    }

    #[test]
    fn each_non_dead_detects_fog() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
        let player0 = 0;
        let player1 = 1;

        // blinker as player 1
        uni.toggle(16, 15, player1).unwrap();
        uni.toggle(16, 16, player1).unwrap();
        uni.toggle(16, 17, player1).unwrap();

        // attempt to view as different player
        uni.each_non_dead(Region::new(0, 0, 80, 80), Some(player0), &mut |col, row, state| {
            assert_eq!(state, CellState::Fog, "expected fog at col {} row {} but found {:?}", col, row, state);
        });
    }

    #[test]
    fn universe_apply_but_already_applied() {
        let mut s_uni = generate_test_universe_with_default_params(UniType::Server);  // server
        let mut c_uni = generate_test_universe_with_default_params(UniType::Client); // client is missing generation 1
        let player1 = 1;
        // glider
        s_uni.toggle(16, 15, player1).unwrap();
        s_uni.toggle(17, 16, player1).unwrap();
        s_uni.toggle(15, 17, player1).unwrap();
        s_uni.toggle(16, 17, player1).unwrap();
        s_uni.toggle(17, 17, player1).unwrap();
        let gens = 4;
        for _ in 0..gens {
            s_uni.next();
        }
        let diff = s_uni.diff(0, 5, None).unwrap();
        c_uni.apply(&diff, None).unwrap();
        assert_eq!(c_uni.apply(&diff, None).unwrap(), None); // applying a second time does nothing
        // earlier gen
        let diff2 = s_uni.diff(0, 4, None).unwrap();
        assert_eq!(c_uni.apply(&diff2, None).unwrap(), None); // applying does noting if our gen is later than diff's gen
    }

    #[test]
    fn universe_apply_but_base_gen_absent() {
        let mut s_uni = generate_test_universe_with_default_params(UniType::Server);  // server
        let mut c_uni = generate_test_universe_with_default_params(UniType::Client); // client is missing generation 1
        let player1 = 1;
        // glider
        s_uni.toggle(16, 15, player1).unwrap();
        s_uni.toggle(17, 16, player1).unwrap();
        s_uni.toggle(15, 17, player1).unwrap();
        s_uni.toggle(16, 17, player1).unwrap();
        s_uni.toggle(17, 17, player1).unwrap();
        let gens = 4;
        for _ in 0..gens {
            s_uni.next();
        }
        let diff = s_uni.diff(1, 5, None).unwrap();
        assert_eq!(c_uni.apply(&diff, None).unwrap(), None); // we don't have base gen (1), so can't apply it
    }

    #[test]
    fn universe_apply_too_large_range() {
        let base = 3;
        let diff = GenStateDiff{gen0: base, gen1: base + GEN_BUFSIZE + 1, pattern: Pattern("!".to_owned())};
        let mut c_uni = generate_test_universe_with_default_params(UniType::Client);
        assert_eq!(c_uni.apply(&diff, None), Err(InvalidData{reason: "diff is across too many generations to be applied: 17 >= 16".to_owned()}));
    }

    #[test]
    fn universe_diff_crazy_numbers_is_none() {
        let uni = generate_test_universe_with_default_params(UniType::Server);
        assert!(uni.diff(123, 456, None).is_none());
    }

    #[test]
    #[should_panic]
    fn universe_diff_crazier_numbers_panics() {
        let uni = generate_test_universe_with_default_params(UniType::Server);
        assert!(uni.diff(456, 456, None).is_none());
    }

    #[test]
    fn universe_diff_good_numbers_is_valid() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
        let player1 = 1;
        // glider
        uni.toggle(16, 15, player1).unwrap();
        uni.toggle(17, 16, player1).unwrap();
        uni.toggle(15, 17, player1).unwrap();
        uni.toggle(16, 17, player1).unwrap();
        uni.toggle(17, 17, player1).unwrap();
        let gens = 4;
        for _ in 0..gens {
            uni.next();
        }
        let diff = uni.diff(2, 3, None).unwrap();
        assert_eq!(diff.gen0, 2);
        assert_eq!(diff.gen1, 3);
        let pat_str = diff.pattern.0.as_str();
        let s = pat_str.split("256\"$")
                       .filter(|&s| {
                           s != "\r\n" && s != ""
                       })
                       .next()
                       .unwrap();
        assert_eq!(s, "15\"b240\"$15\"Bb239\"$17\"B238\"$");
    }

    #[test]
    fn universe_diff_zero_base_gen() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
        let player1 = 1;
        // glider
        uni.toggle(16, 15, player1).unwrap();
        uni.toggle(17, 16, player1).unwrap();
        uni.toggle(15, 17, player1).unwrap();
        uni.toggle(16, 17, player1).unwrap();
        uni.toggle(17, 17, player1).unwrap();
        let gens = 4;
        for _ in 0..gens {
            uni.next();
        }
        let diff = uni.diff(0, 4, None).unwrap();
        assert_eq!(diff.gen0, 0);
        assert_eq!(diff.gen1, 4);
    }

    #[test]
    fn universe_diff_some_player1_sees_cells() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
        let player1 = 1;
        // glider
        uni.toggle(16, 15, player1).unwrap();
        uni.toggle(17, 16, player1).unwrap();
        uni.toggle(15, 17, player1).unwrap();
        uni.toggle(16, 17, player1).unwrap();
        uni.toggle(17, 17, player1).unwrap();
        let gens = 4;
        for _ in 0..gens {
            uni.next();
        }
        let diff = uni.diff(0, 4, Some(player1)).unwrap();
        assert!(diff.pattern.0.find('B').is_some());  // should find cells from player 1
    }

    #[test]
    fn universe_diff_some_other_player_does_not_see_cells() {
        let mut uni = generate_test_universe_with_default_params(UniType::Server);
        let player1 = 1;
        let other_player = 0;
        // glider
        uni.toggle(16, 15, player1).unwrap();
        uni.toggle(17, 16, player1).unwrap();
        uni.toggle(15, 17, player1).unwrap();
        uni.toggle(16, 17, player1).unwrap();
        uni.toggle(17, 17, player1).unwrap();
        let gens = 4;
        for _ in 0..gens {
            uni.next();
        }
        let diff = uni.diff(0, 4, Some(other_player)).unwrap();
        assert!(diff.pattern.0.find('B').is_none());  // should not find cells from player 1
    }
}


mod genstate_tests {
    use universe::test_helpers::*;
    use grids::CharGrid;
    use rle::Pattern;

    #[test]
    fn gen_state_get_run_simple() {
        let mut genstate = make_gen_state();

        Pattern("o!".to_owned()).to_grid(&mut genstate, None).unwrap();
        assert_eq!(genstate.get_run(0, 0, None), (1, 'o'));
    }

    #[test]
    fn gen_state_get_run_wall() {
        let mut genstate = make_gen_state();

        Pattern("4W!".to_owned()).to_grid(&mut genstate, None).unwrap();
        assert_eq!(genstate.get_run(0, 0, None), (4, 'W'));
    }

    #[test]
    fn gen_state_get_run_wall_blank_in_front() {
        let mut genstate = make_gen_state();

        Pattern("12b4W!".to_owned()).to_grid(&mut genstate, None).unwrap();
        assert_eq!(genstate.get_run(12, 0, None), (4, 'W'));
    }

    #[test]
    fn gen_state_get_run_alternating_owned_unowned() {
        let mut genstate = make_gen_state();

        Pattern("15o3A2B9o!".to_owned()).to_grid(&mut genstate, None).unwrap();
        assert_eq!(genstate.get_run(0,      0, None), (15, 'o'));
        assert_eq!(genstate.get_run(15,     0, None), (3,  'A'));
        assert_eq!(genstate.get_run(15+3,   0, None), (2,  'B'));
        assert_eq!(genstate.get_run(15+3+2, 0, None), (9,  'o'));
    }

    #[test]
    fn gen_state_get_run_player1_no_fog() {
        let mut genstate = make_gen_state();

        let write_pattern_as = Some(1);   // avoid clearing fog for players other than player 1
        Pattern("o!".to_owned()).to_grid(&mut genstate, write_pattern_as).unwrap();
        let visibility = Some(1); // as player 1
        assert_eq!(genstate.get_run(0, 0, visibility), (1, 'o'));
    }

    #[test]
    fn gen_state_diff_simple1() {
        let gs0 = make_gen_state();
        let mut gs1 = make_gen_state();
        Pattern("o!".to_owned()).to_grid(&mut gs1, None).unwrap();

        let gsdiff = gs0.diff(&gs1, None);
        assert_eq!(gsdiff.pattern.0.len(), 659);
        let mut gsdiff_pattern_iter = gsdiff.pattern.0.split('$');
        assert_eq!(gsdiff_pattern_iter.next().unwrap(), "o255\"");
        assert_eq!(gsdiff_pattern_iter.next().unwrap(), "256\"");
        assert_eq!(gsdiff_pattern_iter.next().unwrap(), "256\"");
        // if you keep doing this, you'll eventually get a string containing \r\n
    }

    #[test]
    fn gen_state_diff_and_restore_simple1() {
        let gs0 = make_gen_state();
        let mut gs1 = make_gen_state();
        let visibility = None;
        Pattern("o!".to_owned()).to_grid(&mut gs1, visibility).unwrap();

        let gsdiff = gs0.diff(&gs1, visibility);

        let mut new_gs = make_gen_state();

        gsdiff.pattern.to_grid(&mut new_gs, visibility).unwrap();
        assert_eq!(new_gs, gs1);
    }
}


mod region_tests {
    use universe::*;

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

    #[test]
    fn region_no_intersection() {
        let region1 = Region::new(1, 10, 100, 200);
        let region2 = Region::new(-100, -200, 100, 200);
        assert_eq!(region1.intersection(region2), None);
        assert_eq!(region2.intersection(region1), None);
    }

    #[test]
    fn region_intersection_with_self() {
        let region1 = Region::new(1, 10, 100, 200);
        assert_eq!(region1.intersection(region1), Some(region1));
    }

    #[test]
    fn region_intersection_overlap() {
        let region1 = Region::new( 1,  10, 100, 200);
        let region2 = Region::new(90, 120, 100, 200);
        assert_eq!(region1.intersection(region2), Some(Region::new(90, 120, 11, 90)));
    }

    #[test]
    fn region_no_intersection_overlap_one_dim() {
        let region1 = Region::new(0, 0, 2, 2);
        let region2 = Region::new(3, 0, 2, 2);
        assert_eq!(region1.intersection(region2), None);
    }
}

mod cellstate_tests {
    use universe::*;

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

mod grid_tests {
    use universe::Region;
    use rle::Pattern;
    use grids::*;

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
    fn bounding_box_of_empty_bit_grid_is_none() {
        let grid = BitGrid::new(1, 1);
        assert_eq!(grid.bounding_box(), None);
    }

    #[test]
    fn bounding_box_of_empty_bit_grid_is_none2() {
        let grid = BitGrid::new(2, 5);
        assert_eq!(grid.bounding_box(), None);
    }

    #[test]
    fn bounding_box_for_single_off() {
        let grid = Pattern("b!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.bounding_box(), None);
    }

    #[test]
    fn bounding_box_for_single_on() {
        let grid = Pattern("o!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(0, 0, 1, 1)));
    }

    #[test]
    fn bounding_box_for_single_line_complicated() {
        let grid = Pattern("15b87o!".to_owned()).to_new_bit_grid(15+87, 1).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(15, 0, 87, 1)));
    }

    #[test]
    fn bounding_box_for_single_line_complicated2() {
        let grid = Pattern("82b87o!".to_owned()).to_new_bit_grid(82+87, 1).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(82, 0, 87, 1)));
    }

    #[test]
    fn bounding_box_for_multi_line_complicated() {
        let grid = Pattern("4$15b87o!".to_owned()).to_new_bit_grid(15+87, 5).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(15, 4, 87, 1)));
    }

    #[test]
    fn bounding_box_for_multi_line_complicated2() {
        let grid = Pattern("4$15b87o$10b100o$25b3o!".to_owned()).to_new_bit_grid(110, 7).unwrap();
        assert_eq!(grid.bounding_box(), Some(Region::new(10, 4, 100, 3)));
    }


    #[test]
    fn copy_simple() {
        let grid = Pattern("64o$64o!".to_owned()).to_new_bit_grid(64, 2).unwrap();
        let mut grid2 = BitGrid::new(1, 2); // 64x2
        let dst_region = Region::new(0, 0, 32, 3);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "32o$32o!".to_owned());
    }

    #[test]
    fn copy_out_of_bounds() {
        let grid = Pattern("64o$64o!".to_owned()).to_new_bit_grid(64, 2).unwrap();
        let mut grid2 = BitGrid::new(1, 2); // 64x2
        let dst_region = Region::new(17, 3, 32, 5);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "!".to_owned());
    }

    #[test]
    fn copy_simple2() {
        let grid = Pattern("64o$64o!".to_owned()).to_new_bit_grid(64, 2).unwrap();
        let mut grid2 = BitGrid::new(1, 2); // 64x2
        let dst_region = Region::new(16, 1, 32, 3);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "$16b32o!".to_owned());
    }

    #[test]
    fn copy_for_multi_line_complicated() {
        let grid = Pattern("4$b87o$3b100o$3o!".to_owned()).to_new_bit_grid(110, 7).unwrap();
        let mut grid2 = BitGrid::new(2, 10); // 128x10
        let dst_region = Region::new(2, 3, 64, 5);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "7$3b63o!".to_owned());
    }

    #[test]
    fn copy_for_multi_line_complicated2() {
        let grid = Pattern("4$b87o$3b100o$3o!".to_owned()).to_new_bit_grid(110, 7).unwrap();
        let mut grid2 = BitGrid::new(2, 10); // 128x10
        let dst_region = Region::new(2, 3, 65, 5);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "7$3b64o!".to_owned());
    }

    #[test]
    fn copy_for_multi_line_complicated3() {
        let grid = Pattern("4$b87o$3b100o$3o!".to_owned()).to_new_bit_grid(110, 7).unwrap();
        let mut grid2 = BitGrid::new(1, 10); // 64x10
        let dst_region = Region::new(2, 3, 5, 5);
        BitGrid::copy(&grid, &mut grid2, dst_region);
        assert_eq!(grid2.to_pattern(None).0, "7$3b4o!".to_owned());
    }


    #[test]
    #[should_panic]
    fn modify_region_with_a_negative_region_panics() {
        let height = 10;
        let width_in_words = 10;

        let mut grid = BitGrid::new(width_in_words, height);
        let region_neg = Region::new(-1, -1, 1, 1);
        grid.modify_region(region_neg, BitOperation::Set);
    }

    #[test]
    fn get_run_for_single_on() {
        let grid = Pattern("o!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.get_run(0, 0, None), (1, 'o'));
    }

    #[test]
    fn get_run_for_single_off_then_on() {
        let grid = Pattern("bo!".to_owned()).to_new_bit_grid(2, 1).unwrap();
        assert_eq!(grid.get_run(0, 0, None), (1, 'b'));
    }

    #[test]
    fn get_run_for_single_off() {
        let grid = Pattern("b!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.get_run(0, 0, None), (64, 'b'));
    }

    #[test]
    fn get_run_nonzero_col_for_single_off() {
        let grid = Pattern("b!".to_owned()).to_new_bit_grid(1, 1).unwrap();
        assert_eq!(grid.get_run(23, 0, None), (64 - 23, 'b'));
    }

    #[test]
    fn get_run_nonzero_col_complicated() {
        let grid = Pattern("15b87o!".to_owned()).to_new_bit_grid(15+87, 1).unwrap();
        assert_eq!(grid.get_run(15, 0, None), (87, 'o'));
    }

    #[test]
    fn get_run_nonzero_col_multiline_complicated() {
        let grid = Pattern("3$15b87o!".to_owned()).to_new_bit_grid(15+87, 4).unwrap();
        assert_eq!(grid.get_run(15, 3, None), (87, 'o'));
    }



    #[test]
    fn to_pattern_simple() {
        let original_pattern = Pattern("bo!".to_owned());
        let grid = original_pattern.to_new_bit_grid(2, 1).unwrap();
        let pattern = grid.to_pattern(None);
        assert_eq!(pattern, original_pattern);
    }

    #[test]
    fn to_pattern_nonzero_col_3empty_then_complicated() {
        let original_pattern = Pattern("3$15b87o!".to_owned());
        let grid = original_pattern.to_new_bit_grid(15+87, 4).unwrap();
        let pattern = grid.to_pattern(None);
        assert_eq!(pattern, original_pattern);
    }

    #[test]
    fn to_pattern_nonzero_col_veryverycomplicated() {
        let original_pattern = Pattern("b2o23b2o21b$b2o23bo22b$24bobo22b$15b2o7b2o23b$2o13bobo31b$2o13bob2o30b$16b2o31b$16bo32b$44b2o3b$16bo27b2o3b$16b2o31b$2o13bob2o13bo3bo12b$2o13bobo13bo5bo7b2o2b$15b2o14bo13b2o2b$31b2o3bo12b$b2o30b3o13b$b2o46b$33b3o13b$31b2o3bo12b$31bo13b2o2b$31bo5bo7b2o2b$32bo3bo12b2$44b2o3b$44b2o3b5$37b2o10b$37bobo7b2o$39bo7b2o$37b3o9b$22bobo24b$21b3o25b$21b3o25b$21bo15b3o9b$25bobo11bo9b$21b2o4bo9bobo9b$16b2o4bo3b2o9b2o10b$15bobo6bo24b$15bo33b$14b2o!".to_owned());
        let grid = original_pattern.to_new_bit_grid(49, 43).unwrap();
        let pattern = grid.to_pattern(None);
        assert_eq!(pattern.0.as_str(), "b2o23b2o$b2o23bo$24bobo$15b2o7b2o$2o13bobo$2o13bob2o$16b2o$16bo$44b2o$\r\n16bo27b2o$16b2o$2o13bob2o13bo3bo$2o13bobo13bo5bo7b2o$15b2o14bo13b2o$\r\n31b2o3bo$b2o30b3o$b2o$33b3o$31b2o3bo$31bo13b2o$31bo5bo7b2o$32bo3bo2$\r\n44b2o$44b2o5$37b2o$37bobo7b2o$39bo7b2o$37b3o$22bobo$21b3o$21b3o$21bo\r\n15b3o$25bobo11bo$21b2o4bo9bobo$16b2o4bo3b2o9b2o$15bobo6bo$15bo$14b2o!");
        for line in pattern.0.as_str().lines() {
            assert!(line.len() <= 70);
        }
    }

    #[test]
    fn get_run_empty() {
        let grid = BitGrid::new(1,1);
        let pattern = grid.to_pattern(None);
        assert_eq!(pattern.0.as_str(), "!");
    }

    #[test]
    fn clear_all_ones_really_is_clear() {
        let pat = Pattern("64o$64o$64o$64o$64o$64o$64o$64o$64o$64o$64o$64o!".to_owned());
        let mut grid = pat.to_new_bit_grid(64, 12).unwrap();
        grid.clear();
        for row in &grid.0 {
            for col_idx in 0..row.len() {
                assert_eq!(row[col_idx], 0);
            }
        }
    }
}


mod rle_tests {
    use rle::*;
    use grids::BitGrid;
    use std::str::FromStr;

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
