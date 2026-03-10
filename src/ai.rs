use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::path::PathBuf;

const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";

pub struct Categorizer {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    category_embeddings: Vec<(String, Tensor)>,
}

impl Categorizer {
    pub fn new(categories: &[String]) -> Result<Self, Box<dyn std::error::Error>> {
        let device = Device::Cpu;

        let api = Api::new()?;
        let repo = api.repo(Repo::new(MODEL_ID.to_string(), RepoType::Model));

        let tokenizer_path = repo.get("tokenizer.json")?;
        let config_path = repo.get("config.json")?;
        let weights_path = repo.get("model.safetensors")?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| format!("tokenizer error: {e}"))?;

        let config: Config = serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &device)?
        };
        let model = BertModel::load(vb, &config)?;

        let mut categorizer = Self {
            model,
            tokenizer,
            device,
            category_embeddings: Vec::new(),
        };

        // Pre-compute category embeddings
        let mut cat_embeds = Vec::new();
        for cat in categories {
            let emb = categorizer.embed(cat)?;
            cat_embeds.push((cat.clone(), emb));
        }
        categorizer.category_embeddings = cat_embeds;

        Ok(categorizer)
    }

    pub fn categorize(&self, text: &str) -> Result<String, Box<dyn std::error::Error>> {
        if self.category_embeddings.is_empty() {
            return Ok("Misc".to_string());
        }

        let text_emb = self.embed(text)?;

        let mut best_cat = "Misc".to_string();
        let mut best_score = f32::NEG_INFINITY;

        for (cat, cat_emb) in &self.category_embeddings {
            let score = cosine_similarity(&text_emb, cat_emb)?;
            if score > best_score {
                best_score = score;
                best_cat = cat.clone();
            }
        }

        Ok(best_cat)
    }

    fn embed(&self, text: &str) -> Result<Tensor, Box<dyn std::error::Error>> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| format!("encode error: {e}"))?;

        let token_ids = Tensor::new(encoding.get_ids(), &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::zeros_like(&token_ids)?;

        let output = self.model.forward(&token_ids, &token_type_ids, None)?;

        // Mean pooling over sequence dimension
        let (_, seq_len, _) = output.dims3()?;
        let embeddings = (output.sum(1)? / (seq_len as f64))?;
        let embeddings = embeddings.squeeze(0)?;

        // L2 normalize
        let norm = embeddings.sqr()?.sum_all()?.sqrt()?;
        let normalized = embeddings.broadcast_div(&norm)?;

        Ok(normalized)
    }

    pub fn update_categories(&mut self, categories: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let mut cat_embeds = Vec::new();
        for cat in categories {
            let emb = self.embed(cat)?;
            cat_embeds.push((cat.clone(), emb));
        }
        self.category_embeddings = cat_embeds;
        Ok(())
    }

    pub fn model_path() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("thought-train")
            .join("models")
    }
}

fn cosine_similarity(a: &Tensor, b: &Tensor) -> Result<f32, Box<dyn std::error::Error>> {
    let dot = (a * b)?.sum_all()?.to_scalar::<f32>()?;
    Ok(dot)
}
