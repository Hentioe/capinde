mod store;

use log::warn;
pub use store::{add_cache, cleanup_expired, queue_size};

use crate::models::params::verification::Answer::{self, Caassic, Grid, Image, Index};

pub async fn verify(unique_id: &str, answer: &Answer) -> Option<bool> {
    let cached = match store::get_cache(unique_id).await {
        Some(cached_answer) => cached_answer,
        None => return None,
    };

    let is_right = match (&cached, answer) {
        (Caassic(cached), Caassic(answer)) => {
            let ignore_case = if let Some(required_ignore_case) = answer.ignore_case {
                required_ignore_case
            } else {
                cached.ignore_case.unwrap_or(false)
            };

            if ignore_case {
                // 忽略大小写比较
                answer.text.eq_ignore_ascii_case(&cached.text)
            } else {
                // 精确比较
                answer.text == cached.text
            }
        }
        (Grid(cached), Grid(answer)) => {
            let unordered = if let Some(required_unordered) = answer.unordered {
                required_unordered
            } else {
                cached.unordered.unwrap_or(false)
            };

            if unordered {
                // 无序比较
                let mut right_parts = cached.parts.clone();
                let mut answer_parts = answer.parts.clone();
                right_parts.sort_unstable();
                answer_parts.sort_unstable();

                right_parts == answer_parts
            } else {
                // 有序比较
                cached.parts == answer.parts
            }
        }
        (Image(cached), Image(answer)) => {
            // 按照简体、繁体、英文的顺序，某个语言答案存在就比较该语言答案
            if answer.zh_hans.is_some() && cached.zh_hans.is_some() {
                answer.zh_hans == cached.zh_hans
            } else if answer.zh_hant.is_some() && cached.zh_hant.is_some() {
                answer.zh_hant == cached.zh_hant
            } else if answer.en.is_some() && cached.en.is_some() {
                answer.en == cached.en
            } else {
                false
            }
        }
        (Index { value: right }, Index { value: answer }) => right == answer,
        _ => {
            warn!("Cached answer type mismatch: expected {cached:?}, got {answer:?}");

            false
        }
    };

    Some(is_right)
}
