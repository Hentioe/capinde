use super::namegen;
use crate::{
    captchas::{Created, calculate_center_crop_coordinates, check_out_base},
    err,
    errors::Result,
    fail,
    models::{params::ChoicesControl, payload::SpecialPayload},
    provider::{images_get, manifest::I18nName, random_right_with_wrongs},
};
use log::debug;
use magick_rust::{
    MagickWand,
    bindings::{
        DestroyPixelIterator, NewPixelIterator, PixelGetCurrentIteratorRow, PixelSetColor,
        PixelSetIteratorRow, PixelSyncIterator,
    },
    magick_wand_genesis,
};
use rand::{
    Rng, rng,
    seq::{IndexedRandom, SliceRandom},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Once};

const FALLBACK_DYNAMIC_DIGEST: bool = false;
const FALLBACK_WITH_CHOICES: bool = true;
const FALLBACK_CHOICES_COUNT: usize = 4;

#[derive(Debug, Clone, Deserialize)]
pub struct Params {
    // 图片宽度
    pub width: Option<usize>,
    // 图片高度
    pub height: Option<usize>,
    // 居中剪裁
    pub centered_crop: Option<bool>,
    // 动态摘要
    pub dynamic_digest: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    // 正确答案
    pub name: I18nName,
    // 候选项
    pub choices: Vec<I18nName>,
}

static MAGICK_START: Once = Once::new();

pub fn create(
    out_base: &str,
    choices_control: &ChoicesControl,
    params: &Params,
) -> Result<Created> {
    check_out_base(out_base)?;
    let with_choices = choices_control
        .with_choices
        .unwrap_or(FALLBACK_WITH_CHOICES);
    let dynamic_digest = params.dynamic_digest.unwrap_or(FALLBACK_DYNAMIC_DIGEST);
    let choices_count = if with_choices {
        choices_control
            .choices_count
            .unwrap_or(FALLBACK_CHOICES_COUNT)
    } else {
        1
    };

    // 获取随机图集
    let (ref right, wrongs) = random_right_with_wrongs(1, choices_count)?;
    let mut full = [vec![right.clone()], wrongs].concat();
    // 随机化图集顺序
    full.shuffle(&mut rand::rng());

    let mut right_index = 0;
    let mut choices = vec![];
    for (i, album) in full.into_iter().enumerate() {
        if album.id == right.id {
            // 如果是正确答案，则记录位置
            right_index = i;
        }
        // 将名称添加到候选项中
        choices.push(album.name.clone());
    }

    // 选择正确的图片（从数组中随机选择一个）
    let right_images = images_get(&right.id).ok_or(fail!("no album found: {}", right.id))?;
    let right_image = right_images
        .choose(&mut rand::rng())
        .ok_or(fail!("no images found for album {}", right.id))?;
    // 生成新的文件名
    let extension = right_image
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg");
    let file_name = format!("{}.{extension}", namegen());

    // 是否需要缩放
    let needs_resize = params.height.is_some() || params.width.is_some();
    // 是否需要处理（需要缩放或重写摘要）
    let needs_process = dynamic_digest || needs_resize;
    if needs_process {
        MAGICK_START.call_once(magick_wand_genesis);
        let wand = MagickWand::new();
        wand.read_image(right_image.to_str().ok_or(fail!("bad image path"))?)?;
        if needs_resize {
            // 获得缩放后的完整尺寸
            let (width, height) = calculate_proportional_size(
                wand.get_image_width(),
                wand.get_image_height(),
                params.width,
                params.height,
            )?;
            if params.centered_crop.unwrap_or(false) {
                // 居中剪裁
                let cropped = calculate_center_crop_coordinates(
                    wand.get_image_width(),
                    wand.get_image_height(),
                    width,
                    height,
                );
                wand.crop_image(
                    cropped.width,
                    cropped.height,
                    cropped.x as isize,
                    cropped.y as isize,
                )?;
            }
            // 缩放图像
            wand.resize_image(width, height, magick_rust::FilterType::Triangle)?;
        }
        if dynamic_digest {
            // 动态摘要（随机重写像素）
            let width = wand.get_image_width();
            let height = wand.get_image_height();
            // 生成随机行和列
            let mut rng = rng();
            let rand_row = rng.random_range(1..=height);
            let rand_col = rng.random_range(1..=width);
            unsafe {
                // 创建像素迭代器
                let iterator_ptr = NewPixelIterator(wand.wand);
                let row_width_ptr = &mut (1_usize) as *mut usize;
                // 设置当前的像素行
                PixelSetIteratorRow(iterator_ptr, (rand_row - 1) as isize);
                // 获取当前行的像素列表
                let pixels = std::slice::from_raw_parts_mut(
                    PixelGetCurrentIteratorRow(iterator_ptr, row_width_ptr),
                    rand_col,
                );
                // 设置列中的随机像素为黑色
                PixelSetColor(
                    pixels[rand_col - 1],
                    c"#000000".as_ptr() as *const std::ffi::c_char,
                );
                // 同步像素迭代器
                PixelSyncIterator(iterator_ptr);
                // 销毁像素迭代器
                DestroyPixelIterator(iterator_ptr);
            };
        }
        let out_file = PathBuf::from(out_base).join(&file_name);
        wand.write_image(out_file.to_str().ok_or(fail!("bad out file path"))?)?;
    } else {
        // 如果不涉及任何图像处理，则直接复制文件
        debug!(
            "Copying image without processing: {}",
            right_image.display()
        );
        let out_file = PathBuf::from(out_base).join(&file_name);
        std::fs::copy(right_image, out_file)?;
    }

    Ok(Created {
        file_name,
        right_index,
        payload: SpecialPayload::Image(Payload {
            name: right.name.clone(),
            choices,
        }),
    })
}

pub fn calculate_proportional_size(
    original_width: usize,
    original_height: usize,
    target_width: Option<usize>,
    target_height: Option<usize>,
) -> Result<(usize, usize)> {
    // 检查原始尺寸是否有效
    if original_width == 0 || original_height == 0 {
        return err!("original dimensions cannot be zero");
    }

    match (target_width, target_height) {
        // 只指定了目标宽度，按比例计算高度
        (Some(width), None) => {
            if width == 0 {
                return err!("target width cannot be zero");
            }
            let height =
                (width as f64 * original_height as f64 / original_width as f64).round() as usize;
            Ok((width, height))
        }

        // 只指定了目标高度，按比例计算宽度
        (None, Some(height)) => {
            if height == 0 {
                return err!("target height cannot be zero");
            }
            let width =
                (height as f64 * original_width as f64 / original_height as f64).round() as usize;
            Ok((width, height))
        }

        // 同时指定了宽度和高度
        (Some(width), Some(height)) => {
            if width == 0 || height == 0 {
                return err!("target dimensions cannot be zero");
            }
            // 直接返回指定的尺寸（不保持比例）
            Ok((width, height))
        }

        // 没有指定任何目标尺寸
        (None, None) => err!("no target dimensions specified"),
    }
}
