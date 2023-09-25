use ark_bn254::Fr;
use ark_ff::fields::PrimeField;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
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
// - The input polynomials are not made public
// - Suppose that range check is performed on the coeffiicients in order to avoid overflow for happen during the addition

const N: usize = 3;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput<const N: usize> {
    pub a: Vec<u8>, // polynomial coefficients little endian of degree N
    pub b: Vec<u8>, // polynomial coefficients little endian of degree N
}

// this algorithm takes two polynomials a and b of the same degree and output their sum to the public
fn poly_add<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput<N>,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // assert that the input polynomials have the same degree
    assert_eq!(input.a.len() - 1, input.b.len() - 1);
    // assert that degree is equal to the constant DEGREE
    assert_eq!(input.a.len() - 1, N);

    // Assign the input polynomials to the circuit
    let a_assigned: Vec<AssignedValue<F>> = input
        .a
        .iter()
        .map(|x| {
            let result = F::from(*x as u64);
            ctx.load_witness(result)
        })
        .collect();

    let b_assigned: Vec<AssignedValue<F>> = input
        .b
        .iter()
        .map(|x| {
            let result = F::from(*x as u64);
            ctx.load_witness(result)
        })
        .collect();

    // assert the correct length of the assigned polynomails
    assert_eq!(a_assigned.len(), b_assigned.len());

    // Enforce that a_assigned[i] * b_assigned[i] = sum_assigned[i]
    let gate = GateChip::<F>::default();
    let sum_assigned: Vec<AssignedValue<F>> = a_assigned
        .iter()
        .zip(b_assigned.iter())
        .take(2 * N - 1)
        .map(|(&a, &b)| gate.add(ctx, a, b))
        .collect();

    for i in 0..(N + 1) {
        make_public.push(sum_assigned[i]);
    }

    // TEST
    // Perform the addition of the polynomials outside the circuit (using arkworks) to see if this matches the result of the circuit
    let a = DensePolynomial::<Fr>::from_coefficients_vec(
        input.a.iter().map(|x| Fr::from(*x as u64)).collect::<Vec<Fr>>(),
    );

    let b = DensePolynomial::<Fr>::from_coefficients_vec(
        input.b.iter().map(|x| Fr::from(*x as u64)).collect::<Vec<Fr>>(),
    );

    let c: DensePolynomial<Fr> = &a + &b;

    // Turn coefficients to string
    let c_coeffs = c.coeffs.iter().map(|x| x.into_bigint().to_string()).collect::<Vec<String>>();

    // iter over the c coefficients and turn it into F
    let c_f = c_coeffs.iter().map(|x| F::from_str_vartime(x).unwrap()).collect::<Vec<F>>();

    // Compare the result of the circuit with the result of the addition
    for (sum, c) in sum_assigned.iter().zip(c_f) {
        assert_eq!(sum.value(), &c);
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(poly_add, args);
}
