use clap::Parser;
use halo2_base::safe_types::{RangeChip, RangeInstructions};
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
use std::env::var;

// Assumptions:
// - The coefficients of the dividend polynomial are in the range of 16 bits
// - The divisor polynomial is a cyclotomic polynomial of degree M

const N: usize = 3;
const MODULUS: usize = 11;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub poly: Vec<u8>, // polynomial coefficients big endian of degree N (last element = constant term)
    pub out: Vec<u8>, // polynomial coefficients big endian of degree N (last element = constant term)
}

// takes a polynomial represented by its coefficients in a vector (public input)
// and output a new polynomial reduced mod MODULUS (public output)
fn reduce_poly<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {


	// Assert that degree is equal to the constant N
    assert_eq!(input.poly.len() - 1, N);

    // Assign the input polynomials to the circuit
    let in_assigned: Vec<AssignedValue<F>> = input
        .poly
        .iter()
        .map(|x| {
            let result = F::from(*x as u64);
            ctx.load_witness(result) // load the input as a witness
        })
        .collect();

    // needs to be compatible with some backend setup for lookup table to do range check
    // so read from environemntal variable
    let lookup_bits =
        var("LOOKUP_BITS").unwrap_or_else(|_| panic!("LOOKUP_BITS nto set")).parse().unwrap();

    // instead of GateChip create a RangeChip, which allows you to do range check
    let range = RangeChip::default(lookup_bits);

    // Enforce that in_assigned[i] % MODULUS = rem_assigned[i]
    // coefficients of input polynomials are guaranteed to be at most 16 bits by assumption
    let rem_assigned: Vec<AssignedValue<F>> =
        in_assigned.iter().take(2 * N - 1).map(|&x| range.div_mod(ctx, x, MODULUS, 16).1).collect();

    // make the output public
    for i in 0..(N + 1) {
        make_public.push(rem_assigned[i]);
    }


    // check that rem_assigned = output of the circuit
    let out_expected = input.out;

    for i in 0..N {
        assert_eq!(*rem_assigned[i].value(), F::from(out_expected[i] as u64));
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(reduce_poly, args);
}
