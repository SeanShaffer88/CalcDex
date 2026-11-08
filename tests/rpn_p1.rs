//! RPN P1: stack machine, display formats, errors, state file.

use calcdex::rpn::display::{format_num, Disp};
use calcdex::rpn::state::{self, ParseOutcome};
use calcdex::rpn::{Angle, Session};

fn run(line: &str) -> Session {
    let mut s = Session::new();
    s.exec_line(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    s
}

fn x(line: &str) -> f64 {
    run(line).x()
}

fn err(line: &str) -> String {
    let mut s = Session::new();
    s.exec_line(line).unwrap_err().code.to_string()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

#[test]
fn basic_arithmetic_and_order() {
    assert_eq!(x("2 3 + 4 *"), 20.0);
    assert_eq!(x("10 4 -"), 6.0); // Y - X
    assert_eq!(x("20 4 /"), 5.0);
    assert_eq!(x("2 10 Y^X"), 1024.0);
    assert_eq!(x("1.5e-3 2 *"), 3e-3);
    assert_eq!(x("-3 ABS"), 3.0);
    assert_eq!(x("5 CHS"), -5.0);
    assert_eq!(x("7 3 MOD"), 1.0);
    assert_eq!(x("-7 3 MOD"), 2.0); // sign of divisor
}

#[test]
fn stack_lift_enter_and_drop() {
    let s = run("1 2 3 4");
    assert_eq!(s.stack, [4.0, 3.0, 2.0, 1.0]);
    // fifth entry pushes 1 off the top
    assert_eq!(run("1 2 3 4 5").stack, [5.0, 4.0, 3.0, 2.0]);
    // ENTER disables lift: next number replaces X
    let s = run("5 ENTER 3");
    assert_eq!(s.stack[..2], [3.0, 5.0]);
    // binary op drops the stack, T duplicates
    let s = run("1 2 3 4 + ");
    assert_eq!(s.stack, [7.0, 2.0, 1.0, 1.0]);
    assert_eq!(run("1 2 3 4 RDN").stack, [3.0, 2.0, 1.0, 4.0]);
    assert_eq!(run("1 2 3 4 RDN RUP").stack, [4.0, 3.0, 2.0, 1.0]);
    assert_eq!(run("1 2 X<>Y").stack[..2], [1.0, 2.0]);
    assert_eq!(run("1 2 3 CLX").stack[0], 0.0);
    assert_eq!(run("1 2 3 CLST").stack, [0.0; 4]);
}

#[test]
fn lastx_and_percent() {
    assert_eq!(x("6 3 / LASTX"), 3.0);
    let s = run("200 15 %");
    assert_eq!(s.stack[..2], [30.0, 200.0]); // Y kept
    assert!(close(x("50 60 %CH"), 20.0));
    assert!(close(x("3 4 SQRT LASTX +"), 6.0)); // 3, sqrt(4)=2, lastx 4 -> 2+4
}

#[test]
fn scientific_functions() {
    assert!(close(x("2 SQRT"), 2f64.sqrt()));
    assert_eq!(x("4 X^2"), 16.0);
    assert_eq!(x("4 1/X"), 0.25);
    assert!(close(x("1 E^X"), std::f64::consts::E));
    assert_eq!(x("3 10^X"), 1000.0);
    assert!(close(x("100 LOG"), 2.0));
    assert!(close(x("1 E^X LN"), 1.0));
    assert!(close(x("PI"), std::f64::consts::PI));
    assert_eq!(x("5 FACT"), 120.0);
    assert_eq!(x("0 FACT"), 1.0);
    assert_eq!(x("5 2 COMB"), 10.0);
    assert_eq!(x("5 2 PERM"), 20.0);
    assert_eq!(x("-3.7 IP"), -3.0);
    assert!(close(x("3.75 FP"), 0.75));
    assert!(close(x("0 COSH"), 1.0));
    assert!(close(x("1 TANH ATANH"), 1.0));
}

#[test]
fn angle_modes_and_exact_quarters() {
    assert_eq!(x("180 SIN"), 0.0);
    assert_eq!(x("90 SIN"), 1.0);
    assert_eq!(x("90 COS"), 0.0);
    assert_eq!(x("270 SIN"), -1.0);
    assert_eq!(x("-90 SIN"), -1.0);
    assert!(close(x("45 TAN"), 1.0));
    assert!(close(x("1 ASIN"), 90.0));
    assert!(close(x("RAD 1 ASIN"), std::f64::consts::FRAC_PI_2));
    assert_eq!(x("GRAD 100 SIN"), 1.0);
    assert!(close(x("GRAD 1 ASIN"), 100.0));
    assert_eq!(run("RAD").angle, Angle::Rad);
    assert!(close(x("180 D-R"), std::f64::consts::PI));
    assert!(close(x("PI R-D"), 180.0));
}

#[test]
fn errors_have_codes_and_change_nothing() {
    assert_eq!(err("1 0 /"), "RPN-103");
    assert_eq!(err("0 1/X"), "RPN-103");
    assert_eq!(err("-1 SQRT"), "RPN-102");
    assert_eq!(err("0 LN"), "RPN-102");
    assert_eq!(err("2 ASIN"), "RPN-102");
    assert_eq!(err("90 TAN"), "RPN-102");
    assert_eq!(err("1.5 FACT"), "RPN-102");
    assert_eq!(err("171 FACT"), "RPN-104");
    assert_eq!(err("1e308 10 *"), "RPN-104");
    assert_eq!(err("FOO"), "RPN-100");
    assert_eq!(err("inf"), "RPN-100");
    assert_eq!(err("nan"), "RPN-100");
    assert_eq!(err("STO"), "RPN-101");
    assert_eq!(err("RCL \"nothing\""), "RPN-105");
    assert_eq!(err("FIX 12"), "RPN-102");
    assert_eq!(err("RCL \"unterminated"), "RPN-108");
    // failing op leaves the stack as it was
    let mut s = Session::new();
    s.exec_line("4 0").unwrap();
    assert!(s.exec_line("/").is_err());
    assert_eq!(s.stack[..2], [0.0, 4.0]);
}

#[test]
fn variables_and_storage_arithmetic() {
    let s = run("12 STO \"A\" 3 STO 01 RCL \"A\" RCL 01 +");
    assert_eq!(s.x(), 15.0);
    assert_eq!(run("10 STO A 4 STO+ A RCL A").x(), 14.0);
    assert_eq!(run("10 STO A 4 STO- A RCL A").x(), 6.0);
    assert_eq!(run("10 STO A 4 STO* A RCL A").x(), 40.0);
    assert_eq!(run("8 STO A 4 STO/ A RCL A").x(), 2.0);
    assert_eq!(run("3 STO A 10 RCL+ A").x(), 13.0);
    assert_eq!(run("3 STO A 10 RCL- A").x(), 7.0);
    assert_eq!(run("3 STO A 10 RCL* A").x(), 30.0);
    assert_eq!(run("2 STO A 10 RCL/ A").x(), 5.0);
    assert_eq!(err("1 STO A 0 STO/ A"), "RPN-103");
    let s = run("1 STO A CLV A");
    assert!(s.vars.is_empty());
    assert_eq!(err("CLV A"), "RPN-105");
    assert_eq!(err("1 STO \"a|b\""), "RPN-101");
}

#[test]
fn display_formats() {
    assert_eq!(format_num(7.12345678, Disp::Fix(4)), "7.1235");
    assert_eq!(format_num(-0.00001, Disp::Fix(4)), "0.0000");
    assert_eq!(format_num(12.0, Disp::Fix(0)), "12");
    assert_eq!(format_num(1234.5678, Disp::Sci(3)), "1.235E3");
    assert_eq!(format_num(0.00012345, Disp::Sci(2)), "1.23E-4");
    assert_eq!(format_num(0.0, Disp::Sci(2)), "0.00E0");
    assert_eq!(format_num(12345.678, Disp::Eng(2)), "12.3E3");
    assert_eq!(format_num(123.0, Disp::Eng(2)), "123E0");
    assert_eq!(format_num(0.0123, Disp::Eng(1)), "12E-3");
    assert_eq!(format_num(1.0 / 3.0, Disp::All), "0.333333333333");
    assert_eq!(format_num(2.0, Disp::All), "2");
    assert_eq!(format_num(-1234.5, Disp::All), "-1234.5");
    assert_eq!(format_num(1.5e-9, Disp::All), "1.5E-9");
    assert_eq!(format_num(1e20, Disp::All), "1E20");
    assert_eq!(format_num(1e15, Disp::Fix(2)), "1.00E15");
    assert_eq!(format_num(f64::NAN, Disp::All), "Out of Range");
}

#[test]
fn display_mode_tokens() {
    assert_eq!(run("2 SQRT SCI 3").display_x(), "1.414E0");
    assert_eq!(run("2 SQRT FIX 2").display_x(), "1.41");
    assert_eq!(run("1234.5 ENG 2").display_x(), "1.23E3");
    assert_eq!(run("0.5 ALL").display_x(), "0.5");
}

#[test]
fn state_round_trip_exact() {
    let mut s = run("1 3 / 2 SQRT STO \"A\" 1e-300 RAD SCI 5");
    s.exec_line("7 ENTER").unwrap();
    let text = state::serialize(&s);
    assert!(text.starts_with("schema_version=1\n"));
    match state::parse(&text) {
        ParseOutcome::Ok(t) => {
            assert_eq!(t.stack, s.stack);
            assert_eq!(t.lastx, s.lastx);
            assert_eq!(t.lift, s.lift);
            assert_eq!(t.angle, s.angle);
            assert_eq!(t.disp, s.disp);
            assert_eq!(t.vars, s.vars);
        }
        _ => panic!("round trip failed"),
    }
}

#[test]
fn state_parse_rejects_bad_files() {
    for bad in [
        "",
        "stack=1|2|3|4\n",
        "schema_version=1\n",
        "schema_version=1\nstack=1|2|3\n",
        "schema_version=1\nstack=1|2|3|nan\n",
        "schema_version=1\nstack=0|0|0|0\nangle=TURN\n",
        "schema_version=1\nstack=0|0|0|0\ndisp=FIX:99\n",
        "schema_version=1\nstack=0|0|0|0\nvar=noval\n",
        "schema_version=1\nstack=0|0|0|0\nvar=A|inf\n",
        "garbage line\n",
    ] {
        assert!(matches!(state::parse(bad), ParseOutcome::Bad(_)), "{bad:?}");
    }
    assert!(matches!(
        state::parse("schema_version=2\nstack=0|0|0|0\n"),
        ParseOutcome::Newer(2)
    ));
}

#[test]
fn state_file_load_save_quarantine_and_newer() {
    let dir = std::env::temp_dir().join(format!("calcdex-rpn-p1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("CALCDEX_DATA_DIR", &dir);

    assert!(state::load("a b").is_err()); // bad name
    let l = state::load("t1").unwrap(); // missing = fresh
    assert!(l.note.is_none() && !l.read_only);

    let mut s = l.session;
    s.exec_line("42 STO \"V\" 9").unwrap();
    state::save("t1", &s).unwrap();
    let l = state::load("t1").unwrap();
    assert_eq!(l.session.x(), 9.0);
    assert_eq!(l.session.vars.get("V"), Some(&42.0));

    // corrupt file is moved aside, not lost
    let p = dir.join("rpn-t2.txt");
    std::fs::write(&p, "not a state file").unwrap();
    let l = state::load("t2").unwrap();
    assert!(l.note.is_some() && !l.read_only);
    assert!(!p.exists());
    assert!(dir.join("rpn-t2.txt.bad").exists());

    // newer schema: defaults, read-only, file untouched
    let p = dir.join("rpn-t3.txt");
    std::fs::write(&p, "schema_version=9\nstack=1|2|3|4\n").unwrap();
    let l = state::load("t3").unwrap();
    assert!(l.read_only && l.note.is_some());
    assert_eq!(
        std::fs::read_to_string(&p).unwrap(),
        "schema_version=9\nstack=1|2|3|4\n"
    );

    // --clean sees rpn files
    let found = calcdex::paths::purge_candidates(&dir);
    assert!(found.iter().any(|f| f.ends_with("rpn-t1.txt")));
    assert!(found.iter().any(|f| f.ends_with("rpn-t2.txt.bad")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn runaway_input_is_bounded() {
    let mut s = Session::new();
    let big = "1 ".repeat(100_001);
    assert_eq!(s.exec_line(&big).unwrap_err().code, "RPN-109");
    let many: String = (0..1001).map(|i| format!("1 STO v{i} ")).collect();
    assert_eq!(s.exec_line(&many).unwrap_err().code, "RPN-110");
}
