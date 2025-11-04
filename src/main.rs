use easy_smt::{ContextBuilder, Response};

fn init_yices2_ctx() -> easy_smt::Context {
    ContextBuilder::new()
        .solver("yices-smt2")
        .solver_args(["--incremental"])
        .build()
        .expect("Failed to create SMT context with Yices2")
}

fn main() {
    let mut ctx = init_yices2_ctx();

    // Set the logic to support integer arithmetic
    ctx.set_logic("QF_LIA").expect("Failed to set logic");

    // Declare integer variables
    let int_sort = ctx.int_sort();
    let x = ctx.declare_const("x", int_sort).unwrap();
    let y = ctx.declare_const("y", int_sort).unwrap();

    // Build constraints: x > 0 and y > 0 and x + y = 10
    let zero = ctx.numeral(0);
    let ten = ctx.numeral(10);

    let x_gt_0 = ctx.gt(x, zero);
    let y_gt_0 = ctx.gt(y, zero);
    let x_plus_y = ctx.plus(x, y);
    let sum_eq_10 = ctx.eq(x_plus_y, ten);

    // Add constraints
    ctx.assert(x_gt_0).unwrap();
    ctx.assert(y_gt_0).unwrap();
    ctx.assert(sum_eq_10).unwrap();

    println!("Constraints added:");
    println!("  - x > 0");
    println!("  - y > 0");
    println!("  - x + y = 10");
    println!("\nChecking satisfiability...");

    // Check satisfiability
    match ctx.check() {
        Ok(Response::Sat) => {
            println!("Result: SAT (satisfiable)\n");

            // Get model values
            let values = ctx.get_value(vec![x, y]).unwrap();
            for (var, val) in values {
                println!("{} = {}", ctx.display(var), ctx.display(val));
            }
        }
        Ok(Response::Unsat) => {
            println!("Result: UNSAT (unsatisfiable)");
        }
        Ok(Response::Unknown) => {
            println!("Result: UNKNOWN");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    println!("\nYices2 integration successful!");
}
