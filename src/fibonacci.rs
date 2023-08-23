use halo2_proofs::{
    circuit::{Region, Value},
    halo2curves::ff::PrimeField,
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct Fibonacci<F> {
    q_enable: Selector,
    left: Column<Advice>,
    right: Column<Advice>,
    _marker: PhantomData<F>,
}
impl<F: PrimeField> Fibonacci<F> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let left = meta.advice_column();
        let right = meta.advice_column();
        let q_enable = meta.selector();

        meta.enable_equality(left);
        meta.enable_equality(right);

        meta.create_gate("fib", |meta| {
            let q_enable = meta.query_selector(q_enable);
            let left = meta.query_advice(left, Rotation::cur());
            let right_cur = meta.query_advice(right, Rotation::cur());
            let right_next = meta.query_advice(right, Rotation::next());
            vec![q_enable * (left + right_cur - right_next)]
        });
        Self {
            q_enable,
            left,
            right,
            _marker: PhantomData,
        }
    }

    pub fn assign(
        &self,
        region: &mut Region<'_, F>,
        init_left: Value<F>,
        init_right: Value<F>,
    ) -> Result<(), Error> {
        let offset = 0;
        self.q_enable.enable(region, offset)?;
        let mut left = region.assign_advice(|| "init_left", self.left, offset, || init_left)?;
        let mut right = region.assign_advice(|| "init_right", self.right, offset, || init_right)?;

        for offset in 0..4 {
            self.q_enable.enable(region, offset)?;
            let right_next = left
                .value()
                .zip(right.value())
                .map(|(left, right)| left.clone() + right);
            right = region.assign_advice(|| "right_next", self.right, offset + 1, || right_next)?;
            left = right.copy_advice(|| "left_next", region, self.left, offset + 1)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{
        circuit::{Layouter, SimpleFloorPlanner, Value},
        dev::MockProver,
        halo2curves::bn256::Fr,
        plonk::{Circuit, ConstraintSystem, Error},
    };

    #[test]
    fn test_fib_example() {
        #[derive(Default)]
        struct MyCircuit<F> {
            init_left: Value<F>,
            init_right: Value<F>,
        }
        impl<F: PrimeField> Circuit<F> for MyCircuit<F> {
            type Config = Fibonacci<F>;
            type FloorPlanner = SimpleFloorPlanner;

            fn without_witnesses(&self) -> Self {
                Self::default()
            }

            fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
                Fibonacci::configure(meta)
            }

            fn synthesize(
                &self,
                config: Self::Config,
                mut layouter: impl Layouter<F>,
            ) -> Result<(), Error> {
                layouter.assign_region(
                    || "assign x",
                    |mut region| config.assign(&mut region, self.init_left, self.init_right),
                )?;
                Ok(())
            }
        }
        let k = 4;
        let circuit = MyCircuit::<Fr> {
            init_left: Value::known(Fr::from(1)),
            init_right: Value::known(Fr::from(1)),
        };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied()
    }
}
