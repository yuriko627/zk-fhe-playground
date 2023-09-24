use clap::Parser;
use halo2_base::gates::GateChip;
use halo2_base::safe_types::GateInstructions;
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

// Notes:
// - The input polynomial is not made public
// - Q is a public constants of the circuit

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput<const N: usize> {
    pub a: Vec<u64>, // polynomial coefficients little endian of degree N
}

// this algorithm takes a polynomial a and checks if the coefficients of a are in the range [-1, 0, +1].
// if the coefficients are in the range, it means that the polynomial was sampled from the distribution
fn check_poly_from_distribution<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput<N>,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // Since we cannot represent negative numbers in the circuit, the value - 1 is represented as the field element q - 1.
    // Each coefficient of the polynomial should be in range [0, 1, q-1]
    // First of all, test outside the circuit that the coefficients of the polynomial are in the range [0, 1, q-1]
    for i in 0..N {
        assert!((input.a[i] == 0) || (input.a[i] == 1) || (input.a[i] == Q - 1));
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

    // The goal is to check that a_assigned[i] is equal to either 0, 1 or q-1
    // The constraint that we want to enforce is:
    // (a - 0) * (a - 1) * (a - (q-1)) = 0
    let gate = GateChip::<F>::default();

    // loop over all the coefficients of the polynomial
    for i in 0..N {
        let coeff = a_assigned[i];

        // constrain (a - 0)
        let factor_1 = gate.sub(ctx, coeff, Constant(F::from(0)));

        // constrain (a - 1)
        let factor_2 = gate.sub(ctx, coeff, Constant(F::from(1)));

        // constrain (a - (q-1))
        let factor_3 = gate.sub(ctx, coeff, Constant(F::from(Q - 1)));

        // constrain (a - 0) * (a - 1)
        let factor_1_2 = gate.mul(ctx, factor_1, factor_2);

        // constrain (a - 0) * (a - 1) * (a - (q-1))
        let factor_1_2_3 = gate.mul(ctx, factor_1_2, factor_3);

        // constrain (a - 0) * (a - 1) * (a - (q-1)) = 0
        gate.is_zero(ctx, factor_1_2_3);
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(check_poly_from_distribution, args);
}
