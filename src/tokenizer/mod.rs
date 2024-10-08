use std::{
    collections::{HashMap, HashSet},
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

struct TiktokenTokenizer {
    tokenizer: tiktoken_rs::CoreBPE,
}

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

impl TiktokenTokenizer {
    fn new(model: &str) -> TiktokenTokenizer {
        let selected_model = tiktoken_rs::tokenizer::get_tokenizer(model)
            .map(|tokenizer| match tokenizer {
                tiktoken_rs::tokenizer::Tokenizer::O200kBase => "o200k_base",
                tiktoken_rs::tokenizer::Tokenizer::Cl100kBase => "cl100k_base",
                tiktoken_rs::tokenizer::Tokenizer::P50kBase => "p50k_base",
                tiktoken_rs::tokenizer::Tokenizer::P50kEdit => "p50k_edit",
                tiktoken_rs::tokenizer::Tokenizer::R50kBase => "r50k_base",
                tiktoken_rs::tokenizer::Tokenizer::Gpt2 => "gpt2",
            })
            .unwrap_or(model);

        TiktokenTokenizer {
            tokenizer: match selected_model {
                "o200k_base" => tiktoken_rs::o200k_base().unwrap(),
                "cl100k_base" => tiktoken_rs::cl100k_base().unwrap(),
                "p50k_base" => tiktoken_rs::p50k_base().unwrap(),
                "p50k_edit" => tiktoken_rs::p50k_edit().unwrap(),
                "r50k_base" | "gpt2" => tiktoken_rs::r50k_base().unwrap(),
                _ => panic!("Unknown model"),
            },
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

impl Tokenize for TiktokenTokenizer {
    fn tokenize(&self, s: &str) -> Vec<String> {
        self.tokenizer
            .encode(s, HashSet::new())
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
}

type PostgresTokenizer = Box<dyn Tokenize + Sync + Send>;
type MultiOL = OnceLock<Mutex<HashMap<String, PostgresTokenizer>>>;
type SingleOL = OnceLock<PostgresTokenizer>;

static HF_TOKENIZER: MultiOL = OnceLock::new();
static TIKTOKEN_TOKENIZER: MultiOL = OnceLock::new();
static JIEBA_TOKENIZER: SingleOL = OnceLock::new();
static TINIESTSEGMENTER_TOKENIZER: SingleOL = OnceLock::new();
static WHITESPACE_TOKENIZER: SingleOL = OnceLock::new();

fn _hashmap_tokenize<T>(
    lock: &MultiOL,
    model: &str,
    new_fn: impl Fn(&str) -> T,
    s: &str,
) -> Vec<String>
where
    T: Tokenize + Sync + Send + 'static,
{
    let mut lock_guard = lock
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        // on panic, the mutex gets poisoned, so we need a way to handle it.
        .unwrap_or_else(|e: std::sync::PoisonError<_>| e.into_inner());

    lock_guard
        .entry(model.to_string())
        .or_insert_with(|| Box::new(new_fn(model)))
        .tokenize(s)
}

pub fn tokenize(tokenizer: &str, model: Option<&str>, s: &str) -> Vec<String> {
    let selected_tokenizer = match tokenizer {
        "hf" => {
            let selected_model = model.expect("model must be provided for hf tokenizer");
            return _hashmap_tokenize(&HF_TOKENIZER, selected_model, HFTokenizer::new, s);
        }
        "tiktoken" => {
            let selected_model = model.expect("model or encoding must be provided");
            return _hashmap_tokenize(
                &TIKTOKEN_TOKENIZER,
                selected_model,
                TiktokenTokenizer::new,
                s,
            );
        }
        "ws" => WHITESPACE_TOKENIZER.get_or_init(|| Box::new(WhitespaceTokenizer)),
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

    #[test]
    fn test_tiktoken() {
        // Test the encodings first

        // o200k_base
        assert_eq!(
            super::tokenize("tiktoken", Some("o200k_base"), "i want an apple"),
            vec!["72", "1682", "448", "30366"]
        );

        // cl100k_base
        assert_eq!(
            super::tokenize("tiktoken", Some("cl100k_base"), "i want an apple"),
            vec!["72", "1390", "459", "24149"]
        );

        // p50k_base
        assert_eq!(
            super::tokenize("tiktoken", Some("p50k_base"), "i want an apple"),
            vec!["72", "765", "281", "17180"]
        );

        // p50k_edit
        assert_eq!(
            super::tokenize("tiktoken", Some("p50k_edit"), "i want an apple"),
            vec!["72", "765", "281", "17180"]
        );

        // r50k_base
        assert_eq!(
            super::tokenize("tiktoken", Some("r50k_base"), "i want an apple"),
            vec!["72", "765", "281", "17180"]
        );

        //gpt2
        assert_eq!(
            super::tokenize("tiktoken", Some("gpt2"), "i want an apple"),
            vec!["72", "765", "281", "17180"]
        );
    }

    // panic

    #[test]
    #[should_panic]
    fn test_tiktoken_panic() {
        super::tokenize("tiktoken", Some("foo"), "i want an apple");
    }
}
