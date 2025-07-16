use crate::{errors::Result, fail, models::payload::SpecialPayload};

pub mod classic;
pub mod grid;
pub mod image;

pub struct Created {
    pub file_name: String,
    pub right_index: usize,
    pub payload: SpecialPayload,
}

pub fn idgen() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn namegen() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn check_out_base(out_base: &str) -> Result<()> {
    let path = std::path::PathBuf::from(out_base);
    if !path.exists() {
        std::fs::create_dir_all(&path)
            .map_err(|e| fail!("failed to create output directory: {e}"))?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Cropped {
    pub x: isize,
    pub y: isize,
    pub width: usize,
    pub height: usize,
}

pub fn calculate_center_crop_coordinates(
    original_width: usize,
    original_height: usize,
    target_width: usize,
    target_height: usize,
) -> Cropped {
    // 计算目标宽高比
    let target_ratio = target_width as f64 / target_height as f64;

    // 根据目标比例计算在原始图像中的最大可能剪裁尺寸
    let (width, height) = if original_width as f64 / original_height as f64 > target_ratio {
        // 原始图像更宽，高度受限
        let crop_height = original_height;
        let crop_width = (crop_height as f64 * target_ratio) as usize;
        (crop_width, crop_height)
    } else {
        // 原始图像更高或比例相同，宽度受限
        let crop_width = original_width;
        let crop_height = (crop_width as f64 / target_ratio) as usize;
        (crop_width, crop_height)
    };

    // 计算居中剪裁的起始坐标
    let x = ((original_width - width) / 2) as isize;
    let y = ((original_height - height) / 2) as isize;

    Cropped {
        x,
        y,
        width,
        height,
    }
}
