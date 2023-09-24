use std::env::var;

use clap::Parser;
use halo2_base::safe_types::RangeChip;
use halo2_base::safe_types::{GateInstructions, RangeInstructions};
use halo2_base::utils::ScalarField;
use halo2_base::AssignedValue;
#[allow(unused_imports)]
use halo2_base::{
    Context,
    QuantumCell::{Constant, Existing, Witness},
};
use halo2_scaffold::scaffold::cmd::Cli;
use halo2_scaffold::scaffold::run;
use serde::{Deserialize, Serialize};

const N: usize = 3; // degree of the polynomial
const Q: u64 = 2u64.pow(8); // modulus of the field F_q
const B: u64 = 30; // upper bound of the distribution [-b, b]

// Notes:
// - Q and B are public constants of the circuit
// - The input polynomial is not made public

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput<const N: usize> {
    pub a: Vec<u64>, // polynomial coefficients little endian of degree N
}

// this algorithm takes a polynomial a and the upper bound of a distrbution [-b, b] and checks if the coefficients of a are in the range.
// if the coefficients are in the range, it means that the polynomial was sampled from the distribution
fn check_poly_from_distribution<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput<N>,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // Since we cannot represent negative numbers in the circuit, the value - 1 is represented as the field element q - 1.
    // Therefore we split the range [-b, b] into two ranges [0, b] and [q-b, q-1]
    // First of all, test outside the circuit that the coefficients of the polynomial are in the range [0, b] or in the range [q-b, q-1]
    for i in 0..N {
        assert!((input.a[i] <= B) || (Q - B <= input.a[i] && input.a[i] < Q));
    }

    // Assign the input polynomial to the circuit
    let a_assigned: Vec<AssignedValue<F>> = input
        .a
        .iter()
        .map(|x| {
            let result = F::from(*x);
            ctx.load_witness(result)
        })
        .collect();

    // lookup bits must agree with the size of the lookup table, which is specified by an environmental variable
    let lookup_bits =
        var("LOOKUP_BITS").unwrap_or_else(|_| panic!("LOOKUP_BITS not set")).parse().unwrap();

    let range = RangeChip::default(lookup_bits);

    // The goal is to check that a_assigned[i] is in the range [0, b] or in the range [q-b, q-1]
    // We split this check into two checks:
    // - Check that a_assigned[i] is in the range [0, b] and store the boolean result in in_partial_range_1_vec
    // - Check that a_assigned[i] is in the range [q-b, q-1] and store the boolean result in in_partial_range_2_vec
    // We then perform (`in_partial_range_1_vec` OR `in_partial_range_2_vec`) to check that a_assigned[i] is in the range [0, b] [q-b, q-1]
    // The result of this check is stored in the `in_range` vector. The bool value of `in_range` is then enforced to be true

    // 1. Check that a_assigned[i] is in the range [0, b] and store the boolean result in in_partial_range_1_vec
    let mut in_partial_range_1_vec = Vec::new();
    for i in 0..N {
        let in_partial_range_1 = range.is_less_than_safe(ctx, a_assigned[i], B + 1);
        in_partial_range_1_vec.push(in_partial_range_1);
    }

    // 2. Check that a_assigned[i] is in the range [q-b, q-1] and store the result in in_partial_range_2_vec
    // Since we cannot perform such a check directly in halo 2 lib, we need to check that:
    // - Condition `in_range_lower_bound` : a_assigned[i] is greater or equal than q-b -> we express it using `is_less_than_safe` and then negate the result
    // - Condition `in_range_upper_bound` : a_assigned[i] is less than q -> we express it using `is_less_than_safe`
    // The boolean assigned to `in_partial_range_2_vec` is true if both conditions are satisfied (`in_range_lower_bound` AND `in_range_upper_bound`)

    let mut in_range_lower_bound_vec = Vec::new();
    for i in 0..N {
        let not_in_range_lower_bound = range.is_less_than_safe(ctx, a_assigned[i], Q - B);

        let in_range_range_lower_bound = range.gate.not(ctx, not_in_range_lower_bound);
        in_range_lower_bound_vec.push(in_range_range_lower_bound);
    }

    let mut in_range_upper_bound_vec = Vec::new();
    for i in 0..N {
        let in_range_upper_bound = range.is_less_than_safe(ctx, a_assigned[i], Q);
        in_range_upper_bound_vec.push(in_range_upper_bound);
    }

    // Perform (`in_range_lower_bound_vec` AND `in_range_upper_bound_vec`) to check that a_assigned[i] is in the range [q-b, q-1] assign the result to in_partial_range_2_vec
    let mut in_partial_range_2_vec = Vec::new();
    for i in 0..N {
        let in_partial_range_2 =
            range.gate.and(ctx, in_range_lower_bound_vec[i], in_range_upper_bound_vec[i]);
        in_partial_range_2_vec.push(in_partial_range_2);
    }

    // 3. Perform (`in_partial_range_1_vec` OR `in_partial_range_2_vec`) to check that a_assigned[i] is in the range [0, b] [q-b, q-1]
    let mut in_range_vec = Vec::new();
    for i in 0..N {
        let in_range = range.gate.or(ctx, in_partial_range_1_vec[i], in_partial_range_2_vec[i]);
        in_range_vec.push(in_range);
    }

    // 4. Enforce that in_range_vec[i] = true
    for i in 0..N {
        range.gate.is_equal(ctx, in_range_vec[i], Constant(F::from(1)));
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(check_poly_from_distribution, args);
}
