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
            table_col: TableColumn,
            _marker: PhantomData<F>,
        }
        impl<F: FieldExt> LookupA<F> {
            fn configure(meta: &mut ConstraintSystem<F>) -> Self {
                let x = meta.advice_column();
                let q_enable = meta.complex_selector();
                let table_col = meta.lookup_table_column();

                meta.lookup("x", |meta| {
                    let q_enable = meta.query_selector(q_enable);
                    let x = meta.query_advice(x, Rotation::cur());
                    vec![(q_enable * x, table_col)]
                });
                Self {
                    q_enable,
                    x,
                    table_col,
                    _marker: PhantomData,
                }
            }

            fn assign(&self, region: &mut Region<'_, F>, x: Value<F>) -> Result<(), Error> {
                // I think we have 6 unusable rows
                for offset in 0..10 {
                    self.q_enable.enable(region, offset)?;
                    region.assign_advice(|| "x", self.x, offset, || x)?;
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
                    |mut region| config.assign(&mut region, self.x),
                )?;
                Ok(())
            }
        }

        let k = 4;
        let circuit = MyCircuit::<Fr> {
            x: Value::known(Fr::from(3)),
        };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();

    }
}
