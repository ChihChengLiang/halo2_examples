fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{
        arithmetic::FieldExt,
        circuit::{Layouter, Region, SimpleFloorPlanner, Value},
        dev::MockProver,
        halo2curves::bn256::Fr,
        plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Selector, TableColumn},
        poly::Rotation,
    };
    use std::marker::PhantomData;

    #[test]
    fn test_lookup_example() {
        #[derive(Clone)]
        struct LookupA<F> {
            q_enable: Selector,
            x: Column<Advice>,
            y: Column<Advice>,
            table_col: TableColumn,
            _marker: PhantomData<F>,
        }
        impl<F: FieldExt> LookupA<F> {
            fn configure(meta: &mut ConstraintSystem<F>) -> Self {
                let x = meta.advice_column();
                let y = meta.advice_column();
                let q_enable = meta.complex_selector();
                let table_col = meta.lookup_table_column();

                meta.lookup("x and y", |meta| {
                    let q_enable = meta.query_selector(q_enable);
                    let x = meta.query_advice(x, Rotation::cur());
                    let y = meta.query_advice(y, Rotation::cur());
                    vec![(q_enable.clone() * x, table_col), (q_enable * y, table_col)]
                });
                Self {
                    q_enable,
                    x,
                    y,
                    table_col,
                    _marker: PhantomData,
                }
            }

            fn assign(
                &self,
                region: &mut Region<'_, F>,
                x: Value<F>,
                y: Value<F>,
            ) -> Result<(), Error> {
                // I think we have 6 unusable rows
                for offset in 0..10 {
                    self.q_enable.enable(region, offset)?;
                    region.assign_advice(|| "x", self.x, offset, || x)?;
                    region.assign_advice(|| "y", self.y, offset, || y)?;
                }
                Ok(())
            }
            fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
                layouter.assign_table(
                    || "table",
                    |mut table| {
                        for (offset, &value) in [1u64, 2, 3].iter().enumerate() {
                            table.assign_cell(
                                || "table cell",
                                self.table_col,
                                offset,
                                || Value::known(F::from(value)),
                            )?;
                        }
                        Ok(())
                    },
                )
            }
        }

        #[derive(Default)]
        struct MyCircuit<F> {
            x: Value<F>,
            y: Value<F>,
        }
        impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
            type Config = LookupA<F>;
            type FloorPlanner = SimpleFloorPlanner;

            fn without_witnesses(&self) -> Self {
                Self::default()
            }

            fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
                LookupA::configure(meta)
            }

            fn synthesize(
                &self,
                config: Self::Config,
                mut layouter: impl Layouter<F>,
            ) -> Result<(), Error> {
                config.load(&mut layouter)?;
                layouter.assign_region(
                    || "assign x",
                    |mut region| config.assign(&mut region, self.x, self.y),
                )?;
                Ok(())
            }
        }

        fn test_circuit(x: u64, y: u64, success: bool) {
            let k = 4;
            let circuit = MyCircuit::<Fr> {
                x: Value::known(Fr::from(x)),
                y: Value::known(Fr::from(y)),
            };
            let prover = MockProver::run(k, &circuit, vec![]).unwrap();

            let result = prover.verify();
            if success {
                assert!(result.is_ok())
            } else {
                assert!(result.is_err())
            }
        }

        test_circuit(3 , 4, false);
        test_circuit(3 , 3, true);
        test_circuit(2 , 2, true);
    }
}
