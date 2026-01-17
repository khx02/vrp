use std::collections::VecDeque;

use crate::domain::types::Route;
use crate::utils::swaps_overlap;

pub fn choose_best_candidate(
    swap_candidates_ind: &[(f64, (usize, usize))],
    tabu_list: &VecDeque<(usize, usize)>,
    best_so_far: &Route,
    aspiration_threshold: f64,
    parent_swap: &(usize, usize),
) -> (f64, (usize, usize)) {
    let mut chosen_solution = if !swap_candidates_ind.is_empty() {
        swap_candidates_ind[0]
    } else {
        (0.0, (0, 0))
    };

    let cand_pair = {
        let (i, j) = chosen_solution.1;
        if i < j {
            (i, j)
        } else {
            (j, i)
        }
    };

    if tabu_list.contains(&cand_pair) {
        if (best_so_far.fitness - aspiration_threshold..=best_so_far.fitness + aspiration_threshold)
            .contains(&chosen_solution.0)
            && !swaps_overlap(&chosen_solution.1, parent_swap)
        {
            // aspiration grants permission
        } else {
            chosen_solution = swap_candidates_ind
                .iter()
                .find(|sol| {
                    let (i, j) = sol.1;
                    let pair = if i < j { (i, j) } else { (j, i) };
                    !tabu_list.contains(&pair) && !swaps_overlap(&sol.1, parent_swap)
                })
                .copied()
                .unwrap_or(chosen_solution);
        }
    }

    chosen_solution
}

pub fn insert_and_adjust_tabu_list(
    tabu_list: &mut VecDeque<(usize, usize)>,
    swap_move: (usize, usize),
    len_tabu_list: usize,
) {
    let pair = if swap_move.0 < swap_move.1 {
        swap_move
    } else {
        (swap_move.1, swap_move.0)
    };

    tabu_list.push_front(pair);

    while tabu_list.len() > len_tabu_list {
        tabu_list.pop_back();
    }
}
