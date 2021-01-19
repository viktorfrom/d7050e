use core::panic;

use crate::ast::*;
use crate::memory::*;

pub fn interpreter(tree: Vec<Expr>) -> ExprRep {
    let mut res = ExprRep::Null;
    for expr in tree.iter() {
        res = eval_expr(expr.clone());
        match res {
            _ => continue,
        }
    }
    return res;
}

fn eval_expr(expr: Expr) -> ExprRep {
    match expr {
        Expr::Int(i) => ExprRep::Int(i),
        Expr::Bool(b) => ExprRep::Bool(b),
        Expr::Var(n) => read_var(&n),

        Expr::BinExpr(l, op, r) => eval_bin_expr(*l, op, *r),
        Expr::VarExpr(var, op, expr) => eval_var_expr(*var, op, *expr),

        Expr::Let(var, var_type, expr) => eval_let(*var, var_type, *expr),

        Expr::If(cond, block) => eval_if(*cond, block),
        Expr::IfElse(cond, block1, block2) => eval_if_else(*cond, block1, block2),
        Expr::While(cond, block) => eval_while(*cond, block),

        Expr::Fn(fn_var, params, ret_type, block) => eval_fn(*fn_var, params, ret_type, block),
        Expr::FnCall(fn_var, args) => eval_fn_call(*fn_var, args),
        Expr::Return(expr) => eval_return(*expr),
    }
}

fn eval_fn(fn_var: Expr, params: Vec<(Expr, Type)>, ret_type: Type, block: Vec<Expr>) -> ExprRep {
    match fn_var {
        Expr::Var(a) => {
            insert_fn(
                ExprRep::Var(a.to_string()),
                ExprRep::Fn(params, ret_type, block),
            );
        }
        _ => panic!("Fn stmt fail!"),
    }
    return ExprRep::Null;
}

fn eval_fn_call(fn_var: Expr, args: Vec<Expr>) -> ExprRep {
    match fn_var {
        Expr::Var(fn_var) => match read_fn(&fn_var) {
            ExprRep::Fn(params, ret_type, block) => {
                if params.len() != args.clone().len() {
                    panic!("params != args")
                }

                for i in 0..params.len() {
                    for x in params.clone() {
                        match &x {
                            (Expr::Var(v), t) => {
                                let eval_arg = eval_expr(args[i].clone());
                                match (t, eval_arg.clone()) {
                                    (Type::Int, ExprRep::Int(_)) => {
                                        insert_var(ExprRep::Var(v.to_string()), eval_arg)
                                    }
                                    (Type::Bool, ExprRep::Bool(_)) => {
                                        insert_var(ExprRep::Var(v.to_string()), eval_arg)
                                    }
                                    _ => panic!("Return type does not match!"),
                                };
                            }
                            _ => panic!("Invalid param var!"),
                        }
                    }
                }

                let res = interpreter(block);
                match (ret_type, res.clone()) {
                    (Type::Int, ExprRep::Int(_)) => res,
                    (Type::Bool, ExprRep::Bool(_)) => res,
                    _ => panic!("Return type does not match!"),
                }
            }
            _ => panic!("Could not find fn_var in map!"),
        },
        _ => panic!("Invalid fn_var!"),
    }
}

fn eval_if(cond: Expr, block: Vec<Expr>) -> ExprRep {
    match eval_expr(cond) {
        ExprRep::Bool(c) => {
            if c {
                return interpreter(block);
            }
            return ExprRep::Null;
        }
        _ => panic!("If stmt fail!"),
    }
}

fn eval_if_else(cond: Expr, block1: Vec<Expr>, block2: Vec<Expr>) -> ExprRep {
    match eval_expr(cond) {
        ExprRep::Bool(c) => {
            if c {
                return interpreter(block1);
            } else {
                return interpreter(block2);
            }
        }
        _ => panic!("IfElse stmt fail!"),
    }
}

fn eval_while(cond: Expr, block: Vec<Expr>) -> ExprRep {
    match eval_expr(cond) {
        ExprRep::Bool(c) => {
            if c {
                return interpreter(block);
            }
            return ExprRep::Null;
        }
        _ => panic!("While stmt fail!"),
    }
}

fn eval_let(var: Expr, _var_type: Type, expr: Expr) -> ExprRep {
    match (var, eval_expr(expr)) {
        (Expr::Var(n), ExprRep::Int(val)) => insert_var(ExprRep::Var(n), ExprRep::Int(val)),
        (Expr::Var(n), ExprRep::Bool(val)) => insert_var(ExprRep::Var(n), ExprRep::Bool(val)),
        _ => panic!("Invalid let expr!"),
    }
}

fn eval_return(expr: Expr) -> ExprRep {
    return eval_expr(expr);
}

fn eval_bin_expr(l: Expr, op: Op, r: Expr) -> ExprRep {
    match (l, r.clone()) {
        (Expr::Int(left), Expr::Int(right)) => eval_int_expr(left, op, right),
        (Expr::Int(v), Expr::BinExpr(_, _, _)) => match eval_expr(r) {
            ExprRep::Int(r) => eval_int_expr(v, op, r),
            _ => ExprRep::Null,
        },
        (Expr::Var(v), Expr::Int(right)) => match read_var(&v) {
            ExprRep::Int(val) => eval_int_expr(val, op, right),
            _ => ExprRep::Int(right),
        },
        (Expr::Bool(left), Expr::Bool(right)) => eval_bool_expr(left, op, right),
        (Expr::Bool(v), Expr::BinExpr(_, _, _)) => match eval_expr(r) {
            ExprRep::Bool(r) => eval_bool_expr(v, op, r),
            _ => ExprRep::Null,
        },
        (Expr::Var(v), Expr::Bool(right)) => match read_var(&v) {
            ExprRep::Bool(val) => eval_bool_expr(val, op, right),
            _ => ExprRep::Bool(right),
        },
        (Expr::Var(_), Expr::BinExpr(_, _, _)) => eval_expr(r),

        _ => panic!("Invalid bin expr!"),
    }
}

/// Updates existing value in memory
fn eval_var_expr(var: Expr, op: Op, expr: Expr) -> ExprRep {
    match op {
        Op::AriOp(op) => var_ari_op(var, op, expr),
        Op::AssOp(op) => var_ass_op(var, op, expr),
        Op::LogOp(op) => var_log_op(var, op, expr),
        Op::RelOp(op) => var_rel_op(var, op, expr),
    }
}

fn eval_int_expr(l: i32, op: Op, r: i32) -> ExprRep {
    match op {
        Op::AriOp(AriOp::Add) => ExprRep::Int(l + r),
        Op::AriOp(AriOp::Sub) => ExprRep::Int(l - r),
        Op::AriOp(AriOp::Div) => ExprRep::Int(l / r),
        Op::AriOp(AriOp::Mul) => ExprRep::Int(l * r),
        Op::RelOp(RelOp::Eq) => ExprRep::Bool(l == r),
        Op::RelOp(RelOp::Neq) => ExprRep::Bool(l != r),
        Op::RelOp(RelOp::Leq) => ExprRep::Bool(l <= r),
        Op::RelOp(RelOp::Geq) => ExprRep::Bool(l >= r),
        Op::RelOp(RelOp::Les) => ExprRep::Bool(l < r),
        Op::RelOp(RelOp::Gre) => ExprRep::Bool(l > r),
        _ => panic!("Invalid Int expr!"),
    }
}
fn eval_bool_expr(l: bool, op: Op, r: bool) -> ExprRep {
    match op {
        Op::LogOp(LogOp::And) => ExprRep::Bool(l && r),
        Op::LogOp(LogOp::Or) => ExprRep::Bool(l || r),
        Op::RelOp(RelOp::Eq) => ExprRep::Bool(l == r),
        Op::RelOp(RelOp::Neq) => ExprRep::Bool(l != r),
        Op::RelOp(RelOp::Leq) => ExprRep::Bool(l <= r),
        Op::RelOp(RelOp::Geq) => ExprRep::Bool(l >= r),
        Op::RelOp(RelOp::Les) => ExprRep::Bool(l < r),
        Op::RelOp(RelOp::Gre) => ExprRep::Bool(l > r),
        _ => panic!("Invalid Bool expr!"),
    }
}

fn var_ari_op(var: Expr, op: AriOp, expr: Expr) -> ExprRep {
    match op {
        AriOp::Add => match (var, expr) {
            (Expr::Var(v), Expr::Var(expr)) => match (read_var(&v), read_var(&expr)) {
                (ExprRep::Int(v1), ExprRep::Int(v2)) => ExprRep::Int(v1 + v2),
                _ => ExprRep::Null,
            },
            (Expr::Var(v), Expr::Int(expr)) => match read_var(&v) {
                ExprRep::Int(v1) => ExprRep::Int(v1 + expr),
                _ => ExprRep::Null,
            },
            _ => panic!("Var Add fail!"),
        },
        AriOp::Sub => match (var, expr) {
            (Expr::Var(v), Expr::Var(expr)) => match (read_var(&v), read_var(&expr)) {
                (ExprRep::Int(v1), ExprRep::Int(v2)) => ExprRep::Int(v1 - v2),
                _ => ExprRep::Null,
            },
            (Expr::Var(v), Expr::Int(expr)) => match read_var(&v) {
                ExprRep::Int(v1) => ExprRep::Int(v1 - expr),
                _ => ExprRep::Null,
            },
            _ => panic!("Var Sub fail!"),
        },
        AriOp::Div => match (var, expr) {
            (Expr::Var(v), Expr::Var(expr)) => match (read_var(&v), read_var(&expr)) {
                (ExprRep::Int(v1), ExprRep::Int(v2)) => ExprRep::Int(v1 / v2),
                _ => ExprRep::Null,
            },
            (Expr::Var(v), Expr::Int(expr)) => match read_var(&v) {
                ExprRep::Int(v1) => ExprRep::Int(v1 / expr),
                _ => ExprRep::Null,
            },
            _ => panic!("Var Div fail!"),
        },
        AriOp::Mul => match (var, expr) {
            (Expr::Var(v), Expr::Var(expr)) => match (read_var(&v), read_var(&expr)) {
                (ExprRep::Int(v1), ExprRep::Int(v2)) => ExprRep::Int(v1 * v2),
                _ => ExprRep::Null,
            },
            (Expr::Var(v), Expr::Int(expr)) => match read_var(&v) {
                ExprRep::Int(v1) => ExprRep::Int(v1 * expr),
                _ => ExprRep::Null,
            },
            _ => panic!("Var Mul fail!"),
        },
    }
}

fn var_ass_op(var: Expr, ass_op: AssOp, expr: Expr) -> ExprRep {
    match ass_op {
        AssOp::Eq => match (eval_expr(var), eval_expr(expr)) {
            (ExprRep::Var(v), ExprRep::Int(val)) => insert_var(ExprRep::Var(v), ExprRep::Int(val)),
            (ExprRep::Var(v), ExprRep::Bool(val)) => {
                insert_var(ExprRep::Var(v), ExprRep::Bool(val))
            }
            _ => panic!("Var insert fail!"),
        },
        AssOp::AddEq => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(v), ExprRep::Int(old_val), ExprRep::Int(new_val)) => {
                insert_var(ExprRep::Var(v.clone()), ExprRep::Int(old_val + new_val))
            }
            _ => panic!("Var Add update fail!"),
        },
        AssOp::SubEq => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(v), ExprRep::Int(old_val), ExprRep::Int(new_val)) => {
                insert_var(ExprRep::Var(v), ExprRep::Int(old_val - new_val))
            }
            _ => panic!("Var Sub update fail!"),
        },
        AssOp::DivEq => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(v), ExprRep::Int(old_val), ExprRep::Int(new_val)) => {
                insert_var(ExprRep::Var(v), ExprRep::Int(old_val - new_val))
            }
            _ => panic!("Var Sub update fail!"),
        },
        AssOp::MulEq => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(v), ExprRep::Int(old_val), ExprRep::Int(new_val)) => {
                insert_var(ExprRep::Var(v), ExprRep::Int(old_val - new_val))
            }
            _ => panic!("Var Sub update fail!"),
        },
    }
}

fn var_log_op(var: Expr, log_op: LogOp, expr: Expr) -> ExprRep {
    match log_op {
        LogOp::And => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(v), ExprRep::Bool(old_val), ExprRep::Bool(new_val)) => {
                insert_var(ExprRep::Var(v), ExprRep::Bool(old_val && new_val))
            }
            _ => panic!("Var And update fail!"),
        },
        LogOp::Or => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(v), ExprRep::Bool(old_val), ExprRep::Bool(new_val)) => {
                insert_var(ExprRep::Var(v), ExprRep::Bool(old_val || new_val))
            }
            _ => panic!("Var Or update fail!"),
        },
    }
}

fn var_rel_op(var: Expr, rel_op: RelOp, expr: Expr) -> ExprRep {
    match rel_op {
        RelOp::Eq => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(_), ExprRep::Bool(b1), ExprRep::Bool(b2)) => {
                if b1 == b2 {
                    return ExprRep::Bool(true);
                } else {
                    return ExprRep::Bool(false);
                }
            }
            _ => ExprRep::Null,
        },
        RelOp::Neq => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(_), ExprRep::Bool(b1), ExprRep::Bool(b2)) => {
                if b1 != b2 {
                    return ExprRep::Bool(true);
                } else {
                    return ExprRep::Bool(false);
                }
            }
            _ => ExprRep::Null,
        },
        RelOp::Leq => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(_), ExprRep::Int(v1), ExprRep::Int(v2)) => {
                if v1 <= v2 {
                    return ExprRep::Bool(true);
                } else {
                    return ExprRep::Bool(false);
                }
            }
            _ => ExprRep::Null,
        },
        RelOp::Geq => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(_), ExprRep::Int(v1), ExprRep::Int(v2)) => {
                if v1 >= v2 {
                    return ExprRep::Bool(true);
                } else {
                    return ExprRep::Bool(false);
                }
            }
            _ => ExprRep::Null,
        },
        RelOp::Les => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(_), ExprRep::Int(v1), ExprRep::Int(v2)) => {
                if v1 < v2 {
                    return ExprRep::Bool(true);
                } else {
                    return ExprRep::Bool(false);
                }
            }
            _ => ExprRep::Null,
        },
        RelOp::Gre => match (var.clone(), eval_expr(var), eval_expr(expr)) {
            (Expr::Var(_), ExprRep::Int(v1), ExprRep::Int(v2)) => {
                if v1 > v2 {
                    return ExprRep::Bool(true);
                } else {
                    return ExprRep::Bool(false);
                }
            }
            _ => ExprRep::Null,
        },
    }
}

#[cfg(test)]
mod interpreter_tests {
    use super::*;

    #[test]
    fn test_eval_int() {
        assert_eq!(interpreter(vec![Expr::Int(1)]), ExprRep::Int(1));
    }

    #[test]
    fn test_eval_bool() {
        assert_eq!(interpreter(vec![Expr::Bool(true)]), ExprRep::Bool(true));
        assert_eq!(interpreter(vec![Expr::Bool(false)]), ExprRep::Bool(false));
        assert_ne!(interpreter(vec![Expr::Bool(false)]), ExprRep::Bool(true));
    }

    #[test]
    fn test_eval_var() {
        insert_var(ExprRep::Var("a1".to_string()), ExprRep::Int(1));
        assert_eq!(
            interpreter(vec![Expr::Var("a1".to_string())]),
            ExprRep::Int(1)
        );
        insert_var(ExprRep::Var("a2".to_string()), ExprRep::Bool(true));
        assert_eq!(
            interpreter(vec![Expr::Var("a2".to_string())]),
            ExprRep::Bool(true)
        );
        insert_var(
            ExprRep::Var("a3".to_string()),
            ExprRep::Var("a4".to_string()),
        );
        assert_eq!(
            interpreter(vec![Expr::Var("a3".to_string())]),
            ExprRep::Var("a4".to_string())
        );
    }

    #[test]
    fn test_eval_int_expr() {
        assert_eq!(eval_int_expr(1, Op::AriOp(AriOp::Add), 2), ExprRep::Int(3));
        assert_eq!(eval_int_expr(3, Op::AriOp(AriOp::Sub), 2), ExprRep::Int(1));
        assert_eq!(eval_int_expr(10, Op::AriOp(AriOp::Div), 2), ExprRep::Int(5));
        assert_eq!(eval_int_expr(2, Op::AriOp(AriOp::Mul), 5), ExprRep::Int(10));
        assert_eq!(
            eval_int_expr(1, Op::RelOp(RelOp::Eq), 1),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_int_expr(2, Op::RelOp(RelOp::Neq), 1),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_int_expr(1, Op::RelOp(RelOp::Leq), 5),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_int_expr(5, Op::RelOp(RelOp::Geq), 5),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_int_expr(4, Op::RelOp(RelOp::Les), 5),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_int_expr(6, Op::RelOp(RelOp::Gre), 5),
            ExprRep::Bool(true)
        );
    }

    #[test]
    fn test_eval_bool_expr() {
        assert_eq!(
            eval_bool_expr(true, Op::LogOp(LogOp::And), false),
            ExprRep::Bool(false)
        );
        assert_eq!(
            eval_bool_expr(true, Op::LogOp(LogOp::Or), false),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_bool_expr(true, Op::RelOp(RelOp::Eq), true),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_bool_expr(true, Op::RelOp(RelOp::Neq), false),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_bool_expr(false, Op::RelOp(RelOp::Leq), true),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_bool_expr(true, Op::RelOp(RelOp::Geq), false),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_bool_expr(false, Op::RelOp(RelOp::Les), true),
            ExprRep::Bool(true)
        );
        assert_eq!(
            eval_bool_expr(true, Op::RelOp(RelOp::Gre), false),
            ExprRep::Bool(true)
        );
    }

    #[test]
    fn test_eval_bin_expr() {
        assert_eq!(
            interpreter(vec![Expr::BinExpr(
                Box::new(Expr::Int(1)),
                Op::AriOp(AriOp::Add),
                Box::new(Expr::Int(2)),
            )]),
            ExprRep::Int(3)
        );
    }

    #[test]
    fn test_eval_var_expr() {
        interpreter(vec![
            Expr::Let(
                Box::new(Expr::Var("b1".to_string())),
                Type::Int,
                Box::new(Expr::BinExpr(
                    Box::new(Expr::Var("".to_string())),
                    Op::AssOp(AssOp::Eq),
                    Box::new(Expr::Int(2)),
                )),
            ),
            Expr::VarExpr(
                Box::new(Expr::Var("b1".to_string())),
                Op::AssOp(AssOp::AddEq),
                Box::new(Expr::Int(2)),
            ),
        ]);
        assert_eq!(read_var("b1"), ExprRep::Int(4));
        let res = interpreter(vec![
            Expr::Let(
                Box::new(Expr::Var("b2".to_string())),
                Type::Bool,
                Box::new(Expr::BinExpr(
                    Box::new(Expr::Var("".to_string())),
                    Op::AssOp(AssOp::Eq),
                    Box::new(Expr::Bool(false)),
                )),
            ),
            Expr::VarExpr(
                Box::new(Expr::Var("b2".to_string())),
                Op::RelOp(RelOp::Eq),
                Box::new(Expr::Bool(false)),
            ),
        ]);
        assert_eq!(res, ExprRep::Bool(true));
        let res = interpreter(vec![
            Expr::Let(
                Box::new(Expr::Var("b3".to_string())),
                Type::Bool,
                Box::new(Expr::BinExpr(
                    Box::new(Expr::Var("".to_string())),
                    Op::AssOp(AssOp::Eq),
                    Box::new(Expr::Bool(false)),
                )),
            ),
            Expr::VarExpr(
                Box::new(Expr::Var("b3".to_string())),
                Op::RelOp(RelOp::Neq),
                Box::new(Expr::Bool(true)),
            ),
        ]);
        assert_eq!(res, ExprRep::Bool(true));
        let res = interpreter(vec![
            Expr::Let(
                Box::new(Expr::Var("b4".to_string())),
                Type::Int,
                Box::new(Expr::BinExpr(
                    Box::new(Expr::Var("".to_string())),
                    Op::AssOp(AssOp::Eq),
                    Box::new(Expr::Int(1)),
                )),
            ),
            Expr::VarExpr(
                Box::new(Expr::Var("b4".to_string())),
                Op::RelOp(RelOp::Leq),
                Box::new(Expr::Int(5)),
            ),
        ]);
        assert_eq!(res, ExprRep::Bool(true));
        let res = interpreter(vec![
            Expr::Let(
                Box::new(Expr::Var("b5".to_string())),
                Type::Int,
                Box::new(Expr::BinExpr(
                    Box::new(Expr::Var("".to_string())),
                    Op::AssOp(AssOp::Eq),
                    Box::new(Expr::Int(7)),
                )),
            ),
            Expr::VarExpr(
                Box::new(Expr::Var("b5".to_string())),
                Op::RelOp(RelOp::Geq),
                Box::new(Expr::Int(5)),
            ),
        ]);
        assert_eq!(res, ExprRep::Bool(true));
        let res = interpreter(vec![
            Expr::Let(
                Box::new(Expr::Var("b6".to_string())),
                Type::Int,
                Box::new(Expr::BinExpr(
                    Box::new(Expr::Var("".to_string())),
                    Op::AssOp(AssOp::Eq),
                    Box::new(Expr::Int(1)),
                )),
            ),
            Expr::VarExpr(
                Box::new(Expr::Var("b6".to_string())),
                Op::RelOp(RelOp::Les),
                Box::new(Expr::Int(5)),
            ),
        ]);
        assert_eq!(res, ExprRep::Bool(true));
        let res = interpreter(vec![
            Expr::Let(
                Box::new(Expr::Var("b7".to_string())),
                Type::Int,
                Box::new(Expr::BinExpr(
                    Box::new(Expr::Var("".to_string())),
                    Op::AssOp(AssOp::Eq),
                    Box::new(Expr::Int(6)),
                )),
            ),
            Expr::VarExpr(
                Box::new(Expr::Var("b7".to_string())),
                Op::RelOp(RelOp::Gre),
                Box::new(Expr::Int(5)),
            ),
        ]);
        assert_eq!(res, ExprRep::Bool(true));
    }

    #[test]
    fn test_eval_let() {
        interpreter(vec![Expr::Let(
            Box::new(Expr::Var("c1".to_string())),
            Type::Int,
            Box::new(Expr::BinExpr(
                Box::new(Expr::Var("".to_string())),
                Op::AssOp(AssOp::Eq),
                Box::new(Expr::Int(1)),
            )),
        )]);
        assert_eq!(read_var("c1"), ExprRep::Int(1));
        interpreter(vec![Expr::Let(
            Box::new(Expr::Var("c2".to_string())),
            Type::Bool,
            Box::new(Expr::BinExpr(
                Box::new(Expr::Var("".to_string())),
                Op::AssOp(AssOp::Eq),
                Box::new(Expr::Bool(true)),
            )),
        )]);
        assert_eq!(read_var("c2"), ExprRep::Bool(true));
        interpreter(vec![Expr::Let(
            Box::new(Expr::Var("c3".to_string())),
            Type::Bool,
            Box::new(Expr::BinExpr(
                Box::new(Expr::Bool(false)),
                Op::LogOp(LogOp::And),
                Box::new(Expr::Bool(true)),
            )),
        )]);
        assert_eq!(read_var("c3"), ExprRep::Bool(false));
    }

    #[test]
    fn test_eval_return() {
        assert_eq!(
            interpreter(vec![Expr::Return(Box::new(Expr::Int(1)))]),
            ExprRep::Int(1)
        );
        assert_eq!(
            interpreter(vec![Expr::Return(Box::new(Expr::Bool(true)))]),
            ExprRep::Bool(true)
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("d1".to_string())),
                    Type::Int,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Int(2)),
                    )),
                ),
                Expr::Return(Box::new(Expr::Var("d1".to_string())))
            ]),
            ExprRep::Int(2)
        );
        assert_eq!(
            interpreter(vec![Expr::Return(Box::new(Expr::BinExpr(
                Box::new(Expr::Int(1)),
                Op::AriOp(AriOp::Add),
                Box::new(Expr::Int(2)),
            )))]),
            ExprRep::Int(3)
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("d2".to_string())),
                    Type::Int,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Int(2)),
                    )),
                ),
                Expr::Let(
                    Box::new(Expr::Var("d3".to_string())),
                    Type::Int,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Int(2)),
                    )),
                ),
                Expr::Return(Box::new(Expr::VarExpr(
                    Box::new(Expr::Var("d2".to_string())),
                    Op::AriOp(AriOp::Add),
                    Box::new(Expr::Var("d3".to_string())),
                )))
            ]),
            ExprRep::Int(4)
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("d4".to_string())),
                    Type::Int,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Int(2)),
                    )),
                ),
                Expr::VarExpr(
                    Box::new(Expr::Var("d4".to_string())),
                    Op::AssOp(AssOp::AddEq),
                    Box::new(Expr::Int(1))
                ),
                Expr::Return(Box::new(Expr::VarExpr(
                    Box::new(Expr::Var("d4".to_string())),
                    Op::AriOp(AriOp::Add),
                    Box::new(Expr::Int(1)),
                )))
            ]),
            ExprRep::Int(4)
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("d5".to_string())),
                    Type::Int,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Int(2)),
                    )),
                ),
                Expr::Return(Box::new(Expr::VarExpr(
                    Box::new(Expr::Var("d5".to_string())),
                    Op::AriOp(AriOp::Sub),
                    Box::new(Expr::Int(1)),
                )))
            ]),
            ExprRep::Int(1)
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("d6".to_string())),
                    Type::Int,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Int(10)),
                    )),
                ),
                Expr::Return(Box::new(Expr::VarExpr(
                    Box::new(Expr::Var("d6".to_string())),
                    Op::AriOp(AriOp::Div),
                    Box::new(Expr::Int(5)),
                )))
            ]),
            ExprRep::Int(2)
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("d7".to_string())),
                    Type::Int,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Int(2)),
                    )),
                ),
                Expr::Return(Box::new(Expr::VarExpr(
                    Box::new(Expr::Var("d7".to_string())),
                    Op::AriOp(AriOp::Mul),
                    Box::new(Expr::Int(5)),
                )))
            ]),
            ExprRep::Int(10)
        );
    }

    #[test]
    fn test_eval_if() {
        assert_eq!(
            interpreter(vec![Expr::If(
                Box::new(Expr::Bool(true)),
                vec![Expr::Return(Box::new(Expr::Int(1)))]
            )]),
            ExprRep::Int(1)
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("f1".to_string())),
                    Type::Bool,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Bool(true)),
                    )),
                ),
                Expr::Let(
                    Box::new(Expr::Var("f2".to_string())),
                    Type::Bool,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Bool(true)),
                    )),
                ),
                Expr::If(
                    Box::new(Expr::VarExpr(
                        Box::new(Expr::Var("f1".to_string())),
                        Op::RelOp(RelOp::Eq),
                        Box::new(Expr::Var("f2".to_string()))
                    )),
                    vec![Expr::Return(Box::new(Expr::Int(1)))]
                )
            ]),
            ExprRep::Int(1)
        );
    }

    #[test]
    fn test_eval_if_else() {
        assert_eq!(
            interpreter(vec![Expr::IfElse(
                Box::new(Expr::Bool(false)),
                vec![Expr::Return(Box::new(Expr::Int(1)))],
                vec![Expr::Return(Box::new(Expr::Int(2)))],
            )]),
            ExprRep::Int(2)
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("g1".to_string())),
                    Type::Bool,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Bool(true)),
                    )),
                ),
                Expr::IfElse(
                    Box::new(Expr::Var("g1".to_string())),
                    vec![Expr::Return(Box::new(Expr::Int(2)))],
                    vec![Expr::Return(Box::new(Expr::Int(1)))],
                )
            ]),
            ExprRep::Int(2)
        );
    }

    #[test]
    fn test_eval_while() {
        assert_eq!(
            interpreter(vec![Expr::While(
                Box::new(Expr::Bool(true)),
                vec![Expr::Return(Box::new(Expr::Bool(false)))]
            )]),
            ExprRep::Bool(false),
        );
        assert_eq!(
            interpreter(vec![
                Expr::Let(
                    Box::new(Expr::Var("h1".to_string())),
                    Type::Bool,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Bool(true)),
                    )),
                ),
                Expr::Let(
                    Box::new(Expr::Var("h2".to_string())),
                    Type::Bool,
                    Box::new(Expr::BinExpr(
                        Box::new(Expr::Var("".to_string())),
                        Op::AssOp(AssOp::Eq),
                        Box::new(Expr::Bool(false)),
                    )),
                ),
                Expr::While(
                    Box::new(Expr::VarExpr(
                        Box::new(Expr::Var("h1".to_string())),
                        Op::RelOp(RelOp::Neq),
                        Box::new(Expr::Var("h2".to_string()))
                    )),
                    vec![Expr::Return(Box::new(Expr::Int(2)))]
                )
            ]),
            ExprRep::Int(2),
        );
    }
    #[test]
    fn test_eval_fn_call() {
        assert_eq!(
            interpreter(vec![
                Expr::Fn(
                    Box::new(Expr::Var("testfn1".to_string())),
                    vec![(Expr::Var("i1".to_string()), Type::Int,),],
                    Type::Int,
                    vec![Expr::Return(Box::new(Expr::Var("i1".to_string())))]
                ),
                Expr::Return(Box::new(Expr::FnCall(
                    Box::new(Expr::Var("testfn1".to_string())),
                    vec![Expr::Int(5)]
                )))
            ]),
            ExprRep::Int(5)
        );
    }
}
