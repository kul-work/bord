use std::fs::File;
use std::path::PathBuf;
use clap::Parser;
use csv::ReaderBuilder;

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
) -> anyhow::Result<((), Vec<(String, bool, bool, f64)>)> {
    let mut results = Vec::new();

    let mut toxic_count = 0;
    let mut neutral_count = 0;

    for (idx, sample) in samples.iter().enumerate() {
         // Get ground truth (1 if toxic, 0 if neutral)
         let ground_truth = sample.label > 0;

         // Run inference
         match tract_model::classify_sentiment(&sample.text) {
             Ok(sentiment_score) => {
                  // Predict as toxic if score < threshold
                  let predicted = sentiment_score < threshold;

                results.push((
                    sample.id.clone(),
                    predicted,
                    ground_truth,
                    sentiment_score,
                ));

                // Red if toxic, green if neutral (based on model prediction only)
                let color = if predicted { "\x1b[31m" } else { "\x1b[32m" };
                let reset = "\x1b[0m";
                let label = if predicted { "TOXIC" } else { "NEUTRAL" };
                
                // Count predictions
                if predicted {
                    toxic_count += 1;
                } else {
                    neutral_count += 1;
                }
                
                eprintln!(
                    "{}[{}] ID={} | score={:.4}{}",
                    color, label, sample.id, sentiment_score, reset
                );
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
    
    // Print class distribution summary
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("Class Distribution:");
    eprintln!("  Toxic samples:   {}", toxic_count);
    eprintln!("  Neutral samples: {}", neutral_count);
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    Ok(((), results))
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

    // Evaluate across all thresholds
    for threshold in &thresholds {
        eprintln!(
            "Evaluating threshold {:.2}...",
            threshold
        );
        let (_metrics, _results) = evaluate(&samples, *threshold)?;
    }

    Ok(())
}
