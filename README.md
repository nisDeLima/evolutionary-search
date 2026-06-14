# Evolutionary Knapsack Solver

An evolutionary algorithm with simulated annealing post-processing for the 0/1 Knapsack problem. Built in Rust.

## Architecture

```
                    ┌───────────────────────┐
                    │      Initialize       │
                    │  4 greedy seeds +     │
                    │  random greedy fills  │
                    └──────────┬────────────┘
                               │
              ┌────────────────▼────────────────┐
              │                                 │
              │  ┌──────────────────────────┐   │
              │  │        Evaluate          │   │
              │  │  Sort by fitness desc    │   │
              │  └─────────┬────────────────┘   │
              │            │                    │
              │       ┌────┴──────┐             │
              │       │           │             │
              │  ┌────▼─────┐ ┌──▼──────────┐  │
              │  │  Select  │ │  Elitism    │  │
              │  │  Top %   │ │  Keep top N │  │
              │  └────┬─────┘ └──────┬──────┘  │
              │       │              │          │
              │  ┌────▼──────────┐   │          │
              │  │   Crossover   │   │          │
              │  │  Coin flip    │   │          │
              │  │  per gene     │   │          │
              │  └────┬──────────┘   │          │
              │       │              │          │
              │  ┌────▼──────────┐   │          │
              │  │    Repair     │   │          │
              │  │  50/50 smart  │   │          │
              │  │  or random    │   │          │
              │  └────┬──────────┘   │          │
              │       │              │          │
              │  ┌────▼──────────┐   │          │
              │  │    Mutate     │   │          │
              │  │  Flip genes   │   │          │
              │  └────┬──────────┘   │          │
              │       │              │          │
              │  ┌────▼──────────┐   │          │
              │  │    Repair     │   │          │
              │  └────┬──────────┘   │          │
              │       │              │          │
              │  ┌────▼──────────────▼──────┐   │
              │  │        Replace           │   │
              │  │  Elites + children       │   │
              │  └──────────┬───────────────┘   │
              │             │                   │
              │    Repeat N generations         │
              └────────────────────────────────-┘
                               │
              ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ─
                   After evolution loop
              ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ─
                               │
                    ┌──────────▼────────────┐
                    │  Simulated Annealing  │
                    │  Top 10 candidates    │
                    │  Accept worse early,  │
                    │  converge late        │
                    └──────────┬────────────┘
                               │
                    ┌──────────▼────────────┐
                    │  Return best solution │
                    └───────────────────────┘
```

## How It Works

### Candidate Representation

Each candidate is a `Vec<bool>` where index `i` being `true` means item `i` is in the bag. Fitness (total value) and weight are cached and updated incrementally, so crossover and mutation don't need to recalculate from scratch.

```rust
struct Candidate {
    genes: Vec<bool>,   // one bool per item
    fitness: u64,       // total value (cached)
    weight: u64,        // total weight (cached)
}
```

### Phase 1: Initialize

The initial population is seeded with diversity. Four candidates use deterministic greedy strategies with different sorting criteria, ensuring the population starts from different corners of the solution space:

| Seed | Strategy | What it favors |
|---|---|---|
| 1 | Greedy by value/weight density | Best bang per unit weight |
| 2 | Greedy by highest value | Most valuable items first |
| 3 | Greedy by lightest weight | Packs the most items |
| 4 | Greedy by heaviest weight | Catches "heavy hitter" patterns |

The remaining slots are filled with shuffled greedy: randomize item order, then greedily take everything that fits. This produces valid, reasonably packed candidates with natural variation.

### Phase 2: Evaluate

Sort the population by fitness descending. Since fitness is maintained incrementally (updated during add/remove operations), evaluation is just a sort. No recomputation needed.

### Phase 3: Select + Elitism

**Elitism**: the top N candidates (typically 1-2) are copied directly into the next generation, untouched. This guarantees the best solution never degrades across generations.

**Selection**: the top percentage of candidates (e.g. 40%) become eligible parents for crossover. The rest are discarded.

### Phase 4: Crossover

For each child needed (population_size minus elite_count):

1. Pick two random parents from the selected pool
2. For each gene (item), flip a coin to inherit from parent A or parent B
3. Repair if overweight (see below)
4. Optional local search on small problems

This is uniform crossover. It can combine good traits from different parents but may also produce overweight candidates, which is why repair follows immediately.

### Phase 5: Mutate

For each child, iterate through every gene. Each gene has a `mutation_rate` probability of flipping (in becomes out, out becomes in). After mutation, repair if overweight.

Mutation prevents the population from converging too early. Without it, crossover between similar parents produces identical children and evolution stalls.

### Repair (Hybrid)

When a candidate exceeds capacity, items must be removed. The repair function uses a 50/50 strategy:

**Smart repair (50%)**: sort selected items by value/weight density ascending, remove the worst-density items first. This preserves the most valuable items relative to their weight.

**Random repair (50%)**: shuffle selected items, remove in random order. This preserves item combinations that smart repair would destroy. Without random repair, every candidate collapses back toward the greedy solution because smart repair is essentially greedy-in-reverse.

The hybrid approach was critical. Pure smart repair caused the evolutionary solver to match greedy exactly on every level. Random repair lets "weird" combinations survive and gives evolution something different to work with.

### Phase 6: Replace

New generation = elites (unchanged) + children (crossover + mutation). Population size stays constant across generations.

### Post-Processing: Simulated Annealing

After the evolutionary loop finishes, the top 10 candidates undergo simulated annealing. This is where the real optimization happens.

**How SA works:**

1. Start with high temperature (15% of candidate's fitness)
2. Each iteration: pick a random selected item, swap it for a random unselected item that fits
3. If the swap improves fitness: always accept, update best-seen
4. If the swap worsens fitness: accept with probability `e^(delta / temperature)`
5. Cool the temperature by the cooling rate
6. Return the best solution seen across all iterations

**Why SA matters:**

The evolutionary loop generates diverse starting points but struggles to improve beyond greedy on its own. SA can escape local optima by temporarily accepting worse solutions, something pure hill climbing cannot do. In benchmarks, SA accounted for the majority of the improvement over greedy.

**Temperature schedule:**

| Problem size | Iterations | Initial temp | Cooling rate |
|---|---|---|---|
| ≤ 100 items | 500,000 | 15% of fitness | 1 - 5/500000 |
| ≤ 500 | 200,000 | 15% of fitness | 1 - 5/200000 |
| ≤ 2,000 | 100,000 | 15% of fitness | 1 - 5/100000 |
| ≤ 10,000 | 50,000 | 15% of fitness | 1 - 5/50000 |
| ≤ 50,000 | 15,000 | 15% of fitness | 1 - 5/15000 |
| 100,000+ | 5,000 | 15% of fitness | 1 - 5/5000 |

## Configuration

```rust
pub struct EvolutionConfig {
    pub population_size: usize,   // candidates per generation
    pub generations: usize,        // evolution loop iterations
    pub elite_count: usize,        // preserved unchanged (1-2)
    pub selection_rate: f64,       // fraction selected as parents
    pub mutation_rate: f64,        // per-gene flip probability
    pub seed: u64,                 // RNG seed for reproducibility
}
```

Parameters scale with problem size. Small problems use larger populations and higher mutation for aggressive exploration. Large problems use lean configs to stay within time budgets.

| Size | Population | Generations | Mutation |
|---|---|---|---|
| ≤ 100 | 80 | 500 | 8% |
| ≤ 500 | 60 | 300 | 6% |
| ≤ 2,000 | 50 | 200 | 4% |
| ≤ 10,000 | 40 | 100 | 2% |
| ≤ 50,000 | 25 | 50 | 1% |
| 100,000+ | 15 | 25 | 0.5% |

## Usage

```rust
use evolution::{evolve, EvolutionConfig};

let config = EvolutionConfig {
    population_size: 50,
    generations: 200,
    elite_count: 2,
    selection_rate: 0.4,
    mutation_rate: 0.04,
    seed: 42,
};

let selected_indices: Vec<usize> = evolve(&problem, &config);
```

## Complexity

**Time**: O(generations × population_size × n) for evolution, plus O(sa_iterations × n) for simulated annealing.

**Space**: O(population_size × n) for the candidate pool. One bool per item per candidate.

## Dependencies

`rand 0.8` for deterministic RNG and shuffling. Nothing else.
