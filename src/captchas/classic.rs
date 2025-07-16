use crate::{
    captchas::{Created, check_out_base, namegen},
    errors::Result,
    fail,
    models::{params::ChoicesControl, payload::SpecialPayload},
};
use captcha_rs::CaptchaBuilder;
use rand::{Rng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

const FALLBACK_LENGTH: usize = 5;
const FALLBACK_WIDTH: u32 = 130;
const FALLBACK_HEIGHT: u32 = 40;
const FALLBACK_DARK_MODE: bool = false;
const FALLBACK_COMPLEXITY: u32 = 5; // min: 1, max: 10
const FALLBACK_COMPRESSION: u8 = 40; // min: 1, max: 99
const FALLBACK_WITH_CHOICES: bool = false;
const FALLBACK_CHOICES_COUNT: usize = 6;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Params {
    pub length: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub dark_mode: Option<bool>,
    pub complexity: Option<u32>,
    pub compression: Option<u8>,
    // 验证控制
    pub verification_control: Option<VerifyControl>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VerifyControl {
    pub ignore_case: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Payload {
    pub text: String,
    pub choices: Vec<String>,
}

pub fn create(
    out_base: &str,
    choices_control: &ChoicesControl,
    params: &Params,
) -> Result<Created> {
    check_out_base(out_base)?;
    let captcha = CaptchaBuilder::new()
        .length(params.length.unwrap_or(FALLBACK_LENGTH))
        .width(params.width.unwrap_or(FALLBACK_WIDTH))
        .height(params.height.unwrap_or(FALLBACK_HEIGHT))
        .dark_mode(params.dark_mode.unwrap_or(FALLBACK_DARK_MODE))
        .complexity(params.complexity.unwrap_or(FALLBACK_COMPLEXITY)) // min: 1, max: 10
        .compression(params.compression.unwrap_or(FALLBACK_COMPRESSION)) // min: 1, max: 99
        .build();

    let file_name = format!("{}.jpg", namegen());
    let out_file = PathBuf::from(out_base).join(&file_name);
    captcha
        .image
        .save(&out_file)
        .map_err(|e| fail!("failed to save captcha image: {}", e))?;

    let choices = if choices_control
        .with_choices
        .unwrap_or(FALLBACK_WITH_CHOICES)
    {
        let choices_count = choices_control
            .choices_count
            .unwrap_or(FALLBACK_CHOICES_COUNT);
        let mut choices = generate_different_texts(choices_count - 1, &captcha.text);
        choices.push(captcha.text.clone());
        choices.shuffle(&mut rand::rng());

        choices
    } else {
        vec![]
    };

    let mut right_index = 0;
    for (i, choice) in choices.iter().enumerate() {
        if choice == &captcha.text {
            right_index = i;
            break;
        }
    }

    Ok(Created {
        file_name,
        right_index,
        payload: SpecialPayload::Classic(Payload {
            text: captcha.text,
            choices,
        }),
    })
}

/// 生成与正确答案不同但相似的候选项。
/// 注意：由于每一个候选项和正确答案只有一个字符之差，通常个数是很有限的。
fn generate_different_texts(count: usize, right_text: &str) -> Vec<String> {
    // 建立字符相似性映射
    // todo: 将相似性映射提取到静态常量中
    let similarity_map: HashMap<char, Vec<char>> = [
        ('0', vec!['O', 'Q', 'D', '8']),
        ('1', vec!['I', 'l', '|', 'i']),
        ('2', vec!['Z', 'z']),
        ('3', vec!['8', 'B']),
        ('4', vec!['A', 'h']),
        ('5', vec!['S', 's']),
        ('6', vec!['G', 'b']),
        ('7', vec!['T', 'Y']),
        ('8', vec!['B', '3', '0']),
        ('9', vec!['g', 'q']),
        ('A', vec!['4', 'R']),
        ('B', vec!['8', '3', 'R']),
        ('C', vec!['G', 'O']),
        ('D', vec!['O', '0', 'B']),
        ('E', vec!['F', '3']),
        ('F', vec!['E', 'P']),
        ('G', vec!['6', 'C', 'O']),
        ('H', vec!['N', 'M']),
        ('I', vec!['1', 'l', '|']),
        ('J', vec!['i', '1']),
        ('K', vec!['X', 'R']),
        ('L', vec!['1', 'I']),
        ('M', vec!['N', 'H']),
        ('N', vec!['M', 'H']),
        ('O', vec!['0', 'Q', 'D']),
        ('P', vec!['R', 'F']),
        ('Q', vec!['O', '0', 'G']),
        ('R', vec!['P', 'B']),
        ('S', vec!['5', 's']),
        ('T', vec!['7', 'Y']),
        ('U', vec!['V', 'Y']),
        ('V', vec!['U', 'Y']),
        ('W', vec!['M', 'N']),
        ('X', vec!['K', 'Y']),
        ('Y', vec!['V', 'T']),
        ('Z', vec!['2', 'z']),
        ('a', vec!['o', 'e']),
        ('b', vec!['6', 'h']),
        ('c', vec!['e', 'o']),
        ('d', vec!['b', 'o']),
        ('e', vec!['c', 'a']),
        ('f', vec!['t', 'r']),
        ('g', vec!['9', 'q']),
        ('h', vec!['b', 'n']),
        ('i', vec!['1', 'l', 'j']),
        ('j', vec!['i', '1']),
        ('k', vec!['x', 'r']),
        ('l', vec!['1', 'I', 'i']),
        ('m', vec!['n', 'h']),
        ('n', vec!['m', 'h']),
        ('o', vec!['0', 'a', 'e']),
        ('p', vec!['q', 'b']),
        ('q', vec!['p', 'g']),
        ('r', vec!['n', 'f']),
        ('s', vec!['5', 'S']),
        ('t', vec!['f', 'r']),
        ('u', vec!['v', 'o']),
        ('v', vec!['u', 'y']),
        ('w', vec!['m', 'n']),
        ('x', vec!['k', 'y']),
        ('y', vec!['v', 'x']),
        ('z', vec!['2', 'Z']),
    ]
    .iter()
    .cloned()
    .collect();

    let mut rng = rand::rng();
    let mut results = Vec::new();
    let chars: Vec<char> = right_text.chars().collect();
    let mut max_attempts = 0; // 防止无限循环

    while results.len() < count && max_attempts < count * 1000 {
        max_attempts += 1;
        let mut new_chars = chars.clone();

        // 随机选择一个位置进行修改
        let pos: usize = rng.random_range(0..chars.len());
        let original_char = chars[pos];

        // 尝试找到相似的字符
        if let Some(similar_chars) = similarity_map.get(&original_char) {
            if !similar_chars.is_empty() {
                let random_idx = rng.random_range(0..similar_chars.len());
                new_chars[pos] = similar_chars[random_idx];
            }
        } else {
            // 如果没有相似映射，则随机替换为其他字符
            // todo: 将替换字符提取到静态常量中
            let replacement_chars = [
                '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
                'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V',
                'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
                'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
            ];

            // 确保不选择原字符
            let available_chars: Vec<char> = replacement_chars
                .iter()
                .filter(|&&c| c != original_char)
                .copied()
                .collect();

            if !available_chars.is_empty() {
                let random_idx = rng.random_range(0..available_chars.len());
                new_chars[pos] = available_chars[random_idx];
            }
        }

        let new_string: String = new_chars.iter().collect();

        // 确保生成的字符串不同于原字符串且不重复
        if new_string != right_text && !results.contains(&new_string) {
            results.push(new_string);
        }
    }

    results
}
