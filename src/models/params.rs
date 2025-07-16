use crate::captchas;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Input {
    pub namespace: String,
    pub ttl_secs: Option<u64>,
    pub use_index: Option<bool>,
    #[serde(flatten)]
    pub choices_control: Option<ChoicesControl>,
    pub special_params: SpecialParams,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChoicesControl {
    // 启用候选项
    pub with_choices: Option<bool>,
    // 候选项个数
    pub choices_count: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SpecialParams {
    #[serde(rename = "classic")]
    Classic(captchas::classic::Params),
    #[serde(rename = "grid")]
    Grid(captchas::grid::Params),
    #[serde(rename = "image")]
    Image(captchas::image::Params),
}

pub mod verification {
    use crate::{
        err,
        errors::Error,
        models::{params::SpecialParams, payload::SpecialPayload},
    };
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    pub struct Input {
        pub unique_id: String,
        pub answer: Answer,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(tag = "type")]
    pub enum Answer {
        #[serde(rename = "classic")]
        Caassic(Classic),
        #[serde(rename = "grid")]
        Grid(Grid),
        #[serde(rename = "image")]
        Image(Image),
        #[serde[rename = "index"]]
        Index { value: usize },
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Classic {
        // 是否忽略大小写
        pub ignore_case: Option<bool>,
        // 答案文本
        pub text: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Grid {
        // 是否顺序无关
        pub unordered: Option<bool>,
        // 答案组成部分
        pub parts: Vec<usize>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Image {
        // 简体答案
        pub zh_hant: Option<String>,
        // 繁体答案
        pub zh_hans: Option<String>,
        // 英文答案
        pub en: Option<String>,
    }

    impl TryFrom<(&SpecialParams, &SpecialPayload)> for Answer {
        type Error = Error;

        fn try_from(
            (params, payload): (&SpecialParams, &SpecialPayload),
        ) -> Result<Answer, Self::Error> {
            match (params, payload) {
                (SpecialParams::Classic(params), SpecialPayload::Classic(payload)) => {
                    Ok(Answer::Caassic(Classic {
                        ignore_case: params
                            .verification_control
                            .as_ref()
                            .and_then(|v| v.ignore_case),
                        text: payload.text.clone(),
                    }))
                }
                (SpecialParams::Grid(params), SpecialPayload::Grid(payload)) => {
                    Ok(Answer::Grid(Grid {
                        unordered: params
                            .verification_control
                            .as_ref()
                            .and_then(|v| v.unordered),
                        parts: payload.parts.clone(),
                    }))
                }
                (SpecialParams::Image(_params), SpecialPayload::Image(payload)) => {
                    Ok(Answer::Image(Image {
                        zh_hans: payload.name.zh_hans.clone(),
                        zh_hant: payload.name.zh_hant.clone(),
                        en: payload.name.en.clone(),
                    }))
                }

                _ => err!("invalid params and payload combination"),
            }
        }
    }
}
