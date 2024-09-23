pub struct HFTokenizer {
    tokenizer: tokenizers::Tokenizer,
}

pub struct JiebaTokenizer {
    jeiba: jieba_rs::Jieba,
}

impl JiebaTokenizer {
    pub fn new() -> JiebaTokenizer {
        JiebaTokenizer {
            jeiba: jieba_rs::Jieba::new(),
        }
    }
}

impl HFTokenizer {
    pub fn new(model: &str) -> HFTokenizer {
        HFTokenizer {
            tokenizer: tokenizers::tokenizer::Tokenizer::from_pretrained(model, None).unwrap(),
        }
    }
}

pub trait Tokenize {
    // default, just tokenize on whitespace
    fn tokenize(&self, s: &str) -> Vec<String> {
        s.split_whitespace().map(|s| s.to_string()).collect()
    }
}

impl Tokenize for HFTokenizer {
    fn tokenize(&self, s: &str) -> Vec<String> {
        self.tokenizer
            .encode(s, false)
            .expect("failed to tokenize")
            .get_tokens()
            .to_vec()
    }
}

impl Tokenize for JiebaTokenizer {
    fn tokenize(&self, s: &str) -> Vec<String> {
        self.jeiba
            .cut(s, true)
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

type PostgresTokenizer = Box<dyn Tokenize + Sync + Send>;

pub fn get_tokenizer(tokenizer: &str, model: Option<&str>) -> &'static PostgresTokenizer {
    static HF_TOKENIZER: std::sync::OnceLock<PostgresTokenizer> = std::sync::OnceLock::new();
    static JIEBA_TOKENIZER: std::sync::OnceLock<PostgresTokenizer> = std::sync::OnceLock::new();

    match tokenizer {
        "hf" => HF_TOKENIZER.get_or_init(|| {
            Box::new(HFTokenizer::new(
                model.expect("Model path must be provided"),
            ))
        }),
        "jieba" => JIEBA_TOKENIZER.get_or_init(|| Box::new(JiebaTokenizer::new())),
        _ => panic!("Unknown tokenizer"),
    }
}
