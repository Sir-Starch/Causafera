use serde_json::{Value, json};

fn percentile(sorted: &[u64], numerator: usize) -> u64 {
    let rank = (sorted.len() * numerator).div_ceil(100).saturating_sub(1);
    sorted[rank]
}

fn percentile_basis_points(sorted: &[u64], basis_points: usize) -> u64 {
    let rank = (sorted.len() * basis_points)
        .div_ceil(10_000)
        .saturating_sub(1);
    sorted[rank]
}

fn resampled_p95(samples: &[u64], mut seed: u64) -> Vec<u64> {
    let mut resampled = Vec::with_capacity(1_024);
    for _ in 0..1_024 {
        let mut draw = Vec::with_capacity(samples.len());
        for _ in samples {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let divisor = u64::try_from(samples.len()).unwrap_or(1);
            let slot = usize::try_from(seed % divisor).unwrap_or(0);
            draw.push(samples[slot]);
        }
        draw.sort_unstable();
        resampled.push(percentile(&draw, 95));
    }
    resampled
}

/// Deterministic bootstrap of the empirical p95. The seed affects only analysis, never history.
pub(super) fn distribution(samples: &[u64]) -> Value {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let mut resampled = resampled_p95(samples, 0xD1B5_4A32_D192_ED03);
    resampled.sort_unstable();
    json!({
        "count": samples.len(),
        "p50_ns": percentile(&sorted, 50),
        "p95_ns": percentile(&sorted, 95),
        "p99_ns": percentile(&sorted, 99),
        "p95_bootstrap_95_ci_ns": [
            percentile_basis_points(&resampled, 250),
            percentile_basis_points(&resampled, 9_750)
        ]
    })
}

/// Upper 95% bootstrap bound for the ratio of two independently sampled p95s.
pub(super) fn ratio_upper_95(prior: &[u64], mature: &[u64]) -> f64 {
    let prior_p95 = resampled_p95(prior, 0xA5F4_4130_B899_2312);
    let mature_p95 = resampled_p95(mature, 0x81E7_039D_4A43_2861);
    let mut ratios = prior_p95
        .iter()
        .zip(mature_p95.iter())
        .map(|(&prior, &mature)| {
            std::time::Duration::from_nanos(mature).as_secs_f64()
                / std::time::Duration::from_nanos(prior.max(1)).as_secs_f64()
        })
        .collect::<Vec<_>>();
    ratios.sort_by(f64::total_cmp);
    let rank = (ratios.len() * 9_750).div_ceil(10_000).saturating_sub(1);
    ratios[rank]
}

/// Resample whole bundle runs first, then ticks within each selected run.
pub(super) fn grouped_ratio_upper_95(prior: &[Vec<u64>], mature: &[Vec<u64>]) -> f64 {
    fn next(seed: &mut u64) -> u64 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *seed
    }

    fn draw(groups: &[Vec<u64>], seed: &mut u64) -> u64 {
        let mut sample = Vec::new();
        for _ in groups {
            let group = &groups[next(seed) as usize % groups.len()];
            for _ in group {
                sample.push(group[next(seed) as usize % group.len()]);
            }
        }
        sample.sort_unstable();
        percentile(&sample, 95)
    }

    let mut seed = 0x3E98_B26A_781F_94D5;
    let mut ratios = Vec::with_capacity(1_024);
    for _ in 0..1_024 {
        let denominator = draw(prior, &mut seed).max(1) as f64;
        ratios.push(draw(mature, &mut seed) as f64 / denominator);
    }
    ratios.sort_by(f64::total_cmp);
    let rank = (ratios.len() * 9_500).div_ceil(10_000).saturating_sub(1);
    ratios[rank]
}

/// Descriptive least-squares trend over independent per-run p95 estimates.
pub(super) fn regression(x: &[f64], y: &[f64]) -> Value {
    assert_eq!(x.len(), y.len());
    let count = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / count;
    let mean_y = y.iter().sum::<f64>() / count;
    let xx = x.iter().map(|value| (value - mean_x).powi(2)).sum::<f64>();
    let xy = x
        .iter()
        .zip(y)
        .map(|(left, right)| (left - mean_x) * (right - mean_y))
        .sum::<f64>();
    let slope = xy / xx;
    let intercept = mean_y - slope * mean_x;
    let residual = x
        .iter()
        .zip(y)
        .map(|(left, right)| (right - intercept - slope * left).powi(2))
        .sum::<f64>();
    let yy = y.iter().map(|value| (value - mean_y).powi(2)).sum::<f64>();
    let slope_standard_error = (residual / (count - 2.0) / xx).sqrt();
    json!({
        "independent_runs": x.len(),
        "intercept_ns": intercept,
        "slope_ns_per_x_unit": slope,
        "slope_approximate_95_ci_ns_per_x_unit": [
            slope - 1.96 * slope_standard_error,
            slope + 1.96 * slope_standard_error
        ],
        "r_squared": if yy == 0.0 { 1.0 } else { 1.0 - residual / yy }
    })
}
