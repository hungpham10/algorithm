use super::{Rule, RuleError, Variables};

use parquet::arrow::arrow_reader::ParquetRecordBatchReader;
use std::collections::HashMap;

#[macro_export]
macro_rules! expr {
    ( $op:expr, $( $pin:expr ),* ) => {
        Expression {
            operator: $op.to_string(),
            pins: vec![
                $(
                    $pin,
                )*
            ],
        }
    };
}

#[macro_export]
macro_rules! input {
    ( $name:expr ) => {
        Pin {
            name: $name.to_string(),
            value: Some(0.0),
            nested: None,
            threshold: None,
        }
    };
}

#[macro_export]
macro_rules! nested {
    ( $name:expr, $inner:expr, $( $value:expr ),* ) => {

        Pin {
            name: $name.to_string(),
            value: None,
            nested: Some(Expression {
                operator: $inner.to_string(),
                pins: vec![
                    $(
                        $value,
                    )*
                ],
            }),
            threshold: None,
        }
    };
}

macro_rules! threshold {
    ( $name:expr, $value:expr ) => {
        Pin {
            name: $name.to_string(),
            value: None,
            nested: None,
            threshold: Some($value),
        }
    };
}

pub async fn replay(
    reader: &mut ParquetRecordBatchReader,
    num_of_rows: i64,
    scope: &String,
    rule: &Rule,
) -> Result<Variables, RuleError> {
    let mut vars = Variables::new(num_of_rows as usize, num_of_rows as usize);

    for item in reader {
        let mut arrays = Vec::new();
        let batch = item.map_err(|e| RuleError {
            message: format!("Iterate failed: {}", e),
        })?;
        let schema = batch.schema();
        let labels = rule.inputs();

        for (i, field) in schema.fields().into_iter().enumerate() {
            arrays.push(
                batch
                    .column(i)
                    .as_any()
                    .downcast_ref::<arrow::array::Float64Array>()
                    .unwrap(),
            );

            vars.create(&field.name())?;
        }

        for j in 0..num_of_rows {
            let mut inputs = HashMap::new();

            for (i, field) in schema.fields().into_iter().enumerate() {
                vars.update(&scope, &field.name(), arrays[i].value(j as usize))
                    .await?;
            }

            for label in &labels {
                inputs.insert(
                    label.to_string(),
                    vars.get_by_selected_expr(label).map_err(|e| RuleError {
                        message: format!("Failed to get variable {}: {}", label, e),
                    })?,
                );
            }

            if rule.reload(&inputs) == inputs.len() {
                rule.evaluate().map_err(|e| RuleError {
                    message: format!("Failed to evaluate rule: {}", e),
                })?;
            } else {
                return Err(RuleError {
                    message: format!("Failed to reload data"),
                });
            }
        }
    }
    Ok(vars)
}
