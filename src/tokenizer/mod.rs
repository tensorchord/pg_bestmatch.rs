use std::sync::OnceLock;

pub struct WhitespaceTokenizer;

pub struct HFTokenizer {
    tokenizer: tokenizers::Tokenizer,
}

pub struct JiebaTokenizer {
    jeiba: jieba_rs::Jieba,
}

pub struct TiniestsegmenterTokenizer;

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

pub fn get_tokenizer(tokenizer: &str, model: Option<&str>) -> &'static PostgresTokenizer {
    static HF_TOKENIZER: OnceLock<PostgresTokenizer> = OnceLock::new();
    static JIEBA_TOKENIZER: OnceLock<PostgresTokenizer> = OnceLock::new();
    static TINIESTSEGMENTER_TOKENIZER: OnceLock<PostgresTokenizer> = OnceLock::new();
    static WHITESPACE_TOKENIZER: OnceLock<PostgresTokenizer> = OnceLock::new();

    match tokenizer {
        "ws" => WHITESPACE_TOKENIZER.get_or_init(|| Box::new(WhitespaceTokenizer)),
        "hf" => HF_TOKENIZER.get_or_init(|| {
            Box::new(HFTokenizer::new(
                model.expect("Model path must be provided"),
            ))
        }),
        "jieba" => JIEBA_TOKENIZER.get_or_init(|| Box::new(JiebaTokenizer::new())),
        "tiniestsegmenter" => {
            TINIESTSEGMENTER_TOKENIZER.get_or_init(|| Box::new(TiniestsegmenterTokenizer))
        }
        _ => panic!("Unknown tokenizer"),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_whitespace_tokenizer() {
        assert_eq!(
            super::get_tokenizer("ws", None).tokenize("i have an apple"),
            vec!["i", "have", "an", "apple"]
        );
    }

    #[test]
    fn test_hftokenizer() {
        assert_eq!(
            super::get_tokenizer("hf", Some("bert-base-uncased")).tokenize("i have an apple"),
            vec!["i", "have", "an", "apple"]
        );
    }

    #[test]
    fn test_jieba() {
        assert_eq!(
            super::get_tokenizer("jieba", None).tokenize("测试版本将于秋季推出。"),
            vec!["测试", "版本", "将", "于", "秋季", "推出", "。"]
        );
    }

    #[test]
    fn test_tiniestsegmenter() {
        assert_eq!(
            super::get_tokenizer("tiniestsegmenter", None)
                .tokenize("今作の主人公はリンクではなくゼルダ姫"),
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
