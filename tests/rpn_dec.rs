//! Decimal engine vs independently generated reference values
//! (tools/gen_dec_vectors.py, Python decimal at 170 digits).

use calcdex::rpn::dec::{Dec, Rnd};
use calcdex::rpn::dec_math as m;

include!("rpn_dec_vectors.rs");

const P: usize = 50;

fn d(s: &str) -> Dec {
    Dec::parse(s).unwrap()
}

fn r34(x: Dec) -> Dec {
    x.round(34, Rnd::HalfEven)
}

fn compute(f: &str, x: &Dec) -> Dec {
    match f {
        "exp" => m::exp(x, P),
        "ln" => m::ln(x, P),
        "sin" => m::sin_cos(x, P).unwrap().0,
        "cos" => m::sin_cos(x, P).unwrap().1,
        "tan" => {
            let (s, c) = m::sin_cos(x, P).unwrap();
            s.div(&c, P).unwrap()
        }
        "atan" => m::atan(x, P),
        "asin" => m::asin(x, P),
        "acos" => m::acos(x, P),
        "sinh" => m::sinh(x, P),
        "cosh" => m::cosh(x, P),
        "tanh" => m::tanh(x, P),
        "asinh" => m::asinh(x, P),
        "acosh" => m::acosh(x, P),
        "atanh" => m::atanh(x, P),
        "expm1" => m::expm1(x, P),
        "ln1p" => m::ln1p(x, P),
        "sqrt" => x.sqrt(34).unwrap(),
        other => panic!("unknown fn {other}"),
    }
}

#[test]
fn unary_functions_match_reference() {
    let mut bad = Vec::new();
    for (f, x, want) in UNARY {
        let got = r34(compute(f, &d(x)));
        if got != d(want) {
            bad.push(format!("{f}({x}): got {} want {want}", got.to_sci_string()));
        }
    }
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn arithmetic_matches_reference() {
    let mut bad = Vec::new();
    for (op, a, b, want) in BINARY {
        let (x, y) = (d(a), d(b));
        let got = match *op {
            "add" => x.add(&y, 34),
            "sub" => x.sub(&y, 34),
            "mul" => x.mul(&y, 34),
            "div" => x.div(&y, 34).unwrap(),
            _ => unreachable!(),
        };
        if got != d(want) {
            bad.push(format!(
                "{a} {op} {b}: got {} want {want}",
                got.to_sci_string()
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn constants_agree() {
    // pi from the constant vs Machin-derived reference in the generator.
    assert_eq!(
        m::pi(34).to_sci_string(),
        d("3.141592653589793238462643383279503").to_sci_string()
    );
    assert_eq!(m::ln10(34), d("2.302585092994045684017991454684364"));
}
