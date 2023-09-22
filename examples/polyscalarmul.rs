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
// - The input polynomial and the scalar are not made public
// - Suppose that range check is performed on the coeffiicients in order to avoid overflow for happen during the addition

const N: usize = 3;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput<const N: usize> {
    pub a: Vec<u8>, // polynomial coefficients little endian of degree N
    pub k: u8,      // scalar
}

// this algorithm takes a polynomial a and a scalar k output their product to the public
fn poly_scalar_mul<F: ScalarField>(
    ctx: &mut Context<F>,
    input: CircuitInput<N>,
    make_public: &mut Vec<AssignedValue<F>>,
) {
    // Assign the polynomial a and the scalar k to the circuit
    let a_assigned: Vec<AssignedValue<F>> = input
        .a
        .iter()
        .map(|x| {
            let result = F::from(*x as u64);
            ctx.load_witness(result)
        })
        .collect();

    let k_assigned = ctx.load_witness(F::from(input.k as u64));

    // Enforce that a_assigned[i] * k = scalar_prod_assigned[i]
    let gate = GateChip::<F>::default();
    let mut scalar_prod_assigned = Vec::new();
    for i in 0..(N - 1) {
        let scalar_prod = gate.mul(ctx, a_assigned[i], k_assigned);
        scalar_prod_assigned.push(scalar_prod);
        make_public.push(scalar_prod);
    }

    // TEST
    // Perform the scalar multiplcation outside the circuit (using arkworks) to see if this matches the result of the circuit
    let a = DensePolynomial::<Fr>::from_coefficients_vec(
        input.a.iter().map(|x| Fr::from(*x as u64)).collect::<Vec<Fr>>(),
    );

    let k = Fr::from(input.k as u64);

    let c: DensePolynomial<Fr> = &a * k;
    // Turn coefficients to string
    let c_coeffs = c.coeffs.iter().map(|x| x.into_bigint().to_string()).collect::<Vec<String>>();

    // iter over the c coefficients and turn it into F
    let c_f = c_coeffs.iter().map(|x| F::from_str_vartime(x).unwrap()).collect::<Vec<F>>();

    // Compare the result of the circuit with the result of the multiplication
    for (prod, c) in scalar_prod_assigned.iter().zip(c_f) {
        assert_eq!(prod.value(), &c);
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    // run different zk commands based on the command line arguments
    run(poly_scalar_mul, args);
}
