//! Statistics calculators (Omni: Statistics). List calculators: enter
//! any number of values (GUI: one comma/space separated field; CLI: one
//! argument per value).
use crate::features::{linear_regression, regression_predict_x};
use crate::{a, chk, factorial, n, nd, norm_cdf, nz, p_from_z, Calc};

fn reg(v: &mut Vec<Calc>, c: Calc) {
    v.push(c);
}

fn sorted(x: &[f64]) -> Vec<f64> {
    let mut s = x.to_vec();
    s.sort_by(|p, q| p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal));
    s
}
fn mean(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}
fn median(x: &[f64]) -> f64 {
    let s = sorted(x);
    let mid = s.len() / 2;
    if s.len() % 2 == 1 {
        s[mid]
    } else {
        (s[mid - 1] + s[mid]) / 2.0
    }
}

pub fn register(v: &mut Vec<Calc>) {
    reg(
        v,
        Calc::new(
            "mean",
            "Mean (Average)",
            "Statistics",
            "Mean = sum of values / count of values.",
            &["Numbers (list)"],
            |x| {
                Ok(format!(
                    "Count: {}. Sum: {}. Mean: {}.",
                    x.len(),
                    nd(x.iter().sum::<f64>(), 6),
                    nd(mean(x), 6)
                ))
            },
        )
        .list_mode(),
    );
    reg(
        v,
        Calc::new(
            "median",
            "Median",
            "Statistics",
            "The middle value when sorted (or the average of the two middle values).",
            &["Numbers (list)"],
            |x| {
                // Guard first (v0.9.0): an empty list is an error,
                // never an index/unwrap panic — the indexing below
                // is only reached with at least one value.
                if x.is_empty() {
                    return Err("Median needs at least one value — the list is empty.".into());
                }
                Ok(format!(
                    "Median: {}. Smallest: {}. Largest: {}.",
                    nd(median(x), 6),
                    n(sorted(x)[0]),
                    n(*sorted(x).last().unwrap())
                ))
            },
        )
        .list_mode(),
    );
    reg(v, Calc::new("mode-range", "Mode & Range", "Statistics",
        "Mode = most frequent value (first one wins ties — all values shown sorted so you can see ties). Range = max - min.", &["Numbers (list)"],
        |x| {
            let s = sorted(x);
            let mut best = s[0]; let mut best_count = 0usize;
            for val in &s { let c = s.iter().filter(|y| **y == *val).count(); if c > best_count { best_count = c; best = *val; } }
            Ok(format!("Mode: {} (appears {} times). Range: {} ({} to {}). Sorted: {}.", n(best), best_count, nd(s[s.len()-1]-s[0],6), n(s[0]), n(s[s.len()-1]), s.iter().map(|y| n(*y)).collect::<Vec<_>>().join(", ")))
        }).list_mode());
    reg(
        v,
        Calc::new(
            "sum-product",
            "Sum & Product of a List",
            "Statistics",
            "Adds and multiplies all the values.",
            &["Numbers (list)"],
            |x| {
                Ok(format!(
                    "Sum: {}. Product: {}. Count: {}.",
                    nd(x.iter().sum::<f64>(), 6),
                    nd(x.iter().product::<f64>(), 6),
                    x.len()
                ))
            },
        )
        .list_mode(),
    );
    reg(v, Calc::new("std-population", "Standard Deviation (Population)", "Statistics",
        "Population SD = sqrt( average of (value - mean)^2 ). Use when your list IS the whole group.", &["Numbers (list)"],
        |x| { let mu=mean(x); let var=x.iter().map(|y|(y-mu).powi(2)).sum::<f64>()/x.len() as f64; Ok(format!("Mean: {}. Population variance: {}. Population standard deviation: {}.", nd(mu,6), nd(var,6), nd(var.sqrt(),6))) }).list_mode());
    reg(v, Calc::new("std-sample", "Standard Deviation (Sample)", "Statistics",
        "Sample SD = sqrt( sum of (value - mean)^2 / (n - 1) ). Use when your list is a sample of a bigger group. Needs at least 2 values.", &["Numbers (list, at least 2)"],
        |x| { if x.len()<2 { return Err("Sample standard deviation needs at least 2 values.".into()); } let mu=mean(x); let var=x.iter().map(|y|(y-mu).powi(2)).sum::<f64>()/(x.len()-1) as f64; Ok(format!("Mean: {}. Sample variance: {}. Sample standard deviation: {}.", nd(mu,6), nd(var,6), nd(var.sqrt(),6))) }).list_mode());
    reg(
        v,
        Calc::new(
            "geometric-mean",
            "Geometric Mean",
            "Statistics",
            "Geometric mean = (product of values)^(1/n). All values must be over 0.",
            &["Numbers (list, all > 0)"],
            |x| {
                if x.iter().any(|y| *y <= 0.0) {
                    return Err("Geometric mean needs every value over 0.".into());
                }
                let g = x.iter().map(|y| y.ln()).sum::<f64>() / x.len() as f64;
                Ok(format!("Geometric mean: {}.", nd(g.exp(), 6)))
            },
        )
        .list_mode(),
    );
    reg(v, Calc::new("harmonic-mean", "Harmonic Mean", "Statistics",
        "Harmonic mean = n / (sum of 1/value). Used for rates (e.g. average speed over equal distances). No zeros allowed.", &["Numbers (list, no zeros)"],
        |x| { if x.contains(&0.0) { return Err("Harmonic mean cannot include zero.".into()); } Ok(format!("Harmonic mean: {}.", nd(x.len() as f64 / x.iter().map(|y| 1.0/y).sum::<f64>(),6))) }).list_mode());
    reg(v, Calc::new("z-score", "Z-Score", "Statistics",
        "z = (value - mean) / standard deviation. Also gives the percentile from the normal distribution.", &["Value", "Mean", "Standard deviation"],
        |x| { nz(a(x,2),"Standard deviation")?; let z=(a(x,0)-a(x,1))/a(x,2); Ok(format!("z-score: {}. About the {}th percentile (normal distribution).", nd(z,4), nd(norm_cdf(z)*100.0,2))) }));
    reg(
        v,
        Calc::new(
            "percentile-from-z",
            "Percentile / Probability From Z (Normal)",
            "Statistics",
            "Uses the standard normal CDF: P(Z < z), P(Z > z) and the two-sided p-value for |z|.",
            &["z value"],
            |x| {
                let c = norm_cdf(a(x, 0));
                Ok(format!(
                    "P(Z < {}) = {}%. P(Z > z) = {}%. Two-sided p-value for |z|: {}.",
                    nd(a(x, 0), 4),
                    nd(c * 100.0, 4),
                    nd((1.0 - c) * 100.0, 4),
                    nd(p_from_z(a(x, 0)), 6)
                ))
            },
        ),
    );
    reg(
        v,
        Calc::new(
            "p-value-z",
            "P-Value From a Z Statistic",
            "Statistics",
            "Two-sided p = 2 x (1 - Phi(|z|)). One-sided (greater) p also shown.",
            &["z statistic"],
            |x| {
                Ok(format!(
                    "Two-sided p-value: {}. One-sided (greater) p-value: {}.",
                    nd(p_from_z(a(x, 0)), 6),
                    nd(1.0 - norm_cdf(a(x, 0)), 6)
                ))
            },
        ),
    );
    reg(v, Calc::new("probability", "Basic Probability", "Statistics",
        "Probability = favourable outcomes / total outcomes. As a % and as odds in favour (favourable : unfavourable).", &["Favourable outcomes", "Total outcomes"],
        |x| { nz(a(x,1),"Total outcomes")?; if a(x,0)>a(x,1) { return Err("Favourable outcomes cannot be more than the total.".into()); } Ok(format!("Probability: {} ({}%). Odds in favour: {} : {}.", nd(a(x,0)/a(x,1),6), nd(a(x,0)/a(x,1)*100.0,4), n(a(x,0)), n(a(x,1)-a(x,0)))) }));
    reg(
        v,
        Calc::new(
            "combinations",
            "Combinations nCr",
            "Statistics",
            "nCr = n! / (r! (n-r)!) — order does not matter (e.g. lottery picks).",
            &["n (total items)", "r (items chosen)"],
            |x| {
                if a(x, 1) > a(x, 0) {
                    return Err("r cannot be bigger than n.".into());
                }
                let c = factorial(a(x, 0))? / (factorial(a(x, 1))? * factorial(a(x, 0) - a(x, 1))?);
                Ok(format!("C({}, {}) = {}.", n(a(x, 0)), n(a(x, 1)), nd(c, 1)))
            },
        ),
    );
    reg(
        v,
        Calc::new(
            "permutations",
            "Permutations nPr",
            "Statistics",
            "nPr = n! / (n-r)! — order matters.",
            &["n (total items)", "r (positions)"],
            |x| {
                if a(x, 1) > a(x, 0) {
                    return Err("r cannot be bigger than n.".into());
                }
                let p = factorial(a(x, 0))? / factorial(a(x, 0) - a(x, 1))?;
                Ok(format!("P({}, {}) = {}.", n(a(x, 0)), n(a(x, 1)), nd(p, 1)))
            },
        ),
    );
    reg(v, Calc::new("confidence-interval-mean", "Confidence Interval for a Mean (Known SD)", "Statistics",
        "CI = sample mean +- z* x (SD / sqrt(n)). z*: 90% = 1.645, 95% = 1.96, 99% = 2.576 — enter the z* for your level.", &["Sample mean", "Population SD", "Sample size n", "z* (1.96 for 95%)"],
        |x| { if a(x,2)<=0.0 { return Err("Sample size must be over 0.".into()); } let moe=a(x,3)*a(x,1)/a(x,2).sqrt(); Ok(format!("Margin of error: {}. Confidence interval: {} to {}.", nd(moe,6), nd(a(x,0)-moe,6), nd(a(x,0)+moe,6))) }));
    reg(v, Calc::new("margin-of-error", "Margin of Error (Proportion)", "Statistics",
        "For a survey proportion: MoE = z* x sqrt(p(1-p)/n). Use p=0.5 for the safest (largest) estimate. z*: 95% = 1.96.", &["Sample proportion p (0-1)", "Sample size n", "z* (1.96 for 95%)"],
        |x| { if a(x,1)<=0.0 || a(x,0)<0.0 || a(x,0)>1.0 { return Err("Sample size n must be over 0 and p must be between 0 and 1.".into()); } let moe=a(x,2)*(a(x,0)*(1.0-a(x,0))/a(x,1)).sqrt(); Ok(format!("Margin of error: +/- {} ({} percentage points).", nd(moe,6), nd(moe*100.0,3))) }));
    reg(
        v,
        Calc::new(
            "sample-size-proportion",
            "Sample Size Needed (Proportion)",
            "Statistics",
            "n = z*^2 x p(1-p) / MoE^2, rounded UP. Use p=0.5 if unsure.",
            &[
                "Wanted margin of error (0-1, e.g. 0.03)",
                "Proportion p (0-1)",
                "z* (1.96 for 95%)",
            ],
            |x| {
                nz(a(x, 0), "Margin of error")?;
                chk(
                    a(x, 1) > 0.0 && a(x, 1) < 1.0,
                    "The proportion p must be between 0 and 1 (not including 0 or 1 — use 0.5 if you are unsure).",
                )?;
                let req = (a(x, 2).powi(2) * a(x, 1) * (1.0 - a(x, 1)) / a(x, 0).powi(2)).ceil();
                Ok(format!("Sample size needed: {} people/responses.", req))
            },
        ),
    );
    reg(v, Calc::new("expected-value", "Expected Value (Value, Probability Pairs)", "Statistics",
        "Enter pairs: value, probability, value, probability... (probabilities as decimals that should add to 1). EV = sum of value x probability.", &["Pairs: value, probability, ... (list)"],
        |x| { let mut ev=0.0; let mut psum=0.0; let mut i=0; while i+1<x.len() { ev+=x[i]*x[i+1]; psum+=x[i+1]; i+=2; } Ok(format!("Expected value: {}. (Probabilities you entered add to {} — should be 1.)", nd(ev,6), nd(psum,4))) }).pair_mode());
    reg(v, Calc::new("dice-probability", "Dice: Expected Total & Chance (Two Dice)", "Statistics",
        "For two fair six-sided dice: expected total is 7. Chance of a chosen exact total uses the count of pairs (out of 36) that make it.", &["Exact total you want (2-12)"],
        |x| { let t=a(x,0) as i64; if !(2..=12).contains(&t) { return Err("A two-dice total is between 2 and 12.".into()); } let count=6-(7-t).abs(); Ok(format!("Ways to roll {}: {} of 36. Probability: {}%. Expected total of two dice: 7.", t, count, nd(count as f64/36.0*100.0,3))) }));
    reg(v, Calc::new("percentile-rank", "Percentile Rank in a List", "Statistics",
        "Percentile rank of a score = (values below it + 0.5 x values equal to it) / total x 100. Enter the score FIRST, then the data values.", &["Score first, then the data values (list)"],
        |x| { if x.len()<2 { return Err("Enter the score and at least one data value.".into()); } let score=x[0]; let data=&x[1..]; let below=data.iter().filter(|y| **y<score).count() as f64; let equal=data.iter().filter(|y| **y==score).count() as f64; Ok(format!("Percentile rank of {}: {}th percentile ({} values in the data).", n(score), nd((below+0.5*equal)/data.len() as f64*100.0,2), data.len())) }).list_mode());
    reg(v, Calc::new("weighted-average", "Weighted Average (Value, Weight Pairs)", "Statistics",
        "Enter pairs: value, weight, value, weight... Result = sum(value x weight) / sum(weights).", &["Pairs: value, weight, ... (list)"],
        |x| { let mut num=0.0; let mut den=0.0; let mut i=0; while i+1<x.len() { num+=x[i]*x[i+1]; den+=x[i+1]; i+=2; } nz(den,"Total weight")?; Ok(format!("Weighted average: {}.", nd(num/den,6))) }).pair_mode());
    reg(v, Calc::new("linear-regression", "Linear Regression & Calibration", "Statistics",
        "Least-squares line y = m x + b from (x, y) pairs, with correlation r and R\u{b2}. Enter pairs: x, y, x, y, ... Optionally add ONE final value — a measured y — and the calibration part predicts the x that produced it: x = (y - b) / m. Steps show the substituted sums (n, \u{3a3}x, \u{3a3}y, \u{3a3}xy, \u{3a3}x\u{b2}).", &["Pairs: x, y, ... plus optional final measured y (list)"],
        |x| {
            // Pairs, plus an optional odd trailing value = measured y.
            let (data, measured) = if x.len() % 2 == 1 {
                (&x[..x.len()-1], Some(x[x.len()-1]))
            } else {
                (x, None)
            };
            if data.len() < 4 {
                return Err("Linear regression needs at least 2 (x, y) points — that is 4 numbers.".into());
            }
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            let mut i = 0;
            while i + 1 < data.len() { xs.push(data[i]); ys.push(data[i+1]); i += 2; }
            let reg = linear_regression(&xs, &ys)?;
            let residuals: Vec<String> = xs.iter().zip(ys.iter())
                .map(|(xv, yv)| nd(yv - (reg.slope * xv + reg.intercept), 4))
                .collect();
            let mut out = format!(
                "Linear regression (n = {} points): y = {} x + {}.\nSlope m = {}. Intercept b = {}.\nCorrelation r = {}. R\u{b2} = {} ({}% of the variance in y is explained by x).\nMeans: x\u{304} = {}, y\u{304} = {}.\nResiduals (y - predicted y): {}.\nSteps: n = {}, \u{3a3}x = {}, \u{3a3}y = {}, \u{3a3}xy = {}, \u{3a3}x\u{b2} = {}. m = (n\u{3a3}xy - \u{3a3}x\u{3a3}y) / (n\u{3a3}x\u{b2} - (\u{3a3}x)\u{b2}) = {}. b = y\u{304} - m x\u{304} = {}.",
                reg.n, nd(reg.slope, 6), nd(reg.intercept, 6),
                nd(reg.slope, 6), nd(reg.intercept, 6),
                nd(reg.r, 6), nd(reg.r2, 6), nd(reg.r2 * 100.0, 2),
                nd(reg.mean_x, 6), nd(reg.mean_y, 6),
                residuals.join(", "),
                reg.n, nd(reg.sum_x, 6), nd(reg.sum_y, 6), nd(reg.sum_xy, 6), nd(reg.sum_xx, 6),
                nd(reg.slope, 6), nd(reg.intercept, 6)
            );
            match measured {
                Some(y) => match regression_predict_x(&reg, y) {
                    Ok(px) => out.push_str(&format!("\nCalibration: measured y = {} -> predicted x = ({} - {}) / {} = {}.", nd(y, 6), nd(y, 6), nd(reg.intercept, 6), nd(reg.slope, 6), nd(px, 6))),
                    Err(e) => out.push_str(&format!("\nCalibration: {e}")),
                },
                None => out.push_str("\nCalibration: add one extra value at the end of the list (a measured y) and CalcDex predicts its x = (y - b) / m."),
            }
            Ok(out)
        }).pair_tail_mode());
    // v0.7.0 (#33): replicates summary — the lab one-step answer
    // for repeated measurements of the same quantity.
    reg(
        v,
        Calc::new(
            "replicates-summary",
            "Replicates Summary (Mean, SD, SEM, 95% CI)",
            "Statistics",
            "For repeated measurements of the same thing: the mean, the sample standard deviation (SD), the standard error of the mean (SEM = SD / sqrt(n)) and the 95% confidence interval for the mean = mean +- t x SEM, with t from Student's t table at n - 1 degrees of freedom (table for df 1-30; the normal value 1.96 beyond df 30). Needs at least 2 values.",
            &["Measured values (list, at least 2)"],
            |x| {
                if x.len() < 2 {
                    return Err("Replicates summary needs at least 2 measurements of the same thing.".into());
                }
                let mu = mean(x);
                let var = x.iter().map(|y| (y - mu).powi(2)).sum::<f64>() / (x.len() - 1) as f64;
                let sd = var.sqrt();
                let sem = sd / (x.len() as f64).sqrt();
                let t = crate::t_crit_95(x.len() - 1);
                let moe = t * sem;
                Ok(format!(
                    "Replicates: n = {}. Mean = {}. Sample SD = {}. SEM = {}. 95% confidence interval: {} to {} (mean \u{b1} {}). Steps: SD = sqrt( sum of (value - mean)^2 / (n - 1) ) = {}. SEM = SD / sqrt(n) = {} / {} = {}. Margin = t({}) x SEM = {} x {} = {}.",
                    x.len(),
                    nd(mu, 6),
                    nd(sd, 6),
                    nd(sem, 6),
                    nd(mu - moe, 6),
                    nd(mu + moe, 6),
                    nd(moe, 6),
                    nd(sd, 6),
                    nd(sd, 6),
                    nd((x.len() as f64).sqrt(), 6),
                    nd(sem, 6),
                    x.len() - 1,
                    nd(t, 3),
                    nd(sem, 6),
                    nd(moe, 6)
                ))
            },
        )
        .list_mode(),
    );
    // v0.7.0 (#36): outlier check by the 1.5 x IQR rule, with
    // quartiles by the median-of-halves (Tukey) method.
    reg(
        v,
        Calc::new(
            "outlier-check",
            "Outlier Check (IQR Rule)",
            "Statistics",
            "Flags values that fall outside the fences Q1 - 1.5 x IQR and Q3 + 1.5 x IQR (the standard Tukey rule). Q1 and Q3 are the medians of the lower and upper halves of the sorted values. Needs at least 4 values.",
            &["Values (list, at least 4)"],
            |x| {
                if x.len() < 4 {
                    return Err("Outlier check needs at least 4 values — it compares the middle half of the data with the rest.".into());
                }
                let s = sorted(x);
                let med = median(x);
                let mid = s.len() / 2;
                let (lower, upper) = if s.len() % 2 == 1 {
                    (&s[..mid], &s[mid + 1..])
                } else {
                    (&s[..mid], &s[mid..])
                };
                let q1 = median(lower);
                let q3 = median(upper);
                let iqr = q3 - q1;
                let lo = q1 - 1.5 * iqr;
                let hi = q3 + 1.5 * iqr;
                let flagged: Vec<f64> =
                    s.iter().copied().filter(|val| *val < lo || *val > hi).collect();
                let mut out = format!(
                    "Sorted: {}. Q1 = {}. Median = {}. Q3 = {}. IQR = {} (Q3 - Q1). Fences: below {} or above {} (Q1 - 1.5 x IQR, Q3 + 1.5 x IQR).",
                    s.iter().map(|y| n(*y)).collect::<Vec<_>>().join(", "),
                    nd(q1, 6),
                    nd(med, 6),
                    nd(q3, 6),
                    nd(iqr, 6),
                    nd(lo, 6),
                    nd(hi, 6)
                );
                if flagged.is_empty() {
                    out.push_str(" Outliers: none — every value lies inside the fences.");
                } else {
                    out.push_str(&format!(
                        " Outliers ({}): {}.",
                        flagged.len(),
                        flagged.iter().map(|y| n(*y)).collect::<Vec<_>>().join(", ")
                    ));
                }
                Ok(out)
            },
        )
        .list_mode(),
    );
}
