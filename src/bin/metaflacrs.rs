extern crate flacrs;

use flacrs::{Block, Stream};
use std::fs::File;

fn main() -> std::io::Result<()> {
    let files: Vec<String> = std::env::args().skip(1).collect();

    if files.is_empty() {
        println!("usage: metaflacrs [files]");
        std::process::exit(1);
    }

    for path in &files {
        let mut file = File::open(path)?;
        let mut stream = Stream::new(&mut file)?;

        let blocks = stream.blocks()?;
        for (i, block) in blocks.iter().enumerate() {
            let (meta, block) = block;

            // Mimic the output of metaflac.
            println!("METADATA block #{}", i);
            println!(
                "  type: {}\n  is last: {}\n  length: {}",
                meta.block_type, meta.last_block, meta.block_length,
            );

            match &block {
                Block::StreamInfo(info) => {
                    println!("  minimum blocksize: {} samples", info.minimum_block_size);
                    println!("  maximum blocksize: {} samples", info.maximum_block_size);
                    println!("  minimum framesize: {} bytes", info.minimum_frame_size);
                    println!("  maximum framesize: {} bytes", info.maximum_frame_size);
                    println!("  sample_rate: {} Hz", info.sample_rate);
                    println!("  channels: {}", info.channels);
                    println!("  bits-per-sample: {}", info.bits_per_sample);
                    println!("  total samples: {}", info.total_samples);
                    println!("  MD5 signature: {}", hex_string(&info.md5_signature));
                }
                Block::VorbisComment(comment) => {
                    println!("  vendor string: {}", comment.vendor_string);
                    println!("  comments: {}", comment.user_comments.len());
                    for (j, comment) in comment.user_comments.iter().enumerate() {
                        println!("    comment[{}]: {}", j, comment);
                    }
                }
                _ => {
                    // TODO!
                }
            }
        }
    }

    Ok(())
}

fn hex_string(buf: &[u8]) -> String {
    let hex: Vec<String> = buf.iter().map(|b| format!("{:02x}", b)).collect();
    hex.join("")
}
