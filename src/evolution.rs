use crate::KnapsackProblem;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

pub struct EvolutionConfig {
    pub population_size: usize,
    pub generations: usize,
    pub elite_count: usize,
    pub selection_rate: f64,
    pub mutation_rate: f64,
    pub seed: u64,
}

#[derive(Clone)]
struct Candidate {
    genes: Vec<bool>,
    fitness: u64,
    weight: u64,
}

impl Candidate {
    fn new(n: usize) -> Self {
        Candidate {
            genes: vec![false; n],
            fitness: 0,
            weight: 0,
        }
    }

    fn add(&mut self, idx: usize, problem: &KnapsackProblem) {
        if !self.genes[idx] {
            self.genes[idx] = true;
            self.weight += problem.items[idx].weight;
            self.fitness += problem.items[idx].value;
        }
    }

    fn remove(&mut self, idx: usize, problem: &KnapsackProblem) {
        if self.genes[idx] {
            self.genes[idx] = false;
            self.weight -= problem.items[idx].weight;
            self.fitness -= problem.items[idx].value;
        }
    }
}

fn density_order(problem: &KnapsackProblem) -> Vec<usize> {
    let mut order: Vec<usize> = (0..problem.items.len()).collect();
    order.sort_by(|&a, &b| {
        let da = problem.items[a].value as f64 / problem.items[a].weight as f64;
        let db = problem.items[b].value as f64 / problem.items[b].weight as f64;
        db.partial_cmp(&da).unwrap()
    });
    order
}

fn greedy_fill(cand: &mut Candidate, order: &[usize], problem: &KnapsackProblem) {
    for &idx in order {
        if cand.weight + problem.items[idx].weight <= problem.capacity {
            cand.add(idx, problem);
        }
    }
}

pub fn initialize(
    problem: &KnapsackProblem,
    config: &EvolutionConfig,
    rng: &mut StdRng,
) -> Vec<Candidate> {
    let n = problem.items.len();
    let mut candidates: Vec<Candidate> = Vec::with_capacity(config.population_size);

    let by_density = density_order(problem);

    let mut c1 = Candidate::new(n);
    greedy_fill(&mut c1, &by_density, problem);
    candidates.push(c1);

    let mut by_value: Vec<usize> = (0..n).collect();
    by_value.sort_by(|&a, &b| problem.items[b].value.cmp(&problem.items[a].value));
    let mut c2 = Candidate::new(n);
    greedy_fill(&mut c2, &by_value, problem);
    candidates.push(c2);

    let mut by_light: Vec<usize> = (0..n).collect();
    by_light.sort_by(|&a, &b| problem.items[a].weight.cmp(&problem.items[b].weight));
    let mut c3 = Candidate::new(n);
    greedy_fill(&mut c3, &by_light, problem);
    candidates.push(c3);

    let mut by_heavy: Vec<usize> = (0..n).collect();
    by_heavy.sort_by(|&a, &b| problem.items[b].weight.cmp(&problem.items[a].weight));
    let mut c4 = Candidate::new(n);
    greedy_fill(&mut c4, &by_heavy, problem);
    candidates.push(c4);

    while candidates.len() < config.population_size {
        let mut cand = Candidate::new(n);
        let mut order: Vec<usize> = (0..n).collect();
        order.shuffle(rng);
        greedy_fill(&mut cand, &order, problem);
        candidates.push(cand);
    }

    candidates
}

pub fn evaluate(candidates: &mut Vec<Candidate>) {
    candidates.sort_by(|a, b| b.fitness.cmp(&a.fitness));
}

pub fn select(candidates: &[Candidate], config: &EvolutionConfig) -> Vec<usize> {
    let count = (candidates.len() as f64 * config.selection_rate) as usize;
    (0..count.max(2)).collect()
}

pub fn crossover(
    candidates: &[Candidate],
    parent_indices: &[usize],
    problem: &KnapsackProblem,
    config: &EvolutionConfig,
    rng: &mut StdRng,
    swap_budget: usize,
) -> Vec<Candidate> {
    let num_children = config.population_size - config.elite_count;
    let n = problem.items.len();
    let mut children: Vec<Candidate> = Vec::with_capacity(num_children);

    while children.len() < num_children {
        let a = &candidates[parent_indices[rng.gen_range(0..parent_indices.len())]];
        let b = &candidates[parent_indices[rng.gen_range(0..parent_indices.len())]];

        let mut child = Candidate::new(n);
        for i in 0..n {
            let gene = if rng.gen_bool(0.5) {
                a.genes[i]
            } else {
                b.genes[i]
            };
            if gene {
                child.genes[i] = true;
                child.weight += problem.items[i].weight;
                child.fitness += problem.items[i].value;
            }
        }

        repair(&mut child, problem, rng);
        if swap_budget > 0 {
            local_search(&mut child, problem, rng, swap_budget);
        }
        children.push(child);
    }

    children
}

pub fn mutate(
    children: &mut Vec<Candidate>,
    problem: &KnapsackProblem,
    config: &EvolutionConfig,
    rng: &mut StdRng,
    swap_budget: usize,
) {
    for child in children.iter_mut() {
        let mut changed = false;
        for i in 0..child.genes.len() {
            if rng.gen_bool(config.mutation_rate) {
                if child.genes[i] {
                    child.remove(i, problem);
                } else {
                    child.add(i, problem);
                }
                changed = true;
            }
        }
        if changed {
            repair(child, problem, rng);
            if swap_budget > 0 {
                local_search(child, problem, rng, swap_budget);
            }
        }
    }
}

fn repair(candidate: &mut Candidate, problem: &KnapsackProblem, rng: &mut StdRng) {
    if candidate.weight <= problem.capacity {
        return;
    }

    let mut selected: Vec<usize> = candidate
        .genes
        .iter()
        .enumerate()
        .filter(|(_, g)| **g)
        .map(|(i, _)| i)
        .collect();

    if rng.gen_bool(0.5) {
        selected.sort_by(|&a, &b| {
            let da = problem.items[a].value as f64 / problem.items[a].weight as f64;
            let db = problem.items[b].value as f64 / problem.items[b].weight as f64;
            da.partial_cmp(&db).unwrap()
        });
    } else {
        selected.shuffle(rng);
    }

    for idx in selected {
        if candidate.weight <= problem.capacity {
            break;
        }
        candidate.remove(idx, problem);
    }
}

fn local_search(
    candidate: &mut Candidate,
    problem: &KnapsackProblem,
    rng: &mut StdRng,
    max_swaps: usize,
) {
    let n = problem.items.len();
    let sample_size = n.min(20);

    for _ in 0..max_swaps {
        let selected: Vec<usize> = candidate
            .genes
            .iter()
            .enumerate()
            .filter(|(_, g)| **g)
            .map(|(i, _)| i)
            .collect();

        if selected.is_empty() {
            break;
        }

        let remove_idx = selected[rng.gen_range(0..selected.len())];
        let remove_val = problem.items[remove_idx].value;
        let remove_wt = problem.items[remove_idx].weight;
        let space = problem.capacity - candidate.weight + remove_wt;

        let mut best_gain: i64 = 0;
        let mut best_add: Option<usize> = None;

        for _ in 0..sample_size {
            let add_idx = rng.gen_range(0..n);
            if candidate.genes[add_idx] || add_idx == remove_idx {
                continue;
            }
            if problem.items[add_idx].weight > space {
                continue;
            }
            let gain = problem.items[add_idx].value as i64 - remove_val as i64;
            if gain > best_gain {
                best_gain = gain;
                best_add = Some(add_idx);
            }
        }

        if let Some(add_idx) = best_add {
            candidate.remove(remove_idx, problem);
            candidate.add(add_idx, problem);
        }
    }
}

fn exhaustive_hill_climb(candidate: &mut Candidate, problem: &KnapsackProblem) {
    let n = problem.items.len();

    loop {
        let mut best_gain: i64 = 0;
        let mut best_remove: usize = 0;
        let mut best_add: Option<usize> = None;

        let selected: Vec<usize> = candidate
            .genes
            .iter()
            .enumerate()
            .filter(|(_, g)| **g)
            .map(|(i, _)| i)
            .collect();

        for &rem in &selected {
            let space = problem.capacity - candidate.weight + problem.items[rem].weight;
            let rem_val = problem.items[rem].value;

            // Try adding nothing (just removing)
            // Skip — removing without adding is always worse

            // Try every unselected item as replacement
            for add in 0..n {
                if candidate.genes[add] || add == rem {
                    continue;
                }
                if problem.items[add].weight > space {
                    continue;
                }
                let gain = problem.items[add].value as i64 - rem_val as i64;
                if gain > best_gain {
                    best_gain = gain;
                    best_remove = rem;
                    best_add = Some(add);
                }
            }
        }

        // Also try just adding unselected items that fit without removing anything
        let remaining_space = problem.capacity - candidate.weight;
        for add in 0..n {
            if candidate.genes[add] {
                continue;
            }
            if problem.items[add].weight <= remaining_space {
                let gain = problem.items[add].value as i64;
                if gain > best_gain {
                    best_gain = gain;
                    best_remove = 0; // sentinel: no removal needed
                    best_add = Some(add);
                }
            }
        }

        if best_gain <= 0 {
            break;
        }

        if let Some(add) = best_add {
            if best_remove != 0 || candidate.genes[best_remove] {
                // Only remove if we actually identified something to remove
                if candidate.genes[best_remove] && best_add != Some(best_remove) {
                    candidate.remove(best_remove, problem);
                }
            }
            candidate.add(add, problem);
        }
    }
}

fn exhaustive_hill_climb_budgeted(candidate: &mut Candidate, problem: &KnapsackProblem) {
    let n = problem.items.len();

    // For large problems, full O(selected * n) per iteration is too expensive.
    // Cap the number of rounds.
    let max_rounds = if n <= 500 {
        1000
    } else if n <= 5_000 {
        100
    } else if n <= 50_000 {
        10
    } else {
        3
    };

    for _ in 0..max_rounds {
        let mut best_gain: i64 = 0;
        let mut best_swap: Option<(Option<usize>, usize)> = None; // (remove, add)

        let selected: Vec<usize> = candidate
            .genes
            .iter()
            .enumerate()
            .filter(|(_, g)| **g)
            .map(|(i, _)| i)
            .collect();

        // Try pure additions
        let remaining_space = problem.capacity - candidate.weight;
        for add in 0..n {
            if candidate.genes[add] {
                continue;
            }
            if problem.items[add].weight <= remaining_space {
                let gain = problem.items[add].value as i64;
                if gain > best_gain {
                    best_gain = gain;
                    best_swap = Some((None, add));
                }
            }
        }

        // Try 1-for-1 swaps
        // For large n, sample instead of exhaustive
        let sample_limit = n.min(500);
        for &rem in &selected {
            let space = problem.capacity - candidate.weight + problem.items[rem].weight;
            let rem_val = problem.items[rem].value;

            if sample_limit >= n {
                for add in 0..n {
                    if candidate.genes[add] || add == rem {
                        continue;
                    }
                    if problem.items[add].weight > space {
                        continue;
                    }
                    let gain = problem.items[add].value as i64 - rem_val as i64;
                    if gain > best_gain {
                        best_gain = gain;
                        best_swap = Some((Some(rem), add));
                    }
                }
            } else {
                // Sample: check items near the space boundary for best fit
                for add in 0..n {
                    if candidate.genes[add] || add == rem {
                        continue;
                    }
                    if problem.items[add].weight > space {
                        continue;
                    }
                    let gain = problem.items[add].value as i64 - rem_val as i64;
                    if gain > best_gain {
                        best_gain = gain;
                        best_swap = Some((Some(rem), add));
                    }
                }
            }
        }

        if best_gain <= 0 {
            break;
        }

        if let Some((remove, add)) = best_swap {
            if let Some(rem) = remove {
                candidate.remove(rem, problem);
            }
            candidate.add(add, problem);
        }
    }
}

fn simulated_annealing(candidate: &mut Candidate, problem: &KnapsackProblem, rng: &mut StdRng) {
    let n = problem.items.len();

    let max_iterations = if n <= 100 {
        500_000
    } else if n <= 500 {
        200_000
    } else if n <= 2_000 {
        100_000
    } else if n <= 10_000 {
        50_000
    } else if n <= 50_000 {
        15_000
    } else {
        5_000
    };

    let initial_temp: f64 = candidate.fitness as f64 * 0.15;
    let cooling_rate: f64 = 1.0 - (5.0 / max_iterations as f64);

    let mut temp = initial_temp;
    let mut best = candidate.clone();

    for _ in 0..max_iterations {
        if temp < 0.001 {
            break;
        }

        let selected: Vec<usize> = candidate
            .genes
            .iter()
            .enumerate()
            .filter(|(_, g)| **g)
            .map(|(i, _)| i)
            .collect();

        if selected.is_empty() {
            break;
        }

        let rem_idx = selected[rng.gen_range(0..selected.len())];
        let space = problem.capacity - candidate.weight + problem.items[rem_idx].weight;

        let mut add_idx = rng.gen_range(0..n);
        let mut tries = 0;
        while (candidate.genes[add_idx] || problem.items[add_idx].weight > space) && tries < 30 {
            add_idx = rng.gen_range(0..n);
            tries += 1;
        }

        if candidate.genes[add_idx] || problem.items[add_idx].weight > space {
            temp *= cooling_rate;
            continue;
        }

        let old_fitness = candidate.fitness;
        candidate.remove(rem_idx, problem);
        candidate.add(add_idx, problem);

        let delta = candidate.fitness as f64 - old_fitness as f64;

        if delta >= 0.0 {
            if candidate.fitness > best.fitness {
                best = candidate.clone();
            }
        } else {
            let accept_prob = (delta / temp).exp();
            if rng.gen_bool(accept_prob.min(1.0)) {
                // Accept worse solution
            } else {
                candidate.remove(add_idx, problem);
                candidate.add(rem_idx, problem);
            }
        }

        temp *= cooling_rate;
    }

    *candidate = best;
}

pub fn evolve(problem: &KnapsackProblem, config: &EvolutionConfig) -> Vec<usize> {
    let n = problem.items.len();
    let mut rng = StdRng::seed_from_u64(config.seed);

    let swap_budget = if n <= 100 {
        30
    } else if n <= 500 {
        15
    } else if n <= 2_000 {
        5
    } else if n <= 10_000 {
        2
    } else {
        0
    };

    let mut candidates = initialize(problem, config, &mut rng);

    for _ in 0..config.generations {
        evaluate(&mut candidates);

        let parent_indices = select(&candidates, config);

        let elites: Vec<Candidate> = candidates
            .iter()
            .take(config.elite_count)
            .cloned()
            .collect();

        let mut children = crossover(
            &candidates,
            &parent_indices,
            problem,
            config,
            &mut rng,
            swap_budget,
        );
        mutate(&mut children, problem, config, &mut rng, swap_budget);

        candidates = elites;
        candidates.extend(children);
    }

    evaluate(&mut candidates);

    // // Hill climbing pass (commented out — replaced by simulated annealing)
    // let climb_count = candidates.len().min(5);
    // for i in 0..climb_count {
    //     let mut c = candidates[i].clone();
    //     exhaustive_hill_climb_budgeted(&mut c, problem);
    //     candidates[i] = c;
    // }

    // Simulated annealing pass on top candidates
    let sa_count = candidates.len().min(10);
    for i in 0..sa_count {
        let mut c = candidates[i].clone();
        simulated_annealing(&mut c, problem, &mut rng);
        candidates[i] = c;
    }

    evaluate(&mut candidates);

    candidates[0]
        .genes
        .iter()
        .enumerate()
        .filter(|(_, g)| **g)
        .map(|(i, _)| i)
        .collect()
}
