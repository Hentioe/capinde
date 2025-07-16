use super::namegen;
use crate::{
    captchas::{Created, calculate_center_crop_coordinates, check_out_base},
    err,
    errors::{Error, Result},
    fail,
    models::{params::ChoicesControl, payload::SpecialPayload},
    provider::{images_get, manifest::I18nName, random_right_with_wrongs},
};
use magick_rust::{CompositeOperator, DrawingWand, MagickWand, PixelWand, magick_wand_genesis};
use rand::seq::{IndexedRandom, SliceRandom};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf, sync::Once};

const FALLBACK_WATERMARK_FONT_WEIGHT: usize = 600;
const FALLBACK_RIGHT_COUNT: usize = 3;
const FALLBACK_WITH_CHOICES: bool = false;
const FALLBACK_CHOICES_COUNT: usize = 5;
const FALLBACK_UNORDERED_RIGHT_PARTS: bool = false;

#[derive(Debug, Clone, Deserialize)]
pub struct Params {
    // 布局
    pub layout: String,
    // 单元格宽度
    pub cell_width: usize,
    // 单元格高度
    pub cell_height: usize,
    // 居中剪裁
    pub centered_crop: Option<bool>,
    // 水印字体家族
    pub watermark_font_family: String,
    // 水印字体大小
    pub watermark_font_size: Option<f64>,
    // 水印字体粗细
    pub watermark_font_weight: Option<usize>,
    // 正确选项个数
    pub right_count: Option<usize>,
    // 是否无序 right_parts
    pub unordered_right_parts: Option<bool>,
    // 验证控制
    pub verification_control: Option<VerifyControl>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VerifyControl {
    pub unordered: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Payload {
    pub parts: Vec<usize>,
    pub subject: I18nName,
    pub choices: Vec<Vec<usize>>,
    #[serde(skip_serializing)]
    pub images: Vec<PathBuf>,
}

static MAGICK_START: Once = Once::new();

pub fn create(
    out_base: &str,
    choices_control: &ChoicesControl,
    params: &Params,
) -> Result<Created> {
    check_out_base(out_base)?;
    MAGICK_START.call_once(magick_wand_genesis);
    let payload = make(
        &params.layout,
        params.right_count.unwrap_or(FALLBACK_RIGHT_COUNT),
        choices_control
            .with_choices
            .unwrap_or(FALLBACK_WITH_CHOICES),
        choices_control
            .choices_count
            .unwrap_or(FALLBACK_CHOICES_COUNT),
        params
            .unordered_right_parts
            .unwrap_or(FALLBACK_UNORDERED_RIGHT_PARTS),
    )?;
    let watermark_font_size = params
        .watermark_font_size
        .unwrap_or(calculate_watermark_font_size(
            params.cell_width,
            params.cell_height,
        ));
    let watermark_font_weight = params
        .watermark_font_weight
        .unwrap_or(FALLBACK_WATERMARK_FONT_WEIGHT);

    let mut wand = MagickWand::new();
    wand.new_image(
        params.cell_width * 3,
        params.cell_height * 3,
        &PixelWand::new(),
    )?;
    let mut wands = vec![];
    for (i, image) in payload.images.iter().enumerate() {
        let mut wand = MagickWand::new();
        wand.read_image(image.to_str().ok_or(fail!("bad image path"))?)?;
        if params.centered_crop.unwrap_or(false) {
            // 居中剪裁
            let cropped = calculate_center_crop_coordinates(
                wand.get_image_width(),
                wand.get_image_height(),
                params.cell_width,
                params.cell_height,
            );
            wand.crop_image(
                cropped.width,
                cropped.height,
                cropped.x as isize,
                cropped.y as isize,
            )?;
            wand.reset_image_page("")?; // 重置图像页面（因为剪裁改变了虚拟画布，会影响后续的水印定位）
        }
        // 缩放图片到固定大小
        wand.resize_image(
            params.cell_width,
            params.cell_height,
            magick_rust::FilterType::Triangle,
        )?;
        let mut draw = DrawingWand::new();
        let mut fill = PixelWand::new();
        let mut border = PixelWand::new();
        // 设置水印颜色和透明度
        fill.set_color("white")?;
        fill.set_alpha(0.45);
        // 设置水印边框颜色
        border.set_color("black")?;
        // 设置水印的字体家族、大小、粗细、颜色
        draw.set_font_family(&params.watermark_font_family)?;
        draw.set_font_size(watermark_font_size);
        draw.set_font_weight(watermark_font_weight);
        // 设置字体为斜体
        draw.set_font_style(magick_rust::StyleType::Italic);
        draw.set_fill_color(&fill);
        // 设置水印的边框颜色和宽度
        draw.set_stroke_color(&border);
        draw.set_stroke_width(1.0);
        // 绘制水印和位置
        draw.draw_annotation(1.0, watermark_font_size, &(i + 1).to_string())?;
        wand.draw_image(&draw)?;
        wands.push(wand);
    }

    wand.set_format("jpg")?;

    for (i, photo_wand) in wands.iter().enumerate() {
        // todo: 此处未来将根据布局动态计算
        let x = ((i % 3) * params.cell_width) as isize;
        let y = ((i / 3) * params.cell_height) as isize;

        wand.compose_images(photo_wand, CompositeOperator::Over, true, x, y)?;
    }

    let file_name = format!("{}.jpg", namegen());
    let out_file = PathBuf::from(out_base).join(&file_name);

    wand.write_image(out_file.to_str().ok_or(fail!("bad out file path"))?)?;

    // 找出正确答案索引
    let mut right_index = 0;
    for (i, choice) in payload.choices.iter().enumerate() {
        if choice == &payload.parts {
            right_index = i;
            break;
        }
    }

    Ok(Created {
        file_name,
        right_index,
        payload: SpecialPayload::Grid(payload),
    })
}

fn make(
    layout: &str,
    right_count: usize,
    with_choices: bool,
    choices_count: usize,
    unordered_right_parts: bool,
) -> Result<Payload> {
    let mut rng = rand::rng();
    let images_count = layout_to_count(layout)?;
    let (right, wrongs) = random_right_with_wrongs(right_count, images_count)?;
    let right_images = images_get(&right.id).ok_or(fail!("the correct album was not found"))?;

    let right_images = right_images
        .choose_multiple(&mut rng, right_count)
        .cloned()
        .collect::<Vec<_>>();

    let mut full = vec![];
    for album in wrongs.iter() {
        if full.len() >= images_count - right_count {
            break;
        }
        if let Some(images) = images_get(&album.id)
            && !images.is_empty()
        {
            if let Some(image) = images.choose(&mut rng).cloned() {
                full.push(image);
            }
        }
    }

    full.append(&mut right_images.clone());
    full.shuffle(&mut rng);

    let mut parts = vec![];
    for (i, image) in full.iter().enumerate() {
        if right_images.contains(image) {
            parts.push(i + 1);
        }
    }

    if unordered_right_parts {
        parts.shuffle(&mut rng);
    }

    let choices: Vec<Vec<usize>> = if with_choices {
        // 将正确答案添加到选择中并打乱
        let mut choices = generate_different_parts(choices_count - 1, &parts, 1, images_count)?;
        choices.push(parts.clone());
        choices.shuffle(&mut rng);

        choices
    } else {
        vec![]
    };

    Ok(Payload {
        parts,
        subject: right.name.clone(),
        choices,
        images: full,
    })
}

fn layout_to_count(layout: &str) -> Result<usize> {
    if layout == "3x3" {
        Ok(9)
    } else {
        Err(Error::InvalidGridLayout(layout.to_string()))
    }
}

fn generate_different_parts(
    count: usize,
    right_parts: &[usize],
    range_start: usize,
    range_end: usize,
) -> Result<Vec<Vec<usize>>> {
    let target_len = right_parts.len();
    let right_set: HashSet<usize> = right_parts.iter().cloned().collect();

    // 生成所有可能的数字
    let all_numbers: Vec<usize> = (range_start..range_end).collect();

    if all_numbers.len() < target_len {
        // 如果范围内的数字不够生成一个完整的 Vec
        return err!("range too small");
    }

    let mut result = Vec::new();
    let mut used_sets = Vec::new();
    used_sets.push(right_set.clone());

    let mut rng = rand::rng();
    let mut attempts = 0;
    let max_attempts = count * 1000; // 防止无限循环

    while result.len() < count && attempts < max_attempts {
        attempts += 1;

        // 随机选择 target_len 个不重复的数字
        let mut shuffled = all_numbers.clone();
        shuffled.shuffle(&mut rng);
        let candidate: Vec<usize> = shuffled.into_iter().take(target_len).collect();
        let candidate_set: HashSet<usize> = candidate.iter().cloned().collect();

        // 检查是否与已使用的集合重复
        if !used_sets.contains(&candidate_set) {
            used_sets.push(candidate_set);
            result.push(candidate);
        }
    }

    Ok(result)
}

/// 根据宽和高计算字体大小（宽/高最小值的 0.45）
fn calculate_watermark_font_size(width: usize, height: usize) -> f64 {
    let min_dimension = width.min(height);
    (min_dimension as f64) * 0.45
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        crate::provider::init();
    }

    #[test]
    fn test_create() {
        setup();

        let choices_control = ChoicesControl {
            with_choices: Some(true),
            choices_count: Some(4),
        };
        let params = Params {
            layout: String::from("3x3"),
            cell_width: 180,
            cell_height: 140,
            centered_crop: None,
            watermark_font_family: String::from("Open Sans"),
            watermark_font_size: None,
            watermark_font_weight: Some(600),
            right_count: None,
            unordered_right_parts: None,
            verification_control: None,
        };

        let _ = create("namespace/out", &choices_control, &params).unwrap();
    }

    #[test]
    fn test_make() {
        setup();

        let payload = make("3x3", 3, true, 4, false).unwrap();

        assert_eq!(payload.parts.len(), 3);
        // 测试 right_parts 是否有序
        assert!(payload.parts.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(payload.images.len(), 9);
        assert_eq!(payload.choices.len(), 4);
        assert!(payload.choices.iter().all(|c| c.len() == 3));
    }
}
