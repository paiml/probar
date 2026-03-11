//! Scoring engine for LLM inference runtime comparison.
//!
//! Computes weighted composite scores from `LoadTestResult` metrics using
//! absolute thresholds. Each metric gets a 0-100 score, combined into a
//! weighted composite with letter grade (A+ through F).
//!
//! Design: absolute thresholds (not relative best=100) so scores are stable
//! when runtimes are added/removed. Best-in-class gets a +5 bonus per metric.
//!
//! References:
//!   - Metron fluidity-index (arXiv 2407.07000) — jitter penalty on ITL
//!   - MLPerf Inference — per-scenario metrics
//!   - pmat perfection-score — weighted category aggregation

use super::loadtest::{LoadTestResult, TailAnalysis};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Scoring contract (deserialized from YAML or built-in defaults)
// =============================================================================

/// A single metric threshold definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricThreshold {
    /// Value at which score = 100.
    pub excellent: f64,
    /// Value at which score = 75.
    pub good: f64,
    /// Whether higher values are better (true) or lower values are better (false).
    pub higher_is_better: bool,
}

/// Scoring contract with thresholds, weights, and grade boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringContract {
    /// Per-metric threshold definitions.
    pub thresholds: HashMap<String, MetricThreshold>,
    /// Interactive (c=1) weights: metric_name → weight (must sum to 1.0).
    pub interactive_weights: HashMap<String, f64>,
    /// Throughput (c>1) weights: metric_name → weight (must sum to 1.0).
    pub throughput_weights: HashMap<String, f64>,
    /// Best-in-class bonus points (capped at 100).
    pub best_in_class_bonus: u8,
    /// Grade boundaries: (min_score, grade_label).
    pub grades: Vec<(f64, String)>,
}

impl Default for ScoringContract {
    fn default() -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert(
            "decode_tok_s".into(),
            MetricThreshold {
                excellent: 160.0,
                good: 120.0,
                higher_is_better: true,
            },
        );
        thresholds.insert(
            "ttft_p50_ms".into(),
            MetricThreshold {
                excellent: 12.0,
                good: 50.0,
                higher_is_better: false,
            },
        );
        thresholds.insert(
            "itl_p50_ms".into(),
            MetricThreshold {
                excellent: 6.0,
                good: 10.0,
                higher_is_better: false,
            },
        );
        thresholds.insert(
            "ttft_p99_ms".into(),
            MetricThreshold {
                excellent: 15.0,
                good: 50.0,
                higher_is_better: false,
            },
        );
        thresholds.insert(
            "error_rate".into(),
            MetricThreshold {
                excellent: 0.0,
                good: 0.01,
                higher_is_better: false,
            },
        );
        thresholds.insert(
            "aggregate_tok_s".into(),
            MetricThreshold {
                excellent: 600.0,
                good: 300.0,
                higher_is_better: true,
            },
        );
        thresholds.insert(
            "throughput_scaling".into(),
            MetricThreshold {
                excellent: 3.8,
                good: 2.0,
                higher_is_better: true,
            },
        );

        let mut interactive_weights = HashMap::new();
        interactive_weights.insert("decode_tok_s".into(), 0.30);
        interactive_weights.insert("ttft_p50_ms".into(), 0.30);
        interactive_weights.insert("itl_p50_ms".into(), 0.15);
        interactive_weights.insert("ttft_p99_ms".into(), 0.15);
        interactive_weights.insert("error_rate".into(), 0.10);

        let mut throughput_weights = HashMap::new();
        throughput_weights.insert("aggregate_tok_s".into(), 0.30);
        throughput_weights.insert("decode_tok_s".into(), 0.15);
        throughput_weights.insert("ttft_p50_ms".into(), 0.15);
        throughput_weights.insert("itl_p50_ms".into(), 0.15);
        throughput_weights.insert("throughput_scaling".into(), 0.15);
        throughput_weights.insert("error_rate".into(), 0.10);

        let grades = vec![
            (95.0, "A+".into()),
            (90.0, "A".into()),
            (85.0, "A-".into()),
            (80.0, "B+".into()),
            (70.0, "B".into()),
            (60.0, "C+".into()),
            (50.0, "C".into()),
            (40.0, "D".into()),
            (30.0, "D-".into()),
            (0.0, "F".into()),
        ];

        Self {
            thresholds,
            interactive_weights,
            throughput_weights,
            best_in_class_bonus: 5,
            grades,
        }
    }
}

// =============================================================================
// Score results
// =============================================================================

/// Score for a single metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricScore {
    /// Raw value from the benchmark.
    pub value: f64,
    /// Computed score (0-100).
    pub score: u8,
    /// Whether this runtime was best-in-class for this metric.
    pub best: bool,
    /// Jitter penalty applied (only for ITL metric).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter_penalty: Option<u8>,
}

/// Scores for a single runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeScore {
    /// Runtime name (e.g., "realizr", "llamacpp", "vllm", "ollama").
    pub name: String,
    /// Source JSON file.
    pub source_file: String,
    /// Per-metric scores.
    pub metrics: HashMap<String, MetricScore>,
    /// Weighted composite score (0-100).
    pub composite: f64,
    /// Letter grade.
    pub grade: String,
}

/// Complete scorecard output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scorecard {
    /// Contract version used.
    pub contract_version: String,
    /// Timestamp of computation.
    pub timestamp: String,
    /// Concurrency level.
    pub concurrency: usize,
    /// Scored runtimes, sorted by composite descending.
    pub runtimes: Vec<RuntimeScore>,
}

// =============================================================================
// Score computation
// =============================================================================

/// Compute a single metric score from absolute thresholds.
///
/// - Higher-is-better: value >= excellent → 100, value >= good → 75-100, else 0-75
/// - Lower-is-better: value <= excellent → 100, value <= good → 75-100, else 0-75
pub fn compute_metric_score(value: f64, threshold: &MetricThreshold) -> u8 {
    if threshold.higher_is_better {
        score_higher_is_better(value, threshold.excellent, threshold.good)
    } else {
        score_lower_is_better(value, threshold.excellent, threshold.good)
    }
}

fn score_higher_is_better(value: f64, excellent: f64, good: f64) -> u8 {
    if value >= excellent {
        100
    } else if value <= 0.0 {
        0
    } else if value >= good {
        let pct = (value - good) / (excellent - good);
        (75.0 + 25.0 * pct).round() as u8
    } else {
        (75.0 * value / good).round().min(74.0) as u8
    }
}

fn score_lower_is_better(value: f64, excellent: f64, good: f64) -> u8 {
    if value <= excellent {
        100
    } else if value <= good {
        let pct = (good - value) / (good - excellent);
        (75.0 + 25.0 * pct).round() as u8
    } else if good > 0.0 {
        (75.0 * good / value).round().min(74.0).max(0.0) as u8
    } else {
        0
    }
}

/// Compute jitter penalty from tail analysis.
///
/// Deducts up to 30 points from ITL score based on spike count and CV.
/// Based on Metron fluidity-index insight: P50 hides stalls.
pub fn compute_jitter_penalty(tail: &TailAnalysis) -> u8 {
    let spike_penalty = (tail.jitter.spike_count as f64 * 2.0).min(20.0);
    let cv_penalty = (tail.jitter.itl_cv * 100.0).min(10.0);
    (spike_penalty + cv_penalty).round().min(30.0) as u8
}

/// Assign a letter grade from composite score.
pub fn assign_grade(composite: f64, grades: &[(f64, String)]) -> String {
    for (min_score, label) in grades {
        if composite >= *min_score {
            return label.clone();
        }
    }
    "F".into()
}

/// Extract metric values from a `LoadTestResult`.
fn extract_metrics(result: &LoadTestResult, is_throughput: bool) -> HashMap<String, f64> {
    let mut metrics = HashMap::new();
    metrics.insert("decode_tok_s".into(), result.decode_tok_per_sec);
    metrics.insert("ttft_p50_ms".into(), result.ttft_p50_ms);
    metrics.insert("itl_p50_ms".into(), result.itl_p50_ms);
    metrics.insert("ttft_p99_ms".into(), result.ttft_p99_ms);
    metrics.insert("error_rate".into(), result.error_rate);

    if is_throughput {
        metrics.insert("aggregate_tok_s".into(), result.tokens_per_sec);
    }
    metrics
}

/// Compute scorecard from a set of benchmark results.
///
/// All results must share the same concurrency level. For throughput scoring,
/// provide `c1_results` to compute throughput_scaling ratios.
pub fn compute_scorecard(
    results: &[(LoadTestResult, String)], // (result, source_filename)
    c1_results: Option<&[(LoadTestResult, String)]>, // c=1 baselines for scaling ratio
    contract: &ScoringContract,
) -> Scorecard {
    let concurrency = results.first().map(|r| r.0.concurrency).unwrap_or(1);
    let is_throughput = concurrency > 1;
    let weights = if is_throughput {
        &contract.throughput_weights
    } else {
        &contract.interactive_weights
    };

    // Build c=1 decode lookup for throughput_scaling
    let c1_decode: HashMap<String, f64> = c1_results
        .map(|c1| {
            c1.iter()
                .map(|(r, _)| (r.runtime_name.clone(), r.decode_tok_per_sec))
                .collect()
        })
        .unwrap_or_default();

    // Phase 1: Extract raw metric values per runtime
    let mut runtime_metrics: Vec<(String, String, HashMap<String, f64>, Option<&TailAnalysis>)> =
        Vec::new();

    for (result, source_file) in results {
        let mut metrics = extract_metrics(result, is_throughput);

        // Compute throughput_scaling if we have c=1 data
        if is_throughput {
            if let Some(&c1_decode_val) = c1_decode.get(&result.runtime_name) {
                if c1_decode_val > 0.0 {
                    metrics.insert(
                        "throughput_scaling".into(),
                        result.tokens_per_sec / c1_decode_val,
                    );
                }
            }
        }

        runtime_metrics.push((
            result.runtime_name.clone(),
            source_file.clone(),
            metrics,
            result.tail_analysis.as_ref(),
        ));
    }

    // Phase 2: Find best value per metric (for best-in-class bonus)
    let mut best_per_metric: HashMap<String, (f64, usize)> = HashMap::new(); // metric → (best_value, runtime_idx)
    for (idx, (_, _, metrics, _)) in runtime_metrics.iter().enumerate() {
        for (metric_name, &value) in metrics {
            if let Some(threshold) = contract.thresholds.get(metric_name) {
                let is_better = match best_per_metric.get(metric_name) {
                    None => true,
                    Some(&(best_val, _)) => {
                        if threshold.higher_is_better {
                            value > best_val
                        } else {
                            value < best_val
                        }
                    }
                };
                if is_better {
                    best_per_metric.insert(metric_name.clone(), (value, idx));
                }
            }
        }
    }

    // Phase 3: Compute scores per runtime
    let mut scored_runtimes: Vec<RuntimeScore> = Vec::new();

    for (idx, (name, source_file, metrics, tail)) in runtime_metrics.iter().enumerate() {
        let mut metric_scores: HashMap<String, MetricScore> = HashMap::new();
        let mut weighted_sum = 0.0;

        for (metric_name, weight) in weights {
            if let Some(&value) = metrics.get(metric_name) {
                if let Some(threshold) = contract.thresholds.get(metric_name) {
                    let mut score = compute_metric_score(value, threshold);

                    // Apply jitter penalty to ITL
                    let jitter_penalty = if metric_name == "itl_p50_ms" {
                        if let Some(tail_analysis) = tail {
                            let penalty = compute_jitter_penalty(tail_analysis);
                            score = score.saturating_sub(penalty);
                            Some(penalty)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Best-in-class bonus
                    let is_best = best_per_metric
                        .get(metric_name)
                        .is_some_and(|&(_, best_idx)| best_idx == idx);

                    if is_best {
                        score = score
                            .saturating_add(contract.best_in_class_bonus)
                            .min(100);
                    }

                    weighted_sum += *weight * f64::from(score);

                    metric_scores.insert(
                        metric_name.clone(),
                        MetricScore {
                            value,
                            score,
                            best: is_best,
                            jitter_penalty,
                        },
                    );
                }
            }
        }

        let composite = weighted_sum.round().min(100.0);
        let grade = assign_grade(composite, &contract.grades);

        scored_runtimes.push(RuntimeScore {
            name: name.clone(),
            source_file: source_file.clone(),
            metrics: metric_scores,
            composite,
            grade,
        });
    }

    // Sort by composite descending
    scored_runtimes.sort_by(|a, b| b.composite.partial_cmp(&a.composite).unwrap_or(std::cmp::Ordering::Equal));

    Scorecard {
        contract_version: "2.0.0".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        concurrency,
        runtimes: scored_runtimes,
    }
}

/// Format scorecard as a terminal table.
pub fn format_table(scorecard: &Scorecard) -> String {
    let scenario = if scorecard.concurrency == 1 {
        "Interactive (c=1)"
    } else {
        &format!("Throughput (c={})", scorecard.concurrency)
    };

    let mut lines = Vec::new();
    lines.push(format!("Inference Runtime Scorecard — {scenario}"));
    lines.push(String::new());

    if scorecard.concurrency == 1 {
        // Interactive columns
        lines.push(format!(
            "{:<12} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>10}",
            "Runtime", "Decode", "TTFT", "ITL", "Tail", "Error", "Composite"
        ));
        lines.push(format!(
            "{:<12} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>10}",
            "", "tok/s", "P50", "P50", "P99", "Rate", "Score"
        ));
        lines.push(format!("{}", "-".repeat(74)));

        for rt in &scorecard.runtimes {
            let decode = format_metric_cell(rt.metrics.get("decode_tok_s"));
            let ttft = format_metric_cell(rt.metrics.get("ttft_p50_ms"));
            let itl = format_metric_cell(rt.metrics.get("itl_p50_ms"));
            let tail = format_metric_cell(rt.metrics.get("ttft_p99_ms"));
            let error = format_metric_cell(rt.metrics.get("error_rate"));

            lines.push(format!(
                "{:<12} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>5.1} {}",
                rt.name, decode, ttft, itl, tail, error, rt.composite, rt.grade
            ));
        }
    } else {
        // Throughput columns
        lines.push(format!(
            "{:<12} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>10}",
            "Runtime", "Aggr", "Decode", "TTFT", "ITL", "Scale", "Error", "Composite"
        ));
        lines.push(format!(
            "{:<12} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>10}",
            "", "tok/s", "tok/s", "P50", "P50", "ratio", "Rate", "Score"
        ));
        lines.push(format!("{}", "-".repeat(88)));

        for rt in &scorecard.runtimes {
            let aggr = format_metric_cell(rt.metrics.get("aggregate_tok_s"));
            let decode = format_metric_cell(rt.metrics.get("decode_tok_s"));
            let ttft = format_metric_cell(rt.metrics.get("ttft_p50_ms"));
            let itl = format_metric_cell(rt.metrics.get("itl_p50_ms"));
            let scale = format_metric_cell(rt.metrics.get("throughput_scaling"));
            let error = format_metric_cell(rt.metrics.get("error_rate"));

            lines.push(format!(
                "{:<12} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>5.1} {}",
                rt.name, aggr, decode, ttft, itl, scale, error, rt.composite, rt.grade
            ));
        }
    }

    lines.push(String::new());
    lines.push("* = best in class".into());
    lines.join("\n")
}

fn format_metric_cell(metric: Option<&MetricScore>) -> String {
    match metric {
        Some(m) => {
            let star = if m.best { "*" } else { " " };
            format!("{:>3}{}", m.score, star)
        }
        None => "   -".into(),
    }
}

/// Format scorecard as Markdown.
pub fn format_markdown(scorecard: &Scorecard) -> String {
    let scenario = if scorecard.concurrency == 1 {
        "Interactive (c=1)"
    } else {
        &format!("Throughput (c={})", scorecard.concurrency)
    };

    let mut lines = Vec::new();
    lines.push(format!("## Scorecard — {scenario}"));
    lines.push(String::new());

    if scorecard.concurrency == 1 {
        lines.push(
            "| Runtime | Decode | TTFT P50 | ITL P50 | Tail P99 | Error | **Composite** |"
                .into(),
        );
        lines.push("|---------|--------|----------|---------|----------|-------|---------------|".into());

        for rt in &scorecard.runtimes {
            let decode = format_md_cell(rt.metrics.get("decode_tok_s"));
            let ttft = format_md_cell(rt.metrics.get("ttft_p50_ms"));
            let itl = format_md_cell(rt.metrics.get("itl_p50_ms"));
            let tail = format_md_cell(rt.metrics.get("ttft_p99_ms"));
            let error = format_md_cell(rt.metrics.get("error_rate"));

            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | **{:.1} ({})** |",
                rt.name, decode, ttft, itl, tail, error, rt.composite, rt.grade
            ));
        }
    } else {
        lines.push("| Runtime | Aggregate | Decode | TTFT P50 | ITL P50 | Scaling | Error | **Composite** |".into());
        lines.push("|---------|-----------|--------|----------|---------|---------|-------|---------------|".into());

        for rt in &scorecard.runtimes {
            let aggr = format_md_cell(rt.metrics.get("aggregate_tok_s"));
            let decode = format_md_cell(rt.metrics.get("decode_tok_s"));
            let ttft = format_md_cell(rt.metrics.get("ttft_p50_ms"));
            let itl = format_md_cell(rt.metrics.get("itl_p50_ms"));
            let scale = format_md_cell(rt.metrics.get("throughput_scaling"));
            let error = format_md_cell(rt.metrics.get("error_rate"));

            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} | **{:.1} ({})** |",
                rt.name, aggr, decode, ttft, itl, scale, error, rt.composite, rt.grade
            ));
        }
    }

    lines.join("\n")
}

fn format_md_cell(metric: Option<&MetricScore>) -> String {
    match metric {
        Some(m) => {
            let star = if m.best { " **" } else { "" };
            let end = if m.best { "**" } else { "" };
            format!("{star}{}{end}", m.score)
        }
        None => "-".into(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_higher_is_better_at_excellent() {
        let t = MetricThreshold {
            excellent: 160.0,
            good: 120.0,
            higher_is_better: true,
        };
        assert_eq!(compute_metric_score(160.0, &t), 100);
        assert_eq!(compute_metric_score(200.0, &t), 100); // capped
    }

    #[test]
    fn test_higher_is_better_at_good() {
        let t = MetricThreshold {
            excellent: 160.0,
            good: 120.0,
            higher_is_better: true,
        };
        assert_eq!(compute_metric_score(120.0, &t), 75);
    }

    #[test]
    fn test_higher_is_better_below_good() {
        let t = MetricThreshold {
            excellent: 160.0,
            good: 120.0,
            higher_is_better: true,
        };
        let score = compute_metric_score(60.0, &t);
        assert_eq!(score, 38); // 75 * 60/120 = 37.5 → 38
    }

    #[test]
    fn test_higher_is_better_zero() {
        let t = MetricThreshold {
            excellent: 160.0,
            good: 120.0,
            higher_is_better: true,
        };
        assert_eq!(compute_metric_score(0.0, &t), 0);
    }

    #[test]
    fn test_lower_is_better_at_excellent() {
        let t = MetricThreshold {
            excellent: 12.0,
            good: 50.0,
            higher_is_better: false,
        };
        assert_eq!(compute_metric_score(12.0, &t), 100);
        assert_eq!(compute_metric_score(5.0, &t), 100); // better than excellent
    }

    #[test]
    fn test_lower_is_better_at_good() {
        let t = MetricThreshold {
            excellent: 12.0,
            good: 50.0,
            higher_is_better: false,
        };
        assert_eq!(compute_metric_score(50.0, &t), 75);
    }

    #[test]
    fn test_lower_is_better_above_good() {
        let t = MetricThreshold {
            excellent: 12.0,
            good: 50.0,
            higher_is_better: false,
        };
        let score = compute_metric_score(100.0, &t);
        assert_eq!(score, 38); // 75 * 50/100 = 37.5 → 38
    }

    #[test]
    fn test_error_rate_zero_is_perfect() {
        let t = MetricThreshold {
            excellent: 0.0,
            good: 0.01,
            higher_is_better: false,
        };
        assert_eq!(compute_metric_score(0.0, &t), 100);
    }

    #[test]
    fn test_error_rate_low_still_high_score() {
        // F-SCORE-007: 0.7% error should score >= 80
        let t = MetricThreshold {
            excellent: 0.0,
            good: 0.01,
            higher_is_better: false,
        };
        let score = compute_metric_score(0.007, &t);
        assert!(score >= 80, "0.7% error rate scored {score}, expected >= 80");
    }

    #[test]
    fn test_jitter_penalty_clean() {
        let tail = TailAnalysis {
            itl_p999_ms: 7.0,
            itl_p9999_ms: 7.0,
            ttft_p999_ms: 15.0,
            ttft_p9999_ms: 15.0,
            latency_p999_ms: 250.0,
            latency_p9999_ms: 250.0,
            tail_ratio_itl: 1.0,
            tail_ratio_ttft: 1.0,
            tail_ratio_latency: 1.0,
            jitter: super::super::loadtest::JitterAnalysis {
                itl_cv: 0.01,
                itl_iqr_ms: 0.1,
                spike_count: 0,
                spike_threshold_ms: 35.0,
                spikes: vec![],
            },
            drift: super::super::loadtest::DriftAnalysis {
                itl_slope_ms_per_min: 0.0,
                ttft_slope_ms_per_min: 0.0,
                degradation_detected: false,
            },
        };
        assert_eq!(compute_jitter_penalty(&tail), 1); // just 0.01*100 = 1
    }

    #[test]
    fn test_jitter_penalty_spiky() {
        // F-SCORE-003: spiky runtime should get significant penalty
        let tail = TailAnalysis {
            itl_p999_ms: 50.0,
            itl_p9999_ms: 100.0,
            ttft_p999_ms: 15.0,
            ttft_p9999_ms: 15.0,
            latency_p999_ms: 300.0,
            latency_p9999_ms: 350.0,
            tail_ratio_itl: 7.0,
            tail_ratio_ttft: 1.0,
            tail_ratio_latency: 1.2,
            jitter: super::super::loadtest::JitterAnalysis {
                itl_cv: 0.15,
                itl_iqr_ms: 5.0,
                spike_count: 10,
                spike_threshold_ms: 35.0,
                spikes: vec![],
            },
            drift: super::super::loadtest::DriftAnalysis {
                itl_slope_ms_per_min: 0.0,
                ttft_slope_ms_per_min: 0.0,
                degradation_detected: false,
            },
        };
        let penalty = compute_jitter_penalty(&tail);
        assert!(penalty >= 25, "spiky penalty={penalty}, expected >= 25");
        assert!(penalty <= 30, "spiky penalty={penalty}, expected <= 30");
    }

    #[test]
    fn test_grade_assignment() {
        let grades = ScoringContract::default().grades;
        assert_eq!(assign_grade(97.0, &grades), "A+");
        assert_eq!(assign_grade(92.0, &grades), "A");
        assert_eq!(assign_grade(85.0, &grades), "A-");
        assert_eq!(assign_grade(80.0, &grades), "B+");
        assert_eq!(assign_grade(75.0, &grades), "B");
        assert_eq!(assign_grade(60.0, &grades), "C+");
        assert_eq!(assign_grade(50.0, &grades), "C");
        assert_eq!(assign_grade(40.0, &grades), "D");
        assert_eq!(assign_grade(30.0, &grades), "D-");
        assert_eq!(assign_grade(10.0, &grades), "F");
    }

    #[test]
    fn test_no_single_metric_dominates() {
        // F-SCORE-002: zeroing any one metric cannot drop composite below 40
        let contract = ScoringContract::default();
        for (zeroed_metric, _) in &contract.interactive_weights {
            let mut weighted_sum = 0.0;
            for (metric, weight) in &contract.interactive_weights {
                let score = if metric == zeroed_metric { 0.0 } else { 100.0 };
                weighted_sum += weight * score;
            }
            assert!(
                weighted_sum >= 40.0,
                "Zeroing {zeroed_metric} drops composite to {weighted_sum}"
            );
        }
    }

    #[test]
    fn test_weights_sum_to_one() {
        let contract = ScoringContract::default();
        let interactive_sum: f64 = contract.interactive_weights.values().sum();
        assert!(
            (interactive_sum - 1.0).abs() < 0.001,
            "Interactive weights sum to {interactive_sum}"
        );
        let throughput_sum: f64 = contract.throughput_weights.values().sum();
        assert!(
            (throughput_sum - 1.0).abs() < 0.001,
            "Throughput weights sum to {throughput_sum}"
        );
    }

    #[test]
    fn test_score_independence_from_field() {
        // F-SCORE-001: Adding/removing a runtime changes scores by at most the bonus amount
        let contract = ScoringContract::default();

        // Create two fake results
        let result_a = make_test_result("runtime_a", 150.0, 15.0, 7.0, 20.0, 0.0, 1);
        let result_b = make_test_result("runtime_b", 130.0, 30.0, 8.0, 40.0, 0.0, 1);
        let result_c = make_test_result("runtime_c", 100.0, 60.0, 12.0, 80.0, 0.01, 1);

        let card_abc = compute_scorecard(
            &[
                (result_a.clone(), "a.json".into()),
                (result_b.clone(), "b.json".into()),
                (result_c.clone(), "c.json".into()),
            ],
            None,
            &contract,
        );

        let card_ab = compute_scorecard(
            &[
                (result_a.clone(), "a.json".into()),
                (result_b.clone(), "b.json".into()),
            ],
            None,
            &contract,
        );

        let score_a_with_bc = card_abc
            .runtimes
            .iter()
            .find(|r| r.name == "runtime_a")
            .unwrap()
            .composite;
        let score_a_with_b = card_ab
            .runtimes
            .iter()
            .find(|r| r.name == "runtime_a")
            .unwrap()
            .composite;

        let diff = (score_a_with_bc - score_a_with_b).abs();
        assert!(
            diff <= f64::from(contract.best_in_class_bonus),
            "Score changed by {diff} when removing runtime_c (max allowed: {})",
            contract.best_in_class_bonus
        );
    }

    fn make_test_result(
        name: &str,
        decode: f64,
        ttft: f64,
        itl: f64,
        ttft_p99: f64,
        error_rate: f64,
        concurrency: usize,
    ) -> LoadTestResult {
        LoadTestResult {
            total_requests: 100,
            successful: (100.0 * (1.0 - error_rate)) as u64,
            failed: (100.0 * error_rate) as u64,
            throughput_rps: decode / 32.0,
            latency_p50_ms: ttft + itl * 31.0,
            latency_p95_ms: ttft + itl * 31.0 * 1.1,
            latency_p99_ms: ttft + itl * 31.0 * 1.2,
            ttft_p50_ms: ttft,
            tokens_per_sec: decode * concurrency as f64,
            avg_tok_per_req: 32.0,
            itl_p50_ms: itl,
            decode_tok_per_sec: decode,
            prefill_tok_per_sec: 1000.0 / ttft * 23.0,
            timestamp: "2026-03-11T00:00:00Z".into(),
            runtime_name: name.into(),
            elapsed_secs: 60.0,
            concurrency,
            ttft_p90_ms: ttft * 1.1,
            ttft_p95_ms: ttft * 1.2,
            ttft_p99_ms: ttft_p99,
            tpot_p50_ms: itl,
            tpot_p90_ms: itl * 1.1,
            tpot_p95_ms: itl * 1.2,
            tpot_p99_ms: itl * 1.3,
            latency_min_ms: ttft + itl * 30.0,
            latency_max_ms: ttft + itl * 35.0,
            latency_stddev_ms: itl * 0.5,
            error_rate,
            prompt_tokens_total: 2300,
            completion_tokens_total: 3200,
            truncated_pct: 0.0,
            sse_batch_ratio: 1.0,
            goodput_pct: 100.0,
            output_tokens_dist: None,
            decode_us_per_layer: None,
            num_layers: Some(28),
            brick_trace_summary: None,
            request_details: vec![],
            quality: None,
            tail_analysis: None,
            gpu_telemetry: None,
            dataset_stats: None,
        }
    }
}
