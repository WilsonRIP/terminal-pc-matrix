use colored::*;
use meval;
use std::error::Error;

type BoxedError = Box<dyn Error + Send + Sync>;

/// Evaluates a mathematical expression string.
pub fn evaluate_expression(expr: &str) -> Result<f64, BoxedError> {
    match meval::eval_str(expr) {
        Ok(result) => {
            println!("{} {}", "=".green(), result.to_string().bold());
            Ok(result)
        }
        Err(e) => {
            let err_msg = format!("Invalid expression: {}", e);
            eprintln!("{}", err_msg.red());
            Err(err_msg.into())
        }
    }
}
