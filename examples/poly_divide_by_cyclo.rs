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

// Note:
// - The polynomial are not made public to the outside
// - No range check is performed after the division

const N: usize = 4;
const M: usize = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub nominator:  Vec<i8>,// nominator polynomial coefficients little endian of degree N (last element = constant term)
    pub denominator: Vec<i8>, // denominator polynomial coefficients little endian of degree M (last element = constant term)
}

// takes a polynomial represented by its coefficients in a vector (public input)
// and output a polynomial divided by denominator polynomial f(x)=x^m+1 where m is a power of 2 (public output)
fn poly_divide_by_cyclo<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput,
    make_public: &mut Vec<AssignedValue<F>>,
) {

	// Assert that degree of nominator poly is equal to the constant N
    assert_eq!(input.nominator.len() - 1, N);
    // Assert that degree of denominator poly is equal to the constant M
    assert_eq!(input.denominator.len() - 1, M);

    // Assign the input polynomials to the circuit
    let nom_assigned: Vec<AssignedValue<F>> = input
        .nominator
        .iter()
        .map(|x| {
            let result = F::from(*x as u64);
            ctx.load_witness(result)
        })
        .collect();

    let denom_assigned: Vec<AssignedValue<F>> = input
        .denominator
        .iter()
        .map(|x| {
            let result = F::from(*x as u64);
            ctx.load_witness(result)
        })
        .collect();

    // assert the correct length of the assigned polynomails
    assert_eq!(nom_assigned.len() - 1, N);
    assert_eq!(denom_assigned.len() - 1, M);

    // crete gate chip
    let gate = GateChip::<F>::default();

    // long division operation
    let (quot, rem) = div_euclid(&input.nominator, &input.denominator);

    // assign the quot to the gate chip
    let quot_assigned: Vec<AssignedValue<F>> = quot
        .iter()
        .map(|x| {
            let result = F::from(*x as u64);
            ctx.load_witness(result)
        })
        .collect();

    // assign the rem to the gate chip
    // note that it first pads with 0 to make the length of rem and nominator equal
    let initial_size = input.nominator.len() - rem.len();

    let mut rem_assigned = Vec::with_capacity(input.nominator.len());
    for _i in 0..(initial_size) {
        let zero = F::from(0 as u64);
        rem_assigned.push(ctx.load_witness(zero));
    }

    rem
    .iter()
    .for_each(|x| {
        let result = F::from(*x as u64);
        rem_assigned.push(ctx.load_witness(result));
    });

	// make the rem output public
	for i in 0..(rem_assigned.len() - 1) {
		make_public.push(rem_assigned[i]);
	}

    // ---- constraint check -----
    // check that quotient * denominator + rem = nominator
    // quot_assigned * denom_assigned
    let mut prod_val: Vec<AssignedValue<F>> = vec![];
    for i in 0..(2 * M + 1) {
        let mut coefficient_accumaltor: Vec<AssignedValue<F>> = vec![];

        if i < M + 1 {
            for a_idx in 0..=i {
                let a = quot_assigned[a_idx];
                let b = denom_assigned[i - a_idx];
                // push the product of a and b to the coefficient_accumaltor
                coefficient_accumaltor.push(gate.mul(ctx, a, b));
            }
        } else {
            for a_idx in (i - M)..=M {
                let a = quot_assigned[a_idx];
                let b = denom_assigned[i - a_idx];
                // push the product of a and b to the coefficient_accumaltor
                coefficient_accumaltor.push(gate.mul(ctx, a, b));
            }
        }

        let prod_value = coefficient_accumaltor
            .iter()
            .fold(ctx.load_witness(F::zero()), |acc, x| gate.add(ctx, acc, *x));

        prod_val.push(prod_value);
    }

    assert_eq!(prod_val.len(), rem_assigned.len());

    // prod_val + rem_assigned
    let sum_assigned: Vec<AssignedValue<F>> = prod_val
    .iter()
    .zip(rem_assigned.iter())
    .take(2 * N - 1)
    .map(|(&a, &b)| gate.add(ctx, a, b))
    .collect();

    // check that sum_assined coeff = nominator coeff
	let out_expected = input.nominator;

	for (sum, out) in sum_assigned.iter().zip(out_expected) {
        assert_eq!(sum.value().get_lower_32(), out as u32);
    }

    // ---- constraint check -----
}

fn div_euclid(f: &Vec<i8>, g: &Vec<i8>) -> (Vec<i8>, Vec<i8>) {

    if g.is_empty() || g.iter().all(|&x| x == 0) {
        panic!("Cannot divide by a zero polynomial!");
    }

    let mut dividend = f.clone();
    let divisor_degree = g.len() - 1;
    let mut quotient = Vec::new();

    while dividend.len() > divisor_degree {
        let leading_coefficient_ratio = dividend[0] / g[0];
        quotient.push(leading_coefficient_ratio);

        for (i, coeff) in g.iter().enumerate() {
            let diff = dividend[i] - leading_coefficient_ratio * *coeff;
            dividend[i] = diff;
        }

        dividend.remove(0);
    }

    // Trim the leading zeros from quotient and remainder
    while !quotient.is_empty() && quotient[0] == 0 {
        quotient.remove(0);
    }

    while !dividend.is_empty() && dividend[0] == 0 {
        dividend.remove(0);
    }

    (quotient, dividend)

}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(poly_divide_by_cyclo, args);
}