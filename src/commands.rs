use std::{borrow::Cow, ffi::OsStr, fs::File, io::{Read, Cursor}, path::Path};

use crate::{Context, Error};

use gif::{Decoder, Encoder, Frame};
use image::{DynamicImage, EncodableLayout, ImageBuffer, Luma, Rgba, Pixel, Rgb};
use poise::serenity_prelude::AttachmentType;
use reqwest;

pub const PAZE_YELLOW: [u8; 3] = [0xFF, 0xC0, 0x00];
pub const PAZE_BLACK: [u8; 3] = [0x00, 0x00, 0x00];

#[derive(thiserror::Error, Debug, Clone)]
pub enum CommandError {
    #[error("Invalid extension {0}")]
    InvalidExtension(String),
}

pub enum PazeifierType {
    Gif(Decoder<Cursor<Vec<u8>>>),
    Image(DynamicImage)
}

/// Pazeify an image
#[poise::command(slash_command)]
pub async fn pazeify(
    ctx: Context<'_>,
    #[description = "File to pazeify. Leave blank if you would rather use profile picture"]
    file: Option<poise::serenity_prelude::Attachment>,
    #[description = "The threshold of which an image is assigned a yellow or black pixel. Deafault is 128"]
    #[min = 0]
    #[max = 255]
    threshold: Option<u8>,
    #[description = "Whether or not the image is to have inverted colours. Yellow = Black, Black = Yellow."]
    inverted: Option<bool>
) -> Result<(), Error> {
    ctx.defer().await?;

    let threshold = threshold.unwrap_or(128);
    let inverted = inverted.unwrap_or(false);
    match file {
        Some(attatchment) => {
            println!("{}", attatchment.filename);
            let file_name = &attatchment.filename;
            let file_extension = Path::new(&file_name)
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("unknown");

            let buffer = attatchment.download().await?;
            match file_extension {
                "gif" => {
                    handle_gif(ctx, Decoder::new(Cursor::new(buffer))?, threshold, inverted).await
                },
                "png" | "jpg" | "jpeg" | "webp" | "avif" => {
                    handle_image(ctx, image::load_from_memory(&buffer)?, threshold, inverted).await
                },
                extension => {
                    return Err(Box::new(CommandError::InvalidExtension(
                        extension.to_string(),
                    )))
                }
            }
        }
        None => {
            let user = ctx.author();
            let avatar_url = user.avatar_url().expect("no avatar found!");
            let static_avatar_url = user.static_avatar_url().expect("no avatar found!");

            if avatar_url == static_avatar_url {
                let buffer = reqwest::get(static_avatar_url).await?.bytes().await?;
                handle_image(ctx, image::load_from_memory(&buffer)?, threshold, inverted).await
            } else {
                let buffer = Vec::from(reqwest::get(avatar_url).await?.bytes().await?);
                handle_gif(ctx, Decoder::new(Cursor::new(buffer))?, threshold, inverted).await
            }


            // let image_bytes = reqwest::get(avatar_url).await?.bytes().await?;

            // PazeifierType::Image(image::load_from_memory(&image_bytes)?)
        }
    }
    /*
    println!("{is_gif}");
    match is_gif {
        true => handle_gif(ctx, width, height, image_buffer, threshold).await,
        false => handle_image(ctx, width, height, image_buffer, threshold).await,
    }
    */
}

fn pazeify_image(grayscale_image: ImageBuffer<Luma<u8>, Vec<u8>>, threshold: u8, inverted: bool) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let width = grayscale_image.width();
    let height = grayscale_image.height();

    let mut pazeified_image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    for (x, y, pixel) in grayscale_image.enumerate_pixels() {
        let coloured_pixel = if if inverted {pixel[0] < threshold} else {pixel[0] > threshold} {
            Rgb(PAZE_YELLOW)
        } else {
            Rgb(PAZE_BLACK)
        };

        pazeified_image.put_pixel(x, y, coloured_pixel)
    }
    pazeified_image
}

async fn handle_gif(
    ctx: Context<'_>,
    mut gif: Decoder<Cursor<Vec<u8>>>,
    threshold: u8,
    inverted: bool,
) -> Result<(), Error> {
    let width = gif.width();
    let height = gif.height();

    let pazeified_file = File::create("pazeified.gif")?;
    let mut pazeified_gif = Encoder::new(
        pazeified_file,
        width,
        height,
        &[PAZE_BLACK, PAZE_YELLOW].concat(),
    )?;

    loop {
        let palette: Vec<[u8; 3]> = gif.palette()?
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect();

        let Some(frame) = gif.read_next_frame()? else { break };

        let mut frame_image = ImageBuffer::new(width as u32, height as u32);
        for (index, indice) in frame.buffer.iter().enumerate() {
            let color = palette[*indice as usize];
            let pixel = Rgb::from_slice(&color).to_luma();
            frame_image.put_pixel(index as u32 % width as u32, index as u32 / width as u32, pixel)
        }

        let pazeified_frame_image = pazeify_image(frame_image, threshold, inverted);
        let mut pazeified_frame = Frame::from_rgb(width as u16, height as u16, &pazeified_frame_image.as_bytes());
        pazeified_frame.delay = frame.delay;
        pazeified_gif.write_frame(&pazeified_frame)?;
    }

    pazeified_gif.set_repeat(gif::Repeat::Infinite)?;
    drop(pazeified_gif); // this should save the file I think ??

    ctx.send(|m| {
        m
            // .attachment(AttachmentType::Bytes { data, filename: "pazeified.png".to_string() })
            .attachment(AttachmentType::Path(Path::new("./pazeified.gif")))
            .embed(|e| e
                .title("Pazified Result")
                .image("attachment://pazeified.gif").color(0xFFC000)
            )
    })
    .await?;

    Ok(())
}

async fn handle_image(
    ctx: Context<'_>,
    image: DynamicImage,
    threshold: u8,
    inverted: bool,
) -> Result<(), Error> {
    // let image_buffer = image.as_bytes();

    // doesn't work
    // let image_type = image::guess_format(&image_buffer);
    // println!("{:#?}", image_type);
    let grayscale_image = image.to_luma8();

    let pazeified_image = pazeify_image(grayscale_image, threshold, inverted);

    pazeified_image.save("pazeified.png")?;

    ctx.send(|m| {
        m
            // .attachment(AttachmentType::Bytes { data, filename: "pazeified.png".to_string() })
            .attachment(AttachmentType::Path(Path::new("./pazeified.png")))
            .embed(|e| e.image("attachment://pazeified.png").color(0xFFC000))
    })
    .await?;

    Ok(())
}