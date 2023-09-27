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

// Notes:
// - The input polynomials are not made public
// - Suppose that range check is performed on the coeffiicients in order to avoid overflow for happen during the multiplication

// Complexity of the algorithm
// The algorithm involves two nested loops: the outer loop runs for "2N+1" iterations and the inner loop runs for up to "N+1" iterations in the worst case.
// The operations inside the inner loop are additions and multiplications in the field F which are O(1) operations.
// Therefore, the complexity of the algorithm is O((2N+1)*(N+1)*1) = O(N^2)

const N: usize = 3;

// The polynomial multiplication is performed using the direct method.
// Given two polynomials a and b of degree n, the product c = a * b is a polynomial of degree 2n
// The coefficients of c are computed as dot products of the coefficients of a and b
// The coefficients of c are made public
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput<const N: usize> {
    pub a: Vec<u8>, // polynomial coefficients little endian of degree n (first element = constant term)
    pub b: Vec<u8>, // polynomial coefficients little endian of degree n (first element = constant term)
}

// this algorithm takes two polynomials a and b of the same degree and output their product to the public
pub fn poly_mul<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput<N>,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // assert that the input polynomials have the same degree
    assert_eq!(input.a.len() - 1, input.b.len() - 1);
    // assert that degree is equal to the constant N
    assert_eq!(input.a.len() - 1, N);

    // Create a gate chip
    let gate = GateChip::<F>::default();

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

    // Build the product of the polynomials as dot products of the coefficients of a and b
    let mut prod_val: Vec<AssignedValue<F>> = vec![];
    for i in 0..(2 * N + 1) {
        let mut coefficient_accumaltor: Vec<AssignedValue<F>> = vec![];

        if i < N + 1 {
            for a_idx in 0..=i {
                let a = a_assigned[a_idx];
                let b = b_assigned[i - a_idx];
                // push the product of a and b to the coefficient_accumaltor
                coefficient_accumaltor.push(gate.mul(ctx, a, b));
            }
        } else {
            for a_idx in (i - N)..=N {
                let a = a_assigned[a_idx];
                let b = b_assigned[i - a_idx];
                // push the product of a and b to the coefficient_accumaltor
                coefficient_accumaltor.push(gate.mul(ctx, a, b));
            }
        }

        let prod_value = coefficient_accumaltor
            .iter()
            .fold(ctx.load_witness(F::zero()), |acc, x| gate.add(ctx, acc, *x));

        prod_val.push(prod_value);
    }

    // Make the coefficients of the product public. The coefficients are in little endian order
    for i in 0..(2 * N + 1) {
        make_public.push(prod_val[i]);
    }

    // TEST
    // Perform the multiplication of the polynomials outside the circuit (using arkworks) to see if this matches the result of the circuit
    let a = DensePolynomial::<Fr>::from_coefficients_vec(
        input.a.iter().map(|x| Fr::from(*x as u64)).collect::<Vec<Fr>>(),
    );

    let b = DensePolynomial::<Fr>::from_coefficients_vec(
        input.b.iter().map(|x| Fr::from(*x as u64)).collect::<Vec<Fr>>(),
    );

    let c: DensePolynomial<Fr> = &a * &b;

    // Turn coefficients to string
    let c_coeffs = c.coeffs.iter().map(|x| x.into_bigint().to_string()).collect::<Vec<String>>();

    // iter over the c coefficients and turn it into F
    let c_f = c_coeffs.iter().map(|x| F::from_str_vartime(x).unwrap()).collect::<Vec<F>>();

    // Compare the result of the circuit with the result of the multiplication
    for (prod, c) in prod_val.iter().zip(c_f) {
        assert_eq!(prod.value(), &c);
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(poly_mul, args);
}
