//! Materializes the evaluation datasets under scenarios/data/n{N}/dataset.jsonl
//! (gitignored: reproducible from the seed).
use scenarios::gen::{generate, serialize_dataset, SEED, SIZES};
use std::{fs, path::Path};

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    for n in SIZES {
        let bytes = serialize_dataset(&generate(n, SEED));
        let sub = dir.join(format!("n{n}"));
        fs::create_dir_all(&sub).expect("mkdir scenarios/data/n{N}");
        let path = sub.join("dataset.jsonl");
        fs::write(&path, &bytes).expect("write dataset");
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    }
}
