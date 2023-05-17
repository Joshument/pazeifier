use std::{io::Read, borrow::Cow, path::Path};

use crate::{Context, Error};

use image::{ImageBuffer, Rgba, DynamicImage, EncodableLayout};
use poise::serenity_prelude::AttachmentType;
use reqwest;

const PAZE_YELLOW: [u8; 4] = [0xFF, 0xC0, 0x00, 0xFF];
const PAZE_BLACK: [u8; 4] = [0x00, 0x00, 0x00, 0xFF];

/// Pazeify an image
#[poise::command(slash_command)]
pub async fn pazeify(
    ctx: Context<'_>,
    #[description = "Image to pazeify. Leave blank if you would rather use profile picture"]
    image: Option<poise::serenity_prelude::Attachment>,
    #[description = "The threshold of which an image is assigned a yellow or black pixel"]
    #[min = 0]
    #[max = 255]
    threshold: Option<u8>
) -> Result<(), Error> {
    ctx.defer().await?;

    let threshold = threshold.unwrap_or(128);
    let (width, height, image_buffer) = match image {
        Some(attatchment) => {
            let image_bytes = reqwest::get(attatchment.url).await?
                .bytes().await?;

            let avatar = image::load_from_memory(&image_bytes)?;
            let width = avatar.width();
            let height = avatar.height();
            let buffer = avatar.into_bytes();
            (width, height, buffer)
        },
        None => {
            let user = ctx.author();
            let avatar_url = user.avatar_url().expect("no avatar found!");
            let image_bytes = reqwest::get(avatar_url).await?
                .bytes().await?;

            let avatar = image::load_from_memory(&image_bytes)?;
            let width = avatar.width();
            let height = avatar.height();
            let buffer = avatar.into_bytes();
            (width, height, buffer)
        }
    };
    let bytes_per_pixel = image_buffer.len() / (width * height) as usize;
    println!("{bytes_per_pixel}");

    let mut pazeified_image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    // doesn't work
    // let image_type = image::guess_format(&image_buffer);
    // println!("{:#?}", image_type);
    let grayscale_image = match bytes_per_pixel {
        3 => DynamicImage::ImageRgb8(
            ImageBuffer::from_raw(width, height, image_buffer).expect("couldn't make image buffer")
        ),
        4 => DynamicImage::ImageRgba8(
            ImageBuffer::from_raw(width, height, image_buffer).expect("couldn't make image buffer")
        ),
        6 => {
            let casted_slice: &[u16] = bytemuck::try_cast_slice(&image_buffer).unwrap();
            DynamicImage::ImageRgb16(
                ImageBuffer::from_raw(width, height, casted_slice.to_vec()).expect("couldn't make image buffer")
            )
        },   
        _ => panic!("image is not 3-4 bytes per pixel")
    }.to_luma8();

    for (x, y, pixel) in grayscale_image.enumerate_pixels() {
        let coloured_pixel = if pixel[0] < threshold {
            Rgba(PAZE_YELLOW)
        } else {
            Rgba(PAZE_BLACK)
        };

        pazeified_image.put_pixel(x, y, coloured_pixel)
    }

    pazeified_image.save("pazeified.png")?;

    ctx.send(|m| m
        // .attachment(AttachmentType::Bytes { data, filename: "pazeified.png".to_string() })
        .attachment(AttachmentType::Path(Path::new("./pazeified.png")))
        .embed(|e| e
            .image("attachment://pazeified.png")
            .color(0xFFC000)
        )
    ).await?;
    Ok(())
}