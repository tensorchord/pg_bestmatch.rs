use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

struct WhitespaceTokenizer;

struct HFTokenizer {
    tokenizer: tokenizers::Tokenizer,
}

struct JiebaTokenizer {
    jeiba: jieba_rs::Jieba,
}

struct TiniestsegmenterTokenizer;

impl JiebaTokenizer {
    fn new() -> JiebaTokenizer {
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

impl Tokenize for WhitespaceTokenizer {}

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

impl Tokenize for TiniestsegmenterTokenizer {
    fn tokenize(&self, s: &str) -> Vec<String> {
        tiniestsegmenter::tokenize(s)
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

type PostgresTokenizer = Box<dyn Tokenize + Sync + Send>;

pub fn tokenize(tokenizer: &str, model: Option<&str>, s: &str) -> Vec<String> {
    static HF_TOKENIZER: OnceLock<Mutex<HashMap<String, PostgresTokenizer>>> = OnceLock::new();
    static JIEBA_TOKENIZER: OnceLock<PostgresTokenizer> = OnceLock::new();
    static TINIESTSEGMENTER_TOKENIZER: OnceLock<PostgresTokenizer> = OnceLock::new();
    static WHITESPACE_TOKENIZER: OnceLock<PostgresTokenizer> = OnceLock::new();

    let selected_tokenizer = match tokenizer {
        "ws" => WHITESPACE_TOKENIZER.get_or_init(|| Box::new(WhitespaceTokenizer)),
        "hf" => {
            let selected_model = model.expect("model must be provided for hf tokenizer");
            let mut hf_lock = HF_TOKENIZER
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .expect("couldn't lock mutex");

            return hf_lock
                .entry(selected_model.to_string())
                .or_insert_with(|| Box::new(HFTokenizer::new(selected_model)))
                .tokenize(s);
        }
        "jieba" => JIEBA_TOKENIZER.get_or_init(|| Box::new(JiebaTokenizer::new())),
        "tiniestsegmenter" => {
            TINIESTSEGMENTER_TOKENIZER.get_or_init(|| Box::new(TiniestsegmenterTokenizer))
        }
        _ => panic!("Unknown tokenizer"),
    };

    selected_tokenizer.tokenize(s)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_whitespace_tokenizer() {
        assert_eq!(
            *super::tokenize("ws", None, "i have an apple"),
            vec!["i", "have", "an", "apple"]
        );
    }

    #[test]
    fn test_hftokenizer() {
        assert_eq!(
            super::tokenize("hf", Some("bert-base-uncased"), "i have an apple"),
            vec!["i", "have", "an", "apple"]
        );

        assert_eq!(
            super::tokenize("hf", Some("google-t5/t5-base"), "i have an apple"),
            vec!["▁", "i", "▁have", "▁an", "▁apple"]
        );

        assert_eq!(
            super::tokenize("hf", Some("bert-base-uncased"), "i have an apple"),
            vec!["i", "have", "an", "apple"]
        );
    }

    #[test]
    fn test_jieba() {
        assert_eq!(
            super::tokenize("jieba", None, "测试版本将于秋季推出。"),
            vec!["测试", "版本", "将", "于", "秋季", "推出", "。"]
        );
    }

    #[test]
    fn test_tiniestsegmenter() {
        assert_eq!(
            super::tokenize(
                "tiniestsegmenter",
                None,
                "今作の主人公はリンクではなくゼルダ姫"
            ),
            vec![
                "今作",
                "の",
                "主人",
                "公",
                "は",
                "リンク",
                "で",
                "は",
                "なく",
                "ゼルダ",
                "姫"
            ]
        );
    }
}
