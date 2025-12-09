use std::fs::File;
use std::path::PathBuf;
use clap::Parser;
use csv::ReaderBuilder;
use serde_json::json;

// Import tokenizer and tract_model from parent lib
mod tokenizer {
    include!("../tokenizer.rs");
}

mod tract_model {
    include!("../tract_model.rs");
}

#[derive(Parser)]
#[command(name = "eval-batch")]
#[command(about = "Evaluate Tract sentiment model against labeled dataset", long_about = None)]
struct Args {
    /// Path to CSV file with columns: id, comment_text, hate_score
    #[arg(short, long, value_name = "FILE")]
    data: PathBuf,

    /// Output JSON results to file (optional)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Threshold values to test (comma-separated, default: 0.2,0.25,0.3,0.35,0.4)
    #[arg(short, long, default_value = "0.2,0.25,0.3,0.35,0.4")]
    thresholds: String,
}

#[derive(Debug)]
struct Sample {
    id: String,
    text: String,
    label: u32,
}

#[derive(Debug, Clone)]
struct Metrics {
    tp: u32,
    tn: u32,
    fp: u32,
    fn_: u32,
}

impl Metrics {
    fn precision(&self) -> f64 {
        let denom = (self.tp + self.fp) as f64;
        if denom == 0.0 {
            0.0
        } else {
            self.tp as f64 / denom
        }
    }

    fn recall(&self) -> f64 {
        let denom = (self.tp + self.fn_) as f64;
        if denom == 0.0 {
            0.0
        } else {
            self.tp as f64 / denom
        }
    }

    fn f1(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * (p * r) / (p + r)
        }
    }

    fn accuracy(&self) -> f64 {
        let total = (self.tp + self.tn + self.fp + self.fn_) as f64;
        if total == 0.0 {
            0.0
        } else {
            (self.tp + self.tn) as f64 / total
        }
    }
}

fn load_samples(path: &PathBuf) -> anyhow::Result<Vec<Sample>> {
    let file = File::open(path)?;
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .delimiter(b'\t')
        .from_reader(file);

    let mut samples = Vec::new();
    for result in reader.records() {
        let record = result?;
        if record.len() < 3 {
            eprintln!("⚠ Skipping malformed record (expected 3 columns): {:?}", record);
            continue;
        }

        let id = record[0].to_string();
        let text = record[1].to_string();
        let label: u32 = record[2].parse().unwrap_or(0);

        samples.push(Sample { id, text, label });
    }

    Ok(samples)
}

fn evaluate(
    samples: &[Sample],
    threshold: f64,
) -> anyhow::Result<(Metrics, Vec<(String, bool, bool, f64)>)> {
    let mut metrics = Metrics {
        tp: 0,
        tn: 0,
        fp: 0,
        fn_: 0,
    };

    let mut results = Vec::new();

    for (idx, sample) in samples.iter().enumerate() {
        // Get ground truth (1 if toxic, 0 if neutral)
        let ground_truth = sample.label > 0;

        // Run inference
        match tract_model::classify_sentiment(&sample.text) {
            Ok(sentiment_score) => {
                // Predict as toxic if score < threshold
                let predicted = sentiment_score < threshold;

                // Update metrics
                match (predicted, ground_truth) {
                    (true, true) => metrics.tp += 1,
                    (false, false) => metrics.tn += 1,
                    (true, false) => metrics.fp += 1,
                    (false, true) => metrics.fn_ += 1,
                }

                results.push((
                    sample.id.clone(),
                    predicted,
                    ground_truth,
                    sentiment_score,
                ));

                if predicted != ground_truth {
                    let status = if predicted && !ground_truth {
                        "FP (false positive)"
                    } else {
                        "FN (false negative)"
                    };
                    eprintln!(
                        "[{}] {} | {} | score={:.4}",
                        status, sample.id, sample.text, sentiment_score
                    );
                }
            }
            Err(e) => {
                eprintln!("⚠ Inference failed for {}: {}", sample.id, e);
                results.push((sample.id.clone(), false, ground_truth, 0.5));
            }
        }

        // Progress indicator
        if (idx + 1) % 1000 == 0 {
            eprint!(".");
        }
    }

    eprintln!("\n");
    Ok((metrics, results))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Load data
    eprintln!("Loading dataset from {}...", args.data.display());
    let samples = load_samples(&args.data)?;
    eprintln!("✓ Loaded {} samples\n", samples.len());

    if samples.is_empty() {
        eprintln!("⚠ No samples loaded");
        return Ok(());
    }

    // Parse thresholds
    let thresholds: Vec<f64> = args
        .thresholds
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if thresholds.is_empty() {
        return Err(anyhow::anyhow!("No valid thresholds provided"));
    }

    let mut all_results = Vec::new();

    // Evaluate across all thresholds
    for threshold in &thresholds {
        eprintln!(
            "Evaluating threshold {:.2}...",
            threshold
        );
        let (metrics, _results) = evaluate(&samples, *threshold)?;

        println!("\n═════════════════════════════════════════════════════════");
        println!("  Tract Sentiment Evaluation - Threshold {:.2}", threshold);
        println!("═════════════════════════════════════════════════════════");
        println!();
        println!("Samples:       {}", samples.len());
        println!();
        println!("Results:");
        println!("  True Positives:   {:>6}  ({:>5.1}% of positives)", metrics.tp, 
                 (metrics.tp as f64 / (metrics.tp + metrics.fn_) as f64) * 100.0);
        println!("  False Positives:  {:>6}  ({:>5.1}% of predicted toxic)", metrics.fp,
                 (metrics.fp as f64 / (metrics.tp + metrics.fp) as f64) * 100.0);
        println!("  True Negatives:   {:>6}  ({:>5.1}% of negatives)", metrics.tn,
                 (metrics.tn as f64 / (metrics.tn + metrics.fp) as f64) * 100.0);
        println!("  False Negatives:  {:>6}  ({:>5.1}% of negatives)", metrics.fn_,
                 (metrics.fn_ as f64 / (metrics.tn + metrics.fp) as f64) * 100.0);
        println!();
        println!("Metrics:");
        println!("  Precision:  {:.4} ({:.2}%)", metrics.precision(), metrics.precision() * 100.0);
        println!("  Recall:     {:.4} ({:.2}%)", metrics.recall(), metrics.recall() * 100.0);
        println!("  F1 Score:   {:.4}", metrics.f1());
        println!("  Accuracy:   {:.4} ({:.2}%)", metrics.accuracy(), metrics.accuracy() * 100.0);
        println!();

        // Store for JSON output
        let result_obj = json!({
            "threshold": threshold,
            "samples": samples.len(),
            "tp": metrics.tp,
            "tn": metrics.tn,
            "fp": metrics.fp,
            "fn": metrics.fn_,
            "precision": metrics.precision(),
            "recall": metrics.recall(),
            "f1": metrics.f1(),
            "accuracy": metrics.accuracy(),
        });

        all_results.push(result_obj);
    }

    println!("═════════════════════════════════════════════════════════\n");

    // Write JSON output
    if let Some(output_path) = args.output {
        let json_output = serde_json::to_string_pretty(&all_results)?;
        std::fs::write(&output_path, json_output)?;
        eprintln!("✓ Results written to {}", output_path.display());
    } else {
        // Output JSON to stdout for jq piping
        for result in &all_results {
            println!("{}", result.to_string());
        }
    }

    Ok(())
}
