//! Vanta NNUE trainer: (768 -> 256)x2 -> 1, SCReLU, trained with Bullet.
//!
//! Env vars:
//!   DATA_DIR   directory of Bullet ChessBoard .bin files (default "data")
//!   OUT_DIR    checkpoint directory (default "checkpoints")
//!   NET_ID     name prefix for checkpoints (default "vanta")
//!   SUPERBATCHES  number of superbatches / epochs (default 40)
//!   WDL        weight on game result, rest on search score (default 0.3)

use bullet_lib::{
    game::inputs::Chess768,
    nn::optimiser::AdamW,
    trainer::{
        save::SavedFormat,
        schedule::{lr, wdl, TrainingSchedule, TrainingSteps},
        settings::LocalSettings,
    },
    value::{loader, ValueTrainerBuilder},
};

const HIDDEN_SIZE: usize = 256;
const SCALE: i32 = 400;
const QA: i16 = 255;
const QB: i16 = 64;
const BATCH_SIZE: usize = 16_384;
const RECORD_BYTES: u64 = 32;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn main() {
    let data_dir = env_or("DATA_DIR", "data");
    let out_dir = env_or("OUT_DIR", "checkpoints");
    let net_id = env_or("NET_ID", "vanta");
    let superbatches: usize = env_or("SUPERBATCHES", "40").parse().expect("SUPERBATCHES");
    let wdl_weight: f32 = env_or("WDL", "0.3").parse().expect("WDL");

    // Collect every .bin file and count positions.
    let mut files: Vec<String> = std::fs::read_dir(&data_dir)
        .unwrap_or_else(|e| panic!("cannot read DATA_DIR {data_dir}: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |x| x == "bin"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no .bin files in {data_dir}");

    let total_bytes: u64 = files.iter().map(|f| std::fs::metadata(f).unwrap().len()).sum();
    assert!(total_bytes % RECORD_BYTES == 0, "total size not a multiple of 32 bytes");
    let positions = total_bytes / RECORD_BYTES;
    let batches_per_superbatch = ((positions as usize) / BATCH_SIZE).max(1);
    println!(
        "{} files, {} positions, {} batches/superbatch (1 epoch), {} superbatches, wdl {}",
        files.len(),
        positions,
        batches_per_superbatch,
        superbatches,
        wdl_weight
    );

    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(AdamW)
        .inputs(Chess768)
        .save_format(&[
            SavedFormat::id("l0w").round().quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            SavedFormat::id("l1w").round().quantise::<i16>(QB),
            SavedFormat::id("l1b").round().quantise::<i16>(QA * QB),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs| {
            let l0 = builder.new_affine("l0", 768, HIDDEN_SIZE);
            let l1 = builder.new_affine("l1", 2 * HIDDEN_SIZE, 1);

            let stm_hidden = l0.forward(stm_inputs).screlu();
            let ntm_hidden = l0.forward(ntm_inputs).screlu();
            let hidden_layer = stm_hidden.concat(ntm_hidden);
            l1.forward(hidden_layer)
        });

    let schedule = TrainingSchedule {
        net_id: net_id,
        eval_scale: SCALE as f32,
        steps: TrainingSteps {
            batch_size: BATCH_SIZE,
            batches_per_superbatch,
            start_superbatch: 1,
            end_superbatch: superbatches,
        },
        // `value` is the weight on the game result; the rest is the search score.
        wdl_scheduler: wdl::ConstantWDL { value: wdl_weight },
        // lr drops x0.1 at 3/8 and 3/4 of training.
        lr_scheduler: lr::StepLR { start: 0.001, gamma: 0.1, step: (superbatches * 3 / 8).max(1) },
        save_rate: 10,
    };

    let settings = LocalSettings {
        threads: 4,
        test_set: None,
        output_directory: &out_dir,
        batch_queue_size: 64,
    };

    let paths: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
    let data_loader = loader::DirectSequentialDataLoader::new(&paths);

    trainer.run(&schedule, &settings, &data_loader);
}
