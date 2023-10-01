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
const Q: u64 = 2u64.pow(8) + 1; // modulus of the field F_q
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
fn check_poly_from_distribution_chi_error<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput<N>,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // // Since we cannot represent negative numbers in the circuit, the value - 1 is represented as the field element q - 1.
    // // Therefore we split the range [-b, b] into two ranges [0, b] and [q-b, q-1]

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

    // The goal is to check that a_assigned[i] is in the range [0, b] or in the range [q-b, q-1]
    // We split this check into two checks:
    // - Check that a_assigned[i] is in the range [0, b] and store the boolean result in in_partial_range_1_vec
    // - Check that a_assigned[i] is in the range [q-b, q-1] and store the boolean result in in_partial_range_2_vec
    // We then perform (`in_partial_range_1_vec` OR `in_partial_range_2_vec`) to check that a_assigned[i] is in the range [0, b] [q-b, q-1]
    // The result of this check is stored in the `in_range` vector. The bool value of `in_range` is then enforced to be true

    let range = RangeChip::default(lookup_bits);

    let mut in_range_vec = Vec::with_capacity(N + 1);

    for coeff in &a_assigned {
        // Check for the range [0, b]
        let in_partial_range_1 = range.is_less_than_safe(ctx, *coeff, B + 1);

        // Check for the range [q-b, q-1]
        let not_in_range_lower_bound = range.is_less_than_safe(ctx, *coeff, Q - B);
        let in_range_lower_bound = range.gate.not(ctx, not_in_range_lower_bound);
        let in_range_upper_bound = range.is_less_than_safe(ctx, *coeff, Q);
        let in_partial_range_2 = range.gate.and(ctx, in_range_lower_bound, in_range_upper_bound);

        // Combined check for [0, b] OR [q-b, q-1]
        let in_range = range.gate.or(ctx, in_partial_range_1, in_partial_range_2);
        in_range_vec.push(in_range);
    }

    // Enforce that in_range_vec[i] = true
    for in_range in in_range_vec {
        let bool = range.gate.is_equal(ctx, in_range, Constant(F::from(1)));
        range.gate.assert_is_const(ctx, &bool, &F::from(1));
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(check_poly_from_distribution_chi_error, args);
}
