extern crate gurobi;
extern crate itertools;

use std::iter::repeat;
use gurobi::*;
use gurobi::model::Status;
use itertools::*;

fn main() {
  // Set of worker's names
  let workers = vec!["Amy", "Bob", "Cathy", "Dan", "Ed", "Fred", "Gu"];

  // Amount each worker is paid to to work per shift
  let pays = vec![10.0, 12.0, 10.0, 8.0, 8.0, 9.0, 11.0];

  // Set of shift labels
  let shifts = vec!["Mon1", "Tue2", "Wed3", "Thu4", "Fri5", "Sat6", "Sun7", "Mon8", "Tue9", "Wed10", "Thu11", "Fri12",
                    "Sat13", "Sun14"];

  // Number of workers required for each shift
  let shift_requirements = vec![3.0, 2.0, 4.0, 4.0, 5.0, 6.0, 5.0, 2.0, 2.0, 3.0, 4.0, 6.0, 7.0, 5.0];

  // Worker availability: 0 if the worker is unavailable for a shift
  let availability = vec![
     vec![ 0, 1, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1 ],
     vec![ 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 0 ],
     vec![ 0, 0, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1 ],
     vec![ 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1 ],
     vec![ 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1 ],
     vec![ 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 1 ],
     vec![ 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1 ],
   ];

  let mut env = Env::new("workforce2.log").unwrap();
  env.set(param::LogToConsole, 0).unwrap();

  let mut model = env.new_model("assignment").unwrap();

  let mut x = Vec::new();
  for (worker, availability) in Zip::new((workers.iter(), availability.iter())) {

    let mut xshift = Vec::new();
    for (shift, &availability) in Zip::new((shifts.iter(), availability.iter())) {
      let vname = format!("{}.{}", worker, shift);
      let v = model.add_var(vname.as_str(), Continuous(-INFINITY, availability as f64)).unwrap();
      xshift.push(v);
    }

    x.push(xshift);
  }
  model.update().unwrap();

  let objterm = pays.iter().map(|pay| repeat(pay).take(shifts.len()));

  let objexpr = Zip::new((x.iter().flatten(), objterm.flatten())).fold(LinExpr::new(),
                                                                       |expr, (ref x, &c)| expr.term((*x).clone(), c));
  model.set_objective(objexpr, Minimize).unwrap();

  for (s, (shift, &requirement)) in shifts.iter().zip(shift_requirements.iter()).enumerate() {
    model.add_constr(format!("c.{}", shift).as_str(),
                  x.iter().map(|ref x| x[s].clone()).fold(LinExpr::new(), |expr, x| expr.term(x, 1.0)),
                  Equal,
                  requirement)
      .unwrap();
  }

  model.write("assignment.lp").unwrap();

  let mut removed = Vec::new();
  for loop_count in 0..100 {
    println!("[iteration {}]", loop_count);

    model.optimize().unwrap();

    match model.status().unwrap() {
      Status::Optimal => break,

      Status::Infeasible => {
        // compute IIS.
        model.compute_iis().unwrap();
        model.write(&format!("assignment_{}.ilp", loop_count)).unwrap();

        let c = {
          let iis_constrs = model.get_constrs().filter(|c| c.get(&model, attr::IISConstr).unwrap() != 0).collect_vec();
          println!("number of IIS constrs = {}", iis_constrs.len());
          iis_constrs.into_iter().nth(0).cloned()
        };

        match c {
          Some(c) => {
            let cname = c.get(&model, attr::ConstrName).unwrap();
            model.remove_constr(c).unwrap();
            removed.push(cname);
          }
          None => {
            println!("There are any IIS constraints in the model.");
            break;
          }
        }
      }

      Status::InfOrUnbd | Status::Unbounded => {
        println!("The model is unbounded.");
        return;
      }

      status => {
        println!("Optimization is stopped with status {:?}", status);
        return;
      }
    }
  }

  println!("removed variables are: {:?}", removed);
}